use super::*;
fn source() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..3)
            .map(|u| {
                (0..2)
                    .map(|v| vec![u as f64 / 2., if u == 2 { v as f64 } else { 0. }, 0.])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1., 2.], vec![1., 3.], vec![1., 2.]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn rational_tangent_term_is_retained_and_original_frame_is_owned() {
    for end in [false, true] {
        let mut s = source();
        if end {
            s.control_points.reverse();
            s.weights.reverse();
        }
        let r = prepare(&s, end, 10_000_000).unwrap();
        let c = r.certificate.unwrap();
        assert!(std::ptr::eq(c.surface(), &s));
        assert_eq!(c.anchor(), 0);
        let d = [if end { [0.999, 1.] } else { [0., 0.001] }, [0., 0.1]];
        let j = c.bounds(d).unwrap().unwrap();
        assert!(j[0][0].contains(1.));
        assert!(j[1][1].lo > 0.);
        assert!(j[2][0].contains(0.) && j[2][1].contains(0.));
        let d = [d[0], [0., 0.0001]];
        let admitted =
            crate::surface_self_chord::qualify_rational_polar_rectangle(&s, d, &c, 1e-4, 5)
                .unwrap();
        assert!(admitted.certificate.is_some());
        assert_eq!(admitted.spans, 5);
        let limited =
            crate::surface_self_chord::qualify_rational_polar_rectangle(&s, d, &c, 1e-4, 4)
                .unwrap();
        assert!(limited.certificate.is_none() && !limited.complete);
        assert_eq!(limited.spans, 0);
        let foreign = s.clone();
        assert!(crate::surface_self_chord::qualify_rational_polar_rectangle(
            &foreign, d, &c, 1e-4, 5,
        )
        .is_err());
        let normal_report =
            crate::factored_surface_normals::prepare(&s, end, 10_000_000).unwrap();
        let normals = normal_report.certificate.unwrap();
        let near = if end { [1. - 1e-12, 1.] } else { [0., 1e-12] };
        let pair = [[near, [0., 1e-12]], [near, [0.245, 0.245 + 1e-12]]];
        let excluded = crate::surface_self_chord::rational_pole_pair_exclusion(
            &s, pair, &c, &normals, 1e-4, 7,
        )
        .unwrap();
        // Correlated numerator/denominator coefficients now keep the
        // wide angular pair nonsingular, unlike the old independent hulls.
        assert!(excluded.0 && excluded.1 <= 7);
        let classified = crate::surface_self_chord::rational_pole_pair_report(
            &s, pair, &c, &normals, 1e-4, 7,
        )
        .unwrap();
        assert!(
            classified.excluded
                && classified.coordinate_path_excluded
                && classified.coordinate_path_attempted
                && classified.coordinate_path_available
        );

        let narrow = [[near, [0., 1e-12]], [near, [0.1, 0.1 + 1e-12]]];
        let local = crate::surface_self_chord::rational_pole_pair_exclusion(
            &s, narrow, &c, &normals, 1e-4, 7,
        )
        .unwrap();
        assert!(local.0 && local.1 <= 7);
        assert_eq!(
            crate::surface_self_chord::rational_pole_pair_exclusion(
                &s, pair, &c, &normals, 1e-4, 5
            )
            .unwrap(),
            (false, 5)
        );
        assert!(crate::surface_self_chord::rational_pole_pair_exclusion(
            &foreign, pair, &c, &normals, 1e-4, 7
        )
        .is_err());

        let limited = crate::surface_self_chord_coverage::qualify_with_rational_pole(
            &s,
            0.1,
            1e-4,
            crate::surface_self_chord_coverage::Limits { cells: 1, spans: 1 },
            &normals,
            &c,
        )
        .unwrap();
        assert!(limited.certificate.is_none() && limited.pending > 0 && limited.spans <= 1);
        assert_eq!(limited.coordinate_path_attempts, 0);
        assert_eq!(limited.coordinate_path_exclusions, 0);
        assert!(
            crate::surface_self_chord_coverage::qualify_with_rational_pole(
                &foreign,
                0.1,
                1e-4,
                crate::surface_self_chord_coverage::Limits { cells: 1, spans: 1 },
                &normals,
                &c
            )
            .is_err()
        );
        assert!(c.bounds([[-0.1, 1.], [0., 1.]]).is_err());
    }
}
#[test]
fn independent_rational_endpoint_frames_survive_scaled_domains() {
    // Direct quotient formulas, independent of coefficient preparation.
    for end in [false, true] {
        let mut s = source();
        if end {
            s.control_points.reverse();
            s.weights.reverse();
        }
        s.knots_u = vec![2., 2., 2., 7., 7., 7.];
        s.knots_v = vec![-3., -3., 9., 9.];
        let before = s.clone();
        let r = prepare(&s, end, 10_000_000).unwrap();
        let c = r.certificate.unwrap();
        let d = [if end { [6.995, 7.] } else { [2., 2.005] }, [-3., -1.8]];
        let j = c.bounds(d).unwrap().unwrap();
        let path = c.coordinate_path_bounds(d).unwrap().unwrap();
        let local = c.local_coordinate_path_bounds(d).unwrap().unwrap();
        for u in [0.0001, 0.0005, 0.001] {
            for v in [0., 0.05, 0.1] {
                let w = 1. + v + 2. * u * (1. - u) * v;
                let wu = 2. * (1. - 2. * u) * v;
                let wv = 1. + 2. * u * (1. - u);
                let x = u * (1. + 2. * v) - u * u * v;
                let y = 2. * u * u * v;
                let xu = ((1. + 2. * v - 2. * u * v) * w - x * wu) / (w * w);
                let xv = ((2. * u - u * u) * w - x * wv) / (w * w);
                let yu = (4. * u * v * w - y * wu) / (w * w);
                let yv = (2. * u * u * w - y * wv) / (w * w);
                assert!(j[1][0].contains(yu / xu));
                assert!(j[1][1].contains((yv - yu * xv / xu) / (u * u)));
                assert!(path[1][0].contains(yu / xu));
                assert!(path[1][1].contains((yv - yu * xv / xu) / (u * u)));
                assert!(local[1][0].contains(yu / xu));
                assert!(local[1][1].contains((yv - yu * xv / xu) / (u * u)));
                if v == 0.05 {
                    assert!(xv.abs() > 0.);
                }
            }
        }
        assert_eq!(s.control_points, before.control_points);
        assert_eq!(s.weights, before.weights);
        assert_eq!(s.knots_u, before.knots_u);
        assert_eq!(s.knots_v, before.knots_v);
    }
}
#[test]
fn analytic_cubic_chords_and_endpoint_frames_are_enclosed() {
    // Independent polynomial oracle S=(3u,3u²v,u³).
    let s = Surface {
        degree_u: 3,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0usize..4)
            .map(|i| {
                (0..2)
                    .map(|v| {
                        vec![
                            i as f64,
                            (i * i.saturating_sub(1)) as f64 / 2. * v as f64,
                            if i == 3 { 1. } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    let r = prepare(&s, false, 10_000_000).unwrap();
    let c = r.certificate.unwrap();
    let j = c.bounds([[0., 0.25], [0., 0.5]]).unwrap().unwrap();
    let path = c
        .coordinate_path_bounds([[0., 0.25], [0., 0.5]])
        .unwrap()
        .unwrap();
    for (u0, v0, u1, v1) in [
        (0., 0., 0.25, 0.5),
        (0.25, 0.5, 0.01, 0.1),
        (0.1, 0.5, 0.1, 0.),
        (0., 0.1, 0., 0.4),
    ] {
        let (du, dv) = (u1 - u0, v1 - v0);
        let uv = u0 * v0 + (u0 * dv + v0 * du) / 2. + du * dv / 3.;
        let uu = u0 * u0 + u0 * du + du * du / 3.;
        let first = [1., 2. * uv, uu];
        let second = [0., 3., 0.];
        for k in 0..3 {
            assert!(j[k][0].contains(first[k]));
            assert!(j[k][1].contains(second[k]));
        }
        let path_first = [1., (u0 + u1) * v0, (u0 * u0 + u0 * u1 + u1 * u1) / 3.];
        for k in 0..3 {
            assert!(path[k][0].contains(path_first[k]));
            assert!(path[k][1].contains(second[k]));
        }
        let p = |u: f64, v: f64| [3. * u, 3. * u * u * v, u * u * u];
        let (p0, p1) = (p(u0, v0), p(u1, v1));
        for k in 0..3 {
            let chord = first[k] * 3. * du + second[k] * dv * uu;
            assert!((chord - (p1[k] - p0[k])).abs() < 1e-14);
            let path_chord = path_first[k] * 3. * du + second[k] * dv * u1 * u1;
            assert!((path_chord - (p1[k] - p0[k])).abs() < 1e-14);
        }
        for (u, v) in [(u0, v0), (u1, v1)] {
            for (k, x) in [1., 2. * u * v, u * u].into_iter().enumerate() {
                assert!(j[k][0].contains(x));
            }
        }
    }
}
#[test]
fn local_inverse_chart_encloses_independent_frames_without_global_monotonicity() {
    let mut s = source();
    for v in 0..2 {
        s.control_points[1][v][0] = -0.5;
    }
    let c = prepare(&s, false, 10_000_000).unwrap().certificate.unwrap();
    let d = [[0.8, 0.85], [0., 0.1]];
    let local = [[0.45, 0.55], [0., 0.01]];
    assert!(c.local_coordinate_path_bounds(local).unwrap().is_none());
    let refused = c.local_coordinate_path_report(local, 3.).unwrap();
    assert!(refused.0.is_none());
    assert_eq!(refused.1, 1);
    let prefix_refused = c.coordinate_path_report(local).unwrap();
    assert!(prefix_refused.0.is_none());
    assert_eq!(prefix_refused.1, 3);
    let uv_refused = c.bounds_report([[0., 0.8], [0., 0.1]]).unwrap();
    assert!(uv_refused.0.is_none());
    assert_eq!(uv_refused.1, 2);

    let accepted = c.local_coordinate_path_report(local, 0.25).unwrap();
    assert!(accepted.0.is_some());
    assert_eq!(accepted.1, 6);

    let narrow = c.local_coordinate_path_bounds_with_margin(local, 0.25)
        .unwrap().unwrap();
    for u in [0.45, 0.5, 0.55] {
        for v in [0., 0.005, 0.01] {
            let w = 1. + v + 2. * u * (1. - u) * v;
            let wu = 2. * (1. - 2. * u) * v;
            let wv = 1. + 2. * u * (1. - u);
            let x = -u * (1. + 2. * v) + u * u * (2. + 3. * v);
            let y = 2. * u * u * v;
            let xu = ((-1.-2.*v+2.*u*(2.+3.*v))*w-x*wu)/(w*w);
            let xv = ((-2.*u+3.*u*u)*w-x*wv)/(w*w);
            let yu = (4.*u*v*w-y*wu)/(w*w);
            let yv = (2.*u*u*w-y*wv)/(w*w);
            assert!(narrow[1][0].contains(yu/xu));
            assert!(narrow[1][1].contains((yv-yu*xv/xu)/(u*u)));
        }
    }
    assert!(c.local_coordinate_path_bounds_with_margin(local, 0.).is_err());

    assert!(c.coordinate_path_bounds(d).unwrap().is_none());
    let j = c.local_coordinate_path_bounds(d).unwrap().unwrap();
    let prepared = crate::factored_surface_normals::prepare(&s, false, 10_000_000).unwrap();
    let normals = prepared.certificate.unwrap();
    let pair = [[[0.8, 0.805], [0., 0.005]], [[0.845, 0.85], [0.095, 0.1]]];
    let report =
        crate::surface_self_chord::rational_pole_pair_report(&s, pair, &c, &normals, 1e-4, 10)
            .unwrap();
    assert!(
        report.excluded
            && report.local_path_excluded
            && report.local_path_available
            && report.local_path_attempted
            && !report.coordinate_path_available
    );
    assert_eq!(report.spans, 10);
    assert_eq!(report.local_path_margin, Some(3.));
    let alternatives = crate::surface_self_chord::rational_pole_pair_report(
        &s, pair, &c, &normals, 1e-4, 32).unwrap();
    assert!(alternatives.excluded && alternatives.local_path_excluded);
    assert!(alternatives.spans <= 32);

    let cached = normals.pair_bounds(pair).unwrap(); // Two paid restrictions.
    let reused = crate::surface_self_chord::rational_pole_pair_report_cached(
        &s, pair, &c, &normals, 1e-4, 9, Some(&cached)).unwrap();
    assert!(reused.excluded && reused.local_path_excluded);
    assert_eq!(reused.spans, 9);
    let no_matrix_budget = crate::surface_self_chord::rational_pole_pair_report_cached(
        &s, pair, &c, &normals, 1e-4, 8, Some(&cached)).unwrap();
    assert!(!no_matrix_budget.excluded && no_matrix_budget.spans <= 8);
    assert!(crate::surface_self_chord::rational_pole_pair_report_cached(
        &s, [pair[0]; 2], &c, &normals, 1e-4, 0, Some(&cached)).is_err());

    let limited =
        crate::surface_self_chord::rational_pole_pair_report(&s, pair, &c, &normals, 1e-4, 9)
            .unwrap();
    assert!(!limited.excluded && limited.spans == 9 && limited.local_path_available);

    let negative = c
        .local_coordinate_path_bounds([[0.01, 0.02], [0., 0.1]])
        .unwrap()
        .unwrap();
    for (j, parameters) in [(&j, [0.8, 0.825, 0.85]), (&negative, [0.01, 0.015, 0.02])] {
        for u in parameters {
            for v in [0., 0.05, 0.1] {
                let w = 1. + v + 2. * u * (1. - u) * v;
                let wu = 2. * (1. - 2. * u) * v;
                let wv = 1. + 2. * u * (1. - u);
                let x = -u * (1. + 2. * v) + u * u * (2. + 3. * v);
                let y = 2. * u * u * v;
                let xu = ((-1. - 2. * v + 2. * u * (2. + 3. * v)) * w - x * wu) / (w * w);
                let xv = ((-2. * u + 3. * u * u) * w - x * wv) / (w * w);
                let yu = (4. * u * v * w - y * wu) / (w * w);
                let yv = (2. * u * u * w - y * wv) / (w * w);
                assert_eq!(xu > 0., u > 0.25);
                assert!(j[1][0].contains(yu / xu));
                assert!(j[1][1].contains((yv - yu * xv / xu) / (u * u)));
            }
        }
    }
}
#[test]
fn coordinate_path_bounds_entire_prefix_before_endpoint() {
    // At u=.99 both V sides have derivative ~.2, but their longitudinal
    // positions differ by ~3. One cannot carry the high X across V=1,
    // where the original chart ends at X=.2. Local endpoint derivative
    // bounds alone would incorrectly admit that inverse-coordinate path.
    let s = Surface {
        degree_u: 3,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0usize..4)
            .map(|i| {
                (0..2)
                    .map(|v| {
                        let x = if v == 0 {
                            [0., 3., 3.1, 3.2][i]
                        } else {
                            i as f64 * 0.2 / 3.
                        };
                        vec![x, (i * i.saturating_sub(1)) as f64 / 2. * v as f64, 0.]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    let c = prepare(&s, false, 10_000_000).unwrap().certificate.unwrap();
    assert!(c
        .coordinate_path_bounds([[0.99, 0.99], [0., 1.]])
        .unwrap()
        .is_none());
    assert!(c
        .local_coordinate_path_bounds([[0.99, 0.99], [0., 1.]])
        .unwrap()
        .is_none());
}
#[test]
fn coordinate_path_refuses_unconfined_or_nonmonotone_chart() {
    let s = source();
    let c = prepare(&s, false, 10_000_000).unwrap().certificate.unwrap();
    assert!(c
        .coordinate_path_bounds([[0., 1.], [0., 1.]])
        .unwrap()
        .is_none());
    let mut s = source();
    for v in 0..2 {
        s.control_points[1][v][0] = -0.5;
    }
    let c = prepare(&s, false, 10_000_000).unwrap().certificate.unwrap();
    assert!(c
        .coordinate_path_bounds([[0., 1.], [0., 1.]])
        .unwrap()
        .is_none());
}
#[test]
fn partial_work_and_nonzero_transverse_coefficient_refuse() {
    let s = source();
    let r = prepare(&s, false, 1).unwrap();
    assert!(r.certificate.is_none() && r.work <= 1);
    let mut s = source();
    s.control_points[1][1][1] = 1e-12;
    assert!(prepare(&s, false, 10_000_000)
        .unwrap()
        .certificate
        .is_none());
}
