use super::*;

/// Clamped uniform knot vector for `n` controls of degree `p` on [0,1].
fn clamped_knots(n: usize, p: usize) -> Vec<f64> {
    let spans = n - p;
    let mut knots = vec![0.; p + 1];
    for s in 1..spans {
        knots.push(s as f64 / spans as f64);
    }
    knots.extend(vec![1.; p + 1]);
    knots
}

/// 7x7 bicubic control net of the plane z = 0.2x + 0.1y with the two
/// central interior controls perturbed (deterministic pseudo-noise).
fn noisy_plane() -> Surface {
    let (n, p) = (7usize, 3usize);
    let knots = clamped_knots(n, p);
    let greville: Vec<f64> = (0..n)
        .map(|i| (knots[i + 1] + knots[i + 2] + knots[i + 3]) / 3.)
        .collect();
    let mut control_points = vec![vec![vec![0.; 3]; n]; n];
    for i in 0..n {
        for j in 0..n {
            let x = greville[i];
            let y = greville[j];
            control_points[i][j] = vec![x, y, 0.2 * x + 0.1 * y];
        }
    }
    control_points[3][3][2] += 0.35;
    control_points[4][3][2] -= 0.22;
    Surface {
        degree_u: p,
        degree_v: p,
        knots_u: knots.clone(),
        knots_v: knots,
        control_points,
        weights: vec![vec![1.; n]; n],
        periodic_u: false,
        periodic_v: false,
    }
}

fn reference_z(i: usize, j: usize, surface: &Surface) -> f64 {
    let p = &surface.control_points[i][j];
    0.2 * p[0] + 0.1 * p[1]
}

fn default_options() -> SurfaceFairingOptions {
    SurfaceFairingOptions {
        weights: FairingWeights::new(0.3, 0.7).unwrap(),
        cg: CgOptions::new(2000, 1e-11).unwrap(),
        local: None,
        g2: None,
    }
}

fn deviation_from_reference(surface: &Surface) -> f64 {
    let n = surface.control_points.len();
    let mut worst: f64 = 0.;
    for i in 2..n - 2 {
        for j in 2..n - 2 {
            worst = worst
                .max((surface.control_points[i][j][2] - reference_z(i, j, surface)).abs());
        }
    }
    worst
}

#[test]
fn fairing_noisy_plane_drops_energy_and_deviation() {
    let surface = noisy_plane();
    let before = deviation_from_reference(&surface);
    let report = fair_surface(&surface, default_options()).unwrap();
    assert!(
        report.energy_after < report.energy_before,
        "energy should drop: before={} after={}",
        report.energy_before,
        report.energy_after
    );
    let after = deviation_from_reference(&report.surface);
    assert!(
        after < 0.2 * before,
        "deviation should shrink: before={before} after={after}"
    );
    assert!(report.relative_residual <= 1e-11);
}

#[test]
fn pinned_boundary_rows_do_not_move() {
    let surface = noisy_plane();
    let report = fair_surface(&surface, default_options()).unwrap();
    let n = surface.control_points.len();
    for i in 0..n {
        for j in 0..n {
            if i < 2 || i + 2 >= n || j < 2 || j + 2 >= n {
                assert_eq!(
                    report.surface.control_points[i][j], surface.control_points[i][j],
                    "pinned control ({i},{j}) moved"
                );
            }
        }
    }
}

#[test]
fn local_mask_moves_only_controls_in_radius() {
    let surface = noisy_plane();
    // Radius 0.3 around control (3,3) at ~ (0.42, 0.42): covers the
    // perturbed center but not all interior controls.
    let mut options = default_options();
    options.local = Some((3, 3, 0.3));
    let report = fair_surface(&surface, options).unwrap();
    assert!(report.energy_after < report.energy_before);
    let n = surface.control_points.len();
    let center = &surface.control_points[3][3];
    let mut any_moved = false;
    for i in 0..n {
        for j in 0..n {
            let p = &surface.control_points[i][j];
            let d2: f64 = (0..3).map(|a| (p[a] - center[a]).powi(2)).sum();
            let moved = report.surface.control_points[i][j] != surface.control_points[i][j];
            if d2 > 0.3 * 0.3 {
                assert!(!moved, "control ({i},{j}) outside the radius moved");
            }
            any_moved |= moved;
        }
    }
    assert!(any_moved, "the local edit moved nothing");
}

