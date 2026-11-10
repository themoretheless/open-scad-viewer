use super::*;

#[test]
fn rational_boundary_uses_its_active_origin_without_cancellation() {
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0.1, 0., 0.], vec![0.1, 1., 0.]],
            vec![vec![0., 0., 1.], vec![0., 1., 1.]],
        ],
        weights: vec![vec![1., 2.], vec![1., 2.]],
        periodic_u: false,
        periodic_v: false,
    };
    let before = surface.clone();
    let mut cached = SurfaceEvaluator::new(&surface).unwrap();
    let snapshot = SurfaceSampler::new(&surface).unwrap();
    let borrowed = SurfaceRefSampler::new(&surface).unwrap();
    for i in 0..=20 {
        let v = i as f64 / 20.;
        let expected_y = 2. * v / (1. + v);
        let direct = surface.evaluate(1., v).unwrap();
        let ref_eval = borrowed.evaluate(1., v).unwrap();
        let ref_pt = borrowed.point(1., v).unwrap();
        let snap_pt = snapshot.point(1., v).unwrap();
        for axis in 0..3 {
            assert_eq!(ref_eval.point[axis].to_bits(), direct.point[axis].to_bits());
            assert_eq!(ref_pt[axis].to_bits(), direct.point[axis].to_bits());
            assert_eq!(snap_pt[axis].to_bits(), direct.point[axis].to_bits());
        }
        for p in [
            direct,
            cached.evaluate(1., v).unwrap(),
            snapshot.evaluate(1., v).unwrap(),
            ref_eval,
        ] {
            assert_eq!(p.point[0], 0., "v={v}: {:?}", p.point);
            assert_eq!(p.point[2], 1.);
            assert!((p.point[1] - expected_y).abs() <= 4. * f64::EPSILON);
            assert!(p.du.unwrap()[0] < 0.);
            assert!(p.dv.unwrap()[1] > 0.);
        }
    }
    assert_eq!(surface, before);
}
