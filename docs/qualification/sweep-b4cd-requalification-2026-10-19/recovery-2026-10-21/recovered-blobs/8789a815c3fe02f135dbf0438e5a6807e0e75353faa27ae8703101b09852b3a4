use nurbs_core::{
    curve::Curve,
    intersection::{ContactClass, intersect_curve_plane_certified_report as intersect},
    surface::Surface,
};
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 5., 5.],
        knots_v: vec![-3., -3., 7., 7.],
        control_points: vec![
            vec![vec![-2., -2., 0.], vec![-2., 2., 0.]],
            vec![vec![2., -2., 0.], vec![2., 2., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn quadratic() -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0., 0., 0.16], vec![0.5, 0., -0.34], vec![1., 1., 0.16]],
        weights: vec![1.; 3],
        periodic: false,
    }
}
#[test]
fn two_original_spatial_curve_roots_have_certified_plane_parameters() {
    let c = quadratic();
    let s = plane();
    let r = intersect(&c, &s, 1e-6, 10000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.crossings.len(), 2);
    let mut roots: Vec<_> = r
        .crossings
        .iter()
        .map(|p| (p.curve_parameter[0] + p.curve_parameter[1]) * 0.5)
        .collect();
    roots.sort_by(f64::total_cmp);
    assert!((roots[0] - 0.2).abs() < 1e-6 && (roots[1] - 0.8).abs() < 1e-6);
    for p in &r.crossings {
        assert_eq!(p.contact, ContactClass::Transverse);
        assert_eq!(p.position_bounds[2], [0., 0.]);
        let t = (p.curve_parameter[0] + p.curve_parameter[1]) * 0.5;
        let point = c.evaluate(t).unwrap().point;
        for (k, v) in [
            2. + 3. * (point[0] + 2.) / 4.,
            -3. + 10. * (point[1] + 2.) / 4.,
        ]
        .into_iter()
        .enumerate()
        {
            assert!(p.uv_bounds[k][0] - 1e-12 <= v && v <= p.uv_bounds[k][1] + 1e-12);
        }
    }
}
#[test]
fn rational_root_and_exact_clamped_endpoint_are_distinct_contacts() {
    let s = plane();
    let mut c = Curve::from_polyline(vec![vec![0., 0., -1.], vec![1., 1., 1.]]).unwrap();
    c.weights = vec![1., 2.];
    let r = intersect(&c, &s, 1e-6, 10000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.crossings.len(), 1);
    assert!(
        r.crossings[0].curve_parameter[0] <= 1. / 3.
            && r.crossings[0].curve_parameter[1] >= 1. / 3.
    );
    c.control_points[0][2] = 0.;
    let r = intersect(&c, &s, 1e-6, 10000).unwrap();
    assert!(r.complete);
    assert_eq!(r.crossings[0].contact, ContactClass::Boundary);
    assert_eq!(r.crossings[0].curve_parameter, [0., 0.]);
}
#[test]
fn full_rational_overlap_retains_original_curve_and_checked_uv_trace() {
    let s = plane();
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0., 0.], vec![1., 1., 0.], vec![0., 1., 0.]],
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        periodic: false,
    };
    let r = intersect(&c, &s, 1e-10, 1000).unwrap();
    assert!(r.complete && r.crossings.is_empty());
    let (stored, pcurve) = r.overlap.unwrap();
    assert_eq!(stored, c);
    for t in [0., 0.17, 0.5, 0.9, 1.] {
        let p = c.evaluate(t).unwrap().point;
        let uv = pcurve.evaluate(t).unwrap().point;
        let q = s.evaluate(uv[0], uv[1]).unwrap().point;
        for k in 0..3 {
            assert!((p[k] - q[k]).abs() < 1e-14);
        }
    }
}
#[test]
fn clipping_tangency_nonaffine_charts_and_exhaustion_do_not_gain_authority() {
    let c = quadratic();
    let mut clipped = plane();
    for row in &mut clipped.control_points {
        for p in row {
            p[0] = 0.3 + 0.1 * (p[0] + 2.);
        }
    }
    assert!(!intersect(&c, &clipped, 1e-6, 1000).unwrap().complete);
    let mut tangent = c.clone();
    tangent.control_points[0][2] = 0.25;
    tangent.control_points[1][2] = -0.25;
    tangent.control_points[2][2] = 0.25;
    let r = intersect(&tangent, &plane(), 1e-6, 100).unwrap();
    assert!(!r.complete && r.crossings.is_empty());
    let r = intersect(&c, &plane(), 1e-6, 1).unwrap();
    assert!(!r.complete && !r.unresolved_curve_domains.is_empty());
    assert!(r.cells <= 1);
    let mut nonaffine = plane();
    nonaffine.degree_u = 2;
    nonaffine.knots_u = vec![2., 2., 2., 5., 5., 5.];
    nonaffine
        .control_points
        .insert(1, nonaffine.control_points[0].clone());
    nonaffine.weights.insert(1, vec![1.; 2]);
    assert!(!intersect(&c, &nonaffine, 1e-6, 1000).unwrap().complete);
    assert!(intersect(&c, &plane(), 0., 10).is_err());
    assert!(intersect(&c, &plane(), 1e-6, 0).is_err());
}
#[test]
fn a_subnormal_plane_gap_is_excluded_by_original_positive_hull() {
    let gap = f64::from_bits(1);
    let c = Curve::from_polyline(vec![vec![0., 0., -gap], vec![1., 1., -gap]]).unwrap();
    let r = intersect(&c, &plane(), 1e-6, 10).unwrap();
    assert!(r.complete && r.crossings.is_empty() && r.overlap.is_none());
}

#[test]
fn reversed_plane_axes_keep_original_nonunit_uv_domains() {
    let mut s = plane();
    s.control_points.reverse();
    s.weights.reverse();
    for row in &mut s.control_points {
        row.reverse();
    }
    let r = intersect(&quadratic(), &s, 1e-6, 10000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.crossings.len(), 2);
    for p in &r.crossings {
        for (k, domain) in [[2., 5.], [-3., 7.]].into_iter().enumerate() {
            assert!(p.uv_bounds[k][0] >= domain[0] && p.uv_bounds[k][1] <= domain[1]);
        }
    }
}
