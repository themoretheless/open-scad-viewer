use brep_core::profile_certificate as p;
use nurbs_core::curve::Curve;
use std::sync::Arc;
fn q(a: i64, b: i64) -> [String; 2] {
    [a.to_string(), b.to_string()]
}
fn square(lo: f64, hi: f64) -> Vec<Curve> {
    let v = [[lo, lo], [hi, lo], [hi, hi], [lo, hi], [lo, lo]];
    v.windows(2)
        .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
        .collect()
}
fn fill(loops: &[Vec<Curve>], weights: &[i32]) -> Arc<p::OwnedOriginalFilledBoundaries> {
    Arc::new(
        p::assemble_original_filled_boundaries(loops, weights, 1e-12, Default::default()).unwrap(),
    )
}
fn winding(owner: Arc<p::OwnedOriginalFilledBoundaries>, point: [[String; 2]; 2]) -> Vec<i64> {
    let w = p::own_original_boundary_winding(owner, point, Default::default()).unwrap();
    w.reverify_source().unwrap();
    w.windings().to_vec()
}
#[test]
fn exact_component_winding_distinguishes_outer_and_hole() {
    let owner = fill(&[square(0., 4.), square(1., 3.)], &[1, -1]);
    assert_eq!(winding(owner.clone(), [q(2, 1), q(2, 1)]), [1, -1]);
    assert_eq!(winding(owner.clone(), [q(1, 2), q(1, 2)]), [1, 0]);
    assert_eq!(winding(owner, [q(-1, 1), q(2, 1)]), [0, 0]);
}
#[test]
fn artificial_cut_on_query_ray_uses_one_sided_original_germs() {
    let owner = fill(&[square(0., 2.), square(1., 3.)], &[1, 1]);
    assert_eq!(winding(owner.clone(), [q(1, 2), q(1, 1)]), [1]);
    assert_eq!(winding(owner.clone(), [q(3, 2), q(3, 2)]), [1]);
    assert_eq!(winding(owner.clone(), [q(7, 2), q(1, 1)]), [0]);
    assert!(
        p::own_original_boundary_winding(owner, [q(2, 1), q(1, 1)], Default::default()).is_err()
    );
}
#[test]
fn actual_irrational_curve_cuts_are_compared_before_ray_contribution() {
    let arch = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        weights: vec![1.; 3],
        control_points: vec![vec![-3., 0.], vec![0., 6.], vec![3., 0.]],
        periodic: false,
    };
    let base = Curve::from_polyline(vec![vec![3., 0.], vec![-3., 0.]]).unwrap();
    let points = [[-4., 1.5], [4., 1.5], [4., 4.], [-4., 4.], [-4., 1.5]];
    let box_wire = points
        .windows(2)
        .map(|p| Curve::from_polyline(vec![p[0].to_vec(), p[1].to_vec()]).unwrap())
        .collect();
    let owner = fill(&[vec![arch, base], box_wire], &[-1, 1]);
    assert_eq!(winding(owner.clone(), [q(0, 1), q(1, 1)]), [1]);
    assert_eq!(winding(owner.clone(), [q(0, 1), q(5, 4)]), [1]);
    // This original component edge is internal to the retained union.
    assert_eq!(winding(owner.clone(), [q(0,1),q(3,2)]),[1]);
    // The actual retained outer edge must still refuse a winding query.
    assert!(p::own_original_boundary_winding(owner.clone(),[q(-4,1),q(2,1)],Default::default()).is_err());
    assert_eq!(winding(owner.clone(), [q(-5,1),q(3,2)]), [0]);
    assert_eq!(winding(owner.clone(), [q(0, 1), q(7, 2)]), [1]);
    assert_eq!(winding(owner, [q(5, 1), q(3, 2)]), [0]);
}
