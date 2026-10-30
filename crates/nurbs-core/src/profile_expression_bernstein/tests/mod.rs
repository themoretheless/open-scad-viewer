use super::*;
#[test]
fn unsupported_line_degree_does_not_hide_invalid_queries() {
    let s = crate::surface::Surface {
        degree_u:17,degree_v:1,
        knots_u:[vec![0.;18],vec![1.;18]].concat(),knots_v:vec![0.,0.,1.,1.],
        control_points:(0..18).map(|i|vec![vec![i as f64,0.,0.],vec![i as f64,1.,0.]]).collect(),
        weights:vec![vec![1.;2];18],periodic_u:false,periodic_v:false,
    };
    let p = crate::normal_alignment::qualify_weight_profile(&s,[0,0],100).unwrap().0.unwrap();
    let query = |domain,direction| p.inspect_line_intersections(domain,[0.;3],direction,1e-5,100,100,10000);
    assert!(query([[0.,1.];2],[0.;3]).is_err());
    assert!(query([[-1.,1.],[0.,1.]],[0.,0.,1.]).is_err());
    let valid = query([[0.,1.];2],[0.,0.,1.]).unwrap();
    assert!(valid.intersections.is_none());
    assert_eq!(valid.reason,"profile-line-degree-limit");
}
fn evaluate(mut values: Vec<[I; 4]>, t: f64) -> [I; 4] {
    let q = I::point(1.).sub(I::point(t)).unwrap();
    while values.len() > 1 {
        values = values
            .windows(2)
            .map(|p| {
                std::array::from_fn(|k| {
                    p[0][k]
                        .mul(q)
                        .unwrap()
                        .add(p[1][k].mul(I::point(t)).unwrap())
                        .unwrap()
                })
            })
            .collect();
    }
    values[0]
}
#[test]
fn radial_lower_bounds_enclose_exact_cylinder_error_at_multiple_scales() {
    for scale in [1e-50,1.,1e8] {
        let w=std::f64::consts::FRAC_1_SQRT_2;
        let surface=crate::surface::Surface {
            degree_u:1,degree_v:2,knots_u:vec![0.,0.,1.,1.],knots_v:vec![0.,0.,0.,1.,1.,1.],
            control_points:[0.,scale].into_iter().map(|x|vec![vec![x,scale,0.],vec![x,scale,scale],vec![x,0.,scale]]).collect(),
            weights:vec![vec![1.,w,1.];2],periodic_u:false,periodic_v:false,
        };
        let profile=crate::normal_alignment::qualify_weight_profile(&surface,[0,0],6).unwrap().0.unwrap();
        let bridge=crate::normal_alignment::ProfileSectionBridge::new(
            profile.section(0,0.).unwrap(),profile.section(0,1.).unwrap(),[1.;2]).unwrap();
        let bridge=bridge.constrain_original_rails(100).unwrap().0.unwrap();
        let centers=crate::curve::Curve {degree:1,knots:vec![0.,0.,1.,1.],
            control_points:vec![vec![0.,0.,0.],vec![scale,0.,0.]],weights:vec![1.;2],periodic:false};
        for factor in [0.,1.,1.25] {
            let radius=crate::curve::Curve {degree:1,knots:vec![0.,0.,1.,1.],
                control_points:vec![vec![scale*factor,0.];2],weights:vec![1.;2],periodic:false};
            let report=bridge.inspect_radius_coefficients(&centers,&radius,[[0.,1.];2],[[0.,1.];2],100000,100000000).unwrap();
            let exact=scale*(1.-factor).abs();
            let lower=report.error_lower.unwrap();let upper=report.error_upper.unwrap();
            assert!(lower<=exact && exact<=upper,"scale={scale} factor={factor}: {lower} <= {exact} <= {upper}");
            if factor!=1. {assert!(lower>0.);}
        }
    }
}
#[test]
fn corrected_cylinder_coefficients_cover_independent_rational_positions() {
    let w = std::f64::consts::FRAC_1_SQRT_2;
    let surface = crate::surface::Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: [0., 1.]
            .into_iter()
            .map(|x| vec![vec![x, 1., 0.], vec![x, 1., 1.], vec![x, 0., 1.]])
            .collect(),
        weights: vec![vec![1., w, 1.]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let profile = crate::normal_alignment::qualify_weight_profile(&surface, [0, 0], 6)
        .unwrap()
        .0
        .unwrap();
    let mut multiple = surface.clone();
    multiple.knots_u = vec![0., 0., 0.5, 1., 1.];
    multiple.control_points = [0., 0.5, 1.]
        .into_iter()
        .map(|x| vec![vec![x, 1., 0.], vec![x, 1., 1.], vec![x, 0., 1.]])
        .collect();
    multiple.weights = vec![vec![1., w, 1.]; 3];
    let multi = crate::normal_alignment::qualify_weight_profile(&multiple, [0, 0], 9)
        .unwrap()
        .0
        .unwrap();
    let at_knot = multi
        .inspect_line_intersections(
            [[0., 1.]; 2],
            [0.5, 0., 0.],
            [0., 1., 1.],
            1e-5,
            1000,
            10,
            100000,
        )
        .unwrap()
        .intersections
        .unwrap();
    assert!(!at_knot.complete && !at_knot.unresolved.is_empty());
    assert!(at_knot.roots.is_empty());
    let limited = multi
        .inspect_line_intersections(
            [[0., 1.]; 2],
            [0.75, 0., 0.],
            [0., 1., 1.],
            1e-5,
            1000,
            1,
            100000,
        )
        .unwrap();
    assert_eq!(limited.spans, 1);
    let limited = limited.intersections.unwrap();
    assert!(!limited.complete && limited.unresolved.iter().any(|uv| uv[0] == [0.5, 1.]));
    let multi = multi
        .inspect_line_intersections(
            [[0., 1.]; 2],
            [0.25, 0., 0.],
            [0., 1., 1.],
            1e-5,
            1000,
            10,
            100000,
        )
        .unwrap();
    assert_eq!(multi.spans, 2);
    let multi = multi.intersections.unwrap();
    assert!(multi.complete && multi.roots.len() == 1);
    assert!(multi.roots[0].uv[0][0] <= 0.25 && multi.roots[0].uv[0][1] >= 0.25);
    let bridge = crate::normal_alignment::ProfileSectionBridge::new(
        profile.section(0, 0.).unwrap(),
        profile.section(0, 1.).unwrap(),
        [1.; 2],
    )
    .unwrap();
    let corrected = bridge.constrain_original_rails(100).unwrap().0.unwrap();
    let endpoints=[[[0.,1.],[0.1,0.2]],[[0.,1.],[0.8,0.9]]];
    assert_eq!(corrected.inspect_self_chord_pair(endpoints,1e-4,0).unwrap(),(None,0));
    assert!(corrected.inspect_self_chord_pair(endpoints,1e-4,10000).unwrap().0.is_some());
    assert_eq!(corrected.inspect_self_chords([[0.,1.];2],1e-4,0).unwrap(),(None,0));
    assert!(corrected.inspect_self_chords([[0.,1.],[0.2,0.3]],1e-4,10000).unwrap().0.is_some());
    assert!(corrected.inspect_self_chords([[0.,1.],[-0.1,0.3]],1e-4,0).is_err());
    let retained = profile
        .inspect_line_intersections(
            [[0.25, 0.75], [0., 1.]],
            [0.5, 0., 0.],
            [0., 1., 1.],
            1e-5,
            1000,
            100,
            100000,
        )
        .unwrap();
    let retained = retained.intersections.unwrap();
    assert!(retained.complete && retained.roots.len() == 1);
    assert!(retained.roots[0].uv[0][0] <= 0.5 && retained.roots[0].uv[0][1] >= 0.5);
    assert!(
        profile
            .inspect_line_intersections(
                [[0.25, 0.75], [0., 1.]],
                [0.5, 0., 0.],
                [0., 1., 1.],
                1e-5,
                1000,
                0,
                100000
            )
            .unwrap()
            .intersections
            .unwrap()
            .unresolved
            .len()
            == 1
    );

    assert!(
        corrected
            .homogeneous_coefficients([[0., 1.]; 2], 0, 1000000)
            .unwrap()
            .0
            .is_none()
    );
    assert!(
        corrected
            .homogeneous_coefficients([[0., 1.]; 2], 100000, 0)
            .unwrap()
            .0
            .is_none()
    );
    let (net, spans, work) = corrected
        .homogeneous_coefficients([[0., 1.]; 2], 100000, 1000000)
        .unwrap();
    let net = net.unwrap();
    assert!(spans <= 100000 && work <= 1000000);
    let line = corrected
        .inspect_line_intersections(
            [[0., 1.]; 2],
            [0.5, 0., 0.],
            [0., 1., 1.],
            1e-5,
            1000,
            100000,
            100000000,
        )
        .unwrap();
    let roots = line.intersections.unwrap();
    assert!(roots.complete, "{}", line.reason);
    assert_eq!(roots.roots.len(), 1);
    let expected = (1. + 2. * w) / (2. + 2. * w);
    assert!(roots.roots[0].parameter[0] <= expected && roots.roots[0].parameter[1] >= expected);
    let center_law = crate::curve::Curve {
        degree: 1,
        knots: vec![2., 2., 4., 4.],
        control_points: vec![vec![-0.5, 0., 0.], vec![1.5, 0., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let radius_law = crate::curve::Curve {
        degree: 1,
        knots: vec![3., 3., 7., 7.],
        control_points: vec![vec![1., 0.], vec![1., 0.]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let whole = corrected
        .inspect_radius_coefficients(
            &center_law,
            &radius_law,
            [[2.5, 3.5], [3.25, 6.75]],
            [[0., 1.]; 2],
            100000,
            100000000,
        )
        .unwrap();
    assert!(whole.error_upper.unwrap() < 1e-6);
    assert!(whole.spans <= 100000 && whole.coefficient_work <= 100000000);
    assert!(
        corrected
            .inspect_radius_coefficients(
                &center_law,
                &radius_law,
                [[2.5, 3.5], [3.25, 6.75]],
                [[0., 1.]; 2],
                0,
                100000000
            )
            .unwrap()
            .error_upper
            .is_none()
    );
    let centers = vec![
        vec![I::point(0.), I::point(0.), I::point(0.), I::point(1.)],
        vec![I::point(1.), I::point(0.), I::point(0.), I::point(1.)],
    ];
    let radius = vec![vec![I::point(1.), I::point(0.), I::point(1.)]];
    let mut radius_work = 0;
    let mut reason = "unverified";
    let residual = crate::moving_radius::bound_coefficient_nets(
        &net,
        &centers,
        &radius,
        1,
        1.,
        &mut radius_work,
        100000000,
        &mut reason,
    )
    .unwrap()
    .unwrap();
    assert!(residual.0[0] <= 0. && residual.0[1] >= 0.);
    assert!(residual.1 < 1e-6, "coefficient radius error {}", residual.1);
    let wrong = vec![vec![I::point(2.), I::point(0.), I::point(1.)]];
    let mismatch = crate::moving_radius::bound_coefficient_nets(
        &net,
        &centers,
        &wrong,
        1,
        2.,
        &mut radius_work,
        100000000,
        &mut reason,
    )
    .unwrap()
    .unwrap();
    assert!(mismatch.0[0] <= -3. && mismatch.0[1] >= -3. && mismatch.0[1] < 0.);
    assert!(
        crate::moving_radius::bound_coefficient_nets(
            &net,
            &centers,
            &radius,
            1,
            1.,
            &mut 0,
            0,
            &mut reason
        )
        .unwrap()
        .is_none()
    );
    assert_eq!([net.len(), net[0].len()], [6, 5]);
    for free in [0.1, 0.3, 0.7, 0.9] {
        for across in [0.2, 0.5, 0.8] {
            let rows: Vec<_> = net
                .iter()
                .map(|row| evaluate(row.clone(), across))
                .collect();
            let h = evaluate(rows, free);
            let q = 1. - free;
            let denominator = q * q + 2. * w * q * free + free * free;
            let expected = [
                across,
                (q * q + 2. * w * q * free) / denominator,
                (2. * w * q * free + free * free) / denominator,
            ];
            for k in 0..3 {
                let position = h[k].div(h[3]).unwrap();
                assert!(
                    position.contains(expected[k]),
                    "{free} {across} axis{k} {position:?}"
                );
                assert!(position.width() < 1e-7);
            }
        }
    }
}
