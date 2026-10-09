use super::*;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn f64(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.f64()
    }
}

/// Random clamped curve on [0, 1] with well-separated interior knots.
fn random_curve(rng: &mut Rng) -> Curve {
    let p = 1 + (rng.next() % 4) as usize;
    let interior = (rng.next() % 6) as usize;
    let mut knots = vec![0.; p + 1];
    for i in 0..interior {
        knots.push((i + 1) as f64 / (interior + 1) as f64);
    }
    knots.extend(vec![1.; p + 1]);
    let n = knots.len() - p - 1;
    let control_points = (0..n)
        .map(|_| (0..3).map(|_| rng.range(-5., 5.)).collect::<Vec<f64>>())
        .collect();
    let weights = (0..n).map(|_| rng.range(0.5, 2.)).collect();
    let curve = Curve {
        degree: p,
        knots,
        control_points,
        weights,
        periodic: false,
    };
    curve.validate().unwrap();
    curve
}

fn close(a: &[f64], b: &[f64], tol: f64) -> bool {
    a.len() == b.len()
        && a.iter()
            .zip(b)
            .all(|(x, y)| (x - y).abs() <= tol * (1. + x.abs().max(y.abs())))
}

#[test]
fn find_span_matches_linear_scan() {
    let mut rng = Rng(42);
    for _ in 0..30 {
        let c = random_curve(&mut rng);
        let (p, n) = (c.degree, c.control_points.len());
        for _ in 0..20 {
            let u = rng.range(0., 1.);
            let span = find_span(p, &c.knots, n, u).unwrap();
            let mut reference = p;
            while reference + 1 <= n - 1 && c.knots[reference + 1] <= u {
                reference += 1;
            }
            assert_eq!(span, reference);
            assert!(c.knots[span] <= u && u < c.knots[span + 1]);
        }
        assert_eq!(find_span(p, &c.knots, n, 1.).unwrap(), n - 1);
        assert_eq!(find_span(p, &c.knots, n, 0.).unwrap(), p);
        assert!(find_span(p, &c.knots, n, 1.5).is_err());
        assert!(find_span(p, &c.knots, n, -0.5).is_err());
    }
}

#[test]
fn basis_funs_ders_matches_full_basis() {
    let mut rng = Rng(7);
    for _ in 0..30 {
        let c = random_curve(&mut rng);
        let (p, n) = (c.degree, c.control_points.len());
        for _ in 0..25 {
            let u = rng.range(0., 1.);
            let span = find_span(p, &c.knots, n, u).unwrap();
            let local = basis_funs_ders(p, &c.knots, span, u, 2).unwrap();
            let scattered = local.scatter(p, &c.knots, n, u, false).unwrap();
            let full = basis(p, &c.knots, n, u, false).unwrap();
            assert!(
                close(&scattered.basis, &full.basis, 1e-9),
                "basis mismatch at {u}"
            );
            assert!(close(&scattered.d1, &full.d1, 1e-7), "d1 mismatch at {u}");
            assert!(close(&scattered.d2, &full.d2, 1e-5), "d2 mismatch at {u}");
            assert_eq!(scattered.continuity, full.continuity);
            assert_eq!(scattered.derivative_status, full.derivative_status);
            assert_eq!(scattered.derivative_side, full.derivative_side);
            assert!(local.values.iter().all(|v| *v >= 0.));
            assert!((local.values.iter().sum::<f64>() - 1.).abs() < 1e-12);
        }
    }
}

