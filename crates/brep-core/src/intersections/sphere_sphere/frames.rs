use super::*;
pub(super) fn finish(
    center: [f64; 3],
    radius: f64,
    error: f64,
    frames: Vec<PatchFrame>,
    pole_surfaces: Option<Vec<nurbs_core::surface::Surface>>,
) -> Result<Option<CanonicalSphere>> {
    // Global frame: poles must be +/- one axis, four patches per hemisphere;
    // equator bases must tile the four quadrants consistently in both.
    let reference = frames[0].pole;
    let mut axis = [0.; 3];
    let mut north = 0;
    for frame in &frames {
        let alignment = dot(frame.pole, reference);
        if alignment.abs() < 1. - RECOGNITION {
            return Ok(None);
        }
        let sign = alignment.signum();
        north += (sign > 0.) as usize;
        for k in 0..3 {
            axis[k] += sign * frame.pole[k] / 8.;
        }
    }
    if north != 4 {
        return Ok(None);
    }
    let axis_length = dot(axis, axis).sqrt();
    if !axis_length.is_finite() || axis_length <= 0. {
        return Ok(None);
    }
    let axis = axis.map(|x| x / axis_length);
    // X from any equator basis direction projected off the axis.
    let seed = frames[0].a;
    let axial = dot(seed, axis);
    let x_dir = sub(seed, axis.map(|x| x * axial));
    let x_length = dot(x_dir, x_dir).sqrt();
    if !x_length.is_finite() || x_length <= 0. {
        return Ok(None);
    }
    let x_dir = x_dir.map(|x| x / x_length);
    let y_dir = cross(axis, x_dir);
    let quarter = std::f64::consts::FRAC_PI_2;
    let mut seen = [false; 8];
    let mut snapped = Vec::with_capacity(8);
    for frame in &frames {
        let hemisphere = dot(frame.pole, axis).signum();
        let angle = dot(frame.a, y_dir).atan2(dot(frame.a, x_dir));
        let quadrant = (angle / quarter).round() as i64;
        let quadrant = quadrant.rem_euclid(4) as usize;
        let residual = (angle - quadrant as f64 * quarter + std::f64::consts::PI).rem_euclid(TAU)
            - std::f64::consts::PI;
        if residual.abs() > RECOGNITION {
            return Ok(None);
        }
        let slot = if hemisphere > 0. {
            quadrant
        } else {
            4 + quadrant
        };
        if seen[slot] {
            return Ok(None);
        }
        seen[slot] = true;
        let direction = |q: usize| {
            let (sin, cos) = (q as f64 * quarter).sin_cos();
            std::array::from_fn(|k| cos * x_dir[k] + sin * y_dir[k])
        };
        let snapped_a = direction(quadrant);
        let snapped_b = direction((quadrant + 1) % 4);
        if dot(snapped_b, frame.b) < 1. - RECOGNITION {
            return Ok(None);
        }
        snapped.push(PatchFrame {
            a: snapped_a,
            b: snapped_b,
            pole: axis.map(|x| x * hemisphere),
        });
    }
    if seen.into_iter().any(|hit| !hit) {
        return Ok(None);
    }
    Ok(Some(CanonicalSphere {
        center,
        radius,
        error,
        patches: snapped,
        pole_surfaces,
    }))
}
