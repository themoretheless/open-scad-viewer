//! lib algorithm: inject_pressure_advance.
use super::*;
/// Injects firmware-specific Pressure Advance / Linear Advance setup commands into a G-code program
/// (`SET_PRESSURE_ADVANCE` for Klipper, `M900 K...` for Marlin, `M572 D0 S...` for RepRapFirmware).
pub fn inject_pressure_advance(
    gcode: &str,
    flavor: Flavor,
    k_factor: f64,
    smooth_time_s: f64,
) -> Result<String> {
    use std::fmt::Write as _;
    if !k_factor.is_finite()
        || !(0.0..=2.0).contains(&k_factor)
        || !smooth_time_s.is_finite()
        || !(0.001..=0.2).contains(&smooth_time_s)
    {
        return Err(invalid(
            "OPT_INVALID_SETTINGS",
            "Pressure advance k_factor must be in 0..=2.0 and smooth_time_s in 0.001..=0.2",
        ));
    }
    let mut out = String::with_capacity(gcode.len() + 64);
    let write_cmd = |buf: &mut String| match flavor {
        Flavor::Klipper => {
            let _ = writeln!(
                buf,
                "SET_PRESSURE_ADVANCE ADVANCE={k_factor:.5} SMOOTH_TIME={smooth_time_s:.5}"
            );
        }
        Flavor::Marlin => {
            let _ = writeln!(buf, "M900 K{k_factor:.5}");
        }
        Flavor::RepRapFirmware => {
            let _ = writeln!(buf, "M572 D0 S{k_factor:.5}");
        }
    };
    let mut injected = false;
    for line in gcode.lines() {
        out.push_str(line);
        out.push('\n');
        if !injected {
            let trimmed = line.trim_start();
            if !trimmed.is_empty() && !trimmed.starts_with(';') {
                write_cmd(&mut out);
                injected = true;
            }
        }
    }
    if !injected {
        write_cmd(&mut out);
    }
    Ok(out)
}
