use super::*;

/// Type 100 is counterclockwise in its local XY plane; the directory
/// transformation policy is applied before curve dispatch.
pub(super) fn read(entity: &Entity, dim: usize, scale: f64, tolerance: f64) -> Result<Curve> {
    let values = &entity.values;
    let invalid = || {
        composite_error(
            "BREP_IGES_V2_COMPOSITE_UNSUPPORTED",
            "Invalid IGES circular arc",
        )
    };
    if values.len() != 8 || entity.directory.form != 0 {
        return Err(invalid());
    }
    let mut data = [0.; 7];
    for (i, number) in data.iter_mut().enumerate() {
        *number = f(values, i + 1, "arc coordinate")? * scale;
    }
    let [z, cx, cy, sx, sy, ex, ey] = data;
    let (ux, uy) = (sx - cx, sy - cy);
    let (vx, vy) = (ex - cx, ey - cy);
    let radius = ux.hypot(uy);
    let end_radius = vx.hypot(vy);
    if radius == 0. || end_radius == 0. || (radius - end_radius).abs() > tolerance {
        return Err(invalid());
    }
    let sweep = if [sx, sy] == [ex, ey] {
        360.
    } else {
        let sweep = (vy.atan2(vx) - uy.atan2(ux)).to_degrees().rem_euclid(360.);
        if sweep == 0. || sweep == 360. {
            return Err(invalid());
        }
        sweep
    };
    let mut curve =
        nurbs_core::primitives::ellipse_arc([cx, cy, z], [ux, uy, 0.], [-uy, ux, 0.], 0., sweep)?;
    let end = curve.control_points.last().ok_or_else(invalid)?;
    if (end[0] - ex).hypot(end[1] - ey) > tolerance {
        return Err(invalid());
    }
    if dim == 2 {
        if z != 0. {
            return Err(invalid());
        }
        for point in &mut curve.control_points {
            point.truncate(2);
        }
    }
    curve.validate()?;
    Ok(curve)
}