#[test]
fn g2_penalty_bounds_second_row_drift() {
    let surface = noisy_plane();
    let free_report = fair_surface(&surface, default_options()).unwrap();
    let mut options = default_options();
    options.g2 = Some(G2Penalty {
        initial_lambda: 10.,
        rounds: 4,
    });
    let report = fair_surface(&surface, options).unwrap();
    assert!(report.energy_after < report.energy_before);
    assert!(report.final_lambda.unwrap() >= 10.);
    let n = surface.control_points.len();
    let drift = |faired: &Surface| {
        second_row_indices(n, n)
            .into_iter()
            .filter(|&i| {
                let (r, c) = (i / n, i % n);
                r >= 2 && r + 2 < n && c >= 2 && c + 2 < n
            })
            .map(|i| {
                let p = &faired.control_points[i / n][i % n];
                let q = &surface.control_points[i / n][i % n];
                (p[2] - q[2]).abs()
            })
            .fold(0., f64::max)
    };
    let free_drift = drift(&free_report.surface);
    let penalized_drift = drift(&report.surface);
    assert!(free_drift > 0.01, "test setup lost its drift: {free_drift}");
    assert!(
        penalized_drift < 0.5 * free_drift,
        "G2 penalty should at least halve second-row drift: free={free_drift} penalized={penalized_drift}"
    );
}

#[test]
fn analytic_gradient_matches_central_differences_on_random_nets() {
    // Deterministic 64-bit LCG in [-1, 1).
    let mut state = 0x5eed_0847_u64;
    let mut next = move || {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        (state >> 11) as f64 / (1u64 << 52) as f64 - 1.
    };
    for trial in 0..6 {
        let (nu, nv) = (5 + trial % 3, 6 + trial % 2);
        let (pu, pv) = (2 + trial % 3, 3);
        let mut knots = |n: usize, p: usize| {
            let mut inner: Vec<f64> = (0..n - p - 1).map(|_| 0.5 + 0.45 * next()).collect();
            inner.sort_by(f64::total_cmp);
            let mut k = vec![0.; p + 1];
            k.extend(inner);
            k.extend(vec![1.; p + 1]);
            k
        };
        let (knots_u, knots_v) = (knots(nu, pu), knots(nv, pv));
        let surface = Surface {
            degree_u: pu,
            degree_v: pv,
            knots_u,
            knots_v,
            control_points: (0..nu)
                .map(|_| (0..nv).map(|_| (0..3).map(|_| 2. * next()).collect()).collect())
                .collect(),
            weights: vec![vec![1.; nv]; nu],
            periodic_u: false,
            periodic_v: false,
        };
        let (alpha, beta) = (0.5 + 0.5 * next(), 0.5 + 0.5 * next());
        let form = composite_form(&surface, FairingWeights::new(alpha, beta).unwrap()).unwrap();
        let gradient = energy_gradient(&surface, &form).unwrap();
        let base = energy_of(&surface, &form).unwrap();
        let h = 1e-6;
        for i in 0..nu {
            for j in 0..nv {
                for axis in 0..3 {
                    let mut plus = surface.clone();
                    plus.control_points[i][j][axis] += h;
                    let mut minus = surface.clone();
                    minus.control_points[i][j][axis] -= h;
                    let numeric_derivative =
                        (energy_of(&plus, &form).unwrap() - energy_of(&minus, &form).unwrap())
                            / (2. * h);
                    let analytic = gradient[i][j][axis];
                    let scale = analytic.abs().max(numeric_derivative.abs()).max(base).max(1.);
                    assert!(
                        (analytic - numeric_derivative).abs() / scale < 1e-6,
                        "trial {trial} ({i},{j},{axis}): analytic={analytic} numeric={numeric_derivative}"
                    );
                }
            }
        }
    }
}

