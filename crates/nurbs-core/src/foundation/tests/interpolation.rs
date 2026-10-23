use super::*;

fn semicircle(count: usize) -> Vec<[f64; 3]> {
    (0..count)
        .map(|i| {
            let theta = std::f64::consts::PI * i as f64 / (count - 1) as f64;
            [theta.cos(), theta.sin(), 0.]
        })
        .collect()
}

fn residual(curve: &Curve, sites: &[f64], points: &[[f64; 3]]) -> f64 {
    sites
        .iter()
        .zip(points)
        .map(|(&u, p)| distance(&curve.evaluate(u).unwrap().point, p))
        .fold(0., f64::max)
}

#[test]
fn semicircle_degree3_interpolates_to_machine_precision() {
    let points = semicircle(9);
    for param in [
        Parameterization::Uniform,
        Parameterization::ChordLength,
        Parameterization::Centripetal,
    ] {
        let curve = interpolate_curve(&points, 3, param, None).unwrap();
        let u = parameters(&points, param);
        assert!(residual(&curve, &u, &points) < 1e-9);
        assert_eq!(curve.degree, 3);
        assert_eq!(curve.control_points.len(), 9);
    }
}

#[test]
fn end_tangents_reproduce_known_cubic() {
    // f(u) = (u, u², u³) sampled at uniform sites; uniform parameterization
    // makes the data parameters equal u, so the clamped interpolant must
    // reproduce the cubic exactly (uniqueness of cubic interpolation with
    // two derivative conditions).
    let n = 5;
    let points: Vec<[f64; 3]> = (0..n)
        .map(|i| {
            let u = i as f64 / (n - 1) as f64;
            [u, u * u, u * u * u]
        })
        .collect();
    let d0 = [1., 0., 0.];
    let d1 = [1., 2., 3.];
    let curve =
        interpolate_curve(&points, 3, Parameterization::Uniform, Some((d0, d1))).unwrap();
    assert_eq!(curve.control_points.len(), n + 2);
    for k in 0..=20 {
        let u = k as f64 / 20.;
        let q = curve.evaluate(u).unwrap();
        for axis in 0..3 {
            let expected = [u, u * u, u * u * u][axis];
            assert!((q.point[axis] - expected).abs() < 1e-9, "axis {axis} at u={u}");
        }
    }
    let start = curve.evaluate(0.).unwrap();
    let end = curve.evaluate(1.).unwrap();
    for axis in 0..3 {
        assert!((start.d1.as_ref().unwrap()[axis] - d0[axis]).abs() < 1e-9);
        assert!((end.d1.as_ref().unwrap()[axis] - d1[axis]).abs() < 1e-9);
    }
}

#[test]
fn centripetal_and_chord_handle_sharp_corners() {
    let points = [
        [0., 0., 0.],
        [1., 0., 0.],
        [1.001, 0., 0.],
        [1.001, 0., 1.],
        [2., 0., 1.],
        [3., 0.5, 1.],
    ];
    for param in [Parameterization::ChordLength, Parameterization::Centripetal] {
        let curve = interpolate_curve(&points, 3, param, None).unwrap();
        let u = parameters(&points, param);
        assert!(residual(&curve, &u, &points) < 1e-8);
    }
}

#[test]
fn adaptive_fit_of_noisy_arc_converges_with_honest_deviation() {
    let mut points = semicircle(17);
    for (i, p) in points.iter_mut().enumerate() {
        let noise = 0.002 * (17. * i as f64).sin();
        p[0] += noise;
        p[1] += 0.5 * noise;
    }
    let tolerance = 0.01;
    for simplify in [false, true] {
        let report = fit_curve_adaptive(
            &points,
            3,
            tolerance,
            Parameterization::ChordLength,
            None,
            simplify,
        )
        .unwrap();
        assert!(report.converged, "simplify={simplify}");
        assert!(report.max_deviation <= tolerance.next_up());
        assert!(report.curve.control_points.len() <= 256);
        // Independent recomputation of the reported deviation.
        let u = parameters(&points, Parameterization::ChordLength);
        let actual = residual(&report.curve, &u, &points);
        assert!(actual <= report.max_deviation);
        assert!(report.max_deviation - actual < 1e-12);
    }
    assert!(fit_curve_adaptive(&points, 3, 0., Parameterization::ChordLength, None, false)
        .is_err());
}

