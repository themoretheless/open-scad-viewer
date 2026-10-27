use nurbs_core::{
    ss_intersection::{self as ss, QuadraticGraphSSKind as K},
    surface::Surface,
};
fn patch(form: [f64; 4], center: [f64; 2], domain: [[f64; 2]; 2]) -> Surface {
    let [a, b, c, k] = form;
    let squares = center.map(|v| [v * v, v * v - v, (1. - v) * (1. - v)]);
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
                            background(u, v)
                                + a * squares[0][i]
                                + b * squares[1][j]
                                + c * (u - center[0]) * (v - center[1])
                                + k,
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
fn background(u: f64, v: f64) -> f64 {
    2. + 3. * u - v + u * v
}
fn sources(form: [f64; 4], center: [f64; 2]) -> ss::FiberSources {
    ss::FiberSources::new(
        &patch(form, center, [[2., 6.], [-4., 4.]]),
        &patch([0.; 4], center, [[-3., 5.], [7., 11.]]),
    )
    .unwrap()
}
fn contains(i: [f64; 2], x: f64) {
    assert!(i[0] <= x && x <= i[1], "{i:?} misses {x}")
}
fn check_ellipse(form: [f64; 4], center: [f64; 2]) {
    let source = sources(form, center);
    let owner = ss::qualify_quadratic_graph_ss(&source, Default::default()).unwrap();
    assert_eq!(owner.kind(), K::ClosedEllipse);
    assert!(owner.covers_entire_surfaces());
    assert_eq!(owner.branches().len(), 1);
    assert!(owner.branches()[0].closed);
    assert_eq!(owner.edges().len(), 4);
    assert_eq!(owner.vertices().len(), 4);
    assert_eq!(owner.trims().len(), 8);
    let [a, b, c, k] = form;
    let h = c / (2. * a);
    let second = b - a * h * h;
    let radii = [(-k / a).sqrt(), (-k / second).sqrt()];
    for quadrant in 0..4 {
        assert_eq!(
            owner.edges()[quadrant].vertices,
            [quadrant, (quadrant + 1) % 4]
        );
        assert_eq!(owner.edges()[quadrant].multiplicity, 1);
        for t in [0., 0.125, 0.5, 0.875, 1.] {
            let x = (1. - t * t) / (1. + t * t);
            let y = 2. * t / (1. + t * t);
            let [x, y] = match quadrant {
                0 => [x, y],
                1 => [-y, x],
                2 => [-x, -y],
                _ => [y, -x],
            };
            let u = center[0] + radii[0] * x - h * radii[1] * y;
            let v = center[1] + radii[1] * y;
            let du = u - center[0];
            let dv = v - center[1];
            assert!((a * du * du + b * dv * dv + c * du * dv + k).abs() < 1e-13);
            let station = owner.edge_station(quadrant, t, 1024).unwrap();
            contains(station.normalized_uv[0], u);
            contains(station.normalized_uv[1], v);
            contains(station.world[0], u);
            contains(station.world[1], v);
            contains(station.world[2], background(u, v));
            let native = [[2. + 4. * u, -4. + 8. * v], [-3. + 8. * u, 7. + 4. * v]];
            for (side, surface) in [source.first(), source.second()].into_iter().enumerate() {
                let enclosed = if side == 0 {
                    station.first_uv
                } else {
                    station.second_uv
                };
                for axis in 0..2 {
                    contains(enclosed[axis], native[side][axis]);
                }
                let evaluated = surface
                    .evaluate(native[side][0], native[side][1])
                    .unwrap()
                    .point;
                for (actual, expected) in evaluated.iter().zip([u, v, background(u, v)]) {
                    assert!((actual - expected).abs() < 1e-12);
                }
            }
        }
    }
    owner.reverify().unwrap();
    let dispatched = ss::construct_qualified_ss_branches(&source, Default::default()).unwrap();
    assert!(matches!(
        dispatched,
        ss::QualifiedSurfaceIntersection::ClosedOrDegenerateGraph(_)
    ));
    dispatched.reverify().unwrap();
}
#[test]
fn mixed_definite_forms_have_one_complete_original_cycle_and_two_native_uv_maps() {
    for form in [
        [1., 1., 1., -0.015625],
        [1., 1.5, -1., -0.0625],
        [2., 1.5, 2., -0.03125],
        [-1., -1., -1., 0.015625],
    ] {
        check_ellipse(form, [0.5; 2]);
    }
    check_ellipse([1., 1.5, 1., -0.0625], [0.375, 0.625]);
}
#[test]
fn exact_original_axis_extents_admit_an_ellipse_that_a_sum_of_radii_would_reject() {
    // Transformed radii are .375 and .25, shear .5. Their absolute sum
    // reaches the U boundary; the actual amplitude sqrt(.375²+.125²) does not.
    check_ellipse([1., 2.5, 1., -0.140625], [0.5; 2]);
}
#[test]
fn thin_mixed_ellipse_keeps_the_component_instead_of_collapsing_it_to_a_point() {
    check_ellipse([1., 0.25 + 2f64.powi(-24), 1., -2f64.powi(-32)], [0.5; 2]);
}
#[test]
fn mixed_tangent_point_empty_set_and_support_swap_preserve_the_exact_stratum() {
    let s = sources([1., 1., 1., 0.], [0.375, 0.625]);
    let o = ss::qualify_quadratic_graph_ss(&s, Default::default()).unwrap();
    assert_eq!(o.kind(), K::IsolatedTangentPoint);
    assert!(o.edges().is_empty());
    assert!(o.trims().is_empty());
    assert_eq!(o.vertices().len(), 1);
    assert!(o.vertices()[0].singular);
    let p = o.vertex_station(0, 1024).unwrap();
    contains(p.world[0], 0.375);
    contains(p.world[1], 0.625);
    for form in [[1., 1., 1., 0.0625], [1., 1., 1., 0.]] {
        let center = if form[3] == 0. { [1.5, 0.5] } else { [0.5; 2] };
        assert_eq!(
            ss::qualify_quadratic_graph_ss(&sources(form, center), Default::default())
                .unwrap()
                .kind(),
            K::Empty
        );
    }
    let s = sources([1., 1., 1., -0.0625], [0.5; 2]);
    let reverse = ss::FiberSources::new(s.second(), s.first()).unwrap();
    let a = ss::qualify_quadratic_graph_ss(&s, Default::default()).unwrap();
    let b = ss::qualify_quadratic_graph_ss(&reverse, Default::default()).unwrap();
    assert_eq!(a.kind(), b.kind());
    assert_eq!(a.edges(), b.edges());
    assert_eq!(a.branches(), b.branches());
    for edge in 0..4 {
        let p = a.edge_station(edge, 0.5, 1024).unwrap();
        let q = b.edge_station(edge, 0.5, 1024).unwrap();
        assert_eq!(p.first_uv, q.second_uv);
        assert_eq!(p.second_uv, q.first_uv);
        assert!(
            p.world
                .iter()
                .zip(q.world)
                .all(|(p, q)| p[0] <= q[1] && q[0] <= p[1])
        );
    }
    b.reverify().unwrap();
}
#[test]
fn clipped_nonzero_hyperbola_and_inexact_shear_are_not_given_a_complete_closed_graph() {
    for form in [
        [1., 2., 2., -0.1875],
        [1., 1., 3., -0.0625],
        [3., 3., 1., -0.0625],
    ] {
        assert!(
            ss::qualify_quadratic_graph_ss(&sources(form, [0.5; 2]), Default::default()).is_err(),
            "{form:?}"
        );
    }
}
#[test]
fn exact_work_limit_covers_the_shear_and_does_not_buy_a_fresh_fallback_budget() {
    let s = sources([1., 1., 1., -0.0625], [0.5; 2]);
    let o = ss::qualify_quadratic_graph_ss(&s, Default::default()).unwrap();
    let allowance = o.operations();
    assert!(allowance > 1 && allowance < 4096);
    assert!(
        ss::qualify_quadratic_graph_ss(
            &s,
            ss::QuadraticGraphSSLimits {
                max_operations: allowance
            }
        )
        .is_ok()
    );
    let limits = ss::QuadraticGraphSSLimits {
        max_operations: allowance - 1,
    };
    assert_eq!(
        ss::qualify_quadratic_graph_ss(&s, limits).unwrap_err().code,
        "NURBS_RESOURCE_LIMIT"
    );
    let mut build = ss::QualifiedSSBuildLimits::default();
    build.quadratic = limits;
    assert_eq!(
        ss::construct_qualified_ss_branches(&s, build)
            .unwrap_err()
            .code,
        "NURBS_RESOURCE_LIMIT"
    );
    assert!(o.edge_station(0, 0.5, 1).is_err());
    o.reverify().unwrap();
}
fn check_line_stations(
    s: &ss::FiberSources,
    o: &ss::QualifiedQuadraticGraphSS,
    form: [f64; 4],
    center: [f64; 2],
) {
    for edge in 0..o.edges().len() {
        for t in [0., 0.125, 0.5, 1.] {
            let p = o.edge_station(edge, t, 1024).unwrap();
            let uv = p.normalized_uv.map(|v| ((v[0] + v[1]) / 2.).clamp(0., 1.));
            let [a, b, c, k] = form;
            let du = uv[0] - center[0];
            let dv = uv[1] - center[1];
            assert!((a * du * du + b * dv * dv + c * du * dv + k).abs() < 1e-12);
            for axis in 0..3 {
                assert!(p.world[axis][1] - p.world[axis][0] < 1e-11);
            }
            contains(p.world[2], background(uv[0], uv[1]));
            for (side, surface) in [s.first(), s.second()].into_iter().enumerate() {
                let native = if side == 0 {
                    [2. + 4. * uv[0], -4. + 8. * uv[1]]
                } else {
                    [-3. + 8. * uv[0], 7. + 4. * uv[1]]
                };
                let value = surface.evaluate(native[0], native[1]).unwrap().point;
                assert!((value[2] - background(uv[0], uv[1])).abs() < 1e-12);
            }
        }
    }
    o.reverify().unwrap();
}
#[test]
fn mixed_crossings_and_zero_pivots_keep_one_true_singular_node_and_all_four_halfedges() {
    for form in [
        [1., 0., 2., 0.],
        [0., 1., 2., 0.],
        [0., 0., 1., 0.],
        [1., 0., 1., 0.],
        [-1., 0., -2., 0.],
    ] {
        let s = sources(form, [0.5; 2]);
        let o = ss::qualify_quadratic_graph_ss(&s, Default::default()).unwrap();
        assert_eq!(o.kind(), K::CrossingLines);
        assert_eq!(o.branches().len(), 2);
        assert!(o.branches().iter().all(|b| !b.closed && b.edges.len() == 2));
        assert_eq!(o.vertices().len(), 5);
        assert!(o.vertices()[0].singular);
        assert_eq!(o.edges().len(), 4);
        assert_eq!(o.trims().len(), 8);
        assert_eq!(
            o.edges().iter().filter(|e| e.vertices.contains(&0)).count(),
            4
        );
        let p = o.vertex_station(0, 1024).unwrap();
        contains(p.world[0], 0.5);
        contains(p.world[1], 0.5);
        check_line_stations(&s, &o, form, [0.5; 2]);
    }
}
#[test]
fn mixed_double_lines_and_clipped_tangent_corner_points_keep_their_actual_dimension() {
    for form in [[1., 1., 2., 0.], [1., 0.25, 1., 0.], [0.25, 1., 1., 0.]] {
        let s = sources(form, [0.5; 2]);
        let o = ss::qualify_quadratic_graph_ss(&s, Default::default()).unwrap();
        assert_eq!(o.kind(), K::TangentDoubleLine);
        assert_eq!(o.edges().len(), 1);
        assert_eq!(o.edges()[0].multiplicity, 2);
        assert_eq!(o.branches().len(), 1);
        assert!(o.vertices().iter().all(|v| v.singular));
        assert_eq!(o.trims().len(), 2);
        check_line_stations(&s, &o, form, [0.5; 2]);
    }
    for center in [[0.; 2], [1.; 2]] {
        let s = sources([1., 1., 2., 0.], center);
        let o = ss::qualify_quadratic_graph_ss(&s, Default::default()).unwrap();
        assert_eq!(o.kind(), K::IsolatedTangentPoint);
        assert_eq!(o.vertices().len(), 1);
        assert!(o.edges().is_empty() && o.branches().is_empty() && o.trims().is_empty());
        let p = o.vertex_station(0, 1024).unwrap();
        contains(p.world[0], center[0]);
        contains(p.world[1], center[1]);
        o.reverify().unwrap();
    }
    assert_eq!(
        ss::qualify_quadratic_graph_ss(&sources([1., 1., 2., 0.], [1.5; 2]), Default::default())
            .unwrap()
            .kind(),
        K::Empty
    );
}
#[test]
fn mixed_parallel_factors_are_clipped_in_original_coordinates_and_keep_small_separation() {
    for form in [
        [1., 1., 2., -0.0625],
        [-1., -1., -2., 0.0625],
        [1., 0.25, 1., -0.0625],
        [1., 1., 2., -2f64.powi(-40)],
    ] {
        let s = sources(form, [0.5; 2]);
        let o = ss::qualify_quadratic_graph_ss(&s, Default::default()).unwrap();
        assert_eq!(o.kind(), K::ParallelLineFactors);
        assert_eq!(o.edges().len(), 2);
        assert_eq!(o.branches().len(), 2);
        assert_eq!(o.vertices().len(), 4);
        assert_eq!(o.trims().len(), 4);
        assert!(o.edges().iter().all(|e| e.multiplicity == 1));
        check_line_stations(&s, &o, form, [0.5; 2]);
    }
    let s = sources([1., 1., 2., -0.25], [0.875; 2]);
    let o = ss::qualify_quadratic_graph_ss(&s, Default::default()).unwrap();
    assert_eq!(o.kind(), K::ParallelLineFactors);
    assert_eq!(o.edges().len(), 1);
    check_line_stations(&s, &o, [1., 1., 2., -0.25], [0.875; 2]);
    assert_eq!(
        ss::qualify_quadratic_graph_ss(
            &sources([1., 1., 2., -0.0625], [1.5; 2]),
            Default::default()
        )
        .unwrap()
        .kind(),
        K::Empty
    );
}
#[test]
fn nonzero_parabolic_linear_term_and_simple_corner_only_strata_do_not_inherit_a_double_line() {
    let s = sources([1., 1., 2., 0.], [0.5; 2]);
    let mut a = s.first().clone();
    for row in &mut a.control_points {
        for p in row {
            p[2] += p[1];
        }
    }
    assert!(
        ss::qualify_quadratic_graph_ss(
            &ss::FiberSources::new(&a, s.second()).unwrap(),
            Default::default()
        )
        .is_err()
    );
    assert!(
        ss::qualify_quadratic_graph_ss(&sources([1., 1., 2., -1.], [0.5; 2]), Default::default())
            .is_err()
    );
    assert!(
        ss::qualify_quadratic_graph_ss(&sources([0., 0., 1., 0.125], [0.5; 2]), Default::default())
            .is_err()
    );
}
#[test]
fn every_mixed_family_replays_original_sources_and_uses_one_bounded_exact_attempt() {
    for form in [
        [1., 1., 1., -0.0625],
        [0., 0., 1., 0.],
        [1., 1., 2., 0.],
        [1., 1., 2., -0.0625],
    ] {
        let s = sources(form, [0.5; 2]);
        let o = ss::qualify_quadratic_graph_ss(&s, Default::default()).unwrap();
        let mut limits = ss::QualifiedSSBuildLimits::default();
        limits.quadratic.max_operations = o.operations() - 1;
        assert_eq!(
            ss::construct_qualified_ss_branches(&s, limits)
                .unwrap_err()
                .code,
            "NURBS_RESOURCE_LIMIT"
        );
        let own = ss::construct_qualified_ss_branches(&s, Default::default()).unwrap();
        assert!(own.covers_entire_surfaces());
        own.reverify().unwrap();
    }
}
#[test]
fn mixed_uv_lines_keep_the_shared_biquadratic_world_height_instead_of_an_endpoint_chord() {
    let s = sources([1., 0., 2., 0.], [0.5; 2]);
    let mut a = s.first().clone();
    let mut b = s.second().clone();
    // This one Bernstein pole adds exactly u²*v² to BOTH original supports.
    a.control_points[2][2][2] += 1.;
    b.control_points[2][2][2] += 1.;
    let s = ss::FiberSources::new(&a, &b).unwrap();
    let o = ss::qualify_quadratic_graph_ss(&s, Default::default()).unwrap();
    assert_eq!(o.kind(), K::CrossingLines);
    let p = o.edge_station(2, 0.5, 1024).unwrap();
    contains(p.world[0], 0.25);
    contains(p.world[1], 0.625);
    contains(p.world[2], 2.3056640625);
    // The chord between the true world endpoints has z=2.28125 here.
    assert!(p.world[2][0] > 2.30125);
    for edge in 0..4 {
        for t in [0., 0.125, 0.5, 1.] {
            let p = o.edge_station(edge, t, 1024).unwrap();
            let [u, v] = p.normalized_uv.map(|r| ((r[0] + r[1]) / 2.).clamp(0., 1.));
            let expected = background(u, v) + u * u * v * v;
            contains(p.world[2], expected);
            for surface in [s.first(), s.second()] {
                let (ku, kv) = (&surface.knots_u, &surface.knots_v);
                let point = surface
                    .evaluate(
                        ku[0] + (ku.last().unwrap() - ku[0]) * u,
                        kv[0] + (kv.last().unwrap() - kv[0]) * v,
                    )
                    .unwrap()
                    .point;
                assert!((point[2] - expected).abs() < 1e-12);
            }
        }
    }
    o.reverify().unwrap();
}
#[test]
fn continuous_ranges_cover_original_ellipse_arcs_and_both_native_domains() {
    let form = [1., 1., -1., -0.015625];
    let s = sources(form, [0.5; 2]);
    let o = ss::qualify_quadratic_graph_ss(&s, Default::default()).unwrap();
    let h = -0.5;
    let radii = [0.125, (1f64 / 48.).sqrt()];
    for edge in 0..4 {
        for clock in [[0., 1.], [0.125, 0.75], [0.5, 0.5]] {
            let r = o.edge_enclosure(edge, clock, 1024).unwrap();
            assert_eq!(r.clock, clock);
            for i in 0..=40 {
                let t = clock[0] + (clock[1] - clock[0]) * i as f64 / 40.;
                let x = (1. - t * t) / (1. + t * t);
                let y = 2. * t / (1. + t * t);
                let [x, y] = match edge {
                    0 => [x, y],
                    1 => [-y, x],
                    2 => [-x, -y],
                    _ => [y, -x],
                };
                let u = 0.5 + radii[0] * x - h * radii[1] * y;
                let v = 0.5 + radii[1] * y;
                contains(r.normalized_uv[0], u);
                contains(r.normalized_uv[1], v);
                contains(r.first_uv[0], 2. + 4. * u);
                contains(r.first_uv[1], -4. + 8. * v);
                contains(r.second_uv[0], -3. + 8. * u);
                contains(r.second_uv[1], 7. + 4. * v);
                contains(r.world[0], u);
                contains(r.world[1], v);
                contains(r.world[2], background(u, v));
            }
            assert!(r.normalized_uv.iter().all(|v| v[0] >= 0. && v[1] <= 1.));
        }
    }
    // The arc bulges beyond the U coordinate of BOTH endpoints.
    let r = o.edge_enclosure(0, [0., 1.], 1024).unwrap();
    let a = o.edge_station(0, 0., 1024).unwrap();
    let b = o.edge_station(0, 1., 1024).unwrap();
    let u = 0.5 + 0.125 * 0.8 + 0.5 * radii[1] * 0.6;
    assert!(u > a.normalized_uv[0][1].max(b.normalized_uv[0][1]));
    contains(r.normalized_uv[0], u);
    o.reverify().unwrap();
}
#[test]
fn continuous_world_range_contains_an_interior_height_missed_by_the_endpoint_hull() {
    let s = sources([1., 1., 2., 0.], [0.5; 2]);
    let mut a = s.first().clone();
    let mut b = s.second().clone();
    for surface in [&mut a, &mut b] {
        for row in &mut surface.control_points {
            for p in row {
                p[2] = p[2] - background(p[0], p[1]) + p[0] * p[1];
            }
        }
    }
    let o =
        ss::qualify_quadratic_graph_ss(&ss::FiberSources::new(&a, &b).unwrap(), Default::default())
            .unwrap();
    let r = o.edge_enclosure(0, [0., 1.], 1024).unwrap();
    let a = o.edge_station(0, 0., 1024).unwrap();
    let b = o.edge_station(0, 1., 1024).unwrap();
    assert!(a.world[2][1].max(b.world[2][1]) < 1e-12);
    contains(r.world[2], 0.25);
    contains(r.world[2], 0.);
    for i in 0..=64 {
        let t = i as f64 / 64.;
        contains(r.world[2], t * (1. - t));
    }
}
#[test]
fn all_owned_linear_families_have_continuous_ranges_that_refine_and_swap_supports() {
    for form in [[0., 0., 1., 0.], [1., 1., 2., 0.], [1., 1., 2., -0.0625]] {
        let s = sources(form, [0.5; 2]);
        let reverse = ss::FiberSources::new(s.second(), s.first()).unwrap();
        let o = ss::qualify_quadratic_graph_ss(&s, Default::default()).unwrap();
        let q = ss::qualify_quadratic_graph_ss(&reverse, Default::default()).unwrap();
        for edge in 0..o.edges().len() {
            let full = o.edge_enclosure(edge, [0., 1.], 1024).unwrap();
            let part = o.edge_enclosure(edge, [0.25, 0.5], 1024).unwrap();
            let swap = q.edge_enclosure(edge, [0.25, 0.5], 1024).unwrap();
            assert_eq!(part.first_uv, swap.second_uv);
            assert_eq!(part.second_uv, swap.first_uv);
            for axis in 0..2 {
                assert!(
                    full.normalized_uv[axis][0] <= part.normalized_uv[axis][0]
                        && part.normalized_uv[axis][1] <= full.normalized_uv[axis][1]
                );
            }
            assert!(
                part.world
                    .iter()
                    .zip(full.world)
                    .all(|(p, f)| f[0] <= p[0] && p[1] <= f[1])
            );
            for t in [0.25, 0.375, 0.5] {
                let p = o.edge_station(edge, t, 1024).unwrap();
                for axis in 0..3 {
                    contains(part.world[axis], (p.world[axis][0] + p.world[axis][1]) / 2.);
                }
            }
        }
    }
}
#[test]
fn continuous_ranges_validate_clock_indices_and_charge_the_exact_provided_budget() {
    let o = ss::qualify_quadratic_graph_ss(
        &sources([1., 1., 1., -0.0625], [0.5; 2]),
        Default::default(),
    )
    .unwrap();
    for clock in [
        [-0.01, 1.],
        [0., 1.01],
        [0.5, 0.25],
        [f64::NAN, 1.],
        [0., f64::INFINITY],
    ] {
        assert!(o.edge_enclosure(0, clock, 1024).is_err());
    }
    assert!(o.edge_enclosure(4, [0., 1.], 1024).is_err());
    for work in [0, 1, 4097] {
        assert!(o.edge_enclosure(0, [0., 1.], work).is_err());
    }
    let r = o.edge_enclosure(0, [0.25, 0.75], 1024).unwrap();
    assert!(o.edge_enclosure(0, r.clock, r.operations).is_ok());
    assert_eq!(
        o.edge_enclosure(0, r.clock, r.operations - 1)
            .unwrap_err()
            .code,
        "NURBS_RESOURCE_LIMIT"
    );
    for form in [[1., 1., 1., 0.], [1., 1., 1., 0.125]] {
        let o =
            ss::qualify_quadratic_graph_ss(&sources(form, [0.5; 2]), Default::default()).unwrap();
        assert!(o.edge_enclosure(0, [0., 1.], 1024).is_err());
    }
}
#[test]
fn original_native_domains_do_not_require_a_representable_binary64_span() {
    let domains = [[[1e-6, 1.], [1e-8, 0.75]], [[-2e-5, 2.], [0.001, 3.]]];
    let a = patch([1., 1., -1., -0.015625], [0.5; 2], domains[0]);
    let b = patch([0.; 4], [0.5; 2], domains[1]);
    let s = ss::FiberSources::new(&a, &b).unwrap();
    let o = ss::qualify_quadratic_graph_ss(&s, Default::default()).unwrap();
    assert_eq!(o.kind(), K::ClosedEllipse);
    let radii = [0.125, (1f64 / 48.).sqrt()];
    for edge in 0..4 {
        let range = o.edge_enclosure(edge, [0., 1.], 1024).unwrap();
        for t in [0., 0.125, 0.5, 0.75, 1.] {
            let x = (1. - t * t) / (1. + t * t);
            let y = 2. * t / (1. + t * t);
            let [x, y] = match edge {
                0 => [x, y],
                1 => [-y, x],
                2 => [-x, -y],
                _ => [y, -x],
            };
            let uv = [0.5 + radii[0] * x + 0.5 * radii[1] * y, 0.5 + radii[1] * y];
            let point = o.edge_station(edge, t, 1024).unwrap();
            for (side, surface) in [&a, &b].into_iter().enumerate() {
                let native = [0, 1].map(|axis| {
                    (1. - uv[axis]) * domains[side][axis][0] + uv[axis] * domains[side][axis][1]
                });
                let bounds = if side == 0 {
                    point.first_uv
                } else {
                    point.second_uv
                };
                let ranged = if side == 0 {
                    range.first_uv
                } else {
                    range.second_uv
                };
                for axis in 0..2 {
                    contains(bounds[axis], native[axis]);
                    contains(ranged[axis], native[axis]);
                }
                let p = surface.evaluate(native[0], native[1]).unwrap().point;
                for (actual, expected) in p.iter().zip([uv[0], uv[1], background(uv[0], uv[1])]) {
                    assert!((actual - expected).abs() < 1e-12);
                }
            }
        }
    }
    o.reverify().unwrap();
}
