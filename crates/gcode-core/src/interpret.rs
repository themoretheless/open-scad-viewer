use crate::block::{Block, Program};
use crate::command::{classify, layer_index, Command, MotionKind, MoveArgs};
use crate::lex::lex_program;
use crate::preview::{GcodeMove, GcodePreview, ThermalState};
use crate::profile::{ExtrusionMode, MachineProfile, Units};
use crate::{invalid, Result, DIALECT, MAX_ARC_SEGMENTS, MAX_BLOCKS, MAX_INPUT_BYTES, MAX_MOVES};

const INCH: f64 = 25.4;

struct Machine {
    x: f64,
    y: f64,
    z: f64,
    e: f64,
    f_mm_min: f64,
    abs_xyz: bool,
    abs_e: bool,
    units: Units,
    volumetric: bool,
    filament_diameter_mm: f64,
    flow: f64,
    firmware_retract_mm: f64,
    retracted: bool,
    linearize_arcs_mm: Option<f64>,
    thermal: ThermalState,
    filament_mm: f64,
    volume_mm3: f64,
    layers: usize,
    other: usize,
    moves: Vec<GcodeMove>,
    thermals: Vec<ThermalState>,
}

impl Machine {
    fn new(profile: &MachineProfile) -> Self {
        let thermal = ThermalState {
            hotend_c: 0.0,
            bed_c: 0.0,
            chamber_c: 0.0,
            fan_pwm: 0.0,
            wait: false,
            tool: 0,
        };
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            e: 0.0,
            f_mm_min: profile.print_feedrate_mm_s * 60.0,
            abs_xyz: profile.xyz_absolute,
            abs_e: !matches!(profile.e_mode, ExtrusionMode::RelativeFilament),
            units: profile.units,
            volumetric: matches!(profile.e_mode, ExtrusionMode::VolumetricMm3),
            filament_diameter_mm: profile.filament_diameter_mm,
            flow: 1.0,
            firmware_retract_mm: profile.firmware_retract_mm,
            retracted: false,
            linearize_arcs_mm: profile.linearize_arcs_mm,
            thermal,
            filament_mm: 0.0,
            volume_mm3: 0.0,
            layers: 0,
            other: 0,
            moves: Vec::new(),
            thermals: Vec::new(),
        }
    }

    fn to_mm(&self, value: f64) -> f64 {
        match self.units {
            Units::Millimeters => value,
            Units::Inches => value * INCH,
        }
    }

    fn filament_area(&self) -> f64 {
        std::f64::consts::PI * (self.filament_diameter_mm * 0.5).powi(2)
    }

    fn axis_xyz(&self, current: f64, target: Option<f64>) -> f64 {
        match target {
            None => current,
            Some(value) if self.abs_xyz => self.to_mm(value),
            Some(value) => current + self.to_mm(value),
        }
    }

    fn apply_e(&mut self, target: Option<f64>) -> Result<f64> {
        let Some(value) = target else {
            return Ok(0.0);
        };
        if !value.is_finite() {
            return Err(invalid("GCODE_SYNTAX", "E must be finite"));
        }
        let raw = if self.abs_e { value - self.e } else { value };
        self.e = if self.abs_e { value } else { self.e + value };
        Ok(raw)
    }

    fn deposit(&mut self, delta_e: f64, xy_moved: bool) {
        if delta_e <= 0.0 || !xy_moved {
            return;
        }
        let signed_volume = if self.volumetric {
            delta_e * self.flow
        } else {
            delta_e * self.filament_area() * self.flow
        };
        self.volume_mm3 += signed_volume;
        self.filament_mm += signed_volume / self.filament_area();
    }

    fn push_move(&mut self, extruded: bool) -> Result<()> {
        if self.moves.len() >= MAX_MOVES {
            return Err(invalid(
                "GCODE_MOVE_LIMIT",
                "G-code exceeded the move budget",
            ));
        }
        self.moves.push(GcodeMove {
            x: self.x,
            y: self.y,
            z: self.z,
            e: self.e,
            f_mm_min: self.f_mm_min,
            extruded,
        });
        Ok(())
    }

    fn record_thermal(&mut self, wait: bool) {
        self.thermal.wait = wait;
        self.thermals.push(self.thermal.clone());
    }

    fn motion(&mut self, kind: MotionKind, args: &MoveArgs) -> Result<()> {
        if let Some(f) = args.f {
            self.f_mm_min = f;
        }
        match kind {
            MotionKind::Rapid | MotionKind::Linear => self.linear(args),
            MotionKind::ArcCw | MotionKind::ArcCcw => {
                let Some(chord) = self.linearize_arcs_mm else {
                    return Err(invalid(
                        "GCODE_UNSUPPORTED_ARC",
                        "Arc move requires linearize_arcs_mm on the machine profile",
                    ));
                };
                self.arc(args, matches!(kind, MotionKind::ArcCw), chord)
            }
        }
    }

    fn linear(&mut self, args: &MoveArgs) -> Result<()> {
        let xy_moved = args.x.is_some() || args.y.is_some();
        self.x = self.axis_xyz(self.x, args.x);
        self.y = self.axis_xyz(self.y, args.y);
        self.z = self.axis_xyz(self.z, args.z);
        let delta_e = self.apply_e(args.e)?;
        self.deposit(delta_e, xy_moved);
        self.push_move(delta_e > 0.0 && xy_moved)
    }

    fn arc(&mut self, args: &MoveArgs, clockwise: bool, chord: f64) -> Result<()> {
        let start_x = self.x;
        let start_y = self.y;
        let start_z = self.z;
        let end_x = self.axis_xyz(self.x, args.x);
        let end_y = self.axis_xyz(self.y, args.y);
        let end_z = self.axis_xyz(self.z, args.z);
        let (cx, cy) = arc_center(start_x, start_y, end_x, end_y, args, clockwise, self)?;
        let radius = (start_x - cx).hypot(start_y - cy);
        if radius <= 1e-12 {
            return Err(invalid("GCODE_ARC", "Arc radius must be positive"));
        }
        let start_a = (start_y - cy).atan2(start_x - cx);
        let mut end_a = (end_y - cy).atan2(end_x - cx);
        let two_pi = std::f64::consts::TAU;
        if clockwise {
            while end_a >= start_a {
                end_a -= two_pi;
            }
        } else {
            while end_a <= start_a {
                end_a += two_pi;
            }
        }
        let sweep = end_a - start_a;
        let chord = chord.max(1e-6);
        let ratio = (1.0 - chord / radius).clamp(-1.0, 1.0);
        let step = (2.0 * ratio.acos()).max(1e-3);
        let segments = ((sweep.abs() / step).ceil() as usize).clamp(1, MAX_ARC_SEGMENTS);
        let delta_e = self.apply_e(args.e)?;
        let xy_moved = (end_x - start_x).hypot(end_y - start_y) > 1e-12 || sweep.abs() > 1e-12;
        self.deposit(delta_e, xy_moved);
        for i in 1..=segments {
            let t = i as f64 / segments as f64;
            let a = start_a + sweep * t;
            if i == segments {
                self.x = end_x;
                self.y = end_y;
                self.z = end_z;
            } else {
                self.x = cx + radius * a.cos();
                self.y = cy + radius * a.sin();
                self.z = start_z + (end_z - start_z) * t;
            }
            self.push_move(delta_e > 0.0 && xy_moved)?;
        }
        Ok(())
    }

    fn firmware_retract(&mut self, recover: bool) -> Result<()> {
        let delta = if recover {
            if !self.retracted {
                return Ok(());
            }
            self.retracted = false;
            self.firmware_retract_mm
        } else {
            if self.retracted {
                return Ok(());
            }
            self.retracted = true;
            -self.firmware_retract_mm
        };
        self.e += delta;
        self.push_move(false)
    }
}

