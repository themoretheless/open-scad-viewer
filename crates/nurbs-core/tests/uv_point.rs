use nurbs_core::{
    curve::Curve,
    trim_domain::{Location, TrimDomain},
};
fn square(lo: f64, hi: f64) -> Curve {
    Curve::from_polyline(vec![
        vec![lo, lo],
        vec![hi, lo],
        vec![hi, hi],
        vec![lo, hi],
        vec![lo, lo],
    ])
    .unwrap()
}
#[test]
fn holes_and_nested_islands_obey_independent_winding_rules() {
    let outer = square(0., 10.);
    let hole = square(2., 8.).reverse().unwrap();
    let island = square(4., 6.);
    let d = TrimDomain::new(&[vec![outer], vec![hole], vec![island]], 1e-6).unwrap();
    for (p, location, winding) in [
        ([-1., 5.], Location::Outside, 0),
        ([1., 5.], Location::Inside, 1),
        ([3., 5.], Location::Outside, 0),
        ([5., 5.], Location::Inside, 1),
    ] {
        let r = d.classify_point(p, 4096).unwrap();
        assert_eq!(r.location, location);
        assert_eq!(r.winding, Some(winding));
    }
    for p in [[0., 5.], [2., 5.], [4., 5.], [6., 5.], [8., 5.], [10., 5.]] {
        assert_eq!(
            d.classify_point(p, 4096).unwrap().location,
            Location::Unresolved
        );
    }
}
#[test]
fn loop_orientation_is_explicit_not_inferred_from_nesting() {
    let d = TrimDomain::new(&[vec![square(0., 10.)], vec![square(2., 8.)]], 1e-6).unwrap();
    let r = d.classify_point([5., 5.], 4096).unwrap();
    assert_eq!(r.location, Location::Inside);
    assert_eq!(r.winding, Some(2));
    let d = TrimDomain::new(&[vec![square(0., 10.).reverse().unwrap()]], 1e-6).unwrap();
    assert_eq!(d.classify_point([5., 5.], 4096).unwrap().winding, Some(-1));
}
#[test]
fn point_validation_budget_and_rectangle_equivalence() {
    let d = TrimDomain::new(&[vec![square(0., 10.)]], 1e-6).unwrap();
    let p = [5., 5.];
    let a = d.classify_point(p, 4096).unwrap();
    let b = d.classify(p.map(|x| [x, x]), 4096).unwrap();
    assert_eq!(a.location, b.location);
    assert_eq!(a.winding, b.winding);
    assert_eq!(a.cells, b.cells);
    assert!(d.classify_point([f64::NAN, 0.], 4096).is_err());
    assert!(d.classify_point(p, 0).is_err());
    assert!(d.classify_point(p, 100001).is_err());
    assert_eq!(
        d.classify_point(p, 1).unwrap().location,
        Location::Unresolved
    );
}
