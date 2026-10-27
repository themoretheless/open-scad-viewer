use nurbs_core::{
    ss_intersection::{self as ss, QuadraticGraphSSKind as K},
    surface::Surface,
};
fn surface(form: [f64; 4], domain: [[f64; 2]; 2], quartic: bool) -> Surface {
    let q = [0.25, -0.25, 0.25];
    let [a, b, c, k] = form;
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![domain[0][0]; 3]
            .into_iter()
            .chain(vec![domain[0][1]; 3])
            .collect(),
        knots_v: vec![domain[1][0]; 3]
            .into_iter()
            .chain(vec![domain[1][1]; 3])
            .collect(),
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let u = i as f64 / 2.;
                        let v = j as f64 / 2.;
                        vec![
                            u,
                            v,
                            2. + 3. * u - v
                                + u * v
                                + a * q[i]
                                + b * q[j]
                                + c * (u - 0.5) * (v - 0.5)
                                + k
                                + if quartic && i == 2 && j == 2 { 1. } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
fn source(form: [f64; 4], quartic: bool) -> ss::FiberSources {
    ss::FiberSources::new(
        &surface(form, [[2., 6.], [-4., 4.]], quartic),
        &surface([0.; 4], [[-3., 5.], [7., 11.]], quartic),
    )
    .unwrap()
}
fn owner(form: [f64; 4], quartic: bool) -> ss::QualifiedQuadraticGraphSS {
    ss::qualify_quadratic_graph_ss(&source(form, quartic), Default::default()).unwrap()
}
fn tolerances() -> ss::GraphCurveTolerances {
    ss::GraphCurveTolerances {
        world: 1e-7,
        uv: 1e-9,
    }
}
fn build(o: &ss::QualifiedQuadraticGraphSS) -> ss::QualifiedGraphCurves {
    ss::construct_graph_curves(o, tolerances(), Default::default()).unwrap()
}
fn distance(a: &[f64], b: &[f64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f64>()
        .sqrt()
}
#[test]
fn closed_algebraic_branch_has_four_compact_world_curves_and_shared_native_vertices() {
    let o = owner([1., 1., -1., -0.015625], false);
    let result = build(&o);
    assert_eq!(o.kind(), K::ClosedEllipse);
    assert_eq!(result.edges().len(), 4);
    assert_eq!(result.vertices().len(), 4);
    let radii = [0.125, (1f64 / 48.).sqrt()];
    for (i, e) in result.edges().iter().enumerate() {
        assert_eq!(e.original_edge, i);
        assert_eq!(e.world.degree, 4);
        assert_eq!(e.world.control_points.len(), 5);
        assert!(
            e.pcurves
                .iter()
                .all(|c| c.degree == 2 && c.control_points.len() == 3)
        );
        assert!(e.world_error_upper <= tolerances().world);
        for (end, pole) in [0, e.world.control_points.len() - 1]
            .into_iter()
            .enumerate()
        {
            let vertex = &result.vertices()[o.edges()[i].vertices[end]];
            assert_eq!(e.world.control_points[pole], vertex.world);
            for side in 0..2 {
                let c = &e.pcurves[side];
                assert_eq!(
                    c.control_points[if end == 0 {
                        0
                    } else {
                        c.control_points.len() - 1
                    }],
                    vertex.native_uv[side]
                );
            }
        }
        for j in 0..=64 {
            let t = j as f64 / 64.;
            let x = (1. - t * t) / (1. + t * t);
            let y = 2. * t / (1. + t * t);
            let [x, y] = match i {
                0 => [x, y],
                1 => [-y, x],
                2 => [-x, -y],
                _ => [y, -x],
            };
            let u = 0.5 + radii[0] * x + 0.5 * radii[1] * y;
            let v = 0.5 + radii[1] * y;
            let expected = [u, v, 2. + 3. * u - v + u * v];
            let actual = e.world.evaluate(t).unwrap().point;
            assert!(distance(&actual, &expected) <= e.world_error_upper);
            for (side, s) in [o.source().first(), o.source().second()]
                .into_iter()
                .enumerate()
            {
                let uv = e.pcurves[side].evaluate(t).unwrap().point;
                let native = if side == 0 {
                    [2. + 4. * u, -4. + 8. * v]
                } else {
                    [-3. + 8. * u, 7. + 4. * v]
                };
                assert!(distance(&uv, &native) <= e.uv_error_upper[side]);
                let lifted = s.evaluate(uv[0], uv[1]).unwrap().point;
                assert!(distance(&actual, &lifted) <= e.lift_agreement_upper[side]);
            }
        }
    }
    result.reverify().unwrap();
}
#[test]
fn straight_uv_factor_retains_a_quartic_world_curve_in_five_poles() {
    let o = owner([1., 0., 2., 0.], true);
    let result = build(&o);
    assert_eq!(result.edges().len(), 4);
    for e in result.edges() {
        assert_eq!(e.world.degree, 4);
        assert_eq!(e.world.control_points.len(), 5);
        assert!(
            e.pcurves
                .iter()
                .all(|c| c.degree == 1 && c.control_points.len() == 2)
        );
    }
    let e = &result.edges()[2];
    let p = e.world.evaluate(0.5).unwrap().point;
    assert!(distance(&p, &[0.25, 0.625, 2.3056640625]) <= e.world_error_upper);
    assert!(p[2] > 2.30125);
    result.reverify().unwrap();
}
#[test]
fn domain_admission_is_continuous_even_when_an_ellipse_control_pole_is_outside_the_face() {
    let o = owner([1., 1.8125, -1., -0.140625], false);
    let result = build(&o);
    assert!(
        result.edges()[0].pcurves[0]
            .control_points
            .iter()
            .any(|p| p[0] > 6.)
    );
    for e in result.edges() {
        for i in 0..=64 {
            let t = i as f64 / 64.;
            let p = e.pcurves[0].evaluate(t).unwrap().point;
            assert!((2. ..=6.).contains(&p[0]) && (-4. ..=4.).contains(&p[1]));
        }
    }
    result.reverify().unwrap();
}
#[test]
fn double_parallel_empty_and_point_strata_do_not_acquire_fictitious_edges() {
    for form in [
        [1., 1., 2., 0.],
        [1., 1., 2., -0.0625],
        [1., 1., 1., 0.],
        [1., 1., 1., 0.125],
        [0., 0., 0., 0.],
    ] {
        let o = owner(form, false);
        let result = build(&o);
        assert_eq!(result.edges().len(), o.edges().len());
        assert_eq!(result.vertices().len(), o.vertices().len());
        for v in result.vertices() {
            assert!(
                v.lift_agreement_upper
                    .iter()
                    .all(|e| *e <= tolerances().world)
            );
        }
        result.reverify().unwrap();
    }
}
#[test]
fn candidate_work_is_one_shared_allowance_and_tolerance_is_never_silently_increased() {
    let o = owner([1., 1., 1., -0.0625], false);
    let r = build(&o);
    let used = r.operations();
    assert!(used < 200000);
    assert!(
        ss::construct_graph_curves(
            &o,
            tolerances(),
            ss::GraphCurveBuildLimits {
                max_operations: used
            }
        )
        .is_ok()
    );
    assert_eq!(
        ss::construct_graph_curves(
            &o,
            tolerances(),
            ss::GraphCurveBuildLimits {
                max_operations: used - 1
            }
        )
        .unwrap_err()
        .code,
        "NURBS_RESOURCE_LIMIT"
    );
    assert_eq!(
        ss::construct_graph_curves(
            &o,
            tolerances(),
            ss::GraphCurveBuildLimits { max_operations: 1 }
        )
        .unwrap_err()
        .code,
        "NURBS_RESOURCE_LIMIT"
    );
    assert!(
        ss::construct_graph_curves(
            &o,
            ss::GraphCurveTolerances {
                world: 1e-30,
                uv: 1e-30
            },
            Default::default()
        )
        .is_err()
    );
    for world in [0., -1., f64::NAN] {
        assert!(
            ss::construct_graph_curves(
                &o,
                ss::GraphCurveTolerances { world, uv: 1e-9 },
                Default::default()
            )
            .is_err()
        );
    }
    assert!(
        ss::construct_graph_curves(
            &o,
            tolerances(),
            ss::GraphCurveBuildLimits {
                max_operations: 1000001
            }
        )
        .is_err()
    );
}
#[test]
fn isolated_vertex_uv_closeness_cannot_hide_a_wrong_original_world_lift() {
    let domain = [[1., 1f64.next_up()], [0., 1.]];
    let a = surface([1., 1., 1., 0.], domain, false);
    let b = surface([0.; 4], domain, false);
    let o =
        ss::qualify_quadratic_graph_ss(&ss::FiberSources::new(&a, &b).unwrap(), Default::default())
            .unwrap();
    assert_eq!(o.kind(), K::IsolatedTangentPoint);
    assert!(ss::construct_graph_curves(&o, tolerances(), Default::default()).is_err());
}
#[test]
fn nonrepresentable_native_spans_and_support_swap_keep_original_world_and_uv_correspondence() {
    let a = surface([1., 1., -1., -0.015625], [[1e-6, 1.], [1e-8, 0.75]], true);
    let b = surface([0.; 4], [[-2e-5, 2.], [0.001, 3.]], true);
    let source = ss::FiberSources::new(&a, &b).unwrap();
    let reverse = ss::FiberSources::new(&b, &a).unwrap();
    let o = ss::qualify_quadratic_graph_ss(&source, Default::default()).unwrap();
    let q = ss::qualify_quadratic_graph_ss(&reverse, Default::default()).unwrap();
    let p = build(&o);
    let swapped = build(&q);
    for (left, right) in p.edges().iter().zip(swapped.edges()) {
        assert_eq!(left.pcurves[0], right.pcurves[1]);
        assert_eq!(left.pcurves[1], right.pcurves[0]);
        for i in 0..=32 {
            let t = i as f64 / 32.;
            let x = left.world.evaluate(t).unwrap().point;
            let y = right.world.evaluate(t).unwrap().point;
            assert!(distance(&x, &y) <= left.world_error_upper + right.world_error_upper);
            for (side, s) in [&a, &b].into_iter().enumerate() {
                let uv = left.pcurves[side].evaluate(t).unwrap().point;
                let world = s.evaluate(uv[0], uv[1]).unwrap().point;
                assert!(distance(&x, &world) <= left.lift_agreement_upper[side]);
            }
        }
    }
    p.reverify().unwrap();
    swapped.reverify().unwrap();
    assert_eq!(*o.source().first(), a);
    assert_eq!(*o.source().second(), b);
}
#[test]
fn exact_original_boundary_rails_are_preserved_without_an_epsilon_snap() {
    let mut a = surface([0.; 4], [[2., 6.], [-4., 4.]], false);
    for p in &mut a.control_points[2] {
        p[2] += 1.;
    }
    let b = surface([0.; 4], [[-3., 5.], [7., 11.]], false);
    let source = ss::FiberSources::new(&a, &b).unwrap();
    let o = ss::qualify_quadratic_graph_ss(&source, Default::default()).unwrap();
    assert_eq!(o.kind(), K::TangentDoubleLine);
    let proxy = build(&o);
    assert_eq!(proxy.edges().len(), 1);
    let e = &proxy.edges()[0];
    assert!(e.pcurves[0].control_points.iter().all(|p| p[0] == 2.));
    assert!(e.pcurves[1].control_points.iter().all(|p| p[0] == -3.));
    assert!(e.world.control_points.iter().all(|p| p[0] == 0.));
    for v in proxy.vertices() {
        assert_eq!(v.world[0], 0.);
        assert_eq!(v.native_uv[0][0], 2.);
        assert_eq!(v.native_uv[1][0], -3.);
    }
    proxy.reverify().unwrap();
}
#[test]
fn an_exact_constant_height_source_keeps_planarity_and_uses_only_three_poles_per_conic_edge() {
    let mut a = surface([1., 1., 0., -0.0625], [[2., 6.], [-4., 4.]], false);
    let mut b = surface([0.; 4], [[-3., 5.], [7., 11.]], false);
    for s in [&mut a, &mut b] {
        for row in &mut s.control_points {
            for p in row {
                p[2] = p[2] - (2. + 3. * p[0] - p[1] + p[0] * p[1]) + 7.;
            }
        }
    }
    let source = ss::FiberSources::new(&a, &b).unwrap();
    let o = ss::qualify_quadratic_graph_ss(&source, Default::default()).unwrap();
    let r = build(&o);
    assert_eq!(r.edges().len(), 4);
    for edge in r.edges() {
        assert_eq!(edge.world.degree, 2);
        assert_eq!(edge.world.control_points.len(), 3);
        assert!(edge.world.control_points.iter().all(|p| p[2] == 7.));
    }
    assert!(r.vertices().iter().all(|v| v.world[2] == 7.));
    r.reverify().unwrap();
}
