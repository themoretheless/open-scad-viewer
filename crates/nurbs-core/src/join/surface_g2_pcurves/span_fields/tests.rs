use super::*;
fn plane(split: bool, height: f64) -> Surface {
    let rows = if split {
        vec![(0., 0.), (0.5, 0.), (1., height)]
    } else {
        vec![(0., 0.), (1., 0.)]
    };
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: if split {
            vec![0., 0., 0.5, 1., 1.]
        } else {
            vec![0., 0., 1., 1.]
        },
        knots_v: vec![0., 0., 1., 1.],
        control_points: rows
            .iter()
            .map(|&(x, z)| vec![vec![x, 0., z], vec![x, 1., z]])
            .collect(),
        weights: vec![vec![1.; 2]; rows.len()],
        periodic_u: false,
        periodic_v: false,
    }
}
fn trace() -> (Curve, Curve) {
    (
        Curve::from_polyline(vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]]).unwrap(),
        Curve::from_polyline(vec![vec![0.5, 0.], vec![0.5, 1.]]).unwrap(),
    )
}
#[test]
fn all_original_one_sided_fields_cover_a_whole_c0_trace_without_changing_strict_g2() {
    let (c, p) = trace();
    let a = plane(false, 0.);
    let b = plane(true, 0.);
    let saved = b.clone();
    let old = super::super::inspect(
        &c,
        [&p, &p],
        [&a, &b],
        [false; 2],
        1e-7,
        1e-5,
        1e-4,
        128,
        100000,
    )
    .unwrap();
    assert_eq!(old.geometric.decision, Decision::Unresolved);
    let r = inspect_span_fields(
        &c,
        [&p, &p],
        [&a, &b],
        [false; 2],
        1e-7,
        1e-5,
        1e-4,
        128,
        100000,
    )
    .unwrap();
    assert!(r.positions.proven && r.one_sided_regular);
    assert_eq!(r.decision, FieldDecision::Enclosed);
    assert_eq!(r.orientation, Some(Orientation::Same));
    assert_eq!(r.pending, 0);
    assert!(r.cells <= 128);
    assert_eq!(b, saved);
    let exact = inspect_span_fields(
        &c,
        [&p, &p],
        [&a, &b],
        [false; 2],
        1e-7,
        1e-5,
        1e-4,
        r.cells,
        100000,
    )
    .unwrap();
    assert_eq!(exact.decision, FieldDecision::Enclosed);
    let limited = inspect_span_fields(
        &c,
        [&p, &p],
        [&a, &b],
        [false; 2],
        1e-7,
        1e-5,
        1e-4,
        r.cells - 1,
        100000,
    )
    .unwrap();
    assert_eq!(limited.decision, FieldDecision::Unresolved);
    assert!(limited.cells < r.cells);
}
#[test]
fn every_actual_one_sided_kink_branch_remains_a_valid_negative_witness() {
    let (c, p) = trace();
    let a = plane(false, 0.);
    let b = plane(true, 0.25);
    let r = inspect_span_fields(
        &c,
        [&p, &p],
        [&a, &b],
        [false; 2],
        1e-7,
        1e-5,
        1e-4,
        128,
        100000,
    )
    .unwrap();
    assert!(r.positions.proven);
    assert_eq!(r.decision, FieldDecision::ExceedsTolerance);
    assert!(r.branches[0].normal_lower > 0.4 && r.branches[1].normal_lower > 1.);
    assert!(r.orientation.is_none());
}
#[test]
fn a_good_midpoint_does_not_hide_shape_error_elsewhere_on_the_original_curve() {
    let a = plane(false, 0.);
    let coefficients = [0., 1. / 16., -1. / 12., 1. / 16., 0.];
    let b = Surface {
        degree_u: 4,
        degree_v: 1,
        knots_u: [vec![0.; 5], vec![1.; 5]].concat(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: coefficients
            .iter()
            .enumerate()
            .map(|(i, &z)| vec![vec![i as f64 / 4., 0., 0.], vec![i as f64 / 4., 1., z]])
            .collect(),
        weights: vec![vec![1.; 2]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    let c = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
    let p = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    let mut work = 0;
    let mid = field::pair([&p, &p], [&a, &b], [false; 2], [0.5, 0.5], &mut work, 128)
        .unwrap()
        .unwrap();
    assert!(mid.0[0][0] < 1e-5 && mid.0[0][1] < 1e-4);
    let r = inspect_span_fields(
        &c,
        [&p, &p],
        [&a, &b],
        [false; 2],
        1e-7,
        1e-5,
        1e-4,
        10000,
        100000,
    )
    .unwrap();
    assert_eq!(r.decision, FieldDecision::ExceedsTolerance);
    assert!(r.cells <= 10000);
}
#[test]
fn missing_positions_or_work_never_produce_complete_field_coverage() {
    let (c, p) = trace();
    let a = plane(false, 0.);
    let b = plane(true, 0.);
    let r =
        inspect_span_fields(&c, [&p, &p], [&a, &b], [false; 2], 1e-7, 1e-5, 1e-4, 0, 0).unwrap();
    assert_eq!(r.decision, FieldDecision::Unresolved);
    assert!(!r.one_sided_regular && r.cells == 0);
    let mut wrong = c.clone();
    wrong.control_points[1][2] = 0.01;
    let r = inspect_span_fields(
        &wrong,
        [&p, &p],
        [&a, &b],
        [false; 2],
        1e-7,
        1e-5,
        1e-4,
        128,
        100000,
    )
    .unwrap();
    assert_eq!(r.decision, FieldDecision::Unresolved);
    assert!(!r.positions.proven);
}

#[test]
fn mean_value_bounds_keep_rescaled_reversed_clocks_and_cover_analytic_curvature() {
    // z=u² has normal (-2u,0,1)/sqrt(1+4u²), and its curvature-tensor
    // Frobenius norm is 2/(1+4u²)^(3/2). The comparison plane is flat.
    let a = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]],
            vec![vec![1., 0., 1.], vec![1., 1., 1.]],
        ],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let b = plane(false, 0.);
    let mut p = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    p.knots = vec![2., 2., 5., 5.];
    let mut q = Curve::from_polyline(vec![vec![1., 0.], vec![0., 0.]]).unwrap();
    q.knots = vec![-7., -7., -3., -3.];
    let normal = |u: f64| (2. - 2. / (1. + 4. * u * u).sqrt()).sqrt();
    let curvature = |u: f64| 2. / (1. + 4. * u * u).powf(1.5);
    for range in [[0.25, 0.3125], [0.75, 0.8125]] {
        for flipped in [false, true] {
            let mut source = a.clone();
            let mut trace = p.clone();
            if flipped {
                source.control_points.reverse();
                source.weights.reverse();
                trace.control_points.reverse();
            }
            let mut work = 0;
            let (upper, lower) = mean_value::bounds(
                [&trace, &q],
                [&source, &b],
                [false, true],
                range,
                &mut work,
                128,
            )
            .unwrap()
            .unwrap();
            let sign = usize::from(flipped);
            let midpoint = range[0] * 0.5 + range[1] * 0.5;
            assert!(upper[sign][0] >= normal(range[1]) && upper[sign][1] >= curvature(range[0]));
            assert!(
                lower[sign][0] <= normal(midpoint) + 1e-12
                    && lower[sign][1] <= curvature(midpoint) + 1e-12
            );
            assert!(work <= 128);
        }
    }
}

