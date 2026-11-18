//! Sampled rational tensor-bending proposal, never curvature/solid evidence.
//! Only poles at least three rows from EVERY boundary are free. Original
//! boundary two-jets, seam root columns, weights and clocks stay literal.
use super::*;
fn bernstein(p: usize, t: f64) -> Vec<f64> {
    let mut b = vec![1.];
    for _ in 0..p {
        let mut next = vec![0.; b.len() + 1];
        for (i, x) in b.iter().enumerate() {
            next[i] += (1. - t) * x;
            next[i + 1] += t * x;
        }
        b = next;
    }
    b
}
fn basis(p: usize, t: f64) -> [Vec<f64>; 3] {
    let b = bernstein(p, t);
    let l = bernstein(p - 1, t);
    let ll = bernstein(p - 2, t);
    let get = |a: &[f64], i: isize| {
        if i < 0 {
            0.
        } else {
            a.get(i as usize).copied().unwrap_or(0.)
        }
    };
    let d = (0..=p)
        .map(|i| p as f64 * (get(&l, i as isize - 1) - get(&l, i as isize)))
        .collect();
    let dd = (0..=p)
        .map(|i| {
            (p * (p - 1)) as f64
                * (get(&ll, i as isize - 2) - 2. * get(&ll, i as isize - 1) + get(&ll, i as isize))
        })
        .collect();
    [b, d, dd]
}
fn coefficients(s: &Surface, u: f64, v: f64) -> Result<[Vec<f64>; 3]> {
    let p = s.degree_u;
    let q = s.degree_v;
    let mut a = basis(p, u);
    let b = basis(q, v);
    let scale = s.weights.iter().map(|w| w[0]).fold(0., f64::max);
    let mut denominator = [0.; 3];
    for i in 0..=p {
        for d in 0..3 {
            a[d][i] *= s.weights[i][0] / scale;
            denominator[d] += a[d][i];
        }
    }
    require(
        denominator[0].is_finite() && denominator[0] > 0.,
        "Bivariate proposal denominator unresolved",
    )?;
    for i in 0..=p {
        a[0][i] /= denominator[0];
        a[1][i] = (a[1][i] - a[0][i] * denominator[1]) / denominator[0];
        a[2][i] =
            (a[2][i] - 2. * a[1][i] * denominator[1] - a[0][i] * denominator[2]) / denominator[0];
    }
    let hu = s.knots_u[p + 1] - s.knots_u[p];
    let hv = s.knots_v[q + 1] - s.knots_v[q];
    let mut out: [Vec<f64>; 3] = std::array::from_fn(|_| Vec::new());
    for i in 0..=p {
        for j in 0..=q {
            out[0].push(a[2][i] * b[0][j] / (hu * hu));
            out[1].push(a[1][i] * b[1][j] / (hu * hv));
            out[2].push(a[0][i] * b[2][j] / (hv * hv));
        }
    }
    require(
        out.iter().flatten().all(|x| x.is_finite()),
        "Bivariate proposal jet overflow",
    )?;
    Ok(out)
}
pub(super) fn validate(s: &Surface) -> Result<()> {
    s.validate()?;
    let p = s.degree_u;
    let q = s.degree_v;
    require(
        (6..=16).contains(&p)
            && (6..=16).contains(&q)
            && !s.periodic_u
            && !s.periodic_v
            && s.control_points.len() == p + 1
            && s.control_points[0].len() == q + 1
            && s.control_points[0][0].len() == 3
            && s.knots_u[..=p].iter().all(|&t| t == s.knots_u[p])
            && s.knots_u[p + 1..].iter().all(|&t| t == s.knots_u[p + 1])
            && s.knots_v[..=q].iter().all(|&t| t == s.knots_v[q])
            && s.knots_v[q + 1..].iter().all(|&t| t == s.knots_v[q + 1])
            && s.weights.iter().all(|w| w.iter().all(|&x| x == w[0])),
        "Bivariate proposal requires clamped common-denominator cells",
    )
}
pub(super) fn propose(
    source: &[Surface],
    relaxation: f64,
    stock: [[f64; 2]; 3],
    work: &mut Work,
) -> Result<Vec<Surface>> {
    require(
        relaxation.is_finite()
            && (0. ..=1.).contains(&relaxation)
            && stock
                .iter()
                .all(|b| b[0].is_finite() && b[1].is_finite() && b[0] <= b[1]),
        "Bound bivariate proposal controls",
    )?;
    let mut out = Vec::new();
    for s in source {
        validate(s)?;
        let p = s.degree_u;
        let q = s.degree_v;
        let free: Vec<_> = (3..=p - 3)
            .flat_map(|i| (3..=q - 3).map(move |j| (i, j)))
            .collect();
        let n = free.len();
        let count = (p + 1) * (q + 1);
        let mut index = vec![None; count];
        for (k, &(i, j)) in free.iter().enumerate() {
            index[i * (q + 1) + j] = Some(k);
        }
        let origin = &s.control_points[0][0];
        let mut matrix = vec![vec![0.; n]; n];
        let mut rhs = vec![[0.; 3]; n];
        for ui in 1..8 {
            for vi in 1..8 {
                work.charge(128)?;
                for (order, mut c) in coefficients(s, ui as f64 / 8., vi as f64 / 8.)?
                    .into_iter()
                    .enumerate()
                {
                    work.charge(count * 3 + n * n + n * 3)?;
                    if order == 1 {
                        for v in &mut c {
                            *v *= std::f64::consts::SQRT_2;
                        }
                    }
                    let mut row = vec![0.; n];
                    let mut fixed = [0.; 3];
                    for (k, &a) in c.iter().enumerate() {
                        if let Some(f) = index[k] {
                            row[f] = a;
                        } else {
                            let point = &s.control_points[k / (q + 1)][k % (q + 1)];
                            for axis in 0..3 {
                                fixed[axis] += a * (point[axis] - origin[axis]);
                            }
                        }
                    }
                    for i in 0..n {
                        for j in 0..n {
                            matrix[i][j] += row[i] * row[j];
                        }
                        for axis in 0..3 {
                            rhs[i][axis] -= row[i] * fixed[axis];
                        }
                    }
                }
            }
        }
        let normalizer = (0..n).map(|i| matrix[i][i]).fold(0., f64::max);
        require(
            normalizer.is_finite() && normalizer > 0.,
            "Bivariate proposal has no bounded bending system",
        )?;
        let mut solution: Vec<[f64; 3]> = free
            .iter()
            .map(|&(i, j)| std::array::from_fn(|k| s.control_points[i][j][k] - origin[k]))
            .collect();
        for i in 0..n {
            for j in 0..n {
                matrix[i][j] /= normalizer;
            }
            for k in 0..3 {
                rhs[i][k] = rhs[i][k] / normalizer + 1e-8 * solution[i][k];
            }
            matrix[i][i] += 1e-8;
        }
        // Bounded projected coordinate sweeps. Convergence is not claimed;
        // sampled parametric bending energy is not absolute geometric curvature.
        for _ in 0..64 {
            for k in 0..3 {
                for i in 0..n {
                    work.charge(n)?;
                    let mut r = rhs[i][k];
                    for j in 0..n {
                        if j != i {
                            r -= matrix[i][j] * solution[j][k];
                        }
                    }
                    let value = r / matrix[i][i];
                    require(value.is_finite(), "Bivariate proposal solver overflow")?;
                    solution[i][k] = value.clamp(stock[k][0] - origin[k], stock[k][1] - origin[k]);
                }
            }
        }
        let mut candidate = s.clone();
        for (f, &(i, j)) in free.iter().enumerate() {
            for k in 0..3 {
                let target = origin[k] + solution[f][k];
                let old = s.control_points[i][j][k];
                let v = if relaxation == 0. {
                    old
                } else if relaxation == 1. {
                    target
                } else {
                    (1. - relaxation) * old + relaxation * target
                };
                require(
                    v.is_finite() && stock[k][0] <= v && v <= stock[k][1],
                    "Bivariate interior leaves original stock",
                )?;
                candidate.control_points[i][j][k] = v;
            }
        }
        candidate.validate()?;
        out.push(candidate);
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> Surface {
        Surface {
            degree_u: 6,
            degree_v: 7,
            knots_u: [vec![-1.; 7], vec![2.; 7]].concat(),
            knots_v: [vec![-2.; 8], vec![2.; 8]].concat(),
            control_points: (0..7)
                .map(|i| {
                    (0..8)
                        .map(|j| {
                            vec![
                                i as f64 / 6.,
                                j as f64 / 7.,
                                if i == 3 && (j == 3 || j == 4) {
                                    0.5
                                } else {
                                    0.
                                },
                            ]
                        })
                        .collect()
                })
                .collect(),
            weights: (0..7)
                .map(|i| vec![if i % 2 == 0 { 1. } else { 2. }; 8])
                .collect(),
            periodic_u: false,
            periodic_v: false,
        }
    }
    #[test]
    fn rational_bending_coefficients_match_independent_original_evaluation() {
        let s = fixture();
        for t in [0.2, 0.5, 0.8] {
            let (uu, uv, vv) = s
                .evaluate(-1. + 3. * t, -2. + 4. * t)
                .unwrap()
                .second_derivatives()
                .unwrap();
            for (c, reference) in coefficients(&s, t, t).unwrap().iter().zip([uu, uv, vv]) {
                for k in 0..3 {
                    let actual = c
                        .iter()
                        .zip(s.control_points.iter().flatten())
                        .map(|(a, p)| a * p[k])
                        .sum::<f64>();
                    assert!(
                        (actual - reference[k]).abs() < 1e-10,
                        "{actual} {}",
                        reference[k]
                    );
                }
            }
        }
    }
    #[test]
    fn bivariate_bump_is_reduced_with_literal_boundary_jets_and_unchanged_source() {
        let s = fixture();
        let before = s.clone();
        let p = propose(
            std::slice::from_ref(&s),
            1.,
            [[-2., 2.]; 3],
            &mut Work {
                used: 0,
                max: 1000000,
            },
        )
        .unwrap();
        let p = &p[0];
        assert!(
            p.evaluate(0.5, 0.).unwrap().point[2] < s.evaluate(0.5, 0.).unwrap().point[2] / 100.
        );
        for i in 0..7 {
            for j in 0..8 {
                if i < 3 || i + 3 >= 7 || j < 3 || j + 3 >= 8 {
                    assert_eq!(p.control_points[i][j], s.control_points[i][j]);
                }
            }
        }
        assert_eq!(p.weights, s.weights);
        assert_eq!(s, before);
        for v in [-2., 2.] {
            let row = if v == -2. { 0 } else { 7 };
            let c = Curve {
                degree: 6,
                knots: s.knots_u.clone(),
                control_points: s.control_points.iter().map(|p| p[row].clone()).collect(),
                weights: s.weights.iter().map(|w| w[row]).collect(),
                periodic: false,
            };
            let mut uv = Curve::from_polyline(vec![vec![-1., v], vec![2., v]]).unwrap();
            uv.knots = vec![-1., -1., 2., 2.];
            let r = nurbs_core::surface_g2_pcurves::inspect(
                nurbs_core::surface_g2_pcurves::PcurveSeam { curve: &c, pcurves: [&uv, &uv], surfaces: [&s, p], reversed: [false, false] },
                nurbs_core::surface_g2_pcurves::G2PcurveTolerance { position: 1e-7, normal: 1e-5, curvature: 1e-4 },
                2000,
                100000,
            )
            .unwrap();
            assert!(r.positions.proven);
            assert_eq!(
                r.geometric.decision,
                nurbs_core::surface_join::Decision::WithinTolerance
            );
        }
    }
    #[test]
    fn invalid_stock_and_exhausted_work_cannot_return_a_candidate() {
        let s = fixture();
        assert!(
            propose(
                std::slice::from_ref(&s),
                1.,
                [[-2., 2.]; 3],
                &mut Work { used: 0, max: 0 }
            )
            .is_err()
        );
        assert!(
            propose(
                std::slice::from_ref(&s),
                1.,
                [[2., -2.]; 3],
                &mut Work {
                    used: 0,
                    max: 1000000
                }
            )
            .is_err()
        );
        assert!(
            propose(
                std::slice::from_ref(&s),
                f64::NAN,
                [[-2., 2.]; 3],
                &mut Work {
                    used: 0,
                    max: 1000000
                }
            )
            .is_err()
        );
    }
}
