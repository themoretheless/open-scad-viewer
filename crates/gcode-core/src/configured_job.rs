//! Additional export modes keep the strict print-job 1 API unchanged.
use crate::foreign_words::{self, Command};
use crate::{
    Flavor, JobProfile, MAX_LINE_BYTES, MAX_OUTPUT_BYTES, PlannedLayer, Result, emit_job, invalid,
};
use std::fmt::Write;
#[derive(Clone, Debug, Default)]
pub struct JobOutputOptions {
    pub inches: bool,
    pub relative_xyz: bool,
    pub relative_e: bool,
    pub start_template: String,
    pub z_hop_mm: f64,
    pub chamber_temp_c: Option<f64>,
}
/// Emit a foreign-preview-readable configured job, resetting modal state after the template.
pub fn emit_configured_job(
    layers: &[PlannedLayer],
    job: &JobProfile,
    options: &JobOutputOptions,
) -> Result<String> {
    configure_job_gcode(&emit_job(layers, job)?, job, options)
}

/// Convert a strict job after independently validating its native dialect.
pub fn configure_job_gcode(
    canonical: &str,
    job: &JobProfile,
    options: &JobOutputOptions,
) -> Result<String> {
    job.validate()?;
    if crate::job_flavor(canonical)? != job.flavor {
        return Err(invalid(
            "GCODE_FLAVOR",
            "Configured job profile does not match the source flavor",
        ));
    }
    crate::parse_job(canonical)?;
    if !options.z_hop_mm.is_finite() || !(0.0..=100.0).contains(&options.z_hop_mm) {
        return Err(invalid(
            "GCODE_INVALID_SETTINGS",
            "Z hop must be within 0..100 mm",
        ));
    }
    if options.start_template.as_bytes().contains(&0) {
        return Err(invalid("GCODE_SYNTAX", "Start template contains NUL"));
    }
    if options.start_template.len() > 64 * 1024
        || options
            .start_template
            .lines()
            .any(|l| l.len() > MAX_LINE_BYTES)
    {
        return Err(invalid("GCODE_LIMIT", "Start template exceeds bounds"));
    }
    if let Some(t) = options.chamber_temp_c {
        if !t.is_finite() || !(0.0..=150.0).contains(&t) {
            return Err(invalid(
                "GCODE_INVALID_SETTINGS",
                "Chamber target must be within 0..150 C",
            ));
        }
        if job.flavor != Flavor::Marlin {
            return Err(invalid(
                "GCODE_FLAVOR",
                "Chamber export currently requires Marlin M141/M191",
            ));
        }
    }
    if options.inches && job.flavor == Flavor::Klipper {
        return Err(invalid(
            "GCODE_FLAVOR",
            "Klipper does not support inch output",
        ));
    }
    let mut out = String::new();
    let mut position = [0.0; 4];
    let mut known = [false; 3];
    let mut started = false;
    let mut templated = false;
    let mut z = 0.0;
    let scale = if options.inches { 25.4 } else { 1.0 };
    for raw in canonical.lines() {
        if raw.starts_with("; open-scad-viewer/print-job ") {
            writeln!(out, "; open-scad-viewer/configured-job 1").unwrap();
            continue;
        }
        let code = foreign_words::code(raw);
        let mut tokens = foreign_words::words(&code);
        let command = tokens.next().map(foreign_words::command);
        if !templated && !code.trim().is_empty() {
            out.push_str(&options.start_template);
            if !options.start_template.ends_with('\n') {
                out.push('\n')
            }
            out.push_str("G21\nG90\nM82\n");
            templated = true;
        }
        if command == Some(Command::G(21)) && !started {
            if let Some(target) = options.chamber_temp_c {
                writeln!(out, "M141 S{target}\nM191 S{target}").unwrap()
            }
            writeln!(
                out,
                "{}\nG90\n{}\nG92 E0",
                if options.inches { "G20" } else { "G21" },
                if options.relative_e { "M83" } else { "M82" }
            )
            .unwrap();
            started = true;
            continue;
        }
        if started
            && matches!(
                command,
                Some(Command::G(90) | Command::M(82) | Command::G(92))
            )
        {
            continue;
        }
        if started && matches!(command, Some(Command::G(0 | 1))) {
            let mut values = Vec::new();
            for token in tokens {
                let key = token.as_bytes()[0].to_ascii_uppercase();
                let value = crate::number(&token[1..])?;
                values.push((key, value));
            }
            let absolute = options.relative_xyz
                && values.iter().any(|(key, _)| {
                    matches!(*key, b'X' | b'Y' | b'Z')
                        && !known[match key {
                            b'X' => 0,
                            b'Y' => 1,
                            _ => 2,
                        }]
                });
            if options.relative_xyz {
                writeln!(out, "{}", if absolute { "G90" } else { "G91" }).unwrap();
                writeln!(out, "{}", if options.relative_e { "M83" } else { "M82" }).unwrap();
            }
            if command == Some(Command::G(0)) && options.z_hop_mm > 0.0 && known[2] {
                let lifted = z + options.z_hop_mm;
                if lifted > crate::MAX_COORDINATE_MM {
                    return Err(invalid(
                        "GCODE_INVALID_SETTINGS",
                        "Z hop exceeds coordinate bounds",
                    ));
                }
                writeln!(
                    out,
                    "G1 Z{:.8} F{:.6}",
                    if options.relative_xyz && !absolute {
                        options.z_hop_mm / scale
                    } else {
                        lifted / scale
                    },
                    job.machine.travel_feedrate_mm_s * 60.0 / scale
                )
                .unwrap();
            }
            write!(
                out,
                "{}",
                if command == Some(Command::G(0)) {
                    "G0"
                } else {
                    "G1"
                }
            )
            .unwrap();
            for (key, value) in values {
                let axis = match key {
                    b'X' => Some(0),
                    b'Y' => Some(1),
                    b'Z' => Some(2),
                    b'E' => Some(3),
                    _ => None,
                };
                let converted = if let Some(axis) = axis {
                    let relative = if axis == 3 {
                        options.relative_e
                    } else {
                        options.relative_xyz && !absolute
                    };
                    let converted = if relative {
                        value - position[axis]
                    } else {
                        value
                    };
                    position[axis] = value;
                    if axis < 3 {
                        known[axis] = true
                    }
                    if axis == 2 {
                        z = value
                    }
                    converted / scale
                } else if key == b'F' {
                    value / scale
                } else {
                    value
                };
                write!(out, " {}{:.8}", key as char, converted).unwrap();
            }
            out.push('\n');
            if command == Some(Command::G(0)) && options.z_hop_mm > 0.0 && known[2] {
                writeln!(
                    out,
                    "G1 Z{:.8} F{:.6}",
                    if options.relative_xyz && !absolute {
                        -options.z_hop_mm / scale
                    } else {
                        z / scale
                    },
                    job.machine.travel_feedrate_mm_s * 60.0 / scale
                )
                .unwrap()
            }
        } else {
            writeln!(out, "{raw}").unwrap()
        }
        if out.len() > MAX_OUTPUT_BYTES {
            return Err(invalid("GCODE_LIMIT", "Configured job exceeds 4 MiB"));
        }
    }
    Ok(out)
}
