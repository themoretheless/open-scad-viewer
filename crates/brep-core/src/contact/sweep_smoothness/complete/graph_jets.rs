//! Parametrization-independent jets on complete, exactly owned boundaries.
use crate::{Curve, Result, Surface};
use nurbs_core::continuity::{self, ProjectiveSeam, ProjectiveSeamCollectionReport};
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

// Sign of the inward transverse direction relative to the ordered
// (increasing boundary tangent, inward transverse) chart coordinates.
fn inward_sign(label: &str) -> f64 {
    match label {
        "uMax" | "vMin" => 1.,
        "uMin" | "vMax" => -1.,
        _ => unreachable!("validated natural boundary label"),
    }
}

// Positions agree in the common increasing along-edge parameter.
fn inspect(
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
    let [left, right] = boundaries.map(inward_sign);
    let opposite = |a: [f64; 2], b: [f64; 2]| {
        let positive = |bounds: [f64; 2], sign: f64| {
            if sign > 0. {
                bounds[0] > 0.
            } else {
                bounds[1] < 0.
            }
        };
        (positive(a, left) && positive(b, -right)) || (positive(a, -left) && positive(b, right))
    };
    let Some(axis) = (0..3).find(|&i| opposite(na[i], nb[i])) else {
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

// Complete source-side incidence is a separate obligation of the caller.
pub(super) fn qualify(
    patches: &[Surface],
    seams: &[ProjectiveSeam],
    work: u64,
) -> Result<ProjectiveSeamCollectionReport> {
    let mut proof = continuity::inspect_projective_seam_collection_with_rational_scale_proposals(
        patches,
        seams,
        work.min(continuity::MAX_PROJECTIVE_SEAM_COLLECTION_WORK),
    )?;
    if proof.certified {
        return Ok(proof);
    }
    for &i in &proof.unresolved_seams {
        let result = &mut proof.seams[i];
        if !result.affine_parameter_identity {
            continue;
        }
        let seam = &seams[i];
        let (certified, used) = inspect(
            seam.patches.map(|patch| &patches[patch]),
            seam.boundaries.each_ref().map(String::as_str),
            seam.order,
            work - proof.exact_work,
        )?;
        proof.exact_work += used;
        result.work += used;
        if certified {
            result.certified = true;
            result.exact_identity = true;
            result.regularity_certified = true;
            result.reason = "exact-regular-geometric-graph-jets";
        }
    }
    proof
        .unresolved_seams
        .retain(|&i| !proof.seams[i].certified);
    proof.certified = !seams.is_empty() && proof.unresolved_seams.is_empty();
    if proof.certified {
        proof.certified_order = seams.iter().map(|s| s.order).min();
    }
    Ok(proof)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cap(right: bool) -> Surface {
        Surface {
            degree_u: 2,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    let x = i as f64 / 2. - if right { 0. } else { 1. };
                    let shift = if right && i == 2 { 1. / 8. } else { 0. };
                    vec![
                        vec![x, 0., 0.],
                        vec![x + shift / 2., 1., x / 8.],
                        vec![x + shift, 2., (x + shift) / 4.],
                    ]
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn dissimilar_axes_and_reflected_clocks_preserve_geometric_g2() {
        let original = cap(true);
        let mut transformed = Surface {
            degree_u: original.degree_v,
            degree_v: original.degree_u,
            knots_u: original.knots_v.clone(),
            knots_v: original.knots_u.clone(),
            control_points: (0..3)
                .rev()
                .map(|v| {
                    original
                        .control_points
                        .iter()
                        .rev()
                        .map(|row| row[v].clone())
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let (reversed, reflection) =
            super::super::reverse_along(&mut transformed, "vMax", 1_000_000).unwrap();
        assert!(reversed);
        assert_eq!(reflection, 0);
        let patches = [cap(false), transformed];
        let seam = ProjectiveSeam {
            patches: [0, 1],
            boundaries: ["uMax".into(), "vMax".into()],
            order: 2,
            normal_scale: 1.,
        };
        let strict = continuity::inspect_projective_seam_collection_with_rational_scale_proposals(
            &patches,
            std::slice::from_ref(&seam),
            1_000_000,
        )
        .unwrap();
        assert!(!strict.certified && strict.seams[0].affine_parameter_identity);
        let report = qualify(&patches, std::slice::from_ref(&seam), 1_000_000).unwrap();
        assert!(report.certified, "{}", report.seams[0].reason);
        assert_eq!(report.certified_order, Some(2));
        assert_eq!(report.seams[0].reason, "exact-regular-geometric-graph-jets");
        assert_eq!(report.exact_work, report.seams[0].work);
        assert!(
            !qualify(&patches, &[seam], report.exact_work - 1)
                .unwrap()
                .certified
        );

        let same_side = [patches[0].clone(), patches[0].clone()];
        let (certified, used) = inspect(
            [&same_side[0], &same_side[1]],
            ["uMax", "uMax"],
            2,
            1_000_000,
        )
        .unwrap();
        assert!(!certified && used == 2);
    }
}
