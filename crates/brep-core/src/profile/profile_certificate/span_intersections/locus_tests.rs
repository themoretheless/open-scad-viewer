use super::*;
#[test]
fn cross_span_constant_fibers_exclude_the_global_diagonal(){let c=Curve{degree:1,control_points:vec![vec![0.5,0.];3],weights:vec![1.;3],knots:vec![0.,0.,0.5,1.,1.],periodic:false};let proof=certify_span_intersections(&c,&c,true,1e-10).unwrap();let r=verify_span_intersections(&c,&c,&proof).unwrap();assert_eq!(r.algebraic_loci.len(),3);assert!(r.algebraic_loci.iter().all(|l|l.diagonal_excluded));assert_eq!(r.stationary_spans,vec![[0.,0.5],[0.5,1.]])}
fn curve(points: &[[f64; 2]]) -> Curve {
    let d = points.len() - 1;
    Curve {
        degree: d,
        control_points: points.iter().map(|p| p.to_vec()).collect(),
        weights: vec![1.; points.len()],
        knots: [vec![0.; d + 1], vec![1.; d + 1]].concat(),
        periodic: false,
    }
}
#[test]
fn complete_constant_fibers_in_both_parameter_directions() {
    let a = curve(&[[0.5, 0.], [0.5, 0.]]);
    let b = curve(&[[0., 0.], [0., 0.], [1., 0.]]);
    for (a, b) in [(&a, &b), (&b, &a)] {
        let c = certify_span_intersections(a, b, false, 1e-10).unwrap();
        assert_eq!(c.version, 3);
        let r = verify_span_intersections(a, b, &c).unwrap();
        assert_eq!(r.algebraic_loci.len(), 1);
        assert!(!r.algebraic_loci[0].whole_square);
        let mut bad = c.clone();
        bad.pairs[0].relation[0].clear();
        assert!(verify_span_intersections(a, b, &bad).is_err())
    }
}
#[test]
fn complete_constant_self_square_and_stationary_span() {
    let a = curve(&[[2., 3.], [2., 3.], [2., 3.]]);
    let c = certify_span_intersections(&a, &a, true, 1e-10).unwrap();
    let r = verify_span_intersections(&a, &a, &c).unwrap();
    assert!(r.algebraic_loci[0].whole_square && r.algebraic_loci[0].diagonal_excluded);
    assert_eq!(r.stationary_spans, vec![[0., 1.]]);
    let mut bad = c;
    bad.pairs.clear();
    assert!(verify_span_intersections(&a, &a, &bad).is_err())
}
#[test]
fn different_constant_images_have_no_intersections() {
    let a = curve(&[[2., 3.], [2., 3.]]);
    let b = curve(&[[4., 3.], [4., 3.]]);
    let c = certify_span_intersections(&a, &b, false, 1e-10).unwrap();
    let r = verify_span_intersections(&a, &b, &c).unwrap();
    assert!(r.events.is_empty() && r.algebraic_loci.is_empty() && r.overlaps.is_empty())
}
#[test]
fn nonmonotone_retracing_is_a_complete_off_diagonal_locus() {
    let a = curve(&[[1., 0.], [-1., 0.], [1., 0.]]);
    let c = certify_span_intersections(&a, &a, true, 1e-10).unwrap();
    let r = verify_span_intersections(&a, &a, &c).unwrap();
    assert_eq!(r.algebraic_loci.len(), 1);
    assert!(r.algebraic_loci[0].diagonal_excluded);
    assert_eq!(r.singular.len(), 1);
    assert!(r.singular[0][0] <= 0.5 && r.singular[0][1] >= 0.5);
    let mut bad = c;
    bad.pairs[0].relation.clear();
    assert!(verify_span_intersections(&a, &a, &bad).is_err())
}
