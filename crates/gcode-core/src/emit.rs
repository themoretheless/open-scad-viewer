use std::fmt::{self, Write};

use crate::lex::lex_program;
use crate::plan::{path_vertices, require_layers, PlannedLayer};
use crate::profile::{ExtrusionMode, MachineProfile, Units};
use crate::{invalid, Result, DIALECT, MAX_OUTPUT_BYTES};

type FmtResult<T = ()> = std::result::Result<T, fmt::Error>;

struct Limited<W> {
    inner: W,
    used: usize,
    max: usize,
}

impl<W: Write> Write for Limited<W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.used = self.used.saturating_add(s.len());
        if self.used > self.max {
            return Err(fmt::Error);
        }
        self.inner.write_str(s)
    }
}

fn fmt_n(value: f64, digits: usize) -> String {
    format!("{value:.digits$}")
}

fn write_modes(out: &mut dyn Write, machine: &MachineProfile) -> fmt::Result {
    match machine.units {
        Units::Millimeters => writeln!(out, "G21")?,
        Units::Inches => writeln!(out, "G20")?,
    }
    if machine.xyz_absolute {
        writeln!(out, "G90")?;
    } else {
        writeln!(out, "G91")?;
    }
    match machine.e_mode {
        ExtrusionMode::RelativeFilament => writeln!(out, "M83")?,
        ExtrusionMode::AbsoluteFilament | ExtrusionMode::VolumetricMm3 => {
            writeln!(out, "M82")?;
        }
    }
    if matches!(machine.e_mode, ExtrusionMode::VolumetricMm3) {
        writeln!(out, "M200 D{}", fmt_n(machine.filament_diameter_mm, 3))?;
    }
    writeln!(out, "G92 E0")
}

fn write_heat(out: &mut dyn Write, machine: &MachineProfile) -> fmt::Result {
    if machine.hotend_c > 0.0 {
        writeln!(out, "M104 S{}", fmt_n(machine.hotend_c, 0))?;
    }
    if machine.bed_c > 0.0 {
        writeln!(out, "M140 S{}", fmt_n(machine.bed_c, 0))?;
    }
    if machine.chamber_c > 0.0 {
        writeln!(out, "M141 S{}", fmt_n(machine.chamber_c, 0))?;
    }
    if machine.hotend_c > 0.0 {
        writeln!(out, "M109 S{}", fmt_n(machine.hotend_c, 0))?;
    }
    if machine.bed_c > 0.0 {
        writeln!(out, "M190 S{}", fmt_n(machine.bed_c, 0))?;
    }
    if machine.chamber_c > 0.0 {
        writeln!(out, "M191 S{}", fmt_n(machine.chamber_c, 0))?;
    }
    if machine.fan_pwm > 0.0 {
        writeln!(out, "M106 S{}", fmt_n(machine.fan_pwm, 0))?;
    }
    Ok(())
}

fn validate_template(template: &str) -> Result<()> {
    if template.is_empty() {
        return Ok(());
    }
    lex_program(template, crate::MAX_TEMPLATE_BYTES, crate::MAX_BLOCKS)?;
    Ok(())
}

fn write_template(out: &mut dyn Write, template: &str) -> fmt::Result {
    if template.is_empty() {
        return Ok(());
    }
    write!(out, "{template}")?;
    if !template.ends_with('\n') {
        writeln!(out)?;
    }
    Ok(())
}

fn e_increment(length: f64, machine: &MachineProfile) -> f64 {
    let volume = length * machine.bead_area_mm2();
    match machine.e_mode {
        ExtrusionMode::VolumetricMm3 => volume,
        ExtrusionMode::AbsoluteFilament | ExtrusionMode::RelativeFilament => {
            volume / machine.filament_area_mm2()
        }
    }
}

fn write_e(out: &mut dyn Write, machine: &MachineProfile, e_abs: f64, delta: f64) -> fmt::Result {
    let digits = 7;
    match machine.e_mode {
        ExtrusionMode::RelativeFilament => write!(out, " E{}", fmt_n(delta, digits)),
        ExtrusionMode::AbsoluteFilament | ExtrusionMode::VolumetricMm3 => {
            write!(out, " E{}", fmt_n(e_abs, digits))
        }
    }
}

fn map_limit(err: fmt::Error) -> crate::Error {
    let _ = err;
    invalid("GCODE_OUTPUT_LIMIT", "G-code preview exceeds 4 MiB")
}