#[test]
fn adaptive_fit_with_worst_site_at_endpoint_refinement_places_interior_knot() {
    // Regression: the last data site (normalized parameter u = 1) carries
    // the largest residual. The span fallback used to land on the empty
    // [1, 1] end-knot window and error out; it must fall back to the
    // last non-degenerate span instead. Downweighting the last site keeps
    // its residual dominant (residuals are measured unweighted), so every
    // refinement picks u = 1 as the worst site.
    let mut points = semicircle(9);
    points.last_mut().unwrap()[1] += 0.5;
    let mut weights = vec![1.; points.len()];
    *weights.last_mut().unwrap() = 1e-8;
    let report = fit_curve_adaptive(
        &points,
        3,
        0.05,
        Parameterization::ChordLength,
        Some(&weights),
        false,
    )
    .expect("refinement with worst site at u = 1 must place a knot");
    assert!(report.iterations > 0);
    let u = parameters(&points, Parameterization::ChordLength);
    let actual = residual(&report.curve, &u, &points);
    assert!(actual <= report.max_deviation);
}

#[test]
fn heavily_downweighted_outlier_is_ignored() {
    let mut points = vec![
        [0., 0., 0.],
        [1., 0.1, 0.],
        [2., 5., 0.], // outlier
        [3., -0.1, 0.],
        [4., 0., 0.],
        [5., 0.05, 0.],
    ];
    points[2][1] = 5.;
    // A generous tolerance stops both runs at the initial 4-control fit,
    // so the comparison is apples-to-apples at equal control counts.
    let tolerance = 10.0;
    let mut weights = vec![1.; points.len()];
    weights[2] = 1e-8;
    let weighted = fit_curve_adaptive(
        &points,
        3,
        tolerance,
        Parameterization::ChordLength,
        Some(&weights),
        false,
    )
    .unwrap();
    let unweighted =
        fit_curve_adaptive(&points, 3, tolerance, Parameterization::ChordLength, None, false)
            .unwrap();
    let u = parameters(&points, Parameterization::ChordLength);
    let inlier = |curve: &Curve| {
        u.iter()
            .zip(&points)
            .enumerate()
            .filter(|(i, _)| *i != 2)
            .map(|(_, (&t, p))| distance(&curve.evaluate(t).unwrap().point, p))
            .fold(0., f64::max)
    };
    let weighted_inlier = inlier(&weighted.curve);
    let unweighted_inlier = inlier(&unweighted.curve);
    assert!(weighted.converged && unweighted.converged);
    assert_eq!(weighted.iterations, 0);
    assert_eq!(unweighted.iterations, 0);
    assert!(weighted_inlier < unweighted_inlier);
}

/// Deterministic SplitMix64 bit source for reproducible random geometry.
struct SplitMix64(u64);

