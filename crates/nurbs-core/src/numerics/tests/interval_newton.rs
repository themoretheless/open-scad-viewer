use super::*;
use crate::foundation::bracketed::brent_bracketed;

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

/// Product form Π (x − r_i), expanded to power coefficients.
fn from_roots(roots: &[f64]) -> PolynomialFunction {
    let mut c = vec![1.];
    for &r in roots {
        let mut n = vec![0.; c.len() + 1];
        for (i, &a) in c.iter().enumerate() {
            n[i] -= a * r;
            n[i + 1] += a;
        }
        c = n;
    }
    PolynomialFunction::new(c).unwrap()
}

#[test]
fn derivative_coefficients_are_rounded_outward_and_monotone_boxes_refuse_roots() {
    let p = PolynomialFunction::new(vec![0., 0., 0., 0.1]).unwrap();
    let d = p.interval_derivative(Interval::point(1.)).unwrap();
    // Exact 3 times binary64 0.1 lies between these adjacent floats.
    assert!(d.lo <= 0.3 && d.hi >= 0.30000000000000004);
    let decreasing = PolynomialFunction::new(vec![-1., -0.1]).unwrap();
    assert!(matches!(
        classify(&decreasing, Interval::new(0., 100.).unwrap()).unwrap(),
        RootCertificate::Absent
    ));
    let tangent = PolynomialFunction::new(vec![0., 0., 1.]).unwrap();
    assert!(!matches!(
        classify(&tangent, Interval::new(-1., 1.).unwrap()).unwrap(),
        RootCertificate::Absent
    ));
}

#[test]
fn unique_and_absent_certificates_on_known_polynomials() {
    // (x−1)(x−2)(x−3): roots at 1, 2, 3. Power-basis evaluation near
    // x = 2 cancels terms of magnitude ~15, so interval Horner
    // overestimates F′ on wide boxes and uniqueness needs a box whose
    // width is small compared to the cancellation scale.
    let p = from_roots(&[1., 2., 3.]);
    match classify(&p, Interval::new(1.95, 2.05).unwrap()).unwrap() {
        RootCertificate::Unique(n) => assert!(n.contains(2.)),
        other => panic!("expected Unique, got {other:?}"),
    }
    // Absence: no root in [3.99, 4.01]. (Interval arithmetic proves
    // absence only once the box is narrow enough for the cancellation
    // scale; wide root-free boxes are resolved by `isolate_roots`
    // subdivision, see the no-root case below.)
    assert!(matches!(
        classify(&p, Interval::new(3.99, 4.01).unwrap()).unwrap(),
        RootCertificate::Absent
    ));
    // Ambiguous box containing two roots cannot be Unique.
    assert!(!matches!(
        classify(&p, Interval::new(0.5, 2.5).unwrap()).unwrap(),
        RootCertificate::Unique(_)
    ));
}

#[test]
fn isolate_finds_all_roots_including_close_pairs() {
    // Roots at 0.3 and 0.3 + 1e-6, plus 0.7.
    let p = from_roots(&[0.3, 0.3 + 1e-6, 0.7]);
    let roots = isolate_roots(Interval::new(0., 1.).unwrap(), &p, 1e-9, 40).unwrap();
    assert_eq!(roots.len(), 3);
    for (box_, certificate) in &roots {
        // The certified enclosure is tight even when the box that
        // happened to certify first is wide.
        match certificate {
            RootCertificate::Unique(u) => assert!(u.width() < 1e-6),
            _ => panic!("expected Unique for simple separated roots"),
        }
        let _ = box_;
    }
    for r in [0.3, 0.3 + 1e-6, 0.7] {
        assert!(roots.iter().any(|(b, _)| b.contains(r)), "missed {r}");
    }
    // No root at all: proven empty list.
    let q = PolynomialFunction::new(vec![1., 0., 1.]).unwrap(); // 1 + x²
    assert!(
        isolate_roots(Interval::new(-10., 10.).unwrap(), &q, 1e-6, 30)
            .unwrap()
            .is_empty()
    );
}

#[test]
fn tangent_root_is_never_lost() {
    // (x−0.5)²: even multiplicity, no sign change. Newton cannot prove
    // uniqueness (the derivative encloses zero near the root), and
    // f64 evaluation cannot resolve f ≈ 2e-16 against rounding fuzz, so
    // completeness delivers a small cluster of Indeterminate boxes around
    // the root instead of one tight box — but never loses it.
    let p = from_roots(&[0.5, 0.5]);
    let roots = isolate_roots(Interval::new(0., 1.).unwrap(), &p, 1e-8, 40).unwrap();
    assert!(!roots.is_empty(), "double root lost");
    for (b, c) in &roots {
        assert!(matches!(c, RootCertificate::Indeterminate));
        assert!(
            b.lo < 0.5 + 1e-6 && b.hi > 0.5 - 1e-6,
            "box [{}, {}] drifted away from 0.5",
            b.lo,
            b.hi
        );
    }
    assert!(roots.iter().any(|(b, _)| b.contains(0.5)));
}