fn arc_center(
    x: f64,
    y: f64,
    end_x: f64,
    end_y: f64,
    args: &MoveArgs,
    clockwise: bool,
    machine: &Machine,
) -> Result<(f64, f64)> {
    if args.i.is_some() || args.j.is_some() {
        return Ok((
            x + machine.to_mm(args.i.unwrap_or(0.0)),
            y + machine.to_mm(args.j.unwrap_or(0.0)),
        ));
    }
    let Some(r) = args.r else {
        return Err(invalid(
            "GCODE_ARC",
            "Arc move requires I/J offsets or R",
        ));
    };
    let radius = machine.to_mm(r.abs());
    let dx = end_x - x;
    let dy = end_y - y;
    let d = dx.hypot(dy);
    if d < 1e-12 {
        return Err(invalid("GCODE_ARC", "Arc chord length must be positive"));
    }
    let h2 = radius * radius - (d * 0.5).powi(2);
    if h2 < -1e-9 {
        return Err(invalid("GCODE_ARC", "Arc radius is shorter than the chord"));
    }
    let h = h2.max(0.0).sqrt();
    let mx = (x + end_x) * 0.5;
    let my = (y + end_y) * 0.5;
    let ux = -dy / d;
    let uy = dx / d;
    let sign = if clockwise == (r >= 0.0) { 1.0 } else { -1.0 };
    Ok((mx + sign * h * ux, my + sign * h * uy))
}

