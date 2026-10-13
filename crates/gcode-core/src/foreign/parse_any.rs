use super::*;

pub(super) fn arc_points(
    start: [f64; 3],
    end: [f64; 3],
    i: Option<f64>,
    j: Option<f64>,
    r: Option<f64>,
    counter_clockwise: bool,
) -> Result<Vec<[f64; 3]>> {
    let (cx, cy) = if i.is_some() || j.is_some() {
        (start[0] + i.unwrap_or(0.0), start[1] + j.unwrap_or(0.0))
    } else if let Some(r) = r {
        let dx = end[0] - start[0];
        let dy = end[1] - start[1];
        let d = dx.hypot(dy);
        if d == 0.0 || !r.is_finite() || r.abs() * 2.0 < d - 1e-9 {
            return Err(invalid(
                "GCODE_INVALID_COORDINATE",
                "Arc radius cannot reach the end point",
            ));
        }
        let h = (r * r - (d / 2.0).powi(2)).max(0.0).sqrt();
        let sign = if counter_clockwise == (r > 0.0) {
            1.0
        } else {
            -1.0
        };
        (
            (start[0] + end[0]) / 2.0 - sign * h * dy / d,
            (start[1] + end[1]) / 2.0 + sign * h * dx / d,
        )
    } else {
        return Err(invalid(
            "GCODE_SYNTAX",
            "Arc needs I/J offsets or an R radius",
        ));
    };
    let radius = (start[0] - cx).hypot(start[1] - cy);
    if !radius.is_finite() || radius == 0.0 {
        return Err(invalid("GCODE_INVALID_COORDINATE", "Arc has zero radius"));
    }
    let a0 = (start[1] - cy).atan2(start[0] - cx);
    let a1 = (end[1] - cy).atan2(end[0] - cx);
    let mut sweep = a1 - a0;
    let full = std::f64::consts::TAU;
    if counter_clockwise {
        if sweep <= 1e-9 {
            sweep += full;
        }
    } else if sweep >= -1e-9 {
        sweep -= full;
    }
    let segments =
        ((radius * sweep.abs() / ARC_SEGMENT_MM).ceil() as usize).clamp(1, MAX_ARC_SEGMENTS);
    let mut points = Vec::with_capacity(segments);
    for step in 1..=segments {
        let t = step as f64 / segments as f64;
        let angle = a0 + sweep * t;
        points.push([
            cx + radius * angle.cos(),
            cy + radius * angle.sin(),
            start[2] + (end[2] - start[2]) * t,
        ]);
    }
    if let Some(last) = points.last_mut() {
        *last = end;
    }
    Ok(points)
}

/// Dispatches by dialect: native preview/job stay strict; everything else is tolerant.
pub fn parse_any(gcode: &str) -> Result<(GcodePreview, DialectInfo)> {
    let info = detect(gcode);
    let first = gcode.lines().next().unwrap_or("").trim();
    let preview = if first == format!("; {}", crate::JOB_DIALECT) {
        crate::parse_job(gcode)?
    } else if first == format!("; {}", crate::DIALECT) {
        crate::parse(gcode)?
    } else {
        parse_foreign(gcode)?
    };
    Ok((preview, info))
}
