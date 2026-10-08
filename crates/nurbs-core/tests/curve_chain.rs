use nurbs_core::{curve::Curve, curve_chain};
fn square() -> Vec<Curve> {
    let p = [
        vec![0., 0., 2.],
        vec![1., 0., 2.],
        vec![1., 1., 2.],
        vec![0., 1., 2.],
    ];
    [2, 0, 3, 1]
        .iter()
        .map(|&i| Curve::from_polyline(vec![p[i].clone(), p[(i + 1) % 4].clone()]).unwrap())
        .collect()
}
#[test]
fn reordered_closed_chain_has_exact_endpoint_evidence_without_mutation() {
    let c = square();
    let before = c.clone();
    let r = curve_chain::assemble(&c, 1e-8, 16).unwrap();
    assert!(r.all_chains_closed_within_tolerance && r.all_endpoints_equal);
    assert_eq!(r.cyclic_chains.len(), 1);
    assert_eq!(r.cyclic_chains[0].len(), 4);
    assert_eq!(r.pairs.len(), 4);
    assert_eq!(r.pair_tests, 16);
    assert!(r.pairs.iter().all(|p| p.gap_bounds == [0.; 2]));
    assert_eq!(c, before);
}
#[test]
fn gaps_ambiguity_and_work_exhaustion_remain_explicit() {
    let mut c = square();
    c[0].control_points[0][0] += 1e-5;
    let r = curve_chain::assemble(&c, 1e-4, 16).unwrap();
    assert!(r.all_chains_closed_within_tolerance);
    assert!(!r.all_endpoints_equal);
    assert!(
        !curve_chain::assemble(&c, 1e-8, 16)
            .unwrap()
            .all_chains_closed_within_tolerance
    );
    let mut c = square();
    c.push(c[0].clone());
    let r = curve_chain::assemble(&c, 1e-8, 25).unwrap();
    assert!(!r.all_chains_closed_within_tolerance);
    assert!(r.cyclic_chains.is_empty());
    assert_eq!(r.unresolved_curves.len(), 5);
    let r = curve_chain::assemble(&square(), 1e-8, 15).unwrap();
    assert_eq!(r.pair_tests, 0);
    assert_eq!(r.unresolved_curves.len(), 4);
}
#[test]
fn disconnected_cycles_and_single_closed_curve_are_preserved() {
    let mut c = square();
    let mut second = square();
    for curve in &mut second {
        for p in &mut curve.control_points {
            p[0] += 10.;
        }
    }
    c.extend(second);
    let r = curve_chain::assemble(&c, 1e-8, 64).unwrap();
    assert_eq!(r.cyclic_chains.len(), 2);
    assert!(r.all_endpoints_equal);
    let c =
        Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.], vec![0., 1.], vec![0., 0.]]).unwrap();
    assert!(
        curve_chain::assemble(&[c], 1e-8, 1)
            .unwrap()
            .all_endpoints_equal
    );
    assert!(curve_chain::assemble(&[], 1e-8, 0).is_err());
}

#[test]
fn closure_binds_exact_shared_endpoints_and_continuous_rational_change_bound() {
    let mut c = square();
    c[0].control_points[0][0] += 1e-5;
    // Positive unequal weights make the curve nonlinear in its parameter;
    // the control-hull displacement proof still covers the complete curve.
    for edge in &mut c {
        edge.weights = vec![1., 3.];
    }
    let before = c.clone();
    let r = curve_chain::close(&c, 1e-4, 1e-4, 16).unwrap();
    assert!(r.within_tolerance);
    let bound = r.max_deviation_upper_bound.unwrap();
    assert!(bound >= 1e-5 && bound <= 1.000001e-5);
    let output = r.curves.unwrap();
    assert!(
        curve_chain::assemble(&output, 1e-12, 16)
            .unwrap()
            .all_endpoints_equal
    );
    for (a, b) in c.iter().zip(&output) {
        assert_eq!(a.weights, b.weights);
        assert_eq!(a.knots, b.knots);
        assert_eq!(a.degree, b.degree);
        for i in 0..=100 {
            let t = i as f64 / 100.;
            let p = a.evaluate(t).unwrap().point;
            let q = b.evaluate(t).unwrap().point;
            let d = (0..3).map(|j| (p[j] - q[j]).powi(2)).sum::<f64>().sqrt();
            assert!(d <= bound + 1e-15);
        }
    }
    assert_eq!(c, before);
    let r = curve_chain::close(&c, 1e-4, 1e-8, 16).unwrap();
    assert!(!r.within_tolerance);
    assert!(r.curves.is_none());
    assert!(r.max_deviation_upper_bound.is_some());
    assert!(
        curve_chain::close(&square(), 1e-8, 0., 16)
            .unwrap()
            .within_tolerance
    );
    let r = curve_chain::close(&c, 1e-4, 1e-4, 15).unwrap();
    assert!(!r.within_tolerance);
    assert!(r.curves.is_none());
}
