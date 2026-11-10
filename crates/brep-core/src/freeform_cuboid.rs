//! Freeform cuboid solids whose faces are uniform bicubic B-splines (weights 1), used by
//! freeform STEP export and by kernel tests.
use crate::Model;
use nurbs_core::{Error, Result, surface::Surface};

/// Same error code as the freeform STEP writers, which surface these refusals unchanged.
fn refuse(message: &str) -> Error {
    Error::new("BREP_NURBS_STEP_REFUSED", message)
}

fn mix_pt(a: &[f64], b: &[f64], t: f64) -> Vec<f64> {
    a.iter()
        .zip(b.iter())
        .map(|(u, v)| (1. - t) * u + t * v)
        .collect()
}


pub fn is_uniform_bicubic_positive(surface: &Surface) -> bool {
    surface.degree_u == 3
        && surface.degree_v == 3
        && surface.control_points.len() == 4
        && surface.control_points.iter().all(|row| row.len() == 4)
        && surface.knots_u.len() == 8
        && surface.knots_v.len() == 8
        && surface
            .weights
            .iter()
            .flatten()
            .all(|w| w.is_finite() && (*w - 1.).abs() <= 1e-12)
        && !surface.periodic_u
        && !surface.periodic_v
}

/// Elevate planar bilinear 2×2 (or already-uniform bicubic) — same logic as nurbs_ss_g6.
pub fn as_uniform_bicubic(surface: &Surface) -> Option<Surface> {
    if is_uniform_bicubic_positive(surface) {
        return Some(surface.clone());
    }
    if surface.periodic_u || surface.periodic_v {
        return None;
    }
    if surface.degree_u != 1 || surface.degree_v != 1 {
        return None;
    }
    if surface.control_points.len() != 2 || surface.control_points.iter().any(|r| r.len() != 2) {
        return None;
    }
    if surface
        .weights
        .iter()
        .flatten()
        .any(|w| !w.is_finite() || (*w - 1.).abs() > 1e-12)
    {
        return None;
    }
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

/// Cuboid with each face elevated to uniform bicubic (degree 3×3, w≡1) for STEP honesty.
pub fn freeform_cuboid_solid(min: [f64; 3], max: [f64; 3]) -> Result<Model> {
    let mut model = crate::cuboid(min, max)?;
    for face in &mut model.faces {
        let elev = as_uniform_bicubic(&face.surface).ok_or_else(|| {
            refuse("freeform_cuboid_solid requires elevatable planar bilinear faces")
        })?;
        face.surface = elev;
    }
    model.validate()?;
    Ok(model)
}

fn bump_top_surface(min: [f64; 3], max: [f64; 3], bump: f64) -> Surface {
    // Match cuboid +Z face control layout: corners [4,5,6,7] → bilinear then elevate + bump.
    let p00 = [min[0], min[1], max[2]];
    let p10 = [max[0], min[1], max[2]];
    let p01 = [min[0], max[1], max[2]];
    let p11 = [max[0], max[1], max[2]];
    let bilinear = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![p00.to_vec(), p01.to_vec()],
            vec![p10.to_vec(), p11.to_vec()],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let mut s = as_uniform_bicubic(&bilinear).expect("bilinear elevates");
    for row in s.control_points.iter_mut().skip(1).take(2) {
        for point in row.iter_mut().skip(1).take(2) {
            point[2] += bump;
        }
    }
    s
}

/// Cuboid with +Z face replaced by a native bicubic bump (corners match).
pub fn freeform_cuboid_with_bump_face(min: [f64; 3], max: [f64; 3]) -> Result<Model> {
    let mut model = freeform_cuboid_solid(min, max)?;
    // cuboid face 1 is +Z ([4,5,6,7])
    model.faces[1].surface = bump_top_surface(min, max, 0.15 * (max[2] - min[2]).max(0.05));
    if !is_uniform_bicubic_positive(&model.faces[1].surface) {
        return Err(refuse("bump face must be uniform bicubic w≡1"));
    }
    model.validate()?;
    Ok(model)
}