#[test]
fn evaluator_matches_evaluate() {
    let mut rng = Rng(11);
    for _ in 0..25 {
        let c = random_curve(&mut rng);
        let mut evaluator = CurveEvaluator::new(&c).unwrap();
        // Monotone sweep exercises the cached-span fast path.
        let samples: Vec<f64> = (0..=50).map(|i| i as f64 / 50.).collect();
        // Reverse sweep defeats the hint and exercises the fallback.
        let all: Vec<f64> = samples
            .iter()
            .chain(samples.iter().rev())
            .copied()
            .collect();
        for u in all {
            let fast = evaluator.evaluate(u).unwrap();
            let reference = c.evaluate(u).unwrap();
            assert!(
                close(&fast.point, &reference.point, 1e-9),
                "point mismatch at {u}"
            );
            match (&fast.d1, &reference.d1) {
                (Some(a), Some(b)) => assert!(close(a, b, 1e-6), "d1 mismatch at {u}"),
                (None, None) => {}
                _ => panic!("d1 availability mismatch at {u}"),
            }
            match (&fast.d2, &reference.d2) {
                (Some(a), Some(b)) => assert!(close(a, b, 1e-4), "d2 mismatch at {u}"),
                (None, None) => {}
                _ => panic!("d2 availability mismatch at {u}"),
            }
            assert_eq!(fast.continuity, reference.continuity);
            assert_eq!(fast.derivative_status, reference.derivative_status);
            assert_eq!(fast.derivative_side, reference.derivative_side);
            assert_eq!(fast.domain, reference.domain);
        }
    }
}

#[test]
fn de_boor_matches_evaluate() {
    let mut rng = Rng(23);
    for _ in 0..25 {
        let c = random_curve(&mut rng);
        for i in 0..=40 {
            let u = i as f64 / 40.;
            let a = c.evaluate_de_boor(u).unwrap();
            let b = c.evaluate(u).unwrap().point;
            assert!(close(&a, &b, 1e-9), "de Boor mismatch at {u}");
        }
    }
}

#[test]
fn refine_matches_repeated_insert() {
    let mut rng = Rng(31);
    for _ in 0..20 {
        let c = random_curve(&mut rng);
        if c.degree < 2 {
            continue;
        }
        let xs: Vec<f64> = (0..3)
            .map(|_| {
                loop {
                    let x = rng.range(0.05, 0.95);
                    if c.knots.iter().all(|k| *k != x) {
                        break x;
                    }
                }
            })
            .collect();
        let mut xs = xs;
        xs.sort_by(f64::total_cmp);
        let refined = c.refine(&xs).unwrap();
        let mut inserted = c.clone();
        for &x in &xs {
            inserted = inserted.insert(x, 1).unwrap();
        }
        assert_eq!(refined.knots, inserted.knots);
        assert_eq!(refined.degree, c.degree);
        assert_eq!(
            refined.control_points.len(),
            c.control_points.len() + xs.len()
        );
        for i in 0..=20 {
            let u = i as f64 / 20.;
            assert!(close(
                &refined.evaluate(u).unwrap().point,
                &c.evaluate(u).unwrap().point,
                1e-8,
            ));
        }
    }
}

#[test]
fn refine_validates_inputs() {
    let c = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.], vec![1., 1.]]).unwrap();
    assert!(c.refine(&[0.25, 0.75]).is_ok());
    assert!(c.refine(&[]).unwrap() == c);
    assert!(c.refine(&[0.75, 0.25]).is_err());
    assert!(c.refine(&[-0.1]).is_err());
    assert!(c.refine(&[2.1]).is_err());
    assert!(c.refine(&[0.5, 0.5, 0.5]).is_err());
}

#[test]
fn merge_near_knots_collapses_noise() {
    let c = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 0.5, 0.5 + 1e-12, 1., 1., 1., 1.],
        control_points: (0..6).map(|i| vec![i as f64, (i % 3) as f64, 0.]).collect(),
        weights: vec![1.; 6],
        periodic: false,
    };
    c.validate().unwrap();
    let merged = merge_near_knots(&c, 1e-9).unwrap();
    assert_eq!(merged.knots.len(), c.knots.len());
    assert_eq!(merged.knots[4], merged.knots[5]);
    assert_eq!(multiplicity_eps(&merged.knots, merged.knots[4], 0.), 2);
    for i in 0..=20 {
        let u = i as f64 / 20.;
        assert!(close(
            &merged.evaluate(u).unwrap().point,
            &c.evaluate(u).unwrap().point,
            1e-6,
        ));
    }
    assert!(merge_near_knots(&c, 0.).is_err());
}

