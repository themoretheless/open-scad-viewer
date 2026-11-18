//! Compensated endpoint authorship for the NEW product net, not proof arithmetic.
//! Solve P2 using the ACTUALLY STORED P1; otherwise its rounding error is amplified
//! by a tiny rational endpoint weight. All G0/G2/cap gates remain external.
use super::*;
#[derive(Clone, Copy)]
struct D(f64, f64);
impl D {
    fn new(v: f64) -> Self {
        Self(v, 0.)
    }
    fn add(self, b: Self) -> Self {
        let s = self.0 + b.0;
        let v = s - self.0;
        let e = (self.0 - (s - v)) + (b.0 - v) + self.1 + b.1;
        let h = s + e;
        Self(h, e - (h - s))
    }
    fn neg(self) -> Self {
        Self(-self.0, -self.1)
    }
    fn sub(self, b: Self) -> Self {
        self.add(b.neg())
    }
    fn mul(self, b: Self) -> Self {
        let p = self.0 * b.0;
        let e = self.0.mul_add(b.0, -p) + self.0 * b.1 + self.1 * b.0 + self.1 * b.1;
        let h = p + e;
        Self(h, e - (h - p))
    }
    fn div(self, b: Self) -> Self {
        let q = Self::new(self.0 / b.0);
        let r = self.sub(b.mul(q));
        q.add(Self::new((r.0 + r.1) / b.0))
    }
    fn value(self) -> f64 {
        self.0 + self.1
    }
}
fn jets(p: usize, points: [f64; 3], weights: [f64; 3]) -> [D; 2] {
    let p = D::new(p as f64);
    let r1 = D::new(weights[1]).div(D::new(weights[0]));
    let r2 = D::new(weights[2]).div(D::new(weights[0]));
    let first = p.mul(r1).mul(D::new(points[1]).sub(D::new(points[0])));
    let second = p
        .mul(p.sub(D::new(1.)))
        .mul(r2)
        .mul(D::new(points[2]).sub(D::new(points[0])))
        .sub(D::new(2.).mul(p.mul(r1).sub(D::new(1.))).mul(first));
    [first, second]
}
/// Same original cell clock. Only proposed Cartesian coefficients are changed;
/// weights, boundary roots and source definitions are immutable.
pub(super) fn restore_authored(
    source: &Surface,
    start: usize,
    span: usize,
    points: &mut [Vec<Vec<f64>>],
    weights: &[f64],
    work: &mut Work,
    authored: Option<&layers::AuthoredRows>,
) -> Result<()> {
    let p = source.degree_u;
    let q = points.len() - 1;
    if q < 5 {
        return Ok(());
    } // two endpoint 2-jets must have disjoint unknowns
    for (ids, out) in [
        ([start, start + 1, start + 2], [0, 1, 2]),
        ([span, span - 1, span - 2], [q, q - 1, q - 2]),
    ] {
        let r1 = D::new(weights[out[1]]).div(D::new(weights[out[0]]));
        let r2 = D::new(weights[out[2]]).div(D::new(weights[out[0]]));
        let degree = D::new(q as f64);
        for row in 0..6 {
            let source_weights = ids.map(|i| source.weights[i][row]);
            for axis in 0..3 {
                let original = ids.map(|i| source.control_points[i][row][axis]);
                let target = if let Some(a) = authored {
                    let side = usize::from(row >= 3);
                    let gain = if side == 0 {
                        row as f64 * a.gain
                    } else {
                        (5 - row) as f64 * a.gain
                    };
                    let base = &a.base[side];
                    let field = &a.field[side];
                    let b = jets(
                        p,
                        ids.map(|i| base.control_points[i][axis]),
                        ids.map(|i| base.weights[i]),
                    );
                    let f = jets(
                        p,
                        ids.map(|i| field.control_points[i][axis]),
                        ids.map(|i| field.weights[i]),
                    );
                    [
                        b[0].add(D::new(gain).mul(f[0])),
                        b[1].add(D::new(gain).mul(f[1])),
                    ]
                } else {
                    jets(p, original, source_weights)
                };
                let root = D::new(points[out[0]][row][axis]);
                let first = root.add(target[0].div(degree.mul(r1))).value();
                require(first.is_finite(), "Product endpoint first jet overflow")?;
                points[out[1]][row][axis] = first;
                // Do NOT reuse target[0] here: P1 has now rounded. Form the
                // true first jet of the stored proposal before solving P2.
                let actual = degree.mul(r1).mul(D::new(first).sub(root));
                let numerator =
                    target[1].add(D::new(2.).mul(degree.mul(r1).sub(D::new(1.))).mul(actual));
                let second = root
                    .add(numerator.div(degree.mul(degree.sub(D::new(1.))).mul(r2)))
                    .value();
                require(second.is_finite(), "Product endpoint second jet overflow")?;
                points[out[2]][row][axis] = second;
                work.charge(80)?;
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn compensated_products_keep_cancellation_and_stored_first_jet() {
        let a = D::new(1e12).add(D::new(0.125));
        assert_eq!(a.sub(D::new(1e12)).value(), 0.125);
        let original = [0.1, 0.1 - 1e-10, 0.];
        let weights = [1e-9, 0.25, 0.23570226039551584];
        let target = jets(4, original, weights);
        let q = D::new(10.);
        let r1 = D::new(1.13e8);
        let r2 = D::new(2e8);
        let root = D::new(original[0]);
        let first = root.add(target[0].div(q.mul(r1))).value();
        let actual = q.mul(r1).mul(D::new(first).sub(root));
        assert!(actual.sub(target[0]).value().abs() > 1e-12);
        let solve = |d1: D| {
            root.add(
                target[1]
                    .add(D::new(2.).mul(q.mul(r1).sub(D::new(1.))).mul(d1))
                    .div(q.mul(q.sub(D::new(1.))).mul(r2)),
            )
            .value()
        };
        let good = jets(10, [original[0], first, solve(actual)], [1., 1.13e8, 2e8]);
        let bad = jets(
            10,
            [original[0], first, solve(target[0])],
            [1., 1.13e8, 2e8],
        );
        assert!(good[1].sub(target[1]).value().abs() < 1e-6);
        assert!(bad[1].sub(target[1]).value().abs() > 1.);
    }
}

fn motion_upper(old: f64, new: f64, spent: f64) -> Result<f64> {
    use nurbs_core::interval_eval::Interval as I;
    if old == new {
        return Ok(spent);
    }
    let d = I::point(new).sub(I::point(old))?;
    Ok(I::point(spent)
        .add(I::point(d.lo.abs().max(d.hi.abs())))?
        .hi)
}
// A neighbouring P1 float can avoid a large compensating P2 displacement.
// This is a finite NEW-proposal search, never a first-jet/G2 certificate.
/// The clamped endpoint chart: root parameter, degree and the first two
/// reciprocal knot spans.
#[derive(Clone, Copy)]
struct EndpointChart {
    root: D,
    degree: D,
    r1: D,
    r2: D,
}
fn choose_endpoint(
    chart: EndpointChart,
    [first, second]: [D; 2],
    old: [f64; 2],
    spent: [f64; 2],
    budget: Option<f64>,
    work: &mut Work,
) -> Result<([f64; 2], [f64; 2])> {
    let EndpointChart {
        root,
        degree,
        r1,
        r2,
    } = chart;
    let nearest = root.add(first.div(degree.mul(r1))).value();
    let trial = |p1: f64| -> Result<Option<([f64; 2], [f64; 2], f64)>> {
        let actual = degree.mul(r1).mul(D::new(p1).sub(root));
        let p2 = root
            .add(
                second
                    .add(D::new(2.).mul(degree.mul(r1).sub(D::new(1.))).mul(actual))
                    .div(degree.mul(degree.sub(D::new(1.))).mul(r2)),
            )
            .value();
        if !p1.is_finite() || !p2.is_finite() {
            return Ok(None);
        }
        let values = [p1, p2];
        let mut motion = spent;
        if let Some(limit) = budget {
            for k in 0..2 {
                motion[k] = motion_upper(old[k], values[k], spent[k])?;
            }
            if motion.iter().any(|&m| m > limit) {
                return Ok(None);
            }
        }
        Ok(Some((values, motion, actual.sub(first).value().abs())))
    };
    if let Some((p, m, _)) = trial(nearest)? {
        return Ok((p, m));
    }
    let mut best: Option<([f64; 2], [f64; 2], f64)> = None;
    let mut lower = nearest;
    let mut upper = nearest;
    for _ in 0..3 {
        lower = lower.next_down();
        upper = upper.next_up();
        for p1 in [lower, upper] {
            work.charge(100)?;
            if let Some(candidate) = trial(p1)? {
                if best.as_ref().is_none_or(|b| candidate.2 < b.2) {
                    best = Some(candidate);
                }
            }
        }
    }
    best.map(|(p,m,_)|(p,m)).ok_or_else(||refuse("Coupled endpoint jets exceed the unchanged positional proposal budget after bounded float search"))
}
/// NEW coupled proposal: common physical first/second cross-jets at each
/// shared polynomial seam. Roots/weights remain fixed. Source agreement and
/// actual post-rounding G2 must be rechecked; this routine grants neither.
pub(super) fn reconcile(
    patches: &mut [Surface],
    maximum_change: f64,
    work: &mut Work,
) -> Result<()> {
    require(
        maximum_change.is_finite() && maximum_change > 0.,
        "Bound coupled jet displacement",
    )?;
    for s in patches.iter() {
        s.validate()?;
        let p = s.degree_u;
        require(
            p >= 5
                && s.control_points.len() == p + 1
                && (5..=16).contains(&s.degree_v)
                && s.control_points[0].len() == s.degree_v + 1
                && s.control_points[0][0].len() == 3
                && !s.periodic_u
                && !s.periodic_v
                && s.knots_u[..=p].iter().all(|&u| u == s.knots_u[p])
                && s.knots_u[p + 1..].iter().all(|&u| u == s.knots_u[p + 1])
                && s.weights.iter().all(|row| row.iter().all(|&w| w == row[0])),
            "Coupled jets require original common-denominator Bezier patches",
        )?;
    }
    for i in 0..patches.len().saturating_sub(1) {
        let (left, right) = patches.split_at_mut(i + 1);
        let a = &mut left[i];
        let b = &mut right[0];
        let pa = a.degree_u;
        let pb = b.degree_u;
        require(
            pa >= 5
                && pb >= 5
                && a.control_points.last() == b.control_points.first()
                && a.knots_v == b.knots_v,
            "Coupled jets need owned disjoint endpoint columns",
        )?;
        let ha = D::new(a.knots_u[pa + 1]).sub(D::new(a.knots_u[pa]));
        let hb = D::new(b.knots_u[pb + 1]).sub(D::new(b.knots_u[pb]));
        // Prefer the endpoint where a unit jet correction moves Cartesian
        // poles most. Averaging a badly conditioned endpoint's acceleration
        // into its stable neighbor can require a macroscopic shape change.
        // This is only a target proposal; budget and original G2 are gates.
        let sensitivity = |s: &Surface, end: bool, h: D| {
            let p = s.degree_u;
            let (root, second) = if end { (p, p - 2) } else { (0, 2) };
            D::new((p * (p - 1)) as f64)
                .mul(D::new(s.weights[second][0]))
                .div(D::new(s.weights[root][0]))
                .div(h.mul(h))
                .value()
        };
        let ka = sensitivity(a, true, ha);
        let kb = sensitivity(b, false, hb);
        require(
            ka.is_finite() && kb.is_finite() && ka > 0. && kb > 0.,
            "Coupled jet sensitivity overflow",
        )?;
        let reference = if ka / kb > 16. {
            1
        } else if kb / ka > 16. {
            0
        } else {
            2
        };
        for row in 0..a.control_points[0].len() {
            // Outward L1 displacement per ACTUAL control point. Giving each
            // axis only one third of the allowance needlessly rejects a
            // one-axis correction that satisfies the same geometric budget.
            let mut motion = [[0f64; 2]; 2];
            for axis in 0..3 {
                let ja = jets(
                    pa,
                    [pa, pa - 1, pa - 2].map(|j| a.control_points[j][row][axis]),
                    [pa, pa - 1, pa - 2].map(|j| a.weights[j][row]),
                );
                let jb = jets(
                    pb,
                    [0, 1, 2].map(|j| b.control_points[j][row][axis]),
                    [0, 1, 2].map(|j| b.weights[j][row]),
                );
                let first_a = ja[0].neg().div(ha);
                let first_b = jb[0].div(hb);
                let second_a = ja[1].div(ha.mul(ha));
                let second_b = jb[1].div(hb.mul(hb));
                let first = match reference {
                    0 => first_a,
                    1 => first_b,
                    _ => first_a.add(first_b).mul(D::new(0.5)),
                };
                let second = match reference {
                    0 => second_a,
                    1 => second_b,
                    _ => second_a.add(second_b).mul(D::new(0.5)),
                };
                for (side, (s, ids, h, sign)) in [
                    (&mut *a, [pa, pa - 1, pa - 2], ha, -1.),
                    (&mut *b, [0, 1, 2], hb, 1.),
                ]
                .into_iter()
                .enumerate()
                {
                    let degree = D::new(s.degree_u as f64);
                    let root = D::new(s.control_points[ids[0]][row][axis]);
                    let r1 = D::new(s.weights[ids[1]][row]).div(D::new(s.weights[ids[0]][row]));
                    let r2 = D::new(s.weights[ids[2]][row]).div(D::new(s.weights[ids[0]][row]));
                    let target_first = first.mul(h).mul(D::new(sign));
                    let boundary_row = row < 3 || row + 3 >= s.control_points[0].len();
                    let (values, updated) = choose_endpoint(EndpointChart { root: root, degree: degree, r1: r1, r2: r2 }, [target_first, second.mul(h.mul(h))], [
                            s.control_points[ids[1]][row][axis],
                            s.control_points[ids[2]][row][axis],
                        ], motion[side], boundary_row.then_some(maximum_change), work)?;
                    motion[side] = updated;
                    s.control_points[ids[1]][row][axis] = values[0];
                    s.control_points[ids[2]][row][axis] = values[1];
                    work.charge(100)?;
                }
            }
        }
    }
    for s in patches {
        s.validate()?;
    }
    Ok(())
}

#[cfg(test)]
mod reconciliation_tests {
    use super::*;
    fn pair() -> Vec<Surface> {
        [0., 1.]
            .map(|start| Surface {
                degree_u: 5,
                degree_v: 5,
                knots_u: [vec![start; 6], vec![start + 1.; 6]].concat(),
                knots_v: [vec![0.; 6], vec![1.; 6]].concat(),
                control_points: (0..6)
                    .map(|i| {
                        (0..6)
                            .map(|j| {
                                vec![
                                    start + i as f64 / 5.,
                                    j as f64 / 5.,
                                    if start == 1. && i == 2 {
                                        2f64.powi(-20)
                                    } else {
                                        0.
                                    },
                                ]
                            })
                            .collect()
                    })
                    .collect(),
                weights: vec![vec![1.; 6]; 6],
                periodic_u: false,
                periodic_v: false,
            })
            .to_vec()
    }
    #[test]
    fn coupled_original_jets_fix_a_real_curvature_jump_without_moving_roots_or_weights() {
        let mut p = pair();
        let before = p.clone();
        let inspect = |p: &[Surface]| {
            nurbs_core::surface_g2_pcurves::inspect_original_patch_pair(
                &p[0], &p[1], 1e-8, 1e-8, 1000,
            )
            .unwrap()
        };
        assert_eq!(
            inspect(&p).decision,
            nurbs_core::surface_join::Decision::CorrespondenceExceedsTolerance
        );
        reconcile(
            &mut p,
            1e-6,
            &mut Work {
                used: 0,
                max: 100000,
            },
        )
        .unwrap();
        let result = inspect(&p);
        assert_eq!(
            result.decision,
            nurbs_core::surface_join::Decision::WithinTolerance
        );
        assert!(result.regularity_proven && result.pending == 0);
        assert!(
            (p[1].control_points[2][0][2] - before[1].control_points[2][0][2]).abs() > 1e-6 / 3.
        );
        for i in 0..2 {
            for (a, b) in p[i]
                .control_points
                .iter()
                .flatten()
                .zip(before[i].control_points.iter().flatten())
            {
                assert!(a.iter().zip(b).map(|(a, b)| (a - b).abs()).sum::<f64>() <= 1e-6);
            }
        }
        for i in 0..2 {
            assert_eq!(p[i].weights, before[i].weights);
            assert_eq!(p[i].control_points[0], before[i].control_points[0]);
            assert_eq!(p[i].control_points[5], before[i].control_points[5]);
            for (a, b) in p[i]
                .control_points
                .iter()
                .flatten()
                .flatten()
                .zip(before[i].control_points.iter().flatten().flatten())
            {
                assert!((a - b).abs() <= 1e-4 / 3.);
            }
        }
    }
    #[test]
    fn exhausted_work_and_too_large_geometric_change_refuse_the_coupled_candidate() {
        assert!(reconcile(&mut pair(), 1e-4, &mut Work { used: 0, max: 0 }).is_err());
        assert!(
            reconcile(
                &mut pair(),
                1e-20,
                &mut Work {
                    used: 0,
                    max: 100000
                }
            )
            .is_err()
        );
        // Each component fits 7e-7, but their combined actual displacement
        // does not. A per-axis full allowance must not sneak past the gate.
        let mut diagonal = pair();
        for point in &mut diagonal[1].control_points[2] {
            point[1] += 2f64.powi(-20);
        }
        assert!(
            reconcile(
                &mut diagonal,
                7e-7,
                &mut Work {
                    used: 0,
                    max: 100000
                }
            )
            .is_err()
        );
        let mut p = pair();
        p[0].weights[2][1] = 2.;
        assert!(
            reconcile(
                &mut p,
                1e-4,
                &mut Work {
                    used: 0,
                    max: 100000
                }
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod free_row_tests {
    use super::*;
    #[test]
    fn free_interior_rows_can_reconcile_without_spending_boundary_position_budget() {
        let mut patches = [0., 1.]
            .map(|start| Surface {
                degree_u: 5,
                degree_v: 7,
                knots_u: [vec![start; 6], vec![start + 1.; 6]].concat(),
                knots_v: [vec![0.; 8], vec![1.; 8]].concat(),
                control_points: (0..6)
                    .map(|i| {
                        (0..8)
                            .map(|j| {
                                vec![
                                    start + i as f64 / 5.,
                                    j as f64 / 7.,
                                    if start == 1. && i == 2 && j == 3 {
                                        2f64.powi(-10)
                                    } else {
                                        0.
                                    },
                                ]
                            })
                            .collect()
                    })
                    .collect(),
                weights: vec![vec![1.; 8]; 6],
                periodic_u: false,
                periodic_v: false,
            })
            .to_vec();
        let before = patches.clone();
        reconcile(
            &mut patches,
            1e-7,
            &mut Work {
                used: 0,
                max: 100000,
            },
        )
        .unwrap();
        let r = nurbs_core::surface_g2_pcurves::inspect_original_patch_pair(
            &patches[0],
            &patches[1],
            1e-8,
            1e-8,
            2000,
        )
        .unwrap();
        assert_eq!(
            r.decision,
            nurbs_core::surface_join::Decision::WithinTolerance
        );
        assert!(r.regularity_proven && r.pending == 0);
        assert!(
            (patches[1].control_points[2][3][2] - before[1].control_points[2][3][2]).abs() > 1e-7
        );
        for i in 0..2 {
            assert_eq!(patches[i].weights, before[i].weights);
            assert_eq!(patches[i].control_points[0], before[i].control_points[0]);
            assert_eq!(patches[i].control_points[5], before[i].control_points[5]);
            for u in 0..6 {
                for v in [0, 1, 2, 5, 6, 7] {
                    for k in 0..3 {
                        assert!(
                            (patches[i].control_points[u][v][k]
                                - before[i].control_points[u][v][k])
                                .abs()
                                <= 1e-7 / 3.
                        );
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod float_choice_tests {
    use super::*;
    #[test]
    fn neighbouring_float_can_fit_the_same_geometric_budget_without_moving_the_root() {
        let root = 0.1;
        let p1 = 0.1 - 1e-10;
        let old_second = 0.025;
        let intended = old_second + 2.77e-8;
        let target = jets(10, [root, p1, intended], [1., 1e9, 1e9]);
        let (p, m) = choose_endpoint(
            EndpointChart {
                root: D::new(root),
                degree: D::new(10.),
                r1: D::new(1e9),
                r2: D::new(1e9),
            },
            [target[0], target[1]],
            [p1, old_second],
            [0.; 2],
            Some(2.5e-8),
            &mut Work { used: 0, max: 1000 },
        )
        .unwrap();
        assert_ne!(p[0], p1);
        assert!((p[1] - old_second).abs() <= 2.5e-8);
        assert!(m.iter().all(|&x| x <= 2.5e-8));
        let actual = jets(10, [root, p[0], p[1]], [1., 1e9, 1e9]);
        assert!(actual[0].sub(target[0]).value().abs() < 2e-7);
        assert!(
            choose_endpoint(
                EndpointChart {
                    root: D::new(root),
                    degree: D::new(10.),
                    r1: D::new(1e9),
                    r2: D::new(1e9),
                },
                [target[0], target[1]],
                [p1, old_second],
                [0.; 2],
                Some(2.5e-8),
                &mut Work { used: 0, max: 0 },
            )
            .is_err()
        );
    }
}

/// Move one coefficient of an OWNED INTERNAL seam curve, first jet or second
/// jet and reauthor its two adjacent endpoint jets. Only free V rows are eligible; original external two-jet rows
/// and all other coordinates are unchanged. Proposal only, no G2 authority.
pub(super) fn move_free_seam_control(
    pair: &mut [Surface],
    row: usize,
    axis: usize,
    kind: usize,
    value: f64,
    work: &mut Work,
) -> Result<()> {
    require(
        pair.len() == 2 && axis < 3 && kind <= 2 && value.is_finite(),
        "Bound shared seam component",
    )?;
    let (left, right) = pair.split_at_mut(1);
    let a = &mut left[0];
    let b = &mut right[0];
    let pa = a.degree_u;
    let pb = b.degree_u;
    require(
        a.control_points.last() == b.control_points.first()
            && a.knots_v == b.knots_v
            && row >= 3
            && row + 3 < a.control_points[0].len(),
        "Only owned free seam coefficients may move",
    )?;
    work.charge(200)?;
    let ha = D::new(a.knots_u[pa + 1]).sub(D::new(a.knots_u[pa]));
    let hb = D::new(b.knots_u[pb + 1]).sub(D::new(b.knots_u[pb]));
    let sensitivity = |s: &Surface, end: bool, h: D| {
        let p = s.degree_u;
        let (root, second) = if end { (p, p - 2) } else { (0, 2) };
        D::new((p * (p - 1)) as f64)
            .mul(D::new(s.weights[second][row]))
            .div(D::new(s.weights[root][row]))
            .div(h.mul(h))
            .value()
    };
    let ka = sensitivity(a, true, ha);
    let kb = sensitivity(b, false, hb);
    require(
        ka.is_finite() && kb.is_finite() && ka > 0. && kb > 0.,
        "Shared seam sensitivity overflow",
    )?;
    // Targets are captured BEFORE moving the root. Differentiating a moved
    // root with old neighbours would manufacture enormous rational jets.
    let ja = jets(
        pa,
        [pa, pa - 1, pa - 2].map(|i| a.control_points[i][row][axis]),
        [pa, pa - 1, pa - 2].map(|i| a.weights[i][row]),
    );
    let jb = jets(
        pb,
        [0, 1, 2].map(|i| b.control_points[i][row][axis]),
        [0, 1, 2].map(|i| b.weights[i][row]),
    );
    let first_a = ja[0].neg().div(ha);
    let first_b = jb[0].div(hb);
    let second_a = ja[1].div(ha.mul(ha));
    let second_b = jb[1].div(hb.mul(hb));
    let (mut first, mut second) = if ka / kb > 16. {
        (first_b, second_b)
    } else if kb / ka > 16. {
        (first_a, second_a)
    } else {
        (
            first_a.add(first_b).mul(D::new(0.5)),
            second_a.add(second_b).mul(D::new(0.5)),
        )
    };
    if kind > 0 {
        let (reference, ids, h, sign, base) = if ka <= kb {
            (&*a, [pa, pa - 1, pa - 2], ha, -1., ja)
        } else {
            (&*b, [0, 1, 2], hb, 1., jb)
        };
        let mut coordinates = ids.map(|i| reference.control_points[i][row][axis]);
        coordinates[kind] = value;
        let modified = jets(
            reference.degree_u,
            coordinates,
            ids.map(|i| reference.weights[i][row]),
        );
        if kind == 1 {
            first = first.add(modified[0].sub(base[0]).mul(D::new(sign)).div(h));
        } else {
            second = second.add(modified[1].sub(base[1]).div(h.mul(h)));
        }
    }
    let new_root = if kind == 0 {
        value
    } else {
        a.control_points[pa][row][axis]
    };
    for (s, ids, h, sign) in [
        (&mut *a, [pa, pa - 1, pa - 2], ha, -1.),
        (&mut *b, [0, 1, 2], hb, 1.),
    ] {
        let root = D::new(new_root);
        let degree = D::new(s.degree_u as f64);
        let r1 = D::new(s.weights[ids[1]][row]).div(D::new(s.weights[ids[0]][row]));
        let r2 = D::new(s.weights[ids[2]][row]).div(D::new(s.weights[ids[0]][row]));
        let (p, _) = choose_endpoint(EndpointChart { root: root, degree: degree, r1: r1, r2: r2 }, [first.mul(h).mul(D::new(sign)), second.mul(h.mul(h))], [
                s.control_points[ids[1]][row][axis],
                s.control_points[ids[2]][row][axis],
            ], [0.; 2], None, work)?;
        s.control_points[ids[0]][row][axis] = new_root;
        s.control_points[ids[1]][row][axis] = p[0];
        s.control_points[ids[2]][row][axis] = p[1];
    }
    Ok(())
}
