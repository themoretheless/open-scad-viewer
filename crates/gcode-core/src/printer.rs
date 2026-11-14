//! Machine-specific printer profiles stored as `key = value` data files, plus
//! job emission that honours the bed volume, firmware dialect and custom
//! start/end G-code. Per-segment extrusion widths cover thin walls.

use crate::job::emit_job_widths;
use crate::{
    BoundedOutput, Flavor, JobProfile, MAX_LINE_BYTES, MachineProfile, PlannedLayer, Result,
    estimated_output_bytes, invalid, output_limit, rounded_coordinate, valid_dimension,
};

const BUILTIN_SOURCES: [&str; 4] = [
    include_str!("../profiles/creality-ender3-v2.printer"),
    include_str!("../profiles/prusa-mk3s.printer"),
    include_str!("../profiles/voron-2.4-350.printer"),
    include_str!("../profiles/railcore-ii-300zl.printer"),
];

/// A concrete FDM machine: build volume, nozzle, firmware and job defaults.
#[derive(Clone, Debug, PartialEq)]
pub struct PrinterProfile {
    pub id: String,
    pub name: String,
    /// Printable volume from the origin, X/Y/Z in mm.
    pub bed_size_mm: [f64; 3],
    pub nozzle_diameter_mm: f64,
    /// Emitted after the flavor prologue, before the first layer.
    pub start_gcode: Vec<String>,
    /// Emitted after the last layer, before the flavor heater shutdown.
    pub end_gcode: Vec<String>,
    /// Speeds, retraction, temperatures, flavor and extrusion geometry.
    pub job: JobProfile,
}

impl PrinterProfile {
    /// Parse and validate one profile in the `key = value` data format.
    pub fn parse(source: &str) -> Result<PrinterProfile> {
        let mut id = None;
        let mut name = None;
        let mut flavor = None;
        let mut bed = [None; 3];
        let mut nozzle = None;
        let mut start = Vec::new();
        let mut end = Vec::new();
        let mut job = JobProfile {
            home_axes: true,
            ..JobProfile::default()
        };
        let mut seen_numbers = 0u32;
        for raw in source.lines() {
            let line = raw.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .map(|(k, v)| (k.trim(), v.trim()))
                .ok_or_else(|| profile_error("Profile lines must be `key = value`"))?;
            let num = || -> Result<f64> {
                value
                    .parse::<f64>()
                    .ok()
                    .filter(|v| v.is_finite())
                    .ok_or_else(|| profile_error("Profile numbers must be finite decimals"))
            };
            let mut count = || seen_numbers += 1;
            match key {
                "id" => id = Some(value.to_string()),
                "name" => name = Some(value.to_string()),
                "flavor" => flavor = Some(Flavor::from_name(value)?),
                "start_gcode" => start.push(value.to_string()),
                "end_gcode" => end.push(value.to_string()),
                "bed_x_mm" => bed[0] = Some(num()?),
                "bed_y_mm" => bed[1] = Some(num()?),
                "bed_z_mm" => bed[2] = Some(num()?),
                "nozzle_diameter_mm" => nozzle = Some(num()?),
                "filament_diameter_mm" => {
                    job.machine.filament_diameter_mm = num()?;
                    count()
                }
                "layer_height_mm" => {
                    job.machine.layer_height_mm = num()?;
                    count()
                }
                "line_width_mm" => {
                    job.machine.line_width_mm = num()?;
                    count()
                }
                "print_speed_mm_s" => {
                    job.machine.print_feedrate_mm_s = num()?;
                    count()
                }
                "travel_speed_mm_s" => {
                    job.machine.travel_feedrate_mm_s = num()?;
                    count()
                }
                "retract_length_mm" => {
                    job.retract_length_mm = num()?;
                    count()
                }
                "retract_speed_mm_s" => {
                    let speed = num()?;
                    job.retract_feedrate_mm_s = speed;
                    job.unretract_feedrate_mm_s = speed;
                    count()
                }
                "retract_min_travel_mm" => job.retract_min_travel_mm = num()?,
                "nozzle_temp_c" => {
                    job.nozzle_temp_c = num()?;
                    count()
                }
                "bed_temp_c" => {
                    job.bed_temp_c = num()?;
                    count()
                }
                "fan_speed" => {
                    let fan = num()?;
                    if !(0.0..=255.0).contains(&fan) || fan.fract() != 0.0 {
                        return Err(profile_error("fan_speed must be an integer 0..255"));
                    }
                    job.fan_speed = fan as u8;
                }
                _ => return Err(profile_error("Unknown profile key")),
            }
        }
        // Every machine-specific number must be stated, not inherited from defaults.
        if seen_numbers != 9 {
            return Err(profile_error(
                "Profile must set filament, layer, width, speeds, retraction and temperatures exactly once",
            ));
        }
        let (Some(id), Some(name), Some(flavor), Some(x), Some(y), Some(z), Some(nozzle)) =
            (id, name, flavor, bed[0], bed[1], bed[2], nozzle)
        else {
            return Err(profile_error(
                "Profile needs id, name, flavor, bed_x/y/z_mm and nozzle_diameter_mm",
            ));
        };
        job.flavor = flavor;
        let profile = PrinterProfile {
            id,
            name,
            bed_size_mm: [x, y, z],
            nozzle_diameter_mm: nozzle,
            start_gcode: start,
            end_gcode: end,
            job,
        };
        profile.validate()?;
        Ok(profile)
    }

