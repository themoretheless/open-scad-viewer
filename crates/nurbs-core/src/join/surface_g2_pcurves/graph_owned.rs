//! Whole traversal of correlated graph jets, with fresh 3D position ownership.
use super::*;
use crate::analysis::curve_surface_composition::graph_trace::{Pair, curve_box};
pub struct OwnedGraphInspection {
    pub owned: OwnedInspection,
    /// Includes exact rational coefficient operations and precharged interval
    /// convolution bounds. Distinct from predicate work in position proofs.
    pub coefficient_work: usize,
    pub accepted_cells: usize,
    pub last_unresolved_reason: Option<String>,
}
fn spans(s: &Surface, box_: [[f64; 2]; 2]) -> Option<Vec<[usize; 2]>> {
    let mut axes = [Vec::new(), Vec::new()];
    for (a, (p, k, n)) in [
        (s.degree_u, &s.knots_u, s.control_points.len()),
        (s.degree_v, &s.knots_v, s.control_points[0].len()),
    ]
    .into_iter()
    .enumerate()
    {
        if box_[a][0] < k[p] || box_[a][1] > k[n] {
            return None;
        }
        for i in p..n {
            if k[i] < k[i + 1] && box_[a][0] <= k[i + 1] && box_[a][1] >= k[i] {
                axes[a].push(i);
            }
        }
        if axes[a].len() > 2 || axes[a].is_empty() {
            return None;
        }
    }
    Some(
        axes[0]
            .iter()
            .flat_map(|&i| axes[1].iter().map(move |&j| [i, j]))
            .collect(),
    )
}
/// The polynomial graph candidate may have a small non-affine stored XY
/// component; it is explicitly bounded and corrected. Supported source and
/// candidate bases must be C2 at internal knots and pcurves clamped Beziers.
/// Exhausted, singular, or unsupported cases never grant G2 or a closed solid.
pub fn inspect_owned_graph(
    curve: &Curve,
    p: [&Curve; 2],
    s: [&Surface; 2],
    tolerances: [f64; 3],
    max_cells: usize,
    max_exact: u64,
    max_coefficients: usize,
) -> Result<OwnedGraphInspection> {
    check(
        tolerances.iter().all(|x| x.is_finite() && *x > 0.)
            && tolerances[1] <= 2.
            && max_cells <= 100000
            && (max_coefficients as u64) <= 50_000_000_000,
        "Owned graph proof budgets or tolerances invalid",
    )?;
    let positions = verify_positions(curve, p, s, [false; 2], tolerances[0], max_cells, max_exact)?;
    let owner = positions.proven.then(|| OwnedPositions {
        curve: curve.clone(),
        pcurves: [p[0].clone(), p[1].clone()],
        surfaces: [s[0].clone(), s[1].clone()],
        reversed: [false; 2],
        tolerance: tolerances[0],
    });
    let mut geometric = surface_g2::Report {
        position_bounds: [0., positions.upper.unwrap_or(f64::INFINITY)],
        branches: std::array::from_fn(|_| BranchBounds {
            normal_lower: 0.,
            tensor_lower: 0.,
            normal_upper: None,
            tensor_upper: None,
        }),
        orientation: None,
        regularity_proven: false,
        decision: Decision::Unresolved,
        cells: positions.cells,
    };
    let mut coefficient_work = 0;
    let mut accepted_cells = 0;
    let mut last_unresolved_reason = None;
    if positions.proven && positions.cells < max_cells && max_coefficients > 0 {
        let mut pair = Pair::new(s[0], s[1], max_coefficients)?;
        let mut stack = vec![[0., 1.]];
        let mut all = true;
        let mut orientation = None;
        let mut maximum = [0f64; 2];
        while let Some(range) = stack.pop() {
            if geometric.cells >= max_cells || pair.work >= pair.max_work {
                all = false;
                break;
            }
            geometric.cells += 1;
            let mut accepted = false;
            let boxes = (|| -> Result<_> {
                Ok([
                    curve_box(p[0], range, &mut pair.work, pair.max_work)?,
                    curve_box(p[1], range, &mut pair.work, pair.max_work)?,
                ])
            })();
            if let Ok(boxes) = boxes {
                if let (Some(a), Some(b)) = (spans(s[0], boxes[0]), spans(s[1], boxes[1])) {
                    let mut bound = [0f64; 2];
                    let mut good = true;
                    let mut cell_orientation = None;
                    for ia in a {
                        for &ib in &b {
                            match pair.bound(p, range, [ia, ib]) {
                                Ok(proof) => {
                                    if let (Some(upper), Some(same)) =
                                        (proof.upper, proof.same_orientation)
                                    {
                                        if cell_orientation.is_some_and(|old| old != same) {
                                            good = false;
                                        }
                                        cell_orientation = Some(same);
                                        for k in 0..2 {
                                            bound[k] = bound[k].max(upper[k]);
                                        }
                                    } else {
                                        good = false;
                                        last_unresolved_reason=Some("Graph jet denominator or geometric regularity unresolved".into());
                                    }
                                }
                                Err(error) => {
                                    good = false;
                                    last_unresolved_reason = Some(format!("{error:?}"));
                                }
                            }
                        }
                    }
                    if good
                        && bound[0] <= tolerances[1]
                        && bound[1] <= tolerances[2]
                        && !orientation.is_some_and(|old| Some(old) != cell_orientation)
                    {
                        orientation = cell_orientation;
                        for k in 0..2 {
                            maximum[k] = maximum[k].max(bound[k]);
                        }
                        accepted = true;
                        accepted_cells += 1;
                    } else if good {
                        last_unresolved_reason = Some(format!(
                            "Whole-cell upper bounds {:?} exceed limits or change orientation at {:?}",
                            bound, range
                        ));
                    }
                }
            } else if let Err(error) = boxes {
                last_unresolved_reason = Some(format!("{error:?}"));
            }
            if !accepted {
                let middle = range[0] * 0.5 + range[1] * 0.5;
                if middle == range[0] || middle == range[1] {
                    all = false;
                    break;
                }
                stack.push([middle, range[1]]);
                stack.push([range[0], middle]);
            }
        }
        coefficient_work = pair.work;
        if all && stack.is_empty() {
            if let Some(same) = orientation {
                let branch = usize::from(!same);
                geometric.branches[branch].normal_upper = Some(maximum[0]);
                geometric.branches[branch].tensor_upper = Some(maximum[1]);
                geometric.orientation = Some(if same {
                    Orientation::Same
                } else {
                    Orientation::Opposite
                });
                geometric.regularity_proven = true;
                geometric.decision = Decision::WithinTolerance;
                last_unresolved_reason = None;
            }
        }
    }
    Ok(OwnedGraphInspection {
        owned: OwnedInspection {
            report: Report {
                positions,
                geometric,
            },
            positions: owner,
        },
        coefficient_work,
        accepted_cells,
        last_unresolved_reason,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (Surface, Curve, Curve) {
        let s = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            weights: vec![vec![1.; 2]; 2],
            control_points: (0..2)
                .map(|i| {
                    (0..2)
                        .map(|j| {
                            let (u, v) = (i as f64, j as f64);
                            vec![u + v, -u + v, -2. * v]
                        })
                        .collect()
                })
                .collect(),
            periodic_u: false,
            periodic_v: false,
        };
        let p = Curve::from_polyline(vec![vec![0., 0.25], vec![1., 0.25]]).unwrap();
        let c =
            Curve::from_polyline(vec![vec![0.25, 0.25, -0.5], vec![1.25, -0.75, -0.5]]).unwrap();
        (s, p, c)
    }
    #[test]
    fn full_graph_seam_needs_fresh_positions_and_complete_traversal() {
        let (s, p, c) = fixture();
        let proof = inspect_owned_graph(
            &c,
            [&p, &p],
            [&s, &s],
            [1e-7, 1e-5, 1e-4],
            1000,
            100000,
            20_000_000,
        )
        .unwrap();
        assert!(proof.owned.positions.is_some());
        assert_eq!(
            proof.owned.report.geometric.decision,
            Decision::WithinTolerance
        );
        assert_eq!(
            proof.owned.report.geometric.orientation,
            Some(Orientation::Same)
        );
        assert!(proof.coefficient_work > 0);
    }
    #[test]
    fn displaced_original_curve_prevents_graph_admission() {
        let (s, p, mut c) = fixture();
        for pole in &mut c.control_points {
            pole[2] += 0.01;
        }
        let proof = inspect_owned_graph(
            &c,
            [&p, &p],
            [&s, &s],
            [1e-7, 1e-5, 1e-4],
            1000,
            100000,
            20_000_000,
        )
        .unwrap();
        assert!(proof.owned.positions.is_none());
        assert_ne!(
            proof.owned.report.geometric.decision,
            Decision::WithinTolerance
        );
        assert_eq!(proof.coefficient_work, 0);
    }
    #[test]
    fn position_ownership_alone_does_not_admit_zero_coefficient_work() {
        let (s, p, c) = fixture();
        let proof =
            inspect_owned_graph(&c, [&p, &p], [&s, &s], [1e-7, 1e-5, 1e-4], 1000, 100000, 0)
                .unwrap();
        assert!(proof.owned.positions.is_some());
        assert_ne!(
            proof.owned.report.geometric.decision,
            Decision::WithinTolerance
        );
    }
    #[test]
    fn reversed_support_orientation_keeps_one_coherent_geometric_sign() {
        let (s, p, c) = fixture();
        let mut candidate = s.clone();
        for row in &mut candidate.control_points {
            row.reverse();
        }
        let other = Curve::from_polyline(vec![vec![0., 0.75], vec![1., 0.75]]).unwrap();
        let proof = inspect_owned_graph(
            &c,
            [&p, &other],
            [&s, &candidate],
            [1e-7, 1e-5, 1e-4],
            1000,
            100000,
            20_000_000,
        )
        .unwrap();
        assert_eq!(
            proof.owned.report.geometric.decision,
            Decision::WithinTolerance
        );
        assert_eq!(
            proof.owned.report.geometric.orientation,
            Some(Orientation::Opposite)
        );
    }
}
