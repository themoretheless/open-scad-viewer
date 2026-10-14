use super::*;

/// Elevate an admitted Bezier-le3 patch (or already-uniform bicubic) to R1 bicubic.
/// Keeps the historical bilinear 1×1 elevation path; deg 2 and mixed use Bernstein
/// elevation on U then V until 3×3.
pub(super) fn as_uniform_bicubic(surface: &Surface) -> Option<Surface> {
    if is_uniform_bicubic_positive(surface) {
        return Some(surface.clone());
    }
    if !is_bezier_le3_positive(surface) {
        return None;
    }
    if surface.degree_u == 1 && surface.degree_v == 1 {
        return elevate_bilinear_1x1(surface);
    }
    let elevated = surface
        .edit_axis(Axis::U, |c| c.elevate(3))
        .ok()?
        .edit_axis(Axis::V, |c| c.elevate(3))
        .ok()?;
    if is_uniform_bicubic_positive(&elevated) {
        Some(elevated)
    } else {
        None
    }
}

fn elevate_bilinear_1x1(surface: &Surface) -> Option<Surface> {
    let p00 = &surface.control_points[0][0];
    let p10 = &surface.control_points[0][1];
    let p01 = &surface.control_points[1][0];
    let p11 = &surface.control_points[1][1];
    let row = |a: &[f64], b: &[f64]| -> Vec<Vec<f64>> {
        vec![
            a.to_vec(),
            mix_pt(a, b, 1. / 3.),
            mix_pt(a, b, 2. / 3.),
            b.to_vec(),
        ]
    };
    let r0 = row(p00, p10);
    let r1 = row(p01, p11);
    let mut control_points = Vec::with_capacity(4);
    for j in 0..4 {
        let t = j as f64 / 3.;
        control_points.push((0..4).map(|i| mix_pt(&r0[i], &r1[i], t)).collect());
    }
    Some(Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points,
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    })
}

fn mix_pt(a: &[f64], b: &[f64], t: f64) -> Vec<f64> {
    a.iter()
        .zip(b.iter())
        .map(|(u, v)| (1. - t) * u + t * v)
        .collect()
}