    pub fn validate(&self) -> Result<()> {
        if self.id.is_empty()
            || !self
                .id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.' || b == b'_')
        {
            return Err(profile_error("Profile id must be non-empty [A-Za-z0-9._-]"));
        }
        if self.name.is_empty() {
            return Err(profile_error("Profile name must not be empty"));
        }
        if self
            .bed_size_mm
            .iter()
            .any(|&v| !(v.is_finite() && (10.0..=2000.0).contains(&v)))
        {
            return Err(profile_error("Bed size must be 10..2000 mm on every axis"));
        }
        if !(self.nozzle_diameter_mm.is_finite() && (0.1..=2.0).contains(&self.nozzle_diameter_mm))
        {
            return Err(profile_error("Nozzle diameter must be 0.1..2 mm"));
        }
        self.job.validate()?;
        let machine = &self.job.machine;
        if !(self.nozzle_diameter_mm * 0.75..=self.nozzle_diameter_mm * 2.0)
            .contains(&machine.line_width_mm)
        {
            return Err(profile_error(
                "Line width must be 0.75..2 times the nozzle diameter",
            ));
        }
        if machine.layer_height_mm > self.nozzle_diameter_mm * 0.8 {
            return Err(profile_error(
                "Layer height must not exceed 80% of the nozzle diameter",
            ));
        }
        for line in self.start_gcode.iter().chain(&self.end_gcode) {
            admit_custom_line(line)?;
        }
        Ok(())
    }

    /// Machine extrusion geometry (layer height, width, speeds, filament).
    pub fn machine(&self) -> &MachineProfile {
        &self.job.machine
    }
}

/// Custom lines are printable ASCII commands that do not touch the E axis,
/// so the emitter's absolute-E bookkeeping and parse-back stay valid.
fn admit_custom_line(line: &str) -> Result<()> {
    let command = line.split(';').next().unwrap_or("").trim();
    if line.len() > MAX_LINE_BYTES
        || !line.bytes().all(|b| (b' '..=b'~').contains(&b))
        || command.is_empty()
    {
        return Err(profile_error(
            "Custom G-code lines must be non-empty printable ASCII commands",
        ));
    }
    let mut tokens = command.split_whitespace();
    let head = tokens.next().unwrap_or("").to_ascii_uppercase();
    let touches_e = tokens.any(|word| word.starts_with('E') || word.starts_with('e'));
    if head == "G92" || (matches!(head.as_str(), "G0" | "G1" | "G2" | "G3") && touches_e) {
        return Err(profile_error(
            "Custom G-code must not reset or move the extruder axis",
        ));
    }
    Ok(())
}

fn profile_error(message: &str) -> crate::Error {
    invalid("GCODE_PRINTER_PROFILE", message)
}

/// All built-in profiles, parsed and validated from their data files.
pub fn builtin_printer_profiles() -> Result<Vec<PrinterProfile>> {
    BUILTIN_SOURCES
        .iter()
        .map(|s| PrinterProfile::parse(s))
        .collect()
}

/// Ids of the built-in profiles in a stable order.
pub fn builtin_printer_ids() -> Vec<String> {
    builtin_printer_profiles()
        .map(|list| list.into_iter().map(|p| p.id).collect())
        .unwrap_or_default()
}

/// Select a built-in profile by id.
pub fn printer_profile(id: &str) -> Result<PrinterProfile> {
    builtin_printer_profiles()?
        .into_iter()
        .find(|profile| profile.id == id)
        .ok_or_else(|| invalid("GCODE_PRINTER_UNKNOWN", "Unknown printer profile id"))
}