fn interpret_block(machine: &mut Machine, block: &Block) -> Result<()> {
    if let Some(index) = layer_index(block) {
        machine.layers = machine.layers.max(index + 1);
    }
    match classify(block).map_err(|_| {
        invalid(
            "GCODE_UNSUPPORTED",
            "G-code contains a CNC command this FDM library does not execute",
        )
    })? {
        Command::Motion { kind, args } => machine.motion(kind, &args)?,
        Command::Dwell { .. } => {}
        Command::FirmwareRetract => machine.firmware_retract(false)?,
        Command::FirmwareRecover => machine.firmware_retract(true)?,
        Command::PlaneXy => {}
        Command::UnitsInch => machine.units = Units::Inches,
        Command::UnitsMm => machine.units = Units::Millimeters,
        Command::Home { x, y, z } => {
            let all = !x && !y && !z;
            if all || x {
                machine.x = 0.0;
            }
            if all || y {
                machine.y = 0.0;
            }
            if all || z {
                machine.z = 0.0;
            }
            machine.push_move(false)?;
        }
        Command::AbsoluteXyz => machine.abs_xyz = true,
        Command::RelativeXyz => machine.abs_xyz = false,
        Command::SetPosition { x, y, z, e } => {
            if let Some(v) = x {
                machine.x = machine.to_mm(v);
            }
            if let Some(v) = y {
                machine.y = machine.to_mm(v);
            }
            if let Some(v) = z {
                machine.z = machine.to_mm(v);
            }
            if let Some(v) = e {
                machine.e = v;
            }
        }
        Command::Hotend { c, wait, tool } => {
            if let Some(tool) = tool {
                machine.thermal.tool = tool;
            }
            machine.thermal.hotend_c = c;
            machine.record_thermal(wait);
        }
        Command::Bed { c, wait } => {
            machine.thermal.bed_c = c;
            machine.record_thermal(wait);
        }
        Command::Chamber { c, wait } => {
            machine.thermal.chamber_c = c;
            machine.record_thermal(wait);
        }
        Command::Fan { pwm } => {
            machine.thermal.fan_pwm = pwm;
            machine.record_thermal(false);
        }
        Command::ExtrusionAbsolute => machine.abs_e = true,
        Command::ExtrusionRelative => machine.abs_e = false,
        Command::MotorsOff => {}
        Command::Volumetric { diameter_mm } => match diameter_mm {
            Some(d) if d > 0.0 => {
                machine.volumetric = true;
                machine.filament_diameter_mm = d;
            }
            _ => machine.volumetric = false,
        },
        Command::SpeedFactor { .. } => {}
        Command::FlowFactor { percent } => {
            machine.flow = (percent / 100.0).max(0.0);
        }
        Command::Tool { index } => {
            machine.thermal.tool = index;
            machine.record_thermal(false);
        }
        Command::Comment => {}
        Command::Other { .. } => machine.other += 1,
    }
    Ok(())
}