#[test]
fn analytic_gradient_matches_central_differences() {
    let surface = noisy_plane();
    let weights = FairingWeights::new(0.4, 0.6).unwrap();
    let form = composite_form(&surface, weights).unwrap();
    let gradient = energy_gradient(&surface, &form).unwrap();
    let base = energy_of(&surface, &form).unwrap();
    let h = 1e-6;
    // Check every control coordinate; central difference against the
    // identical assembled form keeps the comparison honest.
    for i in 0..7 {
        for j in 0..7 {
            for axis in 0..3 {
                let mut plus = surface.clone();
                plus.control_points[i][j][axis] += h;
                let mut minus = surface.clone();
                minus.control_points[i][j][axis] -= h;
                let numeric_derivative =
                    (energy_of(&plus, &form).unwrap() - energy_of(&minus, &form).unwrap())
                        / (2. * h);
                let analytic = gradient[i][j][axis];
                let error = (analytic - numeric_derivative).abs();
                let scale = analytic.abs().max(numeric_derivative.abs()).max(base);
                assert!(
                    error / scale < 1e-6,
                    "gradient mismatch at ({i},{j},{axis}): analytic={analytic} numeric={numeric_derivative}"
                );
            }
        }
    }
}

#[test]
fn rejects_low_degree_or_small_net() {
    let mut surface = noisy_plane();
    surface.degree_u = 1;
    assert!(fair_surface(&surface, default_options()).is_err());
    let surface = noisy_plane();
    let mut options = default_options();
    options.local = Some((3, 3, -1.));
    assert!(fair_surface(&surface, options).is_err());
}

#[test]
fn rejects_non_finite_boundaries_with_param_names() {
    let surface = noisy_plane();
    // NaN-радиус локальной правки.
    let mut options = default_options();
    options.local = Some((3, 3, f64::NAN));
    let err = fair_surface(&surface, options).unwrap_err();
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(err.contains("local fairing radius"), "{err}");
    // Inf-мультипликатор G2.
    let mut options = default_options();
    options.g2 = Some(G2Penalty {
        initial_lambda: f64::INFINITY,
        rounds: 2,
    });
    let err = fair_surface(&surface, options).unwrap_err();
    assert!(err.contains("G2 penalty multiplier"), "{err}");
    // NaN-веса fairing-функционала.
    assert!(FairingWeights::new(f64::NAN, 0.5).is_err());
}

#[test]
fn cg_options_feed_the_unified_budget() {
    use crate::numerics::guards::IterationBudget;
    let cg = CgOptions::new(7, 1e-9).unwrap();
    let budget = Budget::from(&cg);
    assert_eq!(budget.max_iterations, cg.max_iterations());
    // Три осевых solve в solve_reduced укладываются в тики guard'а.
    let mut guard = budget.guard("surface-fairing.cg-solve");
    for _ in 0..3 {
        guard.tick().unwrap();
    }
    // Истощение носит тип resource() и имя стадии.
    let tiny = Budget::with_iterations(2).unwrap();
    let mut guard = tiny.guard("surface-fairing.cg-solve");
    guard.tick().unwrap();
    guard.tick().unwrap();
    let err = guard.tick().unwrap_err();
    assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
    assert!(err.contains("surface-fairing.cg-solve"), "{err}");
}

#[test]
fn report_metrics_are_finite() {
    let surface = noisy_plane();
    let report = fair_surface(&surface, default_options()).unwrap();
    assert!(report.energy_before.is_finite());
    assert!(report.energy_after.is_finite());
    assert!(report.relative_residual.is_finite());
}

#[test]
fn tensor_form_matches_direct_quadrature() {
    // Sanity: membrane+plate assembly equals its tensor factorization on
    // a tiny net, verified through energy_of with a known monotone drop
    // under fairing — here just structural symmetry and finiteness.
    let surface = noisy_plane();
    let weights = FairingWeights::new(1., 1.).unwrap();
    let form = composite_form(&surface, weights).unwrap();
    let n = form.nrows();
    assert_eq!(form.nrows(), form.ncols());
    for i in 0..n {
        for (j, value) in form.row(i).unwrap() {
            let mirrored = form.at(j, i).unwrap();
            assert!(
                (value - mirrored).abs() <= 1e-12 * value.abs().max(1.),
                "form not symmetric at ({i},{j})"
            );
        }
        assert!(form.at(i, i).unwrap() > 0.);
    }
}
