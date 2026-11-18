use super::*;
#[test]
fn clamped_iso_preserves_authored_coordinates_and_nonuniform_weights() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![0.1, 0.2, 0.3],
            vec![0.7, -0.2, 0.8],
            vec![1.1, 0.4, -0.3],
        ],
        weights: vec![0.7, 1.3, 0.9],
        periodic: false,
    };
    let mut other = c.clone();
    for p in &mut other.control_points {
        p[2] += 2.;
    }
    let surface = loft(&[c.clone(), other.clone()]).unwrap();
    let before = surface.clone();
    assert_eq!(surface.iso(Axis::V, 0.).unwrap(), c);
    assert_eq!(surface.iso(Axis::V, 1.).unwrap(), other);
    assert_eq!(surface, before);
}
#[test]
fn translational_sweep_retains_small_profile_at_distant_path_origin() {
    let profile = Curve::from_polyline(vec![vec![1e-9, 0., 0.], vec![2e-9, 0., 0.]]).unwrap();
    let path = Curve::from_polyline(vec![vec![1e9, 0., 0.], vec![1e9, 0., 5.]]).unwrap();
    let surface = sweep(&profile, &path).unwrap();
    for (i, p) in profile.control_points.iter().enumerate() {
        assert_eq!(&surface.control_points[i][0], p);
    }
    for u in [0., 0.17, 0.5, 0.83, 1.] {
        for v in [0., 0.23, 0.7, 1.] {
            let p = surface.evaluate(u, v).unwrap().point;
            assert!((p[0] - (1. + u) * 1e-9).abs() < 1e-23);
            assert!((p[2] - 5. * v).abs() < 1e-12);
        }
    }
}
#[test]
fn translational_sweep_admits_extreme_common_weight_scales() {
    let mut profile = Curve::from_polyline(vec![vec![1., 0., 0.], vec![2., 0., 0.]]).unwrap();
    let mut path = Curve::from_polyline(vec![vec![0., 0., 0.], vec![0., 0., 5.]]).unwrap();
    for scale in [1e-12, 1e12] {
        profile.weights.fill(scale);
        path.weights.fill(scale);
        let surface = sweep(&profile, &path).unwrap();
        assert!(surface.weights.iter().flatten().all(|w| *w == 1.));
        let p = surface.evaluate(0.37, 0.61).unwrap().point;
        assert!((p[0] - 1.37).abs() < 1e-12 && (p[2] - 3.05).abs() < 1e-12);
    }
    profile.weights = vec![1e-12, 1.];
    path.weights = vec![1e-12, 1.];
    assert!(sweep(&profile, &path).is_err());
}
#[test]
fn rational_sweep_matches_sum_of_curves() {
    let p = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        periodic: false,
    };
    let q = Curve::from_polyline(vec![vec![0.; 3], vec![0., 0., 2.], vec![3., 0., 4.]]).unwrap();
    let s = sweep(&p, &q).unwrap();
    for (u, v) in [(0.2, 0.3), (0.8, 1.5)] {
        let actual = s.evaluate(u, v).unwrap().point;
        let a = p.evaluate(u).unwrap().point;
        let b = q.evaluate(v).unwrap().point;
        for k in 0..3 {
            assert!((actual[k] - a[k] - b[k]).abs() < 1e-10);
        }
    }
}
#[test]
fn loft_aligns_degrees_and_domains() {
    let a = Curve::from_polyline(vec![vec![0.; 3], vec![1., 0., 0.]]).unwrap();
    let mut b = a.elevate(2).unwrap();
    for p in &mut b.control_points {
        p[2] = 3.;
    }
    for k in &mut b.knots {
        *k = 2. + *k * 4.;
    }
    let s = loft_aligned(&[a.clone(), b.clone()]).unwrap();
    for u in [0., 0.2, 0.7, 1.] {
        let x = s.evaluate(u, 0.).unwrap().point;
        let y = s.evaluate(u, 1.).unwrap().point;
        let expected = b.evaluate(2. + u * 4.).unwrap().point;
        for k in 0..3 {
            assert!((x[k] - a.evaluate(u).unwrap().point[k]).abs() < 1e-10);
            assert!((y[k] - expected[k]).abs() < 1e-10);
        }
    }
}
#[test]
fn ruled_surface_interpolates_compatible_and_elevated_curves() {
    let c0 = Curve::from_polyline(vec![vec![0., 0., 0.], vec![2., 0., 0.]]).unwrap();
    let c1 = Curve::from_polyline(vec![vec![0., 3., 1.], vec![2., 3., 1.]]).unwrap();
    let s = ruled_surface(&c0, &c1).unwrap();
    assert_eq!(s.degree_v, 1);
    assert_eq!(s.knots_v, vec![0., 0., 1., 1.]);
    let mid = s.evaluate(0.5, 0.5).unwrap().point;
    assert!((mid[0] - 1.0).abs() < 1e-12);
    assert!((mid[1] - 1.5).abs() < 1e-12);
    assert!((mid[2] - 0.5).abs() < 1e-12);

    let c1_deg2 = c1.elevate(2).unwrap();
    let s_compat = Surface::ruled_surface(&c0, &c1_deg2).unwrap();
    assert_eq!(s_compat.degree_u, 2);
    assert_eq!(s_compat.degree_v, 1);
    assert_eq!(s_compat.knots_v, vec![0., 0., 1., 1.]);
    let mid2 = s_compat.evaluate(0.5, 0.5).unwrap().point;
    for k in 0..3 {
        assert!((mid2[k] - mid[k]).abs() < 1e-12);
    }
}