#[test]
fn projected_directional_fields_cover_every_world_component_and_orientation() {
    let original = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]],
            vec![vec![1., 0., 1.], vec![1., 1., 1.]],
        ],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    for u in [0.25_f64, 0.75] {
        let s = (1. + 4. * u * u).sqrt();
        let normal = [-2. / s.powi(3), 0., -4. * u / s.powi(3)];
        let xx = -40. * u / s.powi(7);
        let xz = (4. - 64. * u * u) / s.powi(7);
        let zz = 16. * u * (1. - 6. * u * u) / s.powi(7);
        let expected = [
            normal[0], normal[1], normal[2], xx, 0., xz, 0., 0., 0., xz, 0., zz,
        ];
        for axes in [[0, 1, 2], [1, 2, 0]] {
            for reversed in [false, true] {
                let mut surface = original.clone();
                if reversed {
                    surface.control_points.reverse();
                    surface.weights.reverse();
                }
                let parameter = if reversed { 1. - u } else { u };
                let frame = (axes, [0.; 2]);
                let jets = crate::surface_measure::jets::calculate_partial_stable_graph_third(
                    &surface,
                    [2, 1],
                    [[parameter, parameter], [0.5, 0.5]],
                    [None; 2],
                    frame,
                )
                .unwrap();
                let direction = if reversed { -1. } else { 1. };
                let bounds = super::super::trace_mean_value::graph_shape_derivative(
                    &jets,
                    [I::point(direction), I::point(0.)],
                    frame,
                )
                .unwrap()
                .unwrap();
                for (i, (bound, value)) in bounds.iter().zip(expected).enumerate() {
                    let value = direction * value;
                    assert!(
                        bound.lo <= value + 1e-12 && value - 1e-12 <= bound.hi,
                        "component={i} axes={axes:?} reversed={reversed}: {bound:?} vs {value}"
                    );
                    assert!(bound.width() < 1e-8);
                }
            }
        }
    }
}

#[test]
fn cubic_curvature_direction_requires_actual_third_original_projected_jets() {
    // x=3u/4, z=x^3. At u=0 the directional curvature-tensor
    // derivative is 6 * dx/du = 4.5 in its xx component.
    let s = Surface {
        degree_u: 3,
        degree_v: 1,
        knots_u: [vec![0.; 4], vec![1.; 4]].concat(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: (0..4)
            .map(|i| {
                [0., 1.]
                    .into_iter()
                    .map(|v| vec![i as f64 / 4., v, if i == 3 { 27. / 64. } else { 0. }])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 2]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    let frame = ([0, 1, 2], [0.; 2]);
    let query = [[0., 0.], [0.5, 0.5]];
    let second = crate::surface_measure::jets::calculate_partial_stable_graph(
        &s,
        [3, 1],
        query,
        [None; 2],
        frame,
    )
    .unwrap();
    assert_eq!(second[3][0][2], I::point(0.));
    let third = crate::surface_measure::jets::calculate_partial_stable_graph_third(
        &s,
        [3, 1],
        query,
        [None; 2],
        frame,
    )
    .unwrap();
    assert!(third[3][0][2].contains(2.53125));
    let output = super::super::trace_mean_value::graph_shape_derivative(
        &third,
        [I::point(1.), I::point(0.)],
        frame,
    )
    .unwrap()
    .unwrap();
    for (i, b) in output.iter().enumerate() {
        let expected = if i == 3 { 4.5 } else { 0. };
        assert!(
            b.contains(expected),
            "component={i}: {b:?} expected={expected}"
        );
        assert!(b.width() < 1e-8);
    }
}
