//! Parametrization-independent jets on complete, exactly owned boundaries.
use crate::{Curve, Result, Surface};
use nurbs_core::continuity::{self, ProjectiveSeam, ProjectiveSeamCollectionReport};
use nurbs_core::{
    factored_surface_normals::{prepare_exact_meridian_jets, prepare_exact_meridian_tangency},
    normal_alignment,
};

mod refinement;

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
    qualify_with_chart_proof(patches, seams, work, ChartProof::Geometric)
}

#[derive(Clone, Copy)]
pub(super) enum ChartProof {
    Geometric,
    Parametric,
}

// Interior knot proofs already operate on an extracted chart partition.
// A further refinement there may use parametric continuity on its pieces.
pub(super) fn qualify_with_chart_proof(
    patches: &[Surface],
    seams: &[ProjectiveSeam],
    work: u64,
    chart_proof: ChartProof,
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
        let seam = &seams[i];
        let surfaces = seam.patches.map(|patch| &patches[patch]);
        let boundaries = seam.boundaries.each_ref().map(String::as_str);
        let budget = work - proof.exact_work;
        let (certified, used) = if surfaces.iter().all(|s| single_bezier_tile(s)) {
            if !result.affine_parameter_identity {
                continue;
            }
            inspect(surfaces, boundaries, seam.order, budget)?
        } else {
            refinement::inspect(surfaces, boundaries, seam.order, budget, chart_proof)?
        };
        proof.exact_work += used;
        result.work += used;
        if certified {
            result.certified = true;
            result.exact_identity = true;
            result.affine_parameter_identity = true;
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

    #[test]
    fn whole_edge_requires_geometric_gluing_at_chart_knot_crossings() {
        // Tangential acceleration jumps at the interior knot. Each side
        // of that knot describes the same physical saddle when bend is zero.
        let chart = |rows: &[[f64; 3]], knots_u: Vec<f64>, knots_v: Vec<f64>, bend: f64| Surface {
            degree_u: rows.len() - 1,
            degree_v: 2,
            knots_u,
            knots_v,
            control_points: rows
                .iter()
                .map(|&[x, y_scale, z_scale]| {
                    [0., 0.5, 1., 1.5, 3.]
                        .into_iter()
                        .map(|y| vec![x, y_scale * y, z_scale * y + if y > 1. { bend } else { 0. }])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 5]; rows.len()],
            periodic_u: false,
            periodic_v: false,
        };
        let charts = |bend| {
            // The right chart has y = (1 + 3u²/8)y(v), adding
            // tangential acceleration to the second cross-edge derivative.
            [
                chart(
                    &[[-2., 1., -0.25], [-1., 1., -0.125], [0., 1., 0.]],
                    vec![0., 0., 0., 1., 1., 1.],
                    vec![0., 0., 0., 0.5, 0.5, 1., 1., 1.],
                    bend,
                ),
                chart(
                    &[
                        [0., 1., 0.],
                        [1., 1., 1. / 8.],
                        [2., 9. / 8., 1. / 4.],
                        [3., 11. / 8., 33. / 64.],
                    ],
                    vec![0., 0., 0., 0., 1., 1., 1., 1.],
                    vec![-9., -9., -9., -5., -5., -1., -1., -1.],
                    bend,
                ),
            ]
        };
        let seam = ProjectiveSeam {
            patches: [0, 1],
            boundaries: ["uMax".into(), "uMin".into()],
            order: 2,
            normal_scale: 1.,
        };
        let seams = std::slice::from_ref(&seam);
        let smooth = charts(0.);
        let raw =
            qualify_with_chart_proof(&smooth, seams, 1_000_000, ChartProof::Parametric).unwrap();
        assert!(!raw.certified);
        let full = qualify(&smooth, seams, 1_000_000).unwrap();
        assert!(full.certified, "{}", full.seams[0].reason);
        assert_eq!(full.certified_order, Some(2));
        assert!(
            !qualify(&smooth, seams, full.exact_work - 1)
                .unwrap()
                .certified
        );

        // The adjoining patches still describe the same graph on each
        // subspan, but a physical crease lies across their internal knot.
        let creased = charts(0.25);
        let tangent = ProjectiveSeam { order: 1, ..seam };
        assert!(!qualify(&creased, &[tangent], 1_000_000).unwrap().certified);
    }

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
    #[test]
    fn different_authored_partitions_cover_every_geometric_jet_span() {
        use nurbs_core::{editing::exact_surface_knot_insertion::insert, surface::Axis};
        let refine = |chart: &Surface, knot| {
            let result = insert(chart, Axis::V, knot, 1, 1_000_000).unwrap();
            assert_eq!(result.inserted, 1, "{:?}", result.refusal);
            result.surface
        };
        let mut left = cap(false);
        for point in left.control_points.iter_mut().flatten() {
            point[0] *= 3.;
            point[2] *= 3.;
        }
        // x = 3u, y = (2 + 3u²/8)v, z = xy/8. The extra second
        // derivative is along the common edge and varies along it.
        let right = Surface {
            degree_u: 3,
            degree_v: 2,
            knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            knots_v: vec![0., 0., 0., 1., 1., 1.],
            control_points: (0..4)
                .map(|i| {
                    let x = i as f64;
                    let y = [2., 2., 17. / 8., 19. / 8.][i];
                    let z = x / 4. + if i == 3 { 9. / 64. } else { 0. };
                    [0., 0.5, 1.]
                        .into_iter()
                        .map(|v| vec![x, y * v, z * v])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 3]; 4],
            periodic_u: false,
            periodic_v: false,
        };
        let seam = ProjectiveSeam {
            patches: [0, 1],
            boundaries: ["uMax".into(), "uMin".into()],
            order: 2,
            normal_scale: 1.,
        };
        let tangent = ProjectiveSeam {
            order: 1,
            boundaries: seam.boundaries.clone(),
            ..seam
        };
        for (offset, width) in [(0., 1.), (-9., 8.)] {
            let mut curves = [refine(&left, 0.25), refine(&right, 0.75)];
            for knot in &mut curves[1].knots_v {
                *knot = offset + *knot * width;
            }
            let seams = std::slice::from_ref(&seam);
            let raw = continuity::inspect_projective_seam_collection_with_rational_scale_proposals(
                &curves, seams, 1_000_000,
            )
            .unwrap();
            assert!(!raw.certified);
            let full = qualify(&curves, seams, 1_000_000).unwrap();
            assert!(full.certified, "right clock [{offset}, {}]", offset + width);
            assert_eq!(full.certified_order, Some(2));
            assert_eq!(full.exact_work, full.seams[0].work);
            assert!(
                !qualify(&curves, seams, full.exact_work - 1)
                    .unwrap()
                    .certified
            );

            // Alter only the right carrier's second transverse jet.
            let mut changed = right.clone();
            changed.control_points[2][2][2] += 1. / 16.;
            curves[1] = refine(&changed, 0.75);
            for knot in &mut curves[1].knots_v {
                *knot = offset + *knot * width;
            }
            assert!(!qualify(&curves, seams, 1_000_000).unwrap().certified);
            assert!(
                qualify(&curves, std::slice::from_ref(&tangent), 1_000_000)
                    .unwrap()
                    .certified
            );
        }
    }
}
