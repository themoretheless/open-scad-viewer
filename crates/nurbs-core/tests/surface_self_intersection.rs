use nurbs_core::{
    surface::Surface,
    surface_self_intersection::{self, Status},
};
fn graph() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 1.], vec![0.5, 1., 1.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn global_graph_and_rotated_graph_are_proven_without_mesh_samples() {
    let mut s = graph();
    assert_eq!(
        surface_self_intersection::inspect(&s, 100).unwrap().status,
        Status::Absent
    );
    for p in s.control_points.iter_mut().flatten() {
        p.swap(1, 2);
    }
    assert_eq!(
        surface_self_intersection::inspect(&s, 100).unwrap().status,
        Status::Absent
    );
}
#[test]
fn exact_fold_has_distinct_interior_witness_not_boundary_alias() {
    let mut s = graph();
    s.control_points[2] = s.control_points[0].clone();
    let r = surface_self_intersection::inspect(&s, 100).unwrap();
    assert_eq!(r.status, Status::Present);
    let w = r.witness.unwrap();
    assert_ne!(w[0], w[1]);
    // x=u(1-u), y=v, z=2u(1-u): analytical coincidence at 1/4 and 3/4.
    for uv in w {
        let p = s.evaluate(uv[0], uv[1]).unwrap().point;
        assert!((p[0] - 0.1875).abs() < 1e-14);
        assert!((p[2] - 0.375).abs() < 1e-14);
    }
}
#[test]
fn incomplete_coverage_and_invalid_input_never_claim_absence() {
    let mut s = graph();
    s.degree_u = 1;
    s.knots_u = vec![0., 0., 0.5, 1., 1.];
    assert_eq!(
        surface_self_intersection::inspect(&s, 1).unwrap().status,
        Status::Unresolved
    );
    assert!(surface_self_intersection::inspect(&s, 0).is_err());
    s.weights[0][0] = f64::NAN;
    assert!(surface_self_intersection::inspect(&s, 100).is_err());
}