#[test]
fn completeness_matches_brent_bruteforce() {
    let mut rng = Rng(41);
    for _ in 0..20 {
        let count = 1 + (rng.next() % 4) as usize;
        let roots: Vec<f64> = (0..count).map(|_| rng.range(-2., 2.)).collect();
        let p = from_roots(&roots);
        let isolated = isolate_roots(Interval::new(-3., 3.).unwrap(), &p, 1e-7, 40).unwrap();
        // Brent brute force: dense scan for sign-change brackets.
        let mut brent_roots = Vec::new();
        for i in 0..4000 {
            let a = -3. + 6. * i as f64 / 4000.;
            let b = -3. + 6. * (i + 1) as f64 / 4000.;
            let (fa, fb) = (p.value_at(a).unwrap(), p.value_at(b).unwrap());
            if fa.signum() != fb.signum() {
                if let Ok(r) = brent_bracketed(|x| p.value_at(x).unwrap(), a, b, 1e-12, 100) {
                    if !brent_roots.iter().any(|q: &f64| (q - r).abs() < 1e-9) {
                        brent_roots.push(r);
                    }
                }
            }
        }
        // Every Brent root lies in some isolated box, and the number of
        // isolated boxes equals the number of distinct roots (simple
        // random roots are simple with probability one; boxes may merge
        // only when roots are within tol, which the scan also merges).
        for r in &brent_roots {
            assert!(
                isolated.iter().any(|(b, _)| b.contains(*r)),
                "Brent root {r} missing from certified list"
            );
        }
        assert_eq!(isolated.len(), brent_roots.len());
    }
}

#[test]
fn krawczyk_scalar_contracts_to_root() {
    let p = from_roots(&[0.25, 0.8]);
    let x = Interval::new(0.1, 0.4).unwrap();
    let k = krawczyk_step(&p, x).unwrap().unwrap();
    assert!(k.lo > x.lo && k.hi < x.hi, "K(X) ⊂ int(X) proves existence");
    assert!(k.contains(0.25));
    // Root in the other half is not claimed here.
    assert!(!k.contains(0.8));
}

struct CircleLine;
impl IntervalSystem for CircleLine {
    // F(x,y) = (x² + y² − 1, x − y); roots at (±√2/2, ±√2/2).
    fn value_at(&self, x: &[f64]) -> Result<Vec<f64>> {
        Ok(vec![x[0] * x[0] + x[1] * x[1] - 1., x[0] - x[1]])
    }
    fn interval_jacobian(&self, x: &[Interval]) -> Result<Vec<Vec<Interval>>> {
        let two = Interval::point(2.);
        Ok(vec![
            vec![two.mul(x[0])?, two.mul(x[1])?],
            vec![Interval::point(1.), Interval::point(-1.)],
        ])
    }
}

#[test]
fn krawczyk_system_2x2() {
    let s = 0.7071067811865476;
    let x = [
        Interval::new(0.4, 1.0).unwrap(),
        Interval::new(0.4, 1.0).unwrap(),
    ];
    let k = krawczyk_step_system(&CircleLine, &x).unwrap().unwrap();
    assert!(k[0].contains(s) && k[1].contains(s));
    assert!(k[0].lo > x[0].lo && k[0].hi < x[0].hi);
    assert!(k[1].lo > x[1].lo && k[1].hi < x[1].hi);
}

#[test]
fn certified_extrema_of_polynomial_curve() {
    // Curve u ↦ (u, (u−0.3)(u−0.6), 0) as a Bézier segment: the y
    // coordinate has exactly one extremum, at u = 0.45.
    let y = |u: f64| (u - 0.3) * (u - 0.6);
    let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![0., y(0.), 0.],
            vec![0.5, 2. * y(0.5) - 0.5 * (y(0.) + y(1.)), 0.],
            vec![1., y(1.), 0.],
        ],
        weights: vec![1.; 3],
        periodic: false,
    };
    curve.validate().unwrap();
    // Sanity: the quadratic is reproduced.
    for i in 0..=10 {
        let u = i as f64 / 10.;
        let p = curve.evaluate(u).unwrap().point;
        assert!((p[1] - y(u)).abs() < 1e-13);
    }
    let extrema = certified_curve_extrema(&curve, 1e-10, 40).unwrap();
    assert_eq!(extrema.len(), 3);
    assert!(extrema[0].is_empty(), "x is monotone");
    assert!(extrema[2].is_empty(), "z is constant");
    assert_eq!(extrema[1].len(), 1);
    assert!(matches!(extrema[1][0].1, RootCertificate::Unique(_)));
    assert!(extrema[1][0].0.contains(0.45));
}
#[test]
fn budget_exhaustion_aborts_isolation_with_resource_error() {
    // Tiny iteration budget: even the first handful of subdivisions
    // cannot run, so isolation must abort with BudgetExhausted rather
    // than returning an incomplete list.
    let p = from_roots(&[0.3, 0.7]);
    let err = isolate_roots_with_budget(
        Interval::new(0., 1.).unwrap(),
        &p,
        1e-9,
        40,
        Budget::new(1, 64, 60_000).unwrap(),
    )
    .unwrap_err();
    assert_eq!(err.code, crate::RESOURCE_LIMIT, "{err:?}");
    assert!(err.contains("isolate_roots"), "{err}");
    assert!(err.contains("iteration budget"), "{err}");
}

#[test]
fn canonical_box_equality_collapses_negative_zero() {
    let a = Interval::new(-0.0, 1.).unwrap();
    let b = Interval::new(0.0, 1.).unwrap();
    assert!(same_box(&a, &b));
    let c = Interval::new(0., 2.).unwrap();
    assert!(!same_box(&a, &c));
}

#[test]
fn structurally_constant_coordinates_have_no_isolated_extrema_and_keep_budget_validation() {        let curve = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![2., -3., 0.]; 3],
        weights: vec![1., 2., 1.],
        periodic: false,
    };
    let report = certified_curve_extrema(&curve, 1e-10, 40).unwrap();
    assert_eq!(report.len(), 3);
    assert!(report.iter().all(Vec::is_empty));
    for tol in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(certified_curve_extrema(&curve, tol, 40).is_err());
    }
    assert!(certified_curve_extrema(&curve, 1e-10, 61).is_err());
}
