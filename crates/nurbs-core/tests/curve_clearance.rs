use nurbs_core::{
    curve::Curve,
    curve_distance::{self, ClearanceStatus},
};
fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
}
#[test]
fn whole_original_product_is_separated_without_minimum_convergence() {
    let a = line([0., 0.], [10., 0.]);
    let b = line([0., 2.], [10., 2.]);
    let r = curve_distance::prove_clearance(&a, &b, 1.5, 1000).unwrap();
    assert_eq!(r.status, ClearanceStatus::AtLeast);
    assert!(r.distance_bounds[0] >= 1.5 && r.distance_bounds[0] <= 2.);
    assert!(r.distance_bounds[1] >= 2.);
    assert!(r.unresolved_domain.is_none());
}
#[test]
fn close_pairs_and_budget_exhaustion_are_distinct() {
    let a = line([0., 0.], [1., 0.]);
    let b = line([0.2, -1.], [0.2, 1.]);
    let r = curve_distance::prove_clearance(&a, &b, 0.1, 10000).unwrap();
    assert_eq!(r.status, ClearanceStatus::Below);
    assert!(r.distance_bounds[1] < 0.1);
    let small = curve_distance::prove_clearance(&a, &b, 0.1, 1).unwrap();
    assert_eq!(small.status, ClearanceStatus::Unresolved);
    assert_eq!(small.unresolved_domain, Some([a.domain(), b.domain()]));
    assert_eq!(small.cells, 1);
    assert!(curve_distance::prove_clearance(&a, &b, -1., 100).is_err());
}