pub fn interpret_blocks(blocks: &[Block], profile: &MachineProfile) -> Result<GcodePreview> {
    profile.validate()?;
    let mut machine = Machine::new(profile);
    for block in blocks {
        interpret_block(&mut machine, block)?;
    }
    if machine.layers == 0 {
        let mut zs = machine
            .moves
            .iter()
            .map(|m| (m.z * 1e5).round() as i64)
            .collect::<Vec<_>>();
        zs.sort_unstable();
        zs.dedup();
        machine.layers = zs.len();
    }
    Ok(GcodePreview {
        layers: machine.layers,
        extrusion_mm: machine.filament_mm,
        deposited_volume_mm3: machine.volume_mm3,
        moves: machine.moves,
        thermal: machine.thermal,
        thermals: machine.thermals,
        other_commands: machine.other,
    })
}

pub fn parse_program(gcode: &str) -> Result<Program> {
    lex_program(gcode, MAX_INPUT_BYTES, MAX_BLOCKS)
}

/// Independent parse of exported preview G-code. Physical print is out of scope.
pub fn parse(gcode: &str) -> Result<GcodePreview> {
    if !gcode.starts_with(&format!("; {DIALECT}")) {
        return Err(invalid(
            "GCODE_DIALECT",
            "Not an open-scad-viewer print preview",
        ));
    }
    parse_fdm(gcode)
}

pub fn parse_fdm(gcode: &str) -> Result<GcodePreview> {
    parse_fdm_with(gcode, &MachineProfile::default())
}

pub fn parse_fdm_with(gcode: &str, profile: &MachineProfile) -> Result<GcodePreview> {
    interpret_blocks(&parse_program(gcode)?.blocks, profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relative_e_volume_uses_deltas() {
        let gcode = "G21\nG90\nM83\nG1 X10 E2\nG1 X20 E2\n";
        let preview = parse_fdm(gcode).unwrap();
        let area = MachineProfile::default().filament_area_mm2();
        assert!((preview.extrusion_mm - 4.0).abs() < 1e-9);
        assert!((preview.deposited_volume_mm3 - 4.0 * area).abs() < 1e-9);
    }

    #[test]
    fn records_hotend_bed_and_fan() {
        let gcode = "M104 S210\nM140 S60\nM109 S210\nM190 S60\nM106 S128\nG1 X1 E0.1\n";
        let preview = parse_fdm(gcode).unwrap();
        assert_eq!(preview.thermal.hotend_c, 210.0);
        assert_eq!(preview.thermal.bed_c, 60.0);
        assert_eq!(preview.thermal.fan_pwm, 128.0);
        assert!(preview.thermals.iter().any(|t| t.wait && t.hotend_c == 210.0));
    }

    #[test]
    fn relative_xyz_accumulates() {
        let preview = parse_fdm("G21\nG91\nG1 X10 Y0\nG1 X10 Y0\n").unwrap();
        let last = preview.moves.last().unwrap();
        assert!((last.x - 20.0).abs() < 1e-9);
    }

    #[test]
    fn g92_resets_absolute_e_without_dropping_volume() {
        let preview = parse_fdm("G90\nM82\nG1 X10 E5\nG92 E0\nG1 X20 E2\n").unwrap();
        assert!((preview.extrusion_mm - 7.0).abs() < 1e-9);
    }

    #[test]
    fn volumetric_e_is_mm3() {
        let preview = parse_fdm("G90\nM82\nM200 D1.75\nG1 X10 E2\n").unwrap();
        assert!((preview.deposited_volume_mm3 - 2.0).abs() < 1e-9);
    }
}
