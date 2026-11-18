use super::*;
#[test]
fn sphere_boundary_parameter_preserves_legacy_and_rational_traversals(){
    for weights in [vec![1.,sphere_sphere::ARC_WEIGHT,1.],vec![1.,1.,2.]] {
        let curve=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],
            control_points:vec![vec![1.,0.],vec![1.,1.],vec![0.,1.]],weights,periodic:false};
        assert!(sphere_sphere::unit_quarter_arc(&curve));
        for i in 0..=100 {
            let t=i as f64/100.;let point=curve.evaluate(t).unwrap().point;
            let back=piece_tau(1,[point[0],point[1]],&curve);
            assert!((back-t).abs()<5e-16,"weights={:?}, t={t}, recovered={back}",curve.weights);
        }
    }
}