/// Serialize a completed print plan. Units follow the profile; default E is filament length.
pub fn emit(layers: &[PlannedLayer], machine: &MachineProfile) -> Result<String> {
    let mut out = String::new();
    emit_to(layers, machine, &mut out)?;
    Ok(out)
}

pub fn emit_to(
    layers: &[PlannedLayer],
    machine: &MachineProfile,
    out: &mut impl Write,
) -> Result<()> {
    machine.validate()?;
    require_layers(layers)?;
    validate_template(&machine.start_gcode)?;
    validate_template(&machine.end_gcode)?;
    let mut writer = Limited {
        inner: out,
        used: 0,
        max: MAX_OUTPUT_BYTES,
    };
    emit_body(layers, machine, &mut writer).map_err(map_limit)
}

fn emit_body(
    layers: &[PlannedLayer],
    machine: &MachineProfile,
    out: &mut dyn Write,
) -> FmtResult {
    let xyz = machine.xyz_decimals();
    let print_f = machine.print_feedrate_mm_s * 60.0;
    let travel_f = machine.travel_feedrate_mm_s * 60.0;
    let retract_f = machine.retract_feedrate_mm_s * 60.0;
    writeln!(out, "; {DIALECT}")?;
    write_modes(out, machine)?;
    write_heat(out, machine)?;
    write_template(out, &machine.start_gcode)?;
    let mut e = 0.0;
    let mut retracted = false;
    let remaining = layers
        .iter()
        .flat_map(|layer| layer.paths.iter())
        .filter(|path| !path.points.is_empty())
        .count();
    let mut finished = 0;
    for (index, layer) in layers.iter().enumerate() {
        writeln!(out, ";LAYER:{index}")?;
        writeln!(out, ";Z:{}", fmt_n(layer.z_mm, xyz))?;
        writeln!(
            out,
            "G1 Z{} F{}",
            fmt_n(layer.z_mm, xyz),
            fmt_n(travel_f, 3)
        )?;
        let z = layer.z_mm;
        for path in &layer.paths {
            if path.points.is_empty() {
                continue;
            }
            let start = path.points[0];
            writeln!(
                out,
                "G0 X{} Y{} F{}",
                fmt_n(start[0], xyz),
                fmt_n(start[1], xyz),
                fmt_n(travel_f, 3)
            )?;
            if retracted {
                if machine.z_hop_mm > 0.0 {
                    writeln!(out, "G0 Z{} F{}", fmt_n(z, xyz), fmt_n(travel_f, 3))?;
                }
                e = retract(out, machine, e, false, retract_f)?;
                retracted = false;
            }
            for [a, b] in path_vertices(path).array_windows() {
                let length = (b[0] - a[0]).hypot(b[1] - a[1]);
                let delta = e_increment(length, machine);
                e += delta;
                write!(
                    out,
                    "G1 X{} Y{}",
                    fmt_n(b[0], xyz),
                    fmt_n(b[1], xyz)
                )?;
                write_e(out, machine, e, delta)?;
                writeln!(out, " F{}", fmt_n(print_f, 3))?;
            }
            finished += 1;
            if machine.retract_mm > 0.0 && finished < remaining {
                e = retract(out, machine, e, true, retract_f)?;
                if machine.z_hop_mm > 0.0 {
                    writeln!(
                        out,
                        "G0 Z{} F{}",
                        fmt_n(z + machine.z_hop_mm, xyz),
                        fmt_n(travel_f, 3)
                    )?;
                }
                retracted = true;
            }
        }
    }
    if machine.end_gcode.is_empty() {
        writeln!(out, "M107")?;
        writeln!(out, "M104 S0")?;
        writeln!(out, "M140 S0")?;
        writeln!(out, "G28")?;
    } else {
        write_template(out, &machine.end_gcode)?;
    }
    Ok(())
}

fn retract(
    out: &mut dyn Write,
    machine: &MachineProfile,
    e: f64,
    pull: bool,
    retract_f: f64,
) -> FmtResult<f64> {
    let delta = if pull {
        -machine.retract_mm
    } else {
        machine.retract_mm
    };
    let next = e + delta;
    write!(out, "G1")?;
    write_e(out, machine, next, delta)?;
    writeln!(out, " F{}", fmt_n(retract_f, 3))?;
    Ok(next)
}
