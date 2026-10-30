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
    for i in 0..=20 {
        let v = i as f64 / 20.;
        let expected_y = 2. * v / (1. + v);
        for p in [
            surface.evaluate(1., v).unwrap(),
            cached.evaluate(1., v).unwrap(),
            snapshot.evaluate(1., v).unwrap(),
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