/// Every extruding point and layer Z must lie inside the printer's build volume.
fn require_bed_bounds(layers: &[PlannedLayer], profile: &PrinterProfile) -> Result<()> {
    let [bx, by, bz] = profile.bed_size_mm;
    for layer in layers {
        let z = rounded_coordinate(layer.z_mm);
        if !(z > 0.0 && z <= bz) {
            return Err(invalid(
                "GCODE_BED_BOUNDS",
                "Layer Z is outside the printer build height",
            ));
        }
        for path in &layer.paths {
            for p in &path.points {
                let (x, y) = (rounded_coordinate(p[0]), rounded_coordinate(p[1]));
                if !(0.0..=bx).contains(&x) || !(0.0..=by).contains(&y) {
                    return Err(invalid(
                        "GCODE_BED_BOUNDS",
                        "Toolpath leaves the printer bed; place the part on the bed first",
                    ));
                }
            }
        }
    }
    Ok(())
}

fn segment_count(points: usize, closed: bool) -> usize {
    points.saturating_sub(1) + usize::from(closed && points >= 3)
}

/// Check a per-segment width table against the plan.
fn require_widths(
    layers: &[PlannedLayer],
    widths: &[Vec<Vec<f64>>],
    machine: &MachineProfile,
) -> Result<()> {
    if widths.is_empty() {
        return Ok(());
    }
    let shape_error = || {
        invalid(
            "GCODE_INVALID_WIDTH",
            "Width table must give one width per segment of every path",
        )
    };
    if widths.len() != layers.len() {
        return Err(shape_error());
    }
    for (layer, layer_widths) in layers.iter().zip(widths) {
        if layer_widths.len() != layer.paths.len() {
            return Err(shape_error());
        }
        for (path, list) in layer.paths.iter().zip(layer_widths) {
            if list.len() != segment_count(path.points.len(), path.closed) {
                return Err(shape_error());
            }
            if list
                .iter()
                .any(|&w| !valid_dimension(w) || w > machine.line_width_mm * 4.0)
            {
                return Err(invalid(
                    "GCODE_INVALID_WIDTH",
                    "Segment widths must be positive and at most 4x the line width",
                ));
            }
        }
    }
    Ok(())
}

/// Filament length for one bead: `width * height * length / filament_area`.
pub fn extrusion_length_mm(
    width_mm: f64,
    layer_height_mm: f64,
    length_mm: f64,
    filament_diameter_mm: f64,
) -> f64 {
    let r = filament_diameter_mm * 0.5;
    width_mm * layer_height_mm * length_mm / (std::f64::consts::PI * r * r)
}

/// Job with per-segment extrusion widths and no printer-specific envelope.
/// `widths[layer][path][segment]`; an empty table means nominal width.
pub fn emit_job_variable_width(
    layers: &[PlannedLayer],
    widths: &[Vec<Vec<f64>>],
    job: &JobProfile,
) -> Result<String> {
    job.validate()?;
    require_widths(layers, widths, &job.machine)?;
    let mut out = BoundedOutput(String::with_capacity(estimated_output_bytes(layers)));
    emit_job_widths(layers, widths, job, &mut out)?;
    Ok(out.0)
}

/// Machine job for a concrete printer: bed bounds checked, flavor dialect,
/// custom start G-code after the prologue and end G-code before cooldown.
pub fn emit_printer_job(
    layers: &[PlannedLayer],
    widths: &[Vec<Vec<f64>>],
    profile: &PrinterProfile,
) -> Result<String> {
    profile.validate()?;
    require_bed_bounds(layers, profile)?;
    let body = emit_job_variable_width(layers, widths, &profile.job)?;
    let flavor = profile.job.flavor;
    let prologue_end = flavor.prologue().last().copied().unwrap_or("G92 E0");
    let shutdown_start = flavor.shutdown_commands()[0];
    let lines: Vec<&str> = body.lines().collect();
    let start_at = lines
        .iter()
        .position(|l| *l == prologue_end)
        .map(|i| i + 1)
        .ok_or_else(|| invalid("GCODE_NUMERIC", "Job prologue missing"))?;
    let end_at = lines
        .iter()
        .rposition(|l| *l == shutdown_start)
        .ok_or_else(|| invalid("GCODE_NUMERIC", "Job shutdown missing"))?;
    let mut out = BoundedOutput(String::with_capacity(body.len() + 256));
    use std::fmt::Write;
    let mut emit = |line: &str| writeln!(out, "{line}").map_err(output_limit);
    for (index, line) in lines.iter().enumerate() {
        if index == 1 {
            emit(&format!(";PRINTER:{}", profile.id))?;
        }
        if index == start_at {
            emit(";START_GCODE")?;
            for custom in &profile.start_gcode {
                emit(custom)?;
            }
        }
        if index == end_at {
            emit(";END_GCODE")?;
            for custom in &profile.end_gcode {
                emit(custom)?;
            }
        }
        emit(line)?;
    }
    Ok(out.0)
}
