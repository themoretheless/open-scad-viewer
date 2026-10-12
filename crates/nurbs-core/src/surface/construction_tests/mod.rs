use super::*;
#[test]
fn clamped_iso_preserves_authored_coordinates_and_nonuniform_weights() {
    let c=Curve {degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.1,0.2,0.3],vec![0.7,-0.2,0.8],vec![1.1,0.4,-0.3]],weights:vec![0.7,1.3,0.9],periodic:false};
    let mut other=c.clone();
    for p in &mut other.control_points {p[2]+=2.;}
    let surface=loft(&[c.clone(),other.clone()]).unwrap();
    let before=surface.clone();
    assert_eq!(surface.iso(Axis::V,0.).unwrap(),c);
    assert_eq!(surface.iso(Axis::V,1.).unwrap(),other);
    assert_eq!(surface,before);
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
    let q =
        Curve::from_polyline(vec![vec![0.; 3], vec![0., 0., 2.], vec![3., 0., 4.]]).unwrap();
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
