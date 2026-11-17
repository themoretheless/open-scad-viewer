use num_rational::BigRational as Q;
use nurbs_core::{
    editing::exact_surface_derivative::{
        PlaneGraphPosition, Refusal, SurfacePlaneGraphSampleOptions, SurfacePlaneGraphStatus,
        SurfacePlanePolylineOptions, SurfacePlaneSide, SurfacePlaneWorldPolylineOptions,
        approximate_surface_plane_graph, approximate_surface_plane_graph_xyz,
        audit_surface_plane_graphs, audit_surface_plane_side, refine_surface_plane_side,
        sample_surface_plane_graph, split_surface_cell_by_plane, store_surface_plane_section_curve,
        store_surface_plane_section_pair,
    },
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn q(n: i64) -> Q {
    Q::from_integer(n.into())
}

fn surface(degree: usize, knots: Vec<f64>, heights: &[f64]) -> Surface {
    Surface {
        degree_u: degree,
        degree_v: 1,
        knots_u: knots,
        knots_v: vec![0., 0., 1., 1.],
        control_points: heights
            .iter()
            .enumerate()
            .map(|(i, &z)| vec![vec![i as f64, 0., z], vec![i as f64, 1., z]])
            .collect(),
        weights: (0..heights.len())
            .map(|i| vec![i as f64 + 1., i as f64 + 2.])
            .collect(),
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn certifies_closed_span_sides_at_a_rational_knot_seam() {
    let surface = surface(1, vec![0., 0., 1., 2., 2.], &[1., 0., -1.]);
    let report = audit_surface_plane_side(&surface, [0., 0., 1.], 0., WORK).unwrap();
    let cells = report.cells.unwrap();
    assert_eq!(cells.len(), 2);
    assert_eq!(cells[0].domain, [[0., 1.], [0., 1.]]);
    assert_eq!(cells[0].side, SurfacePlaneSide::Nonnegative);
    assert_eq!(cells[0].residual, [q(0), q(1)]);
    assert_eq!(cells[1].domain, [[1., 2.], [0., 1.]]);
    assert_eq!(cells[1].side, SurfacePlaneSide::Nonpositive);
    assert_eq!(cells[1].residual, [q(-1), q(0)]);
    assert_eq!(&cells[0].corners[2..], &cells[1].corners[..2]);
}

#[test]
fn distinguishes_corner_crossings_from_unresolved_mixed_coefficients() {
    let crossing = surface(1, vec![0., 0., 1., 1.], &[-1., 1.]);
    let crossing = audit_surface_plane_side(&crossing, [0., 0., 1.], 0., WORK)
        .unwrap()
        .cells
        .unwrap();
    assert_eq!(crossing[0].side, SurfacePlaneSide::Straddles);
    let folded = surface(2, vec![0., 0., 0., 1., 1., 1.], &[1., -3., 1.]);
    let folded = audit_surface_plane_side(&folded, [0., 0., 1.], 0., WORK)
        .unwrap()
        .cells
        .unwrap();
    assert_eq!(folded[0].side, SurfacePlaneSide::Unresolved);
    assert_eq!(folded[0].corners, std::array::from_fn(|_| q(1)));
    assert_eq!(folded[0].residual, [q(-3), q(1)]);
}

#[test]
fn proves_coplanarity_for_unclamped_spans_and_shares_the_arithmetic_budget() {
    let flat = surface(2, vec![-2., -1., 0., 1., 2., 3., 4.], &[3.; 4]);
    let report = audit_surface_plane_side(&flat, [0., 0., 2.], 6., WORK).unwrap();
    let cells = report.cells.unwrap();
    assert_eq!(cells.len(), 2);
    assert!(
        cells
            .iter()
            .all(|cell| cell.side == SurfacePlaneSide::Coplanar && cell.residual == [q(0), q(0)])
    );
    let limited = audit_surface_plane_side(&flat, [0., 0., 2.], 6., report.work_used - 1).unwrap();
    assert_eq!(limited.cells, Err(Refusal::ResourceLimit));
    assert!(limited.work_used < report.work_used);
}

#[test]
fn proves_a_mixed_control_net_is_strictly_separated_after_subdivision() {
    let folded = surface(2, vec![0., 0., 0., 1., 1., 1.], &[1., -0.125, 1.]);
    let audit = audit_surface_plane_side(&folded, [0., 0., 1.], 0., WORK)
        .unwrap()
        .cells
        .unwrap();
    assert_eq!(audit[0].side, SurfacePlaneSide::Unresolved);
    let refined = refine_surface_plane_side(&folded, [0., 0., 1.], 0., 8, WORK)
        .unwrap()
        .regions
        .unwrap();
    assert_eq!(refined.len(), 2);
    assert_eq!(refined[0].cell.domain, [[0., 0.5], [0., 1.]]);
    assert_eq!(refined[1].cell.domain, [[0.5, 1.], [0., 1.]]);
    assert!(
        refined
            .iter()
            .all(|region| region.cell.side == SurfacePlaneSide::Positive)
    );
    assert_eq!(&refined[0].cell.corners[2..], &refined[1].cell.corners[..2]);
}

#[test]
fn localizes_a_folded_crossing_and_keeps_the_whole_original_domain_covered() {
    let folded = surface(2, vec![0., 0., 0., 1., 1., 1.], &[1., -3., 1.]);
    let report = refine_surface_plane_side(&folded, [0., 0., 1.], 0., 8, WORK).unwrap();
    let regions = report.regions.unwrap();
    let area: f64 = regions
        .iter()
        .map(|region| {
            region
                .cell
                .domain
                .iter()
                .map(|[a, b]| b - a)
                .product::<f64>()
        })
        .sum();
    assert_eq!(area, 1.);
    assert!(
        regions
            .iter()
            .any(|region| region.cell.side == SurfacePlaneSide::Positive)
    );
    assert!(
        regions
            .iter()
            .any(|region| region.cell.side == SurfacePlaneSide::Negative)
    );
    assert!(
        regions
            .iter()
            .any(|region| region.cell.side == SurfacePlaneSide::Straddles)
    );
    let limited =
        refine_surface_plane_side(&folded, [0., 0., 1.], 0., 8, report.work_used - 1).unwrap();
    assert_eq!(limited.regions, Err(Refusal::ResourceLimit));
}

#[test]
fn certifies_unique_intersection_graphs_in_the_interior_and_on_an_edge() {
    use nurbs_core::editing::exact_projective::UvAxis;
    let crossing = surface(1, vec![0., 0., 1., 1.], &[-1., 1.]);
    let report = audit_surface_plane_graphs(&crossing, [0., 0., 1.], 0., WORK).unwrap();
    let cells = report.cells.unwrap();
    let SurfacePlaneGraphStatus::Graphs(graphs) = &cells[0].status else {
        panic!("{:?}", cells[0].status);
    };
    let graph = graphs[0].as_ref().unwrap();
    assert_eq!(graph.solve_axis, UvAxis::U);
    assert_eq!(graph.position, PlaneGraphPosition::Interior);
    assert_eq!(graph.derivative, [q(3), q(5)]);
    assert!(graphs[1].is_none());

    let edge = surface(1, vec![-2., -2., 4., 4.], &[0., 1.]);
    let report = audit_surface_plane_graphs(&edge, [0., 0., -1.], 0., WORK).unwrap();
    let cells = report.cells.unwrap();
    let SurfacePlaneGraphStatus::Graphs(graphs) = &cells[0].status else {
        panic!("{:?}", cells[0].status);
    };
    assert_eq!(
        graphs[0].as_ref().unwrap().position,
        PlaneGraphPosition::LowerEdge
    );
    assert_eq!(graphs[0].as_ref().unwrap().derivative, [q(-3), q(-2)]);
}

#[test]
fn handles_a_diagonal_across_closed_edges_and_refuses_to_certify_a_fold() {
    let mut diagonal = surface(1, vec![0., 0., 1., 1.], &[-1., 0.]);
    diagonal.weights = vec![vec![1.; 2]; 2];
    diagonal.control_points[0][1][2] = 0.;
    diagonal.control_points[1][1][2] = 1.;
    let report = audit_surface_plane_graphs(&diagonal, [0., 0., 1.], 0., WORK).unwrap();
    let cells = report.cells.unwrap();
    let SurfacePlaneGraphStatus::Graphs(graphs) = &cells[0].status else {
        panic!("{:?}", cells[0].status);
    };
    assert!(graphs.iter().all(|graph| {
        graph
            .as_ref()
            .is_some_and(|g| g.position == PlaneGraphPosition::Closed)
    }));
    let limited =
        audit_surface_plane_graphs(&diagonal, [0., 0., 1.], 0., report.work_used - 1).unwrap();
    assert_eq!(limited.cells, Err(Refusal::ResourceLimit));

    let folded = surface(2, vec![0., 0., 0., 1., 1., 1.], &[1., -3., 1.]);
    let report = audit_surface_plane_graphs(&folded, [0., 0., 1.], 0., WORK).unwrap();
    assert_eq!(
        report.cells.unwrap()[0].status,
        SurfacePlaneGraphStatus::Unresolved
    );
}

#[test]
fn samples_weighted_branches_at_nonbinary_parameters_without_rounding() {
    use nurbs_core::editing::exact_projective::UvAxis;
    let crossing = surface(1, vec![-2., -2., 4., 4.], &[-1., 1.]);
    let options = SurfacePlaneGraphSampleOptions {
        span: [1, 1],
        solve_axis: UvAxis::U,
        max_normalized_width: 1e-18,
        max_work: WORK,
    };
    let third = Q::new(1.into(), 3.into());
    let inputs = [q(0), third, q(1)];
    let report = sample_surface_plane_graph(&crossing, [0., 0., 1.], 0., &inputs, options).unwrap();
    let result = report.outcome.unwrap();
    assert_eq!(result.domain, [[-2., 4.], [0., 1.]]);
    let exact = [
        Q::new(1.into(), 3.into()),
        Q::new(4.into(), 11.into()),
        Q::new(2.into(), 5.into()),
    ];
    for ((sample, t), root) in result.samples.iter().zip(&inputs).zip(exact) {
        assert_eq!(&sample.transverse, t);
        assert!(sample.root[0] <= root && root <= sample.root[1]);
        assert!(
            &sample.root[1] - &sample.root[0]
                <= Q::from_float(options.max_normalized_width).unwrap()
        );
    }
    let reversed = sample_surface_plane_graph(&crossing, [0., 0., -1.], 0., &inputs, options)
        .unwrap()
        .outcome
        .unwrap();
    assert_eq!(result.samples, reversed.samples);
    let limited = sample_surface_plane_graph(
        &crossing,
        [0., 0., 1.],
        0.,
        &inputs,
        SurfacePlaneGraphSampleOptions {
            max_work: report.work_used - 1,
            ..options
        },
    )
    .unwrap();
    assert_eq!(limited.outcome, Err(Refusal::ResourceLimit));
}

#[test]
fn retains_exact_edge_samples_and_requires_a_unique_full_graph() {
    use nurbs_core::editing::exact_projective::UvAxis;
    let flat_edge = surface(1, vec![0., 0., 1., 1.], &[0., 1.]);
    let options = SurfacePlaneGraphSampleOptions {
        span: [1, 1],
        solve_axis: UvAxis::U,
        max_normalized_width: 1e-12,
        max_work: WORK,
    };
    let report =
        sample_surface_plane_graph(&flat_edge, [0., 0., 1.], 0., &[q(0), q(1)], options).unwrap();
    assert!(
        report
            .outcome
            .unwrap()
            .samples
            .iter()
            .all(|sample| sample.root == [q(0), q(0)])
    );

    let folded = surface(2, vec![0., 0., 0., 1., 1., 1.], &[1., -3., 1.]);
    let report = sample_surface_plane_graph(
        &folded,
        [0., 0., 1.],
        0.,
        &[q(0)],
        SurfacePlaneGraphSampleOptions {
            span: [2, 1],
            ..options
        },
    )
    .unwrap();
    assert_eq!(report.outcome, Err(Refusal::UnprovedIdentity));
}

#[test]
fn certifies_continuous_chord_error_for_a_parabolic_branch() {
    use nurbs_core::editing::exact_projective::UvAxis;
    let surface = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 0.5, 0.], vec![0., 1., -1.]],
            vec![vec![1., 0., 1.], vec![1., 0.5, 1.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let options = SurfacePlanePolylineOptions {
        span: [1, 2],
        solve_axis: UvAxis::U,
        max_normalized_error: 1. / 64.,
        max_segments: 16,
        max_work: WORK,
    };
    let report = approximate_surface_plane_graph(&surface, [0., 0., 1.], 0., options).unwrap();
    let result = report.outcome.unwrap();
    assert_eq!(result.uv.len(), 9);
    assert_eq!(result.normalized_error_upper, Q::new(1.into(), 256.into()));
    for edge in result.uv.windows(2) {
        let mid_u = (&edge[0][0] + &edge[1][0]) / q(2);
        let mid_v = (&edge[0][1] + &edge[1][1]) / q(2);
        assert_eq!(mid_u - mid_v.pow(2), result.normalized_error_upper);
    }
    let limited = approximate_surface_plane_graph(
        &surface,
        [0., 0., 1.],
        0.,
        SurfacePlanePolylineOptions {
            max_segments: 2,
            ..options
        },
    )
    .unwrap();
    assert_eq!(limited.outcome, Err(Refusal::ResourceLimit));
}

#[test]
fn represents_a_straight_graph_exactly_with_one_segment() {
    use nurbs_core::editing::exact_projective::UvAxis;
    let mut diagonal = surface(1, vec![0., 0., 1., 1.], &[0., -1.]);
    diagonal.weights = vec![vec![1.; 2]; 2];
    diagonal.control_points[0][1][2] = 1.;
    diagonal.control_points[1][1][2] = 0.;
    let options = SurfacePlanePolylineOptions {
        span: [1, 1],
        solve_axis: UvAxis::V,
        max_normalized_error: 1e-20,
        max_segments: 1,
        max_work: WORK,
    };
    let report = approximate_surface_plane_graph(&diagonal, [0., 0., 1.], 0., options).unwrap();
    let result = report.outcome.unwrap();
    assert_eq!(result.uv, [[q(0), q(0)], [q(1), q(1)]]);
    assert_eq!(result.normalized_error_upper, q(0));
}

#[test]
fn bounds_the_xyz_chord_error_on_a_curved_surface_plane_section() {
    use nurbs_core::editing::exact_projective::UvAxis;
    let surface = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 0.5, 0.], vec![0., 1., -1.]],
            vec![vec![1., 0., 1.], vec![1., 0.5, 1.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let options = SurfacePlaneWorldPolylineOptions {
        span: [1, 2],
        solve_axis: UvAxis::U,
        max_xyz_error: 1. / 64.,
        max_segments: 32,
        max_work: WORK,
    };
    let report = approximate_surface_plane_graph_xyz(&surface, [0., 0., 1.], 0., options).unwrap();
    let result = report.outcome.unwrap();
    assert!(result.xyz.len() > 2);
    assert!(result.xyz_error_upper <= Q::from_float(options.max_xyz_error).unwrap());
    for edge in result.xyz.windows(2) {
        let mid: [Q; 3] = std::array::from_fn(|i| (&edge[0][i] + &edge[1][i]) / q(2));
        let squared_error = (&mid[0] - mid[1].pow(2)).pow(2) + mid[2].pow(2);
        assert!(squared_error <= result.xyz_error_upper.pow(2));
    }
    let limited = approximate_surface_plane_graph_xyz(
        &surface,
        [0., 0., 1.],
        0.,
        SurfacePlaneWorldPolylineOptions {
            max_work: report.work_used - 1,
            ..options
        },
    )
    .unwrap();
    assert_eq!(limited.outcome, Err(Refusal::ResourceLimit));
}

#[test]
fn preserves_an_affine_xyz_section_as_one_exact_segment() {
    use nurbs_core::editing::exact_projective::UvAxis;
    let mut patch = surface(1, vec![0., 0., 1., 1.], &[-1., 1.]);
    patch.weights = vec![vec![1.; 2]; 2];
    let options = SurfacePlaneWorldPolylineOptions {
        span: [1, 1],
        solve_axis: UvAxis::U,
        max_xyz_error: 1e-22,
        max_segments: 1,
        max_work: WORK,
    };
    let result = approximate_surface_plane_graph_xyz(&patch, [0., 0., -1.], 0., options)
        .unwrap()
        .outcome
        .unwrap();
    let half = Q::new(1.into(), 2.into());
    assert_eq!(result.xyz, [[half.clone(), q(0), q(0)], [half, q(1), q(0)]]);
    assert_eq!(result.xyz_error_upper, q(0));
}

#[test]
fn stores_a_nonbinary_section_and_refuses_a_tolerance_below_rounding_error() {
    use nurbs_core::editing::exact_projective::UvAxis;
    let mut patch = surface(1, vec![0., 0., 1., 1.], &[-1., 2.]);
    patch.weights = vec![vec![1.; 2]; 2];
    let options = SurfacePlaneWorldPolylineOptions {
        span: [1, 1],
        solve_axis: UvAxis::U,
        max_xyz_error: 1e-12,
        max_segments: 1,
        max_work: WORK,
    };
    let stored = store_surface_plane_section_curve(&patch, [0., 0., 1.], 0., options)
        .unwrap()
        .outcome
        .unwrap();
    assert_eq!(stored.curve.degree, 1);
    assert_eq!(stored.curve.domain(), [0., 1.]);
    assert_eq!(stored.plane_residual_upper, q(0));
    let exact = Q::new(1.into(), 3.into());
    assert!(stored.curve.control_points.iter().all(|p| {
        let delta = Q::from_float(p[0]).unwrap() - &exact;
        delta.pow(2) <= stored.xyz_error_upper.pow(2) && p[2] == 0.
    }));
    let impossible = store_surface_plane_section_curve(
        &patch,
        [0., 0., 1.],
        0.,
        SurfacePlaneWorldPolylineOptions {
            max_xyz_error: 1e-20,
            ..options
        },
    )
    .unwrap();
    assert!(matches!(impossible.outcome, Err(Refusal::NotRepresentable)));
}

#[test]
fn projects_all_controls_to_the_plane_and_keeps_exact_integer_knots() {
    use nurbs_core::editing::exact_projective::UvAxis;
    let patch = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 0.5, 0.], vec![0., 1., -1.]],
            vec![vec![1., 0., 1.], vec![1., 0.5, 1.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let options = SurfacePlaneWorldPolylineOptions {
        span: [1, 2],
        solve_axis: UvAxis::U,
        max_xyz_error: 1. / 64.,
        max_segments: 32,
        max_work: WORK,
    };
    let report = store_surface_plane_section_curve(&patch, [0., 0., 1.], 0., options).unwrap();
    let stored = report.outcome.unwrap();
    let segments = stored.curve.control_points.len() - 1;
    assert!(segments > 1);
    assert_eq!(stored.curve.domain(), [0., segments as f64]);
    assert!(stored.curve.knots.iter().all(|k| k.fract() == 0.));
    assert_eq!(stored.plane_residual_upper, q(0));
    assert!(stored.xyz_error_upper <= Q::from_float(options.max_xyz_error).unwrap());
    let limited = store_surface_plane_section_curve(
        &patch,
        [0., 0., 1.],
        0.,
        SurfacePlaneWorldPolylineOptions {
            max_work: report.work_used - 1,
            ..options
        },
    )
    .unwrap();
    assert!(matches!(limited.outcome, Err(Refusal::ResourceLimit)));
}

#[test]
fn stores_an_exact_affine_pair_in_the_original_uv_clock() {
    use nurbs_core::editing::exact_projective::UvAxis;
    let mut patch = surface(1, vec![-2., -2., 4., 4.], &[-1., 1.]);
    patch.weights = vec![vec![1.; 2]; 2];
    let options = SurfacePlaneWorldPolylineOptions {
        span: [1, 1],
        solve_axis: UvAxis::U,
        max_xyz_error: 1e-20,
        max_segments: 1,
        max_work: WORK,
    };
    let report = store_surface_plane_section_pair(&patch, [0., 0., 1.], 0., options, 0.).unwrap();
    let pair = report.outcome.unwrap();
    assert_eq!(pair.pcurve.knots, pair.spatial.curve.knots);
    assert_eq!(pair.pcurve.control_points, [vec![1., 0.], vec![1., 1.]]);
    assert_eq!(
        pair.spatial.curve.control_points,
        [vec![0.5, 0., 0.], vec![0.5, 1., 0.]]
    );
    assert_eq!(pair.correspondence_tolerance, q(0));
    let exhausted = store_surface_plane_section_pair(
        &patch,
        [0., 0., 1.],
        0.,
        SurfacePlaneWorldPolylineOptions {
            max_work: report.work_used - 1,
            ..options
        },
        0.,
    )
    .unwrap();
    assert!(matches!(exhausted.outcome, Err(Refusal::ResourceLimit)));
}

#[test]
fn verifies_the_lifted_gap_of_a_curved_pair_between_vertices() {
    use nurbs_core::editing::exact_projective::UvAxis;
    let patch = Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 0.5, 0.], vec![0., 1., -1.]],
            vec![vec![1., 0., 1.], vec![1., 0.5, 1.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let options = SurfacePlaneWorldPolylineOptions {
        span: [1, 2],
        solve_axis: UvAxis::U,
        max_xyz_error: 1. / 64.,
        max_segments: 32,
        max_work: WORK,
    };
    let tolerance = 1. / 64.;
    let pair = store_surface_plane_section_pair(&patch, [0., 0., 1.], 0., options, tolerance)
        .unwrap()
        .outcome
        .unwrap();
    assert_eq!(pair.pcurve.knots, pair.spatial.curve.knots);
    for edge in pair.pcurve.control_points.windows(2) {
        let u = (edge[0][0] + edge[1][0]) / 2.;
        let v = (edge[0][1] + edge[1][1]) / 2.;
        assert!((u - v * v).abs() <= tolerance);
    }
    let exact = store_surface_plane_section_pair(&patch, [0., 0., 1.], 0., options, 0.).unwrap();
    assert!(matches!(exact.outcome, Err(Refusal::UnprovedIdentity)));
}

#[test]
fn splits_an_original_cell_into_closed_complementary_sign_regions() {
    use nurbs_core::{
        editing::exact_projective::UvAxis,
        trim::trim_domain::{Location, TrimDomain, exact_loop_joins},
    };
    let mut patch = surface(1, vec![-2., -2., 4., 4.], &[-1., 1.]);
    patch.weights = vec![vec![1.; 2]; 2];
    let options = SurfacePlaneWorldPolylineOptions {
        span: [1, 1],
        solve_axis: UvAxis::U,
        max_xyz_error: 1e-12,
        max_segments: 1,
        max_work: WORK,
    };
    let report = split_surface_cell_by_plane(&patch, [0., 0., 1.], 0., options, 0.).unwrap();
    let split = report.outcome.unwrap();
    for (loops, x, other) in [(&split.negative, 0., 2.), (&split.positive, 2., 0.)] {
        assert_eq!(exact_loop_joins(loops).unwrap(), Some(true));
        let domain = TrimDomain::new(&[loops.to_vec()], 1e-8).unwrap();
        let inside = domain.classify_point([x, 0.5], 100).unwrap();
        assert_eq!(inside.location, Location::Inside);
        assert_eq!(inside.winding, Some(1));
        assert_eq!(
            domain.classify_point([other, 0.5], 100).unwrap().location,
            Location::Outside
        );
    }
    assert_eq!(
        split.negative[0].control_points,
        split.section.pcurve.control_points
    );
    let reversed = split_surface_cell_by_plane(&patch, [0., 0., -1.], 0., options, 0.)
        .unwrap()
        .outcome
        .unwrap();
    assert_eq!(
        reversed.negative[0].control_points,
        split.positive[0].control_points
    );
    assert_eq!(
        reversed.positive[0].control_points,
        split.negative[0].control_points
    );
    let limited = split_surface_cell_by_plane(
        &patch,
        [0., 0., 1.],
        0.,
        SurfacePlaneWorldPolylineOptions {
            max_work: report.work_used - 1,
            ..options
        },
        0.,
    )
    .unwrap();
    assert!(matches!(limited.outcome, Err(Refusal::ResourceLimit)));
}

#[test]
fn closes_corner_to_corner_trims_and_rejects_a_boundary_only_section() {
    use nurbs_core::{
        editing::exact_projective::UvAxis,
        trim::trim_domain::{Location, TrimDomain, exact_loop_joins},
    };
    let mut diagonal = surface(1, vec![0., 0., 1., 1.], &[0., -1.]);
    diagonal.weights = vec![vec![1.; 2]; 2];
    diagonal.control_points[0][1][2] = 1.;
    diagonal.control_points[1][1][2] = 0.;
    let options = SurfacePlaneWorldPolylineOptions {
        span: [1, 1],
        solve_axis: UvAxis::V,
        max_xyz_error: 1e-12,
        max_segments: 1,
        max_work: WORK,
    };
    let split = split_surface_cell_by_plane(&diagonal, [0., 0., 1.], 0., options, 0.)
        .unwrap()
        .outcome
        .unwrap();
    for (loops, point) in [
        (&split.negative, [0.75, 0.25]),
        (&split.positive, [0.25, 0.75]),
    ] {
        assert_eq!(exact_loop_joins(loops).unwrap(), Some(true));
        assert_eq!(loops[1].control_points.len(), 3);
        let d = TrimDomain::new(&[loops.to_vec()], 1e-8)
            .unwrap()
            .classify_point(point, 100)
            .unwrap();
        assert_eq!(d.location, Location::Inside);
        assert_eq!(d.winding, Some(1));
    }
    let mut edge = surface(1, vec![0., 0., 1., 1.], &[0., 1.]);
    edge.weights = vec![vec![1.; 2]; 2];
    let report = split_surface_cell_by_plane(
        &edge,
        [0., 0., 1.],
        0.,
        SurfacePlaneWorldPolylineOptions {
            solve_axis: UvAxis::U,
            ..options
        },
        1e-6,
    )
    .unwrap();
    assert!(matches!(report.outcome, Err(Refusal::UnprovedIdentity)));
}
