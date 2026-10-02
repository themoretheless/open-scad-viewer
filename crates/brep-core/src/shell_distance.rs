//! Global distance between the trimmed boundary images of two B-rep models.
//! Filled-volume containment is deliberately a separate predicate.
use crate::{Error, Model, Result, face_domain::FaceDomain};
use nurbs_core::{
    surface_distance::{enclosure_distance, rectangle_bounds},
    trimmed_surface_distance::{self, TrimmedDistance},
};

pub struct ShellDistance {
    pub lower_bound_mm: f64,
    pub upper_bound_mm: Option<f64>,
    pub faces: Option<[usize; 2]>,
    pub witness: Option<TrimmedDistance>,
    pub converged: bool,
    pub reason: &'static str,
    pub pairs: usize,
    pub evaluated_pairs: usize,
    pub cells: usize,
    pub domain_cells: usize,
}
struct Pair {
    slots: [usize; 2],
    faces: [usize; 2],
    lower: f64,
}
/// Every face pair contributes a lower bound, even when the shared budget ends.
/// The upper bound always comes from admitted points on two authored faces.
pub fn distance(
    a: &Model,
    b: &Model,
    tolerance_mm: f64,
    tolerance_uv: f64,
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<ShellDistance> {
    distance_between_face_sets(
        a,
        &(0..a.faces.len()).collect::<Vec<_>>(),
        b,
        &(0..b.faces.len()).collect::<Vec<_>>(),
        tolerance_mm,
        tolerance_uv,
        max_cells,
        max_domain_cells,
    )
}
/// Complete clearance between selected unions of authored trimmed faces.
/// Every selected pair retains a lower bound, including pairs not subdivided.
/// This does not certify a wall-thickness direction or a material chord.
pub fn distance_between_face_sets(
    a: &Model,
    faces_a: &[usize],
    b: &Model,
    faces_b: &[usize],
    tolerance_mm: f64,
    tolerance_uv: f64,
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<ShellDistance> {
    a.validate()?;
    b.validate()?;
    if !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
        || !(1..=1000000).contains(&max_cells)
        || !(1..=8000000).contains(&max_domain_cells)
    {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Shell distance requires positive tolerance, 1..1000000 geometry cells and 1..8000000 domain cells",
        ));
    }
    for (model, selected) in [(a, faces_a), (b, faces_b)] {
        if selected.is_empty()
            || selected.len() > model.faces.len()
            || selected.iter().any(|&face| face >= model.faces.len())
            || selected
                .iter()
                .copied()
                .collect::<std::collections::HashSet<_>>()
                .len()
                != selected.len()
        {
            return Err(Error::new(
                "BREP_INVALID_INPUT",
                "Face groups require nonempty unique original face indices",
            ));
        }
    }
    let pairs = faces_a.len().saturating_mul(faces_b.len());
    if pairs == 0 || pairs > 100000 {
        return Err(Error::new(
            "BREP_RESOURCE_LIMIT",
            "Shell distance requires 1..100000 face pairs",
        ));
    }
    let prepare = |m: &Model, selected: &[usize]| -> Result<_> {
        selected
            .iter()
            .map(|&i| {
                let f = &m.faces[i];
                let s = &f.surface;
                let domain = [
                    [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                    [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
                ];
                Ok((
                    FaceDomain::new(m, i, tolerance_uv)?,
                    rectangle_bounds(s, domain)?,
                    // Radius is an optional tightening; numeric range failure keeps the Cartesian bound.
                    nurbs_core::radial_bounds::radius_bounds(s, [0.; 3])
                        .ok()
                        .flatten(),
                    std::array::from_fn::<_, 3, _>(|axis| {
                        nurbs_core::radial_bounds::axis_radius_bounds(s, axis, [0.; 3])
                            .ok()
                            .flatten()
                    }),
                ))
            })
            .collect::<Result<Vec<_>>>()
    };
    let aa = prepare(a, faces_a)?;
    let bb = prepare(b, faces_b)?;
    let mut queue = Vec::with_capacity(pairs);
    for (i, (_, ba, ra, axes_a)) in aa.iter().enumerate() {
        for (j, (_, bb, rb, axes_b)) in bb.iter().enumerate() {
            let radial = match (ra, rb) {
                (Some(a), Some(b)) => {
                    enclosure_distance(&[*a, [0.; 2], [0.; 2]], &[*b, [0.; 2], [0.; 2]])?.0
                }
                _ => 0.,
            };
            let axial = axes_a
                .iter()
                .zip(axes_b)
                .filter_map(|(a, b)| match (a, b) {
                    (Some(a), Some(b)) => Some((a[0] - b[1]).max(b[0] - a[1]).next_down().max(0.)),
                    _ => None,
                })
                .fold(0_f64, f64::max);
            queue.push(Pair {
                slots: [i, j],
                faces: [faces_a[i], faces_b[j]],
                lower: enclosure_distance(ba, bb)?.0.max(radial).max(axial),
            })
        }
    }
    queue.sort_by(|a, b| a.lower.total_cmp(&b.lower).then(a.faces.cmp(&b.faces)));
    let mut upper = f64::INFINITY;
    let mut lower = f64::INFINITY;
    let mut witness = None;
    let mut faces = None;
    let mut cells = 0;
    let mut domain_cells = 0;
    let mut evaluated_pairs = 0;
    let mut reason = "pair-resolution-limit";
    for (pair_index, pair) in queue.iter().enumerate() {
        // Bounds for pruned and unvisited pairs still participate in the result.
        if pair.lower >= upper || upper - pair.lower <= tolerance_mm {
            lower = lower.min(pair.lower);
            continue;
        }
        if cells == max_cells || domain_cells == max_domain_cells {
            lower = lower.min(pair.lower);
            reason = if domain_cells == max_domain_cells {
                "domain-work-limit"
            } else {
                "work-limit"
            };
            continue;
        }
        let [i, j] = pair.faces;
        let [slot_a, slot_b] = pair.slots;
        // Reserve subdivision work for every remaining pair. Otherwise one
        // near pair can consume the entire budget while overlapping AABBs of
        // all later (actually separated) pairs retain a zero lower bound.
        let remaining = ((max_cells - cells) / (queue.len() - pair_index))
            .max(1)
            .min(100000);
        // Initial knot pairs must fit before entering the face solver.
        let spans = |s: &nurbs_core::surface::Surface| {
            let count = |knots: &[f64], degree: usize, n: usize| {
                (degree..n).filter(|&k| knots[k] < knots[k + 1]).count()
            };
            count(&s.knots_u, s.degree_u, s.control_points.len())
                * count(&s.knots_v, s.degree_v, s.control_points[0].len())
        };
        let initial = spans(&a.faces[i].surface).saturating_mul(spans(&b.faces[j].surface));
        if initial > remaining {
            lower = lower.min(pair.lower);
            reason = "work-limit";
            continue;
        }
        evaluated_pairs += 1;
        let mut pair_lower = pair.lower;
        let mut empty_pair = false;
        // First seek witnesses with full initial coverage and no subdivision.
        // A strong global enclosure can already establish the final tolerance.
        for budget in [initial, remaining.saturating_sub(initial)] {
            if budget < initial || cells == max_cells || domain_cells == max_domain_cells {
                break;
            }
            let r = trimmed_surface_distance::distance(
                &a.faces[i].surface,
                &aa[slot_a].0.region,
                &b.faces[j].surface,
                &bb[slot_b].0.region,
                tolerance_mm,
                budget.min(max_cells - cells),
                max_domain_cells - domain_cells,
            )?;
            cells += r.cells;
            domain_cells += r.domain_cells;
            if r.reason == "empty-domain" {
                empty_pair = true;
                break;
            }
            pair_lower = pair_lower.max(r.lower_bound_mm);
            if let Some(u) = r.upper_bound_mm {
                if u < upper {
                    upper = u;
                    faces = Some([i, j]);
                    witness = Some(r)
                }
            }
            if upper - pair_lower <= tolerance_mm {
                break;
            }
        }
        if !empty_pair {
            lower = lower.min(pair_lower)
        }
    }
    let empty = lower == f64::INFINITY && upper == f64::INFINITY;
    let converged = upper.is_finite() && upper - lower.min(upper) <= tolerance_mm;
    Ok(ShellDistance {
        lower_bound_mm: if empty { 0. } else { lower.min(upper) },
        upper_bound_mm: upper.is_finite().then_some(upper),
        faces,
        witness,
        converged,
        reason: if converged {
            "tolerance"
        } else if empty {
            "empty-domain"
        } else {
            reason
        },
        pairs,
        evaluated_pairs,
        cells,
        domain_cells,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn complete_opposing_face_groups_keep_original_indices_and_global_coverage() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let before = format!("{model:?}");
        for (a, b, expected) in [
            (vec![10, 0, 5], vec![25, 8, 3, 17, 13, 21], 13.75),
            (vec![0, 5, 10], vec![4, 9, 14, 18, 22, 26], 4.75),
            (vec![2, 7, 12, 16, 20, 24], vec![3, 8, 13, 17, 21, 25], 15.),
        ] {
            let r = distance_between_face_sets(&model, &a, &model, &b, 1e-5, 1e-8, 10000, 1000000)
                .unwrap();
            assert!(
                r.converged,
                "{expected}: {} [{}, {:?}]",
                r.reason, r.lower_bound_mm, r.upper_bound_mm
            );
            assert_eq!(r.pairs, a.len() * b.len());
            assert!(r.lower_bound_mm <= expected && r.upper_bound_mm.unwrap() >= expected);
            assert!(r.upper_bound_mm.unwrap() - r.lower_bound_mm <= 1e-5);
            let faces = r.faces.unwrap();
            assert!(a.contains(&faces[0]) && b.contains(&faces[1]));
            let witness = r.witness.unwrap();
            let uv = witness.parameters.unwrap();
            let points = witness.points.unwrap();
            for side in 0..2 {
                let actual = model.faces[faces[side]]
                    .surface
                    .evaluate(uv[side][0], uv[side][1])
                    .unwrap()
                    .point;
                assert!(
                    actual
                        .iter()
                        .zip(points[side])
                        .all(|(a, b)| (a - b).abs() < 1e-10)
                );
            }
        }
        assert_eq!(format!("{model:?}"), before);
        for group in [vec![], vec![0, 0], vec![27]] {
            assert!(
                distance_between_face_sets(&model, &group, &model, &[3], 1e-5, 1e-8, 10, 100)
                    .is_err()
            );
        }
        let exhausted = distance_between_face_sets(
            &model,
            &[0, 5, 10],
            &model,
            &[3, 8, 13, 17, 21, 25],
            1e-5,
            1e-8,
            1,
            10000,
        )
        .unwrap();
        assert!(!exhausted.converged && exhausted.pairs == 18 && exhausted.cells == 1);
        assert!(exhausted.lower_bound_mm <= 13.75);
        let no_witness = distance_between_face_sets(
            &model,
            &[0, 5, 10],
            &model,
            &[3, 8, 13, 17, 21, 25],
            1e-5,
            1e-8,
            10000,
            1,
        )
        .unwrap();
        assert!(!no_witness.converged && no_witness.witness.is_none());
        assert!(no_witness.lower_bound_mm <= 13.75);
        let same =
            distance_between_face_sets(&model, &[5], &model, &[5], 1e-5, 1e-8, 10, 10000).unwrap();
        assert!(same.converged && same.lower_bound_mm == 0. && same.upper_bound_mm.unwrap() < 1e-5);
        assert_eq!(same.faces, Some([5, 5]));
    }

    #[test]
    fn separated_boxes_have_global_face_witnesses() {
        let a = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let b = crate::cuboid([3., 0., 0.], [4., 1., 1.]).unwrap();
        let r = distance(&a, &b, 0.001, 1e-7, 100000, 1000000).unwrap();
        assert!(r.converged);
        assert!(r.lower_bound_mm <= 2. && r.upper_bound_mm.unwrap() >= 2.);
        assert_eq!(r.pairs, 36);
        assert!(r.evaluated_pairs < 36);
        assert!(r.faces.is_some());
    }
    #[test]
    fn exhausted_budget_keeps_unvisited_pairs_in_lower_bound() {
        let a = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        let b = crate::cuboid([3., 0., 0.], [4., 1., 1.]).unwrap();
        let r = distance(&a, &b, 0.001, 1e-7, 1, 1).unwrap();
        assert!(!r.converged);
        assert!(r.upper_bound_mm.is_none());
        assert!(r.lower_bound_mm <= 2.);
        assert!(r.cells <= 1 && r.domain_cells <= 1);
        assert_eq!(r.reason, "domain-work-limit");
    }
    #[test]
    fn nested_shells_do_not_claim_zero_volume_clearance() {
        let a = crate::cuboid([0.; 3], [10.; 3]).unwrap();
        let b = crate::cuboid([2.; 3], [8.; 3]).unwrap();
        let r = distance(&a, &b, 0.001, 1e-7, 100000, 1000000).unwrap();
        assert!(r.converged);
        assert!(r.lower_bound_mm <= 2. && r.upper_bound_mm.unwrap() >= 2.);
    }
    #[test]
    fn diagonal_and_touching_boxes_match_analytic_distance() {
        let a = crate::cuboid([0.; 3], [1.; 3]).unwrap();
        for (origin, expected) in [([2., 3., 4.], 14_f64.sqrt()), ([1., 0., 0.], 0.)] {
            let b = crate::cuboid(origin, origin.map(|v| v + 1.)).unwrap();
            let r = distance(&a, &b, 0.003, 1e-7, 1000000, 8000000).unwrap();
            assert!(r.converged, "{} {:?}", r.reason, r.upper_bound_mm);
            assert!(r.lower_bound_mm <= expected && r.upper_bound_mm.unwrap() >= expected);
        }
    }
    #[test]
    fn all_face_pairs_respect_a_hole_in_the_top_cap() {
        use nurbs_core::curve::Curve;
        let line = |points: Vec<[f64; 2]>| {
            Curve::from_polyline(points.into_iter().map(|p| p.to_vec()).collect()).unwrap()
        };
        let outer = line(vec![[0., 0.], [10., 0.], [10., 10.], [0., 10.], [0., 0.]]);
        let hole = line(vec![[4., 4.], [4., 6.], [6., 6.], [6., 4.], [4., 4.]]);
        let a = crate::prism::extrude(&[vec![outer], vec![hole]], -1., 0.).unwrap();
        let b = crate::cuboid([4.75, 4.75, 2.], [5.25, 5.25, 3.]).unwrap();
        let r = distance(&a, &b, 0.003, 1e-7, 1000000, 8000000).unwrap();
        let expected = (4_f64 + 0.75 * 0.75).sqrt();
        assert!(
            r.converged,
            "{} {} {:?}",
            r.reason, r.lower_bound_mm, r.upper_bound_mm
        );
        assert!(r.lower_bound_mm <= expected && r.upper_bound_mm.unwrap() >= expected);
        assert!(r.cells <= 1000000 && r.domain_cells <= 8000000);
    }
    #[test]
    fn deterministic_box_family_matches_independent_axis_gap_formula() {
        for k in 0..24 {
            let origin = [
                (k % 4) as f64 * 0.7,
                (k % 3) as f64 * 1.1,
                (k % 5) as f64 * 0.9,
            ];
            let b = crate::cuboid(origin, origin.map(|x| x + 0.5)).unwrap();
            let a = crate::cuboid([0.; 3], [1.; 3]).unwrap();
            // Restrict this test to disjoint or touching boxes: volume overlap
            // requires a separate shell-intersection or containment formula.
            if origin.iter().all(|&x| x < 1.) {
                continue;
            }
            let expected = origin
                .iter()
                .map(|&x| (x - 1.).max(0.).powi(2))
                .sum::<f64>()
                .sqrt();
            let r = distance(&a, &b, 0.003, 1e-7, 1000000, 8000000).unwrap();
            assert!(r.converged, "case {k}: {}", r.reason);
            assert!(
                r.lower_bound_mm <= expected && r.upper_bound_mm.unwrap() >= expected,
                "case {k}"
            );
            let witness = r.witness.unwrap();
            let points = witness.points.unwrap();
            let sampled = points[0]
                .iter()
                .zip(points[1])
                .map(|(x, y)| (x - y).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!((sampled - expected).abs() <= 0.003);
        }
    }
    #[test]
    fn concentric_rational_spheres_keep_a_positive_shell_gap() {
        let a = crate::analytic::sphere(2.).unwrap();
        let b = crate::analytic::sphere(5.).unwrap();
        let r = distance(&a, &b, 0.01, 1e-7, 1000000, 8000000).unwrap();
        assert!(r.lower_bound_mm <= 3. && r.upper_bound_mm.unwrap() >= 3.);
        assert!(
            r.converged,
            "{} {} {:?}",
            r.reason, r.lower_bound_mm, r.upper_bound_mm
        );
    }
}