#[test]
fn multiplicity_eps_counts_with_tolerance() {
    let knots = [0., 0., 0.25, 0.25 + 1e-13, 0.5, 1.];
    assert_eq!(multiplicity_eps(&knots, 0.25, 0.), 1);
    assert_eq!(multiplicity_eps(&knots, 0.25, 1e-12), 2);
    assert_eq!(multiplicity_eps(&knots, 0., 1e-12), 2);
}

#[test]
fn clamped_and_normalize_preserve_geometry() {
    let mut rng = Rng(37);
    for _ in 0..15 {
        let c = random_curve(&mut rng);
        let clamped = clamped(&c).unwrap();
        assert!(!clamped.periodic);
        assert_eq!(clamped.domain(), c.domain());
        assert_eq!(multiplicity(&clamped.knots, 0.), c.degree + 1);
        assert_eq!(multiplicity(&clamped.knots, 1.), c.degree + 1);
        let scaled = Curve {
            knots: c.knots.iter().map(|k| 2. + 3. * k).collect(),
            ..c.clone()
        };
        scaled.validate().unwrap();
        let normalized = normalize_knots(&scaled).unwrap();
        assert_eq!(normalized.domain(), [0., 1.]);
        for i in 0..=10 {
            let u = i as f64 / 10.;
            assert!(close(
                &clamped.evaluate(u).unwrap().point,
                &c.evaluate(u).unwrap().point,
                1e-9,
            ));
            assert!(close(
                &normalized.evaluate(u).unwrap().point,
                &scaled.evaluate(2. + 3. * u).unwrap().point,
                1e-9,
            ));
        }
    }
}

#[test]
fn evaluate_validated_skips_revalidation() {
    let c = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.], vec![2., 0.]]).unwrap();
    let a = c.evaluate(0.4).unwrap();
    let b = c.evaluate_validated(0.4).unwrap();
    assert!(close(&a.point, &b.point, 0.));
}

/// Double-double (two-float) accumulator used as a near-exact reference
/// for the compensated-summation tests.
#[derive(Clone, Copy)]
struct DoubleDouble {
    hi: f64,
    lo: f64,
}
impl DoubleDouble {
    fn add(&mut self, x: f64) {
        let s = self.hi + x;
        let z = s - self.hi;
        self.lo += (self.hi - (s - z)) + (x - z);
        self.hi = s;
    }
    fn normalized(self) -> (f64, f64) {
        let s = self.hi + self.lo;
        (s, self.lo + (self.hi - s))
    }
}

/// Naive (uncompensated) replica of the evaluation weighted sums, used to
/// compare against the compensated production path.
fn naive_weighted_point(c: &Curve, u: f64) -> Vec<f64> {
    let b = basis(c.degree, &c.knots, c.control_points.len(), u, c.periodic).unwrap();
    let dim = c.control_points[0].len();
    let scale = c.weights.iter().copied().fold(0., f64::max);
    let index = b.basis.iter().position(|v| *v > 0.).unwrap();
    let origin = &c.control_points[index];
    let mut weight = 0.;
    let mut point = vec![0.; dim];
    for i in 0..c.control_points.len() {
        let w = c.weights[i] / scale;
        weight += b.basis[i] * w;
        for axis in 0..dim {
            let coordinate = c.control_points[i][axis] - origin[axis];
            point[axis] += b.basis[i] * w * coordinate;
        }
    }
    point.iter_mut().for_each(|x| *x /= weight);
    point
}

