use super::*;
fn source(m: usize, weighted: bool) -> Surface {
    let p = 2 * m;
    let choose = |n: usize, k: usize| {
        if n < k {
            0
        } else {
            (0..k).fold(1, |v, i| v * (n - i) / (i + 1))
        }
    };
    Surface {
        degree_u: p,
        degree_v: 1,
        knots_u: [vec![0.; p + 1], vec![1.; p + 1]].concat(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..=p)
            .map(|i| {
                (0..2)
                    .map(|v| {
                        vec![
                            choose(i, m) as f64,
                            (choose(i, m) * v) as f64,
                            f64::from(i == p),
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: (0..=p)
            .map(|i| {
                if weighted {
                    vec![2f64.powi(i as i32 % 3 - 1), 2f64.powi(i as i32 % 3 - 2)]
                } else {
                    vec![1.; 2]
                }
            })
            .collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn oriented(mut s: Surface, b: Boundary) -> Surface {
    let (axis, end) = axes(b);
    if end {
        s.control_points.reverse();
        s.weights.reverse();
    }
    if axis == 1 {
        s = view(&s, Boundary::VMin);
    }
    s
}
#[test]
fn original_rational_plane_limits_cover_all_boundary_axes_and_vanishing_orders() {
    for m in [1, 2, 3] {
        for weighted in [false, true] {
            for boundary in [
                Boundary::UMin,
                Boundary::UMax,
                Boundary::VMin,
                Boundary::VMax,
            ] {
                let s = oriented(source(m, weighted), boundary);
                let before = s.clone();
                let r = prepare_plane_limit(&s, boundary, 100000000).unwrap();
                let proof = r.certificate.expect(r.reason);
                assert!(std::ptr::eq(proof.surface(), &s));
                assert_eq!(axes(proof.boundary()), axes(boundary));
                assert_eq!(proof.factor_order(), 2 * m - 1);
                assert_eq!(proof.point(), [0.; 3]);
                let n = proof.normal_bounds();
                let negative = (axes(boundary).0 == 1) ^ axes(boundary).1;
                assert!(if negative { n[2][1] < 0. } else { n[2][0] > 0. });
                assert_eq!(s, before);
            }
        }
    }
}
#[test]
fn equal_meridian_endpoint_normals_do_not_prove_one_whole_pole_plane() {
    let s = Surface {
        degree_u: 2,
        degree_v: 4,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: [vec![0.; 5], vec![1.; 5]].concat(),
        control_points: (0..3)
            .map(|u| {
                (0..5)
                    .map(|v| {
                        vec![
                            u as f64 * 0.5,
                            if u == 2 { v as f64 * 0.25 } else { 0. },
                            f64::from(u == 2 && v == 2),
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 5]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    // S=(u,u²v,u²*6v²(1-v)²): endpoint limits both point along Z,
    // but interior meridians have genuinely different limiting tangent planes.
    let old = prepare(&s, false, 100000000)
        .unwrap()
        .certificate
        .unwrap()
        .boundary_limits(1e-9)
        .unwrap();
    assert!(!old.distinct_tangent_limits);
    assert!(old.sine_squared_interval.unwrap()[1] < 1e-9);
    let r = prepare_plane_limit(&s, Boundary::UMin, 100000000).unwrap();
    assert!(r.certificate.is_none());
    assert_eq!(r.reason, "pole-plane-common-line-unproven");
}
#[test]
fn affine_rational_limits_compare_exactly_without_rounded_normal_candidates() {
    let mut a = source(1, true);
    let mut b = source(2, true);
    for s in [&mut a, &mut b] {
        for point in s.control_points.iter_mut().flatten() {
            let [x, y, z] = [point[0], point[1], point[2]];
            *point = vec![3. + x + 0.25 * y, -7. + 2. * y, 5. + 0.5 * x + 0.25 * y + z];
        }
    }
    let p = prepare_plane_limit(&a, Boundary::UMin, 100000000)
        .unwrap()
        .certificate
        .unwrap();
    let q = prepare_plane_limit(&b, Boundary::UMin, 100000000)
        .unwrap()
        .certificate
        .unwrap();
    assert_eq!(
        compare_plane_limits(&p, &q, 1000000).unwrap().same_plane,
        Some(true)
    );
    let mut wrong = b.clone();
    for point in wrong.control_points.iter_mut().flatten() {
        point[2] += point[0] - 3.;
    }
    let wrong = prepare_plane_limit(&wrong, Boundary::UMin, 100000000)
        .unwrap()
        .certificate
        .unwrap();
    assert_eq!(
        compare_plane_limits(&p, &wrong, 1000000)
            .unwrap()
            .same_plane,
        Some(false)
    );
    let short = compare_plane_limits(&p, &q, 1).unwrap();
    assert!(short.same_plane.is_none());
    assert!(short.work <= 1);
}
#[test]
fn collapse_nonzero_normal_and_one_short_work_are_mandatory() {
    let s = source(1, true);
    let full = prepare_plane_limit(&s, Boundary::UMin, 100000000).unwrap();
    assert!(full.certificate.is_some());
    assert!(full.work > 1);
    let short = prepare_plane_limit(&s, Boundary::UMin, full.work - 1).unwrap();
    assert!(short.certificate.is_none());
    assert!(short.work < full.work);
    assert!(prepare_plane_limit(&s, Boundary::UMin, 0).is_err());
    let mut two_dimensional = s.clone();
    for p in two_dimensional.control_points.iter_mut().flatten() {
        p.truncate(2);
    }
    assert!(prepare_plane_limit(&two_dimensional, Boundary::UMin, 100000000).is_err());
    let mut not_collapsed = s.clone();
    not_collapsed.control_points[0][1][0] = 1e-12;
    assert!(
        prepare_plane_limit(&not_collapsed, Boundary::UMin, 100000000)
            .unwrap()
            .certificate
            .is_none()
    );
    let mut degenerate = s;
    for point in degenerate.control_points.iter_mut().flatten() {
        *point = vec![0.; 3];
    }
    assert!(
        prepare_plane_limit(&degenerate, Boundary::UMin, 100000000)
            .unwrap()
            .certificate
            .is_none()
    );
}
