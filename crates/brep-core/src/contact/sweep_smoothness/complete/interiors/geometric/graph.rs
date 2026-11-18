//! Parametrization-independent jets on one exact boundary of adjacent tiles.
use crate::{Curve, Result, Surface};
use nurbs_core::{
    factored_surface_normals::{prepare_exact_meridian_jets, prepare_exact_meridian_tangency},
    normal_alignment,
};

fn border(surface: &Surface, label: &str) -> (Curve, [[f64; 2]; 2]) {
    let mut domain = [
        [
            surface.knots_u[surface.degree_u],
            surface.knots_u[surface.control_points.len()],
        ],
        [
            surface.knots_v[surface.degree_v],
            surface.knots_v[surface.control_points[0].len()],
        ],
    ];
    let fixed = usize::from(label.starts_with('v'));
    domain[fixed] = [domain[fixed][usize::from(label.ends_with("Max"))]; 2];
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![domain[0][0], domain[1][0]],
            vec![domain[0][1], domain[1][1]],
        ],
        weights: vec![1.; 2],
        periodic: false,
    };
    (curve, domain)
}

fn single_bezier_tile(s: &Surface) -> bool {
    !s.periodic_u
        && !s.periodic_v
        && [
            (&s.knots_u, s.degree_u, s.control_points.len()),
            (&s.knots_v, s.degree_v, s.control_points[0].len()),
        ]
        .iter()
        .all(|&(knots, degree, count)| {
            (1..=8).contains(&degree)
                && count == degree + 1
                && knots[..count].iter().all(|&x| x == knots[degree])
                && knots[count..].iter().all(|&x| x == knots[count])
        })
}

/// The caller has already proved positional equality in the common increasing
/// border parameter and that these are opposite sides of adjacent source tiles.
pub(super) fn inspect(
    surfaces: [&Surface; 2],
    boundaries: [&str; 2],
    order: usize,
    work: u64,
) -> Result<(bool, u64)> {
    if work < 3 || !surfaces.iter().all(|s| single_bezier_tile(s)) {
        return Ok((false, 0));
    }
    let [(a, ua), (b, ub)] = std::array::from_fn(|i| border(surfaces[i], boundaries[i]));
    let (na, first) = normal_alignment::normal_bounds(surfaces[0], ua, 1)?;
    let (nb, second) = normal_alignment::normal_bounds(surfaces[1], ub, 1)?;
    let mut used = (first + second) as u64;
    let (Some(na), Some(nb)) = (na, nb) else {
        return Ok((false, used));
    };
    // Equal strict signs ensure both inverse charts cover opposite sides of
    // the seam's projection, with nonzero Jacobian along the entire seam.
    let Some(axis) =
        (0..3).find(|&i| (na[i][0] > 0. && nb[i][0] > 0.) || (na[i][1] < 0. && nb[i][1] < 0.))
    else {
        return Ok((false, used));
    };
    let budget = work - used;
    let (certified, proof_work) = if order == 1 {
        let proof = prepare_exact_meridian_tangency(
            surfaces,
            [&a, &b],
            [false; 2],
            [[1.; 2]; 2],
            axis,
            budget,
        )?;
        (proof.certificate.is_some(), proof.work)
    } else {
        let proof = prepare_exact_meridian_jets(
            surfaces,
            [&a, &b],
            [false; 2],
            [[1.; 2]; 2],
            axis,
            budget,
        )?;
        (proof.certificate.is_some(), proof.work)
    };
    used += proof_work;
    Ok((certified, used))
}