/// Near-exact double-double replica of the same sums.
fn reference_weighted_point(c: &Curve, u: f64) -> Vec<f64> {
    let b = basis(c.degree, &c.knots, c.control_points.len(), u, c.periodic).unwrap();
    let dim = c.control_points[0].len();
    let scale = c.weights.iter().copied().fold(0., f64::max);
    let index = b.basis.iter().position(|v| *v > 0.).unwrap();
    let origin = &c.control_points[index];
    let mut weight = DoubleDouble { hi: 0., lo: 0. };
    let mut point = vec![DoubleDouble { hi: 0., lo: 0. }; dim];
    for i in 0..c.control_points.len() {
        let w = c.weights[i] / scale;
        weight.add(b.basis[i] * w);
        for axis in 0..dim {
            let coordinate = c.control_points[i][axis] - origin[axis];
            point[axis].add(b.basis[i] * w * coordinate);
        }
    }
    let (wh, wl) = weight.normalized();
    point
        .iter()
        .map(|a| {
            let (h, l) = a.normalized();
            // One Newton refinement of the double-double quotient.
            let q = h / wh;
            q + (l - q * wl) / wh
        })
        .collect()
}

#[test]
fn compensation_beats_naive_on_adversarial_weights() {
    // Absorption case: t1 ≈ +2e8, t2 ≈ 1e-8 (below half an ulp of t1, so
    // naive summation drops it entirely), t3 ≈ -2e8. The true numerator is
    // dominated by the tiny middle term; weights span 1..1e-12 (the
    // validation ceiling ratio 1e12).
    let c = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![
            vec![0.5, 0.25],
            vec![0.5 + 2e8 / 0.288, 1.],
            vec![0.5 + 1e-8 / (0.432e-12), 2.],
            vec![0.5 - 2e8 / 0.216, 3.],
        ],
        weights: vec![1., 1., 1e-12, 1.],
        periodic: false,
    };
    c.validate().unwrap();
    let u = 0.6;
    let reference = reference_weighted_point(&c, u);
    let naive = naive_weighted_point(&c, u);
    let e = c.evaluate(u).unwrap();
    let compensated: Vec<f64> = e
        .point
        .iter()
        .zip(&c.control_points[0])
        .map(|(x, o)| x - o)
        .collect();
    let naive_err = (naive[0] - reference[0]).abs();
    let compensated_err = (compensated[0] - reference[0]).abs();
    assert!(
        naive_err > 1e-10,
        "case not adversarial: naive error {naive_err:e}"
    );
    assert!(
        compensated_err <= naive_err,
        "compensated {compensated_err:e} must not exceed naive {naive_err:e}"
    );
    assert!(
        compensated_err < naive_err * 0.01,
        "expected real improvement: compensated {compensated_err:e} vs naive {naive_err:e}"
    );
    assert!(e.rounding_bound.is_finite() && e.rounding_bound >= 0.);
    // CurveEvaluator agrees with evaluate and carries the same evidence.
    let mut evaluator = CurveEvaluator::new(&c).unwrap();
    let fast = evaluator.evaluate(u).unwrap();
    assert!(fast.rounding_bound.is_finite() && fast.rounding_bound >= 0.);
    assert!(close(&fast.point, &e.point, 1e-15));
}

#[test]
fn repeated_evaluation_is_stable_and_finite() {
    let mut rng = Rng(101);
    let c = random_curve(&mut rng);
    let mut evaluator = CurveEvaluator::new(&c).unwrap();
    let mut checksum = 0.;
    for i in 0..100_000 {
        let u = (i % 1001) as f64 / 1000.;
        let e = evaluator.evaluate(u).unwrap();
        assert!(e.point.iter().all(|v| v.is_finite()));
        assert!(e.rounding_bound.is_finite() && e.rounding_bound >= 0.);
        checksum += e.point[0];
    }
    assert!(checksum.is_finite());
    // Cross-check a sample against the independent de Boor path.
    for i in 0..=20 {
        let u = i as f64 / 20.;
        assert!(close(
            &c.evaluate(u).unwrap().point,
            &c.evaluate_de_boor(u).unwrap(),
            1e-9,
        ));
    }
}
