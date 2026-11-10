//! Joint INTERNAL seam/interior proposal. Approximate scores never grant G2,
//! curvature, material or body admission. Every exterior two-jet pole stays fixed.
use super::*;
type Score = (f64, f64);
fn better(a: Score, b: Score) -> bool {
    a.0 < b.0 || (a.0 == b.0 && a.1 < b.1)
}
fn overall(patches: &[Score], seams: &[Score]) -> Score {
    patches
        .iter()
        .chain(seams)
        .fold((0f64, 0.), |a, b| (a.0.max(b.0), a.1 + b.1))
}
fn seam_score(a: &Surface, b: &Surface, work: &mut Work) -> Result<Score> {
    let sensitivity = |s: &Surface, end: bool| {
        let p = s.degree_u;
        let (root, second) = if end { (p, p - 2) } else { (0, 2) };
        let h = s.knots_u[p + 1] - s.knots_u[p];
        (p * (p - 1)) as f64 * s.weights[second][0] / s.weights[root][0] / (h * h)
    };
    let (s, end) = if sensitivity(a, true) <= sensitivity(b, false) {
        (a, true)
    } else {
        (b, false)
    };
    let u = if end {
        s.knots_u[s.degree_u + 1]
    } else {
        s.knots_u[s.degree_u]
    };
    let q = s.degree_v;
    let v0 = s.knots_v[q];
    let v1 = s.knots_v[q + 1];
    let mut score = (0f64, 0.);
    // One numerically stable side proposes a score; independent original field
    // checks still cover BOTH actual sides and every seam after construction.
    for t in [0., 0.125, 0.25, 0.5, 0.75, 0.875, 1.] {
        work.charge(1)?;
        let e = s.evaluate(u, v0 + (v1 - v0) * t)?;
        let Some((g, h)) = e.curvatures() else {
            return Ok((f64::INFINITY, f64::INFINITY));
        };
        let k = h.abs() + (h * h - g).max(0.).sqrt();
        if !k.is_finite() {
            return Ok((f64::INFINITY, f64::INFINITY));
        }
        score.0 = score.0.max(k);
        score.1 += k * k;
    }
    Ok(score)
}
fn inside(s: &Surface, stock: [[f64; 2]; 3]) -> bool {
    s.control_points
        .iter()
        .flatten()
        .all(|p| (0..3).all(|k| p[k].is_finite() && stock[k][0] <= p[k] && p[k] <= stock[k][1]))
}
pub(super) fn propose(
    source: &[Surface],
    sweeps: usize,
    stock: [[f64; 2]; 3],
    evaluations: &mut Work,
    repairs: &mut Work,
) -> Result<Vec<Surface>> {
    require(
        (2..=32).contains(&source.len())
            && (1..=8).contains(&sweeps)
            && stock
                .iter()
                .all(|b| b[0].is_finite() && b[1].is_finite() && b[0] <= b[1]),
        "Bound joint seam search",
    )?;
    for s in source {
        bivariate_fairing::validate(s)?;
        require(inside(s, stock), "Joint seam input leaves original stock")?;
    }
    for pair in source.windows(2) {
        require(
            pair[0].knots_v == pair[1].knots_v
                && pair[0].control_points.last() == pair[1].control_points.first(),
            "Joint search lost literal seam ownership",
        )?;
    }
    let mut net = source.to_vec();
    let mut ps = net
        .iter()
        .map(|s| curvature_search::score(s, evaluations))
        .collect::<Result<Vec<_>>>()?;
    let mut ss = net
        .windows(2)
        .map(|p| seam_score(&p[0], &p[1], evaluations))
        .collect::<Result<Vec<_>>>()?;
    let mut best = overall(&ps, &ss);
    let spread = (0..3)
        .map(|k| {
            let lo = net
                .iter()
                .flat_map(|s| s.control_points.iter().flatten())
                .map(|p| p[k])
                .fold(f64::INFINITY, f64::min);
            let hi = net
                .iter()
                .flat_map(|s| s.control_points.iter().flatten())
                .map(|p| p[k])
                .fold(f64::NEG_INFINITY, f64::max);
            hi - lo
        })
        .fold(0f64, f64::max);
    let mut step = (spread / 8.).min(0.1);
    for sweep in 0..sweeps {
        // Alternate scalar and paired V modes. Coupled opposite changes can
        // reduce endpoint curvature without forcing a large midpoint bulge;
        // they must be evaluated atomically, never as two scalar acceptances.
        for kind in 0..3 {
            for i in 0..net.len() - 1 {
                let rows = (3..=net[i].degree_v - 3).collect::<Vec<_>>();
                let mut patterns: Vec<Vec<(usize, f64)>> = Vec::new();
                if sweep % 2 == 0 {
                    for row in rows {
                        patterns.push(vec![(row, 1.)]);
                    }
                } else {
                    for pair in rows.chunks(2) {
                        if pair.len() == 2 {
                            patterns.push(vec![(pair[0], 1.), (pair[1], 1.)]);
                            patterns.push(vec![(pair[0], 1.), (pair[1], -1.)]);
                        } else {
                            patterns.push(vec![(pair[0], 1.)]);
                        }
                    }
                }
                for pattern in patterns {
                    for axis in 0..3 {
                        let original = [net[i].clone(), net[i + 1].clone()];
                        let mut parameters = Vec::new();
                        for &(row, sign) in &pattern {
                            let sensitivity = |s: &Surface, end: bool| {
                                let p = s.degree_u;
                                let (root, second) = if end { (p, p - 2) } else { (0, 2) };
                                let h = s.knots_u[p + 1] - s.knots_u[p];
                                (p * (p - 1)) as f64 * s.weights[second][row]
                                    / s.weights[root][row]
                                    / (h * h)
                            };
                            let reference = usize::from(
                                sensitivity(&original[0], true) > sensitivity(&original[1], false),
                            );
                            let r = &original[reference];
                            let root = if reference == 0 { r.degree_u } else { 0 };
                            let id = if reference == 0 {
                                r.degree_u - kind
                            } else {
                                kind
                            };
                            let old = if kind == 0 {
                                original[1].control_points[0][row][axis]
                            } else {
                                r.control_points[id][row][axis]
                            };
                            let h = r.knots_u[r.degree_u + 1] - r.knots_u[r.degree_u];
                            let scaling = match kind {
                                1 => h * r.weights[root][row] / r.weights[id][row],
                                2 => h * h * r.weights[root][row] / r.weights[id][row],
                                _ => 1.,
                            };
                            require(
                                scaling.is_finite() && scaling > 0.,
                                "Joint jet step overflow",
                            )?;
                            let block = if pattern.len() > 1 {
                                match kind {
                                    1 => 4.,
                                    2 => 16.,
                                    _ => 1.,
                                }
                            } else {
                                1.
                            };
                            parameters.push((row, old, sign * scaling * block));
                        }
                        let mut chosen = original.clone();
                        let mut chosen_ps = [ps[i], ps[i + 1]];
                        let mut chosen_ss = ss[i];
                        let mut chosen_score = best;
                        for direction in [-1., 1.] {
                            let values = parameters
                                .iter()
                                .map(|&(row, old, scale)| {
                                    (
                                        row,
                                        old,
                                        (old + direction * step * scale)
                                            .clamp(stock[axis][0], stock[axis][1]),
                                    )
                                })
                                .collect::<Vec<_>>();
                            if values.iter().all(|(_, old, v)| old == v) {
                                continue;
                            }
                            require(
                                repairs.max - repairs.used >= 1400 * values.len(),
                                "Joint seam repair allowance exhausted",
                            )?;
                            let mut trial = original.clone();
                            let mut valid = true;
                            for &(row, old, value) in &values {
                                if value != old
                                    && endpoint_jets::move_free_seam_control(
                                        &mut trial, row, axis, kind, value, repairs,
                                    )
                                    .is_err()
                                {
                                    valid = false;
                                    break;
                                }
                            }
                            if !valid || !trial.iter().all(|s| inside(s, stock)) {
                                continue;
                            }
                            let trial_ps = [
                                curvature_search::score(&trial[0], evaluations)?,
                                curvature_search::score(&trial[1], evaluations)?,
                            ];
                            let trial_ss = seam_score(&trial[0], &trial[1], evaluations)?;
                            let mut all_ps = ps.clone();
                            all_ps[i] = trial_ps[0];
                            all_ps[i + 1] = trial_ps[1];
                            let mut all_ss = ss.clone();
                            all_ss[i] = trial_ss;
                            let score = overall(&all_ps, &all_ss);
                            if better(score, chosen_score) {
                                chosen = trial;
                                chosen_ps = trial_ps;
                                chosen_ss = trial_ss;
                                chosen_score = score;
                            }
                        }
                        net[i] = chosen[0].clone();
                        net[i + 1] = chosen[1].clone();
                        ps[i] = chosen_ps[0];
                        ps[i + 1] = chosen_ps[1];
                        ss[i] = chosen_ss;
                        best = chosen_score;
                    }
                }
            }
        }
        // Free interiors follow the moved seams. Boundary two-jets are never
        // included in this coordinate set; no post-hoc source snapping occurs.
        for p in 0..net.len() {
            for i in 3..=net[p].degree_u - 3 {
                for j in 3..=net[p].degree_v - 3 {
                    for k in 0..3 {
                        let old = net[p].control_points[i][j][k];
                        let mut chosen = old;
                        let mut chosen_ps = ps[p];
                        let mut chosen_score = best;
                        for direction in [-1., 1.] {
                            let value = (old + direction * step).clamp(stock[k][0], stock[k][1]);
                            if value == old {
                                continue;
                            }
                            net[p].control_points[i][j][k] = value;
                            let score = curvature_search::score(&net[p], evaluations)?;
                            let mut all = ps.clone();
                            all[p] = score;
                            let total = overall(&all, &ss);
                            if better(total, chosen_score) {
                                chosen = value;
                                chosen_ps = score;
                                chosen_score = total;
                            }
                        }
                        net[p].control_points[i][j][k] = chosen;
                        ps[p] = chosen_ps;
                        best = chosen_score;
                    }
                }
            }
        }
        step *= 0.5;
    }
    require(
        best.0.is_finite(),
        "Joint seam search has no regular sampled candidate",
    )?;
    for s in &net {
        s.validate()?;
    }
    Ok(net)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Vec<Surface> {
        [-1., 0.]
            .map(|start| Surface {
                degree_u: 6,
                degree_v: 7,
                knots_u: [vec![start; 7], vec![start + 1.; 7]].concat(),
                knots_v: [vec![0.; 8], vec![1.; 8]].concat(),
                control_points: (0..7)
                    .map(|i| {
                        (0..8)
                            .map(|j| {
                                vec![
                                    start + i as f64 / 6.,
                                    j as f64 / 7.,
                                    if (j == 3 || j == 4)
                                        && (if start < 0. { i >= 4 } else { i <= 2 })
                                    {
                                        0.05
                                    } else {
                                        0.
                                    },
                                ]
                            })
                            .collect()
                    })
                    .collect(),
                weights: vec![vec![1.; 8]; 7],
                periodic_u: false,
                periodic_v: false,
            })
            .to_vec()
    }
    #[test]
    fn actual_shared_curve_changes_with_external_two_jets_and_g2_preserved() {
        let s = fixture();
        let before = s.clone();
        let old = seam_score(
            &s[0],
            &s[1],
            &mut Work {
                used: 0,
                max: 100000,
            },
        )
        .unwrap();
        let q = propose(
            &s,
            4,
            [[-2., 2.], [0., 1.], [0., 1.]],
            &mut Work {
                used: 0,
                max: 100000,
            },
            &mut Work {
                used: 0,
                max: 100000,
            },
        )
        .unwrap();
        let new = seam_score(
            &q[0],
            &q[1],
            &mut Work {
                used: 0,
                max: 100000,
            },
        )
        .unwrap();
        assert!(new.0 < old.0 / 10., "{old:?} -> {new:?}");
        assert_ne!(q[0].control_points[6], s[0].control_points[6]);
        assert_eq!(q[0].control_points.last(), q[1].control_points.first());
        for p in 0..2 {
            for i in 0..7 {
                for j in [0, 1, 2, 5, 6, 7] {
                    assert_eq!(q[p].control_points[i][j], s[p].control_points[i][j]);
                }
            }
            assert_eq!(q[p].weights, s[p].weights);
        }
        assert_eq!(q[0].control_points[0], s[0].control_points[0]);
        assert_eq!(q[1].control_points[6], s[1].control_points[6]);
        let proof = nurbs_core::surface_g2_pcurves::inspect_original_patch_pair(
            &q[0], &q[1], 1e-5, 1e-4, 2000,
        )
        .unwrap();
        assert_eq!(
            proof.decision,
            nurbs_core::surface_join::Decision::WithinTolerance
        );
        assert!(proof.regularity_proven && proof.pending == 0);
        assert_eq!(s, before);
    }
    #[test]
    fn degree9_network_keeps_external_rows_and_owned_g2_seam() {
        let source = fixture();
        let elevated = interior_fairing::elevate_v(
            &source,
            9,
            &mut Work {
                used: 0,
                max: 10000,
            },
        )
        .unwrap();
        for (s, t) in source.iter().zip(&elevated) {
            for u in [s.knots_u[6], s.knots_u[7]] {
                for v in [0., 0.125, 0.5, 0.875, 1.] {
                    let a = s.evaluate(u, v).unwrap().point;
                    let b = t.evaluate(u, v).unwrap().point;
                    for k in 0..3 {
                        assert!((a[k] - b[k]).abs() < 1e-13);
                    }
                }
            }
        }
        let result = propose(
            &elevated,
            3,
            [[-2., 2.], [0., 1.], [0., 1.]],
            &mut Work {
                used: 0,
                max: 100000,
            },
            &mut Work {
                used: 0,
                max: 100000,
            },
        )
        .unwrap();
        for (s, t) in elevated.iter().zip(&result) {
            assert_eq!(t.degree_v, 9);
            for i in 0..7 {
                for j in [0, 1, 2, 7, 8, 9] {
                    assert_eq!(s.control_points[i][j], t.control_points[i][j]);
                }
            }
        }
        assert_eq!(
            result[0].control_points.last(),
            result[1].control_points.first()
        );
        let proof = nurbs_core::surface_g2_pcurves::inspect_original_patch_pair(
            &result[0], &result[1], 1e-5, 1e-4, 2000,
        )
        .unwrap();
        assert_eq!(
            proof.decision,
            nurbs_core::surface_join::Decision::WithinTolerance
        );
        assert!(proof.regularity_proven && proof.pending == 0);
    }
    #[test]
    fn exhausted_joint_search_does_not_publish_a_partial_network() {
        let s = fixture();
        let before = s.clone();
        assert!(
            propose(
                &s,
                4,
                [[-2., 2.], [0., 1.], [0., 1.]],
                &mut Work { used: 0, max: 1 },
                &mut Work {
                    used: 0,
                    max: 100000
                }
            )
            .is_err()
        );
        assert!(
            propose(
                &s,
                4,
                [[-2., 2.], [0., 1.], [0., 1.]],
                &mut Work {
                    used: 0,
                    max: 100000
                },
                &mut Work { used: 0, max: 0 }
            )
            .is_err()
        );
        assert_eq!(s, before);
    }
}

