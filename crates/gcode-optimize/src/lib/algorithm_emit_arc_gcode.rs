//! lib algorithm: emit_arc_gcode.
use super::*;
/// Emits G-code with `G2` / `G3` circular arcs (`I` / `J` center offsets) and optional
/// `Retract` + `Z-hop` travel protection between paths.
pub fn emit_arc_gcode(
    layers: &[PlannedLayer],
    machine: &MachineProfile,
    arc_config: &ArcWelderConfig,
    retract_zhop: Option<&RetractZHopConfig>,
) -> Result<String> {
    machine.validate()?;
    let filament_area =
        std::f64::consts::PI * (0.5 * machine.filament_diameter_mm).powi(2);
    let extrusion_per_mm =
        (machine.layer_height_mm * machine.line_width_mm) / filament_area.max(1e-9);
    let print_f = machine.print_feedrate_mm_s * 60.0;
    let travel_f = machine.travel_feedrate_mm_s * 60.0;

    let mut out = String::from("G90\nM82\nG92 E0\n");
    let mut e_pos = 0.0_f64;
    let mut current_xy: Option<[f64; 2]> = None;

    for layer in layers {
        let z = layer.z_mm;
        for path in &layer.paths {
            let segments = arc_fit_polyline(&path.points, path.closed, arc_config);
            let Some(first) = segments.first() else {
                continue;
            };
            let start = match *first {
                FittedSegment::Linear { start, .. } | FittedSegment::Arc { start, .. } => start,
            };
            let travel_dist = current_xy
                .map(|prev| (start[0] - prev[0]).hypot(start[1] - prev[1]))
                .unwrap_or(0.0);
            if let Some(rz) = retract_zhop
                && current_xy.is_some()
                && travel_dist >= rz.min_travel_mm
            {
                let retract_f = rz.retract_speed_mm_s * 60.0;
                if rz.retract_length_mm > 0.0 {
                    e_pos -= rz.retract_length_mm;
                    out.push_str(&format!("G1 E{e_pos:.5} F{retract_f:.1}\n"));
                }
                if rz.z_hop_mm > 0.0 {
                    let z_lift = z + rz.z_hop_mm;
                    out.push_str(&format!("G0 Z{z_lift:.3} F{travel_f:.1}\n"));
                }
                out.push_str(&format!(
                    "G0 X{:.3} Y{:.3} F{travel_f:.1}\n",
                    start[0], start[1]
                ));
                out.push_str(&format!("G0 Z{z:.3} F{travel_f:.1}\n"));
                if rz.retract_length_mm > 0.0 {
                    e_pos += rz.retract_length_mm;
                    out.push_str(&format!("G1 E{e_pos:.5} F{retract_f:.1}\n"));
                }
            } else {
                out.push_str(&format!(
                    "G0 X{:.3} Y{:.3} Z{z:.3} F{travel_f:.1}\n",
                    start[0], start[1]
                ));
            }
            for seg in segments {
                match seg {
                    FittedSegment::Linear { start: a, end: b } => {
                        let len = (b[0] - a[0]).hypot(b[1] - a[1]);
                        if len > 1e-9 {
                            e_pos += len * extrusion_per_mm;
                            out.push_str(&format!(
                                "G1 X{:.3} Y{:.3} E{e_pos:.5} F{print_f:.1}\n",
                                b[0], b[1]
                            ));
                        }
                        current_xy = Some(b);
                    }
                    FittedSegment::Arc {
                        start: a,
                        end: b,
                        ij_offset: [i_off, j_off],
                        radius,
                        clockwise,
                    } => {
                        let chord = (b[0] - a[0]).hypot(b[1] - a[1]);
                        let angle = 2.0 * (0.5 * chord / radius.max(1e-6)).clamp(-1.0, 1.0).asin();
                        let arc_len = radius * angle;
                        e_pos += arc_len * extrusion_per_mm;
                        let cmd = if clockwise { "G2" } else { "G3" };
                        out.push_str(&format!(
                            "{cmd} X{:.3} Y{:.3} I{i_off:.3} J{j_off:.3} E{e_pos:.5} F{print_f:.1}\n",
                            b[0], b[1]
                        ));
                        current_xy = Some(b);
                    }
                }
            }
        }
    }
    Ok(out)
}