impl SplitMix64 {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// Uniform double in [0, 1).
    fn f64(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

/// Random clamped knot vector on [0, 1] with `count` control points.
fn random_clamped_knots(rng: &mut SplitMix64, degree: usize, count: usize) -> Vec<f64> {
    let interior = count - degree - 1;
    let mut middle: Vec<f64> = (0..interior).map(|_| 0.05 + 0.9 * rng.f64()).collect();
    middle.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let mut knots = vec![0.; degree + 1];
    knots.extend(middle);
    knots.extend(std::iter::repeat_n(1., degree + 1));
    knots
}

#[test]
fn marsden_identity_holds_for_random_clamped_knot_vectors() {
    // Marsden: (s − t)^p = Σ_i N_{i,p}(t)·ψ_i(s) with
    // ψ_i(s) = ∏_{j=1..p} (s − u_{i+j}).
    let mut rng = SplitMix64(0x5EED_5EED_5EED_5EED);
    for _ in 0..40 {
        let degree = 1 + (rng.next() % 4) as usize; // 1..=4
        let count = degree + 1 + (rng.next() % 5) as usize; // up to degree+5
        let knots = random_clamped_knots(&mut rng, degree, count);
        for _ in 0..10 {
            let s = rng.f64();
            let t = rng.f64();
            let row = basis_row(degree, &knots, count, t).unwrap();
            let mut sum = 0.;
            for (i, &n) in row.iter().enumerate() {
                let psi: f64 = (1..=degree).map(|j| s - knots[i + j]).product();
                sum += n * psi;
            }
            let expected = (s - t).powi(degree as i32);
            // Values are O(1) on [0,1]; allow modest accumulation slack.
            let tolerance = 1e-9 * (count * degree) as f64;
            assert!(
                (sum - expected).abs() <= tolerance,
                "degree={degree} count={count} s={s} t={t}: |{sum} − {expected}|"
            );
        }
    }
}

#[test]
fn blossom_control_net_reproduces_polynomials() {
    // Marsden's identity gives t^m = Σ_i N_{i,p}(t)·e_m(u_{i+1..i+p})/C(p,m)
    // with e_m the elementary symmetric polynomial. A B-spline whose
    // control points are P_i = Σ_m c_m·e_m/C(p,m) therefore reproduces
    // the polynomial f(t) = Σ_m c_m t^m (degree ≤ p) exactly. (Greville
    // abscissae alone — the m = 1 case — reproduce only linears.)
    let mut rng = SplitMix64(0xABCD_EF01_2345_6789);
    let binom = |p: usize, m: usize| -> f64 {
        let m = m.min(p - m);
        (1..=m).fold(1., |acc, k| acc * (p + 1 - k) as f64 / k as f64)
    };
    // Elementary symmetric polynomial e_m of a knot window, by DP.
    let elementary = |window: &[f64], m: usize| -> f64 {
        let mut e = vec![0.; m + 1];
        e[0] = 1.;
        for &x in window {
            for j in (1..=m).rev() {
                e[j] += x * e[j - 1];
            }
        }
        e[m]
    };
    for _ in 0..30 {
        let degree = 1 + (rng.next() % 4) as usize; // 1..=4
        let count = degree + 1 + (rng.next() % 5) as usize;
        let knots = random_clamped_knots(&mut rng, degree, count);
        // Random polynomial of degree ≤ p, per axis, coefficients ≤ 1.
        let coeffs: Vec<Vec<f64>> = (0..3)
            .map(|_| {
                (0..=degree)
                    .map(|_| 2. * rng.f64() - 1.)
                    .collect::<Vec<f64>>()
            })
            .collect();
        let evaluate_poly = |axis: usize, t: f64| {
            coeffs[axis]
                .iter()
                .enumerate()
                .map(|(j, c)| c * t.powi(j as i32))
                .sum::<f64>()
        };
        let control_points: Vec<Vec<f64>> = (0..count)
            .map(|i| {
                let window = &knots[i + 1..i + 1 + degree];
                (0..3)
                    .map(|axis| {
                        coeffs[axis]
                            .iter()
                            .enumerate()
                            .map(|(m, c)| c * elementary(window, m) / binom(degree, m))
                            .sum()
                    })
                    .collect()
            })
            .collect();
        let curve = Curve {
            degree,
            knots,
            control_points,
            weights: vec![1.; count],
            periodic: false,
        };
        curve.validate().unwrap();
        for _ in 0..10 {
            let t = rng.f64();
            let q = curve.evaluate(t).unwrap();
            for axis in 0..3 {
                let expected = evaluate_poly(axis, t);
                assert!(
                    (q.point[axis] - expected).abs() <= 1e-9 * (count * degree) as f64,
                    "degree={degree} count={count} axis={axis} t={t}: {} vs {expected}",
                    q.point[axis]
                );
            }
        }
    }
}

#[test]
fn degenerate_collocation_is_rejected_before_solving() {
    // Cubic, 5 basis functions; row 1's parameter sits at the left edge
    // of N_1's support where N_1 vanishes.
    let degree = 3;
    let knots = vec![0., 0., 0., 0., 0.5, 1., 1., 1., 1.];
    let parameters = vec![0., 0., 0.45, 0.9, 1.];
    let report = validate_collocation(degree, &knots, &parameters, 1e-12).unwrap();
    assert!(!report.ok);
    assert_eq!(report.worst_row, 1);
    assert!(report.min_support_value <= 1e-12);
    // A sound scheme passes.
    let good = vec![0., 0.2, 0.45, 0.7, 1.];
    let report = validate_collocation(degree, &knots, &good, 1e-12).unwrap();
    assert!(report.ok);
    assert!(report.min_support_value > 1e-12);
    // Non-square schemes and bad thresholds are input errors.
    assert!(validate_collocation(degree, &knots, &good[..4], 1e-12).is_err());
    assert!(validate_collocation(degree, &knots, &good, 0.).is_err());
    assert!(validate_collocation(degree, &knots, &[0., 0.2, 0.45, 0.7, 2.], 1e-12).is_err());
}

#[test]
fn clustered_sites_fail_schoenberg_whitney_in_interpolate_curve() {
    // Chord-length parameterization makes the site parameters follow the
    // point positions exactly. Sites 1..4 are clustered within ~3e-7, so
    // the averaged knots u_4 = mean(t_1..t_3) and u_5 = mean(t_2..t_4)
    // squeeze t_1 to the extreme right edge of N_1's support
    // [u_1, u_5): the Cox–de Boor products carry a factor
    // (u_5 − t_1)/u_5 ≈ 3e-7 twice, driving N_1(t_1) ≈ 1e-13 below the
    // collocation threshold. The pre-check must reject before the solve.
    let us = [0., 0.6, 0.6000001, 0.6000002, 0.6000003, 1.];
    let points: Vec<[f64; 3]> = us.iter().map(|&u| [u, 0., 0.]).collect();
    let result = interpolate_curve(&points, 3, Parameterization::ChordLength, None);
    assert!(result.is_err());
    // Well-spread sites on the same degree pass the pre-check.
    let spread: Vec<[f64; 3]> = (0..6).map(|i| [i as f64 / 5., 0., 0.]).collect();
    assert!(interpolate_curve(&spread, 3, Parameterization::Uniform, None).is_ok());
}