#[cfg(test)]
mod common_jet_tests {
    use super::*;
    #[test]
    fn common_second_jet_changes_without_moving_the_owned_curve_or_external_rows() {
        let mut p = [-1., 0.]
            .map(|start| Surface {
                degree_u: 6,
                degree_v: 7,
                knots_u: [vec![start; 7], vec![start + 1.; 7]].concat(),
                knots_v: [vec![0.; 8], vec![1.; 8]].concat(),
                control_points: (0..7)
                    .map(|i| {
                        (0..8)
                            .map(|j| {
                                vec![
                                    start + i as f64 / 6.,
                                    j as f64 / 7.,
                                    if (j == 3 || j == 4) && i == (if start < 0. { 4 } else { 2 }) {
                                        0.05
                                    } else {
                                        0.
                                    },
                                ]
                            })
                            .collect()
                    })
                    .collect(),
                weights: vec![vec![1.; 8]; 7],
                periodic_u: false,
                periodic_v: false,
            })
            .to_vec();
        let before = p.clone();
        for row in [3, 4] {
            endpoint_jets::move_free_seam_control(
                &mut p,
                row,
                2,
                2,
                0.,
                &mut Work {
                    used: 0,
                    max: 10000,
                },
            )
            .unwrap();
        }
        assert_eq!(p[0].control_points[6], before[0].control_points[6]);
        assert_eq!(p[1].control_points[0], before[1].control_points[0]);
        for i in 0..2 {
            for u in 0..7 {
                for v in [0, 1, 2, 5, 6, 7] {
                    assert_eq!(p[i].control_points[u][v], before[i].control_points[u][v]);
                }
            }
        }
        let r = nurbs_core::surface_g2_pcurves::inspect_original_patch_pair(
            &p[0], &p[1], 1e-8, 1e-8, 2000,
        )
        .unwrap();
        assert_eq!(
            r.decision,
            nurbs_core::surface_join::Decision::WithinTolerance
        );
        assert!(r.regularity_proven && r.pending == 0);
        assert!(
            p.iter()
                .flat_map(|s| s.control_points.iter().flatten())
                .all(|v| v[2] == 0.)
        );
    }
}
