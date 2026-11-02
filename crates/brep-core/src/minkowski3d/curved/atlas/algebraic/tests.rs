use super::*;
fn q(n: i64, d: i64) -> ExactRatio {
    roots::ratio(&Q::new(n.into(), d.into()))
}
fn parabola() -> crate::Curve {
    crate::Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        weights: vec![1.; 3],
        periodic: false,
        control_points: vec![vec![0., 0., 0.], vec![0., 0.5, 0.], vec![1., 1., 0.]],
    }
}
#[test]
fn nonlinear_knot_has_an_actual_irrational_root_and_verified_sides() {
    let c = parabola();
    let level = q(1, 2);
    let cert = certify_algebraic_knot_parameters(&c, 0, &level).unwrap();
    assert_eq!(cert.roots.len(), 1);
    assert_eq!(cert.roots[0].sides, [-1, 1]);
    let r = &cert.roots[0].interval;
    assert_ne!(r[0], r[1]);
    verify_algebraic_knot_parameters(&c, 0, &level, &cert).unwrap();
    let mut bad = cert.clone();
    bad.roots[0].sides = [1, -1];
    assert!(verify_algebraic_knot_parameters(&c, 0, &level, &bad).is_err());
    let mut bad = cert.clone();
    bad.roots.clear();
    assert!(verify_algebraic_knot_parameters(&c, 0, &level, &bad).is_err());
    let mut bad = cert.clone();
    bad.roots[0].interval = [q(1, 2), q(2, 3)];
    assert!(verify_algebraic_knot_parameters(&c, 0, &level, &bad).is_err());
    assert!(verify_algebraic_knot_parameters(&c, 1, &level, &cert).is_err());
    let mut uv = c.clone();
    for point in &mut uv.control_points {
        point.pop();
    }
    let uv_cert = certify_algebraic_knot_parameters(&uv, 0, &level).unwrap();
    verify_algebraic_knot_parameters(&uv, 0, &level, &uv_cert).unwrap();
    let mut changed = c;
    changed.knots = vec![0., 0., 0., 2., 2., 2.];
    assert!(verify_algebraic_knot_parameters(&changed, 0, &level, &cert).is_err());
}
#[test]
fn rational_spatial_vertex_can_have_an_irrational_seam_parameter() {
    let mut c = parabola();
    c.control_points[1][1] = 0.;
    // A(t)=(t²,t²,0), point=(1/2,1/2,0), parameter=sqrt(1/2).
    let point = [q(1, 2), q(1, 2), q(0, 1)];
    let cert = certify_algebraic_seam_parameters(&c, &point).unwrap();
    verify_algebraic_seam_parameters(&c, &point, &cert).unwrap();
    assert_ne!(cert.roots[0].interval[0], cert.roots[0].interval[1]);
    // A fake rational root INSIDE the honest isolating interval passes both
    // interval root counts, but must fail the exact source-coordinate identity.
    let lo = roots::decode(&cert.roots[0].interval[0]).unwrap();
    let hi = roots::decode(&cert.roots[0].interval[1]).unwrap();
    let midpoint = (lo + hi) / integer(2);
    let mut fake = cert.clone();
    fake.roots[0].polynomial = roots::polynomial(&[-midpoint, Q::one()]);
    assert!(verify_algebraic_seam_parameters(&c, &point, &fake).is_err());
    let mut bad = cert.clone();
    bad.roots.push(bad.roots[0].clone());
    assert!(verify_algebraic_seam_parameters(&c, &point, &bad).is_err());
    let mut bad = cert.clone();
    bad.roots[0].polynomial = vec![q(-1, 3), q(0, 1), q(1, 1)];
    assert!(verify_algebraic_seam_parameters(&c, &point, &bad).is_err());
    assert!(verify_algebraic_seam_parameters(&c, &[q(1, 2), q(1, 3), q(0, 1)], &cert).is_err());
}
#[test]
fn irrational_tangency_retains_even_multiplicity() {
    let c = crate::Curve {
        degree: 4,
        knots: vec![0., 0., 0., 0., 0., 1., 1., 1., 1., 1.],
        weights: vec![1.; 5],
        periodic: false,
        control_points: vec![15., 15., 5., -15., 15.]
            .into_iter()
            .map(|x| vec![x, 0., 0.])
            .collect(),
    };
    let cert = certify_algebraic_knot_parameters(&c, 0, &q(0, 1)).unwrap();
    assert_eq!(cert.roots.len(), 1);
    assert_eq!(cert.roots[0].sides, [1, 1]);
    assert_ne!(cert.roots[0].interval[0], cert.roots[0].interval[1]);
    verify_algebraic_knot_parameters(&c, 0, &q(0, 1), &cert).unwrap();
}
#[test]
fn tangent_and_multiple_branches_keep_the_complete_root_census() {
    let mut c = parabola();
    c.control_points = vec![vec![0., 0., 0.], vec![0.5, 0., 0.], vec![0., 1., 0.]];
    let cert = certify_algebraic_knot_parameters(&c, 0, &q(1, 8)).unwrap();
    assert_eq!(cert.roots.len(), 2);
    assert_eq!(cert.roots[0].sides, [-1, 1]);
    assert_eq!(cert.roots[1].sides, [1, -1]);
    let mut bad = cert.clone();
    bad.roots.pop();
    assert!(verify_algebraic_knot_parameters(&c, 0, &q(1, 8), &bad).is_err());
    let tangent = certify_algebraic_knot_parameters(&c, 0, &q(1, 4)).unwrap();
    assert_eq!(tangent.roots.len(), 1);
    assert_eq!(tangent.roots[0].interval, [q(1, 2), q(1, 2)]);
    let endpoints = certify_algebraic_knot_parameters(&c, 0, &q(0, 1)).unwrap();
    assert_eq!(endpoints.roots.len(), 2);
    assert!(certify_algebraic_knot_parameters(&c, 2, &q(0, 1)).is_err());
}
