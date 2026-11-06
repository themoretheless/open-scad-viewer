//! Bounded approximate curvature search for NEW interior proposals.
//! Every score is sampled and approximate: only independent original interval
//! fields may grant curvature admission. Boundary two-jets stay literal.
use super::*;
pub(super) fn score(s: &Surface, work: &mut Work) -> Result<(f64, f64)> {
    let p = s.degree_u;
    let q = s.degree_v;
    let (a, b) = (s.knots_u[p], s.knots_u[p + 1]);
    let (c, d) = (s.knots_v[q], s.knots_v[q + 1]);
    let mut maximum = 0f64;
    let mut sum = 0.;
    for u in [0.125, 0.25, 0.5, 0.75, 0.875] {
        for v in [0.125, 0.25, 0.5, 0.75, 0.875] {
            work.charge(1)?;
            let evaluation = s.evaluate(a + (b - a) * u, c + (d - c) * v)?;
            let Some((gaussian, mean)) = evaluation.curvatures() else {
                return Ok((f64::INFINITY, f64::INFINITY));
            };
            let k = mean.abs() + (mean * mean - gaussian).max(0.).sqrt();
            if !k.is_finite() {
                return Ok((f64::INFINITY, f64::INFINITY));
            }
            maximum = maximum.max(k);
            sum += k * k;
        }
    }
    Ok((maximum, sum))
}
fn better(a: (f64, f64), b: (f64, f64)) -> bool {
    a.0 < b.0 || (a.0 == b.0 && a.1 < b.1)
}
pub(super) fn propose(
    source: &[Surface],
    sweeps: usize,
    stock: [[f64; 2]; 3],
    work: &mut Work,
) -> Result<Vec<Surface>> {
    require(
        (1..=8).contains(&sweeps)
            && stock
                .iter()
                .all(|b| b[0].is_finite() && b[1].is_finite() && b[0] <= b[1]),
        "Bound interior curvature proposal search",
    )?;
    let mut out = Vec::new();
    for s in source {
        bivariate_fairing::validate(s)?;
        let mut candidate = s.clone();
        let mut best = score(&candidate, work)?;
        let spread = (0..3)
            .map(|k| {
                let low = s
                    .control_points
                    .iter()
                    .flatten()
                    .map(|p| p[k])
                    .fold(f64::INFINITY, f64::min);
                let high = s
                    .control_points
                    .iter()
                    .flatten()
                    .map(|p| p[k])
                    .fold(f64::NEG_INFINITY, f64::max);
                high - low
            })
            .fold(0., f64::max);
        let mut step = (spread / 8.).min(0.1);
        require(step.is_finite(), "Curvature proposal step overflow")?;
        for _ in 0..sweeps {
            for i in 3..=s.degree_u - 3 {
                for j in 3..=s.degree_v - 3 {
                    for k in 0..3 {
                        let old = candidate.control_points[i][j][k];
                        let mut chosen = old;
                        let mut chosen_score = best;
                        for direction in [-1., 1.] {
                            let value = (old + direction * step).clamp(stock[k][0], stock[k][1]);
                            if value == old {
                                continue;
                            }
                            candidate.control_points[i][j][k] = value;
                            let trial = score(&candidate, work)?;
                            if better(trial, chosen_score) {
                                chosen = value;
                                chosen_score = trial;
                            }
                        }
                        candidate.control_points[i][j][k] = chosen;
                        best = chosen_score;
                    }
                }
            }
            step *= 0.5;
        }
        require(
            best.0.is_finite(),
            "Curvature search has no regular sampled candidate",
        )?;
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
            knots_u: [vec![0.; 7], vec![1.; 7]].concat(),
            knots_v: [vec![0.; 8], vec![1.; 8]].concat(),
            control_points: (0..7)
                .map(|i| {
                    (0..8)
                        .map(|j| {
                            vec![
                                i as f64 / 6.,
                                j as f64 / 7.,
                                if i == 3 && (j == 3 || j == 4) {
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
        }
    }
    #[test]
    fn actual_sample_curvature_decreases_without_changing_any_boundary_two_jet_pole() {
        let s = fixture();
        let before = s.clone();
        let start = score(
            &s,
            &mut Work {
                used: 0,
                max: 100000,
            },
        )
        .unwrap();
        let p = propose(
            std::slice::from_ref(&s),
            6,
            [[0., 1.]; 3],
            &mut Work {
                used: 0,
                max: 100000,
            },
        )
        .unwrap();
        let p = &p[0];
        let end = score(
            p,
            &mut Work {
                used: 0,
                max: 100000,
            },
        )
        .unwrap();
        assert!(end.0 < start.0 / 10., "{start:?} -> {end:?}");
        for i in 0..7 {
            for j in 0..8 {
                if i < 3 || i + 3 >= 7 || j < 3 || j + 3 >= 8 {
                    assert_eq!(p.control_points[i][j], s.control_points[i][j]);
                }
            }
        }
        assert_eq!(s, before);
        assert_eq!(p.weights, s.weights);
        // An independent original interval point bound, never a whole-field
        // claim from the search score. This fixture becomes essentially flat.
        let r = nurbs_core::surface_shape_operator::rectangle_bounds(p, [[0.5; 2]; 2], 8).unwrap();
        let bounds =
            nurbs_core::surface_shape_operator::absolute_spectral_bounds(r.shape.tensor.unwrap())
                .unwrap()
                .unwrap();
        assert!(bounds[1] < 1e-6, "{bounds:?}");
    }
    #[test]
    fn exhausted_sample_allowance_and_invalid_stock_do_not_publish_partial_searches() {
        let s = fixture();
        assert!(
            propose(
                std::slice::from_ref(&s),
                6,
                [[0., 1.]; 3],
                &mut Work { used: 0, max: 25 }
            )
            .is_err()
        );
        assert!(
            propose(
                std::slice::from_ref(&s),
                9,
                [[0., 1.]; 3],
                &mut Work {
                    used: 0,
                    max: 100000
                }
            )
            .is_err()
        );
        let mut collapsed = s.clone();
        for p in collapsed.control_points.iter_mut().flatten() {
            *p = vec![0.5; 3];
        }
        assert!(
            propose(
                std::slice::from_ref(&collapsed),
                6,
                [[0.5, 0.5]; 3],
                &mut Work {
                    used: 0,
                    max: 100000
                }
            )
            .is_err()
        );
        assert!(
            propose(
                std::slice::from_ref(&s),
                6,
                [[1., 0.]; 3],
                &mut Work {
                    used: 0,
                    max: 100000
                }
            )
            .is_err()
        );
    }
}
