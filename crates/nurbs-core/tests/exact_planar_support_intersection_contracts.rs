use num_rational::BigRational as Q;
use nurbs_core::{
    analysis::{
        exact_affine_hull::{
            PlanarProjectionOutcome, PlanarSurfaceProjection, project_planar_surface,
        },
        exact_plane_intersection::{
            AdaptiveBoundaryOptions, AlgebraicBoundaryClock, AlgebraicPlanarContacts,
            BoundaryResolution, BoundedPlanarBoundaryIntersection, ClassifiedPlanarSection,
            ExactPlanarIntersectionSegment, PlanarBoundaryIntersection, PlanarFaceContactPartition,
            PlanarIntersectionSegments, PlanarSectionLength, PlanarSectionMaterial,
            PlanarSectionOutcome, PlanarSectionWindingOptions, PlanarSegmentStorage,
            StoredPlanarIntersection, SupportPlaneIntersection, TrimmedPlanarIntersection,
            assemble_quadratic_planar_section_material, bound_planar_face_boundary_intersection,
            build_planar_intersection_segments, classify_planar_face_contacts,
            classify_quadratic_planar_section, enclose_algebraic_planar_boundary_point,
            intersect_planar_face_boundaries, intersect_planar_face_boundary_contacts,
            intersect_planar_supports, intersect_trimmed_planar_faces,
            isolate_ordered_algebraic_boundary_clocks, match_quadratic_planar_boundary_contacts,
            measure_quadratic_planar_section_material,
            optimize_planar_intersection_segment_storage, order_algebraic_boundary_clocks,
            partition_quadratic_planar_section, refine_planar_face_boundary_extents,
            resolve_planar_face_boundary_intersection, store_planar_intersection_segment,
            store_trimmed_planar_intersection,
        },
        planar_coplanar_ray::CoplanarRayPartitionOptions,
        planar_trim_projection::ProjectedTrimmedFace,
    },
    curve::Curve,
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn plane(z: impl Fn(f64, f64) -> f64) -> PlanarSurfaceProjection {
    let surface = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., -1., 1., 1.],
        knots_v: vec![-1., -1., 1., 1.],
        control_points: [-1., 1.]
            .into_iter()
            .map(|x| [-1., 1.].into_iter().map(|y| vec![x, y, z(x, y)]).collect())
            .collect(),
        weights: vec![vec![1., 2.], vec![4., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    let report = project_planar_surface(&surface, WORK).unwrap();
    let PlanarProjectionOutcome::Projected(chart) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    chart
}

fn q(n: i64, d: i64) -> Q {
    Q::new(n.into(), d.into())
}

#[test]
fn keeps_a_nonbinary_line_clock_identical_under_input_swap() {
    let a = plane(|x, y| x + y);
    let b = plane(|x, y| 2. * x - y + 1.);
    let report = intersect_planar_supports(&a, &b, WORK).unwrap();
    let SupportPlaneIntersection::Line(line) = report.outcome else {
        panic!("{:?}", report.outcome)
    };
    assert_eq!(line.clock_axis, 2);
    assert_eq!(line.origin, [q(-1, 3), q(1, 3), q(0, 1)]);
    assert_eq!(line.direction, [q(2, 3), q(1, 3), q(1, 1)]);
    assert_eq!(
        intersect_planar_supports(&b, &a, WORK).unwrap().outcome,
        SupportPlaneIntersection::Line(line)
    );
}

#[test]
fn distinguishes_coincident_and_parallel_supports_even_with_different_chart_axes() {
    let a = plane(|x, y| x + y);
    let mut b = a.clone();
    // Same plane, this time omitting z rather than x.
    b.omitted_axis = 2;
    b.retained_axes = [0, 1];
    b.offset = q(0, 1);
    b.slopes = [q(1, 1), q(1, 1)];
    assert_eq!(
        intersect_planar_supports(&a, &b, WORK).unwrap().outcome,
        SupportPlaneIntersection::Coincident
    );
    b.offset = q(1, 3);
    assert_eq!(
        intersect_planar_supports(&a, &b, WORK).unwrap().outcome,
        SupportPlaneIntersection::Parallel
    );
    let limited = intersect_planar_supports(&a, &b, 1).unwrap();
    assert_eq!(limited.outcome, SupportPlaneIntersection::ResourceLimit);
    assert!(limited.work_used <= 1);
}

fn rectangle(x: [f64; 2], y: [f64; 2], hole: bool) -> Vec<Curve> {
    let [a, b] = x;
    let [c, d] = y;
    let mut control_points = vec![vec![a, c], vec![b, c], vec![b, d], vec![a, d], vec![a, c]];
    if hole {
        control_points.reverse();
    }
    vec![Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 3., 4., 4.],
        control_points,
        weights: vec![1., 2., 3., 1., 1.],
        periodic: false,
    }]
}

fn options(cells: usize) -> CoplanarRayPartitionOptions {
    CoplanarRayPartitionOptions {
        max_curve_parameter_width: 1e-9,
        winding_tolerance: 1e-7,
        max_winding_cells: cells,
        max_work: WORK,
    }
}

#[test]
fn clips_both_signs_of_a_nonbinary_line_and_preserves_holes() {
    let a = ProjectedTrimmedFace {
        projection: plane(|x, y| x + y),
        loops: vec![
            rectangle([-2., 2.], [-2., 2.], false),
            rectangle([-0.5, 0.5], [-0.5, 0.5], true),
        ],
    };
    let b = ProjectedTrimmedFace {
        projection: plane(|x, y| 2. * x - y + 1.),
        loops: vec![rectangle([-1.5, 1.5], [-1.5, 1.5], false)],
    };
    let full = intersect_trimmed_planar_faces(&a, &b, options(10_000)).unwrap();
    let TrimmedPlanarIntersection::Transverse {
        line,
        shared_interior,
        possible,
    } = &full.outcome
    else {
        panic!("{:?}", full.outcome);
    };
    assert_eq!(line.origin, [q(-1, 3), q(1, 3), q(0, 1)]);
    assert_eq!(shared_interior.len(), 2);
    assert_eq!(possible.len(), 2);
    assert!(shared_interior[0][0] < q(-1, 1) && q(-1, 1) < shared_interior[0][1]);
    assert!(shared_interior[1][0] < q(1, 1) && q(1, 1) < shared_interior[1][1]);
    assert!(possible[0][1] < q(0, 1) && q(0, 1) < possible[1][0]);
    assert_eq!(
        intersect_trimmed_planar_faces(&b, &a, options(10_000))
            .unwrap()
            .outcome,
        full.outcome
    );

    let limited = intersect_trimmed_planar_faces(&a, &b, options(1)).unwrap();
    assert!(limited.winding_cells <= 1);
    let TrimmedPlanarIntersection::Transverse {
        shared_interior,
        possible,
        ..
    } = limited.outcome
    else {
        panic!("{:?}", limited.outcome);
    };
    assert!(shared_interior.is_empty());
    assert!(
        possible
            .iter()
            .any(|band| band[0] < q(0, 1) && q(0, 1) < band[1])
    );
}

#[test]
fn preserves_a_boundary_only_contact_and_proves_disjoint_trims() {
    let z0 = plane(|_, _| 0.);
    let mut y0 = z0.clone();
    y0.retained_axes = [0, 2];
    y0.omitted_axis = 1;
    let a = ProjectedTrimmedFace {
        projection: z0,
        loops: vec![rectangle([-2., 0.], [-1., 1.], false)],
    };
    let mut b = ProjectedTrimmedFace {
        projection: y0,
        loops: vec![rectangle([0., 2.], [-1., 1.], false)],
    };
    let result = intersect_trimmed_planar_faces(&a, &b, options(10_000)).unwrap();
    let TrimmedPlanarIntersection::Transverse {
        shared_interior,
        possible,
        ..
    } = result.outcome
    else {
        panic!("{:?}", result.outcome);
    };
    assert!(shared_interior.is_empty());
    assert_eq!(possible.len(), 1);
    assert!(possible[0][0] <= q(0, 1) && q(0, 1) <= possible[0][1]);
    assert!(possible[0][1].clone() - &possible[0][0] < q(1, 1_000_000));

    b.loops = vec![rectangle([1., 2.], [-1., 1.], false)];
    let result = intersect_trimmed_planar_faces(&a, &b, options(10_000)).unwrap();
    assert!(
        matches!(result.outcome, TrimmedPlanarIntersection::Transverse { shared_interior, possible, .. }
        if shared_interior.is_empty() && possible.is_empty())
    );
}

#[test]
fn builds_matching_spatial_and_chart_segments_without_rounding() {
    let a = ProjectedTrimmedFace {
        projection: plane(|x, y| x + y),
        loops: vec![rectangle([-2., 2.], [-2., 2.], false)],
    };
    let b = ProjectedTrimmedFace {
        projection: plane(|x, y| 2. * x - y + 1.),
        loops: vec![rectangle([-1.5, 1.5], [-1.5, 1.5], false)],
    };
    let clipped = intersect_trimmed_planar_faces(&a, &b, options(10_000)).unwrap();
    let report = build_planar_intersection_segments(&a, &b, options(10_000)).unwrap();
    let PlanarIntersectionSegments::Transverse {
        line,
        segments,
        possible,
    } = report.outcome
    else {
        panic!("{:?}", report.outcome);
    };
    let TrimmedPlanarIntersection::Transverse {
        shared_interior,
        possible: expected_possible,
        ..
    } = clipped.outcome
    else {
        panic!("{:?}", clipped.outcome);
    };
    assert_eq!(possible, expected_possible);
    assert_eq!(segments.len(), 1);
    let segment = &segments[0];
    assert_eq!(segment.parameter, shared_interior[0]);
    assert_eq!(
        segment.xyz.clone().map(|p| p[line.clock_axis].clone()),
        segment.parameter
    );
    // Interpolate at the canonical clock zero, where neither x nor y is binary.
    let s = -&segment.parameter[0] / (&segment.parameter[1] - &segment.parameter[0]);
    assert!(q(0, 1) < s && s < q(1, 1));
    let xyz = std::array::from_fn(|i| {
        &segment.xyz[0][i] + &s * (&segment.xyz[1][i] - &segment.xyz[0][i])
    });
    assert_eq!(xyz, [q(-1, 3), q(1, 3), q(0, 1)]);
    for (i, face) in [&a, &b].into_iter().enumerate() {
        let chart = &face.projection;
        let uv: [Q; 2] = std::array::from_fn(|k| {
            let [start, end] = &segment.pcurves[i];
            &start[k] + &s * (&end[k] - &start[k])
        });
        assert_eq!(uv, chart.retained_axes.map(|axis| xyz[axis].clone()));
        assert_eq!(
            &chart.offset + &chart.slopes[0] * &uv[0] + &chart.slopes[1] * &uv[1],
            xyz[chart.omitted_axis]
        );
    }

    let mut bounded = options(10_000);
    bounded.max_work = clipped.work_used;
    let limited = build_planar_intersection_segments(&a, &b, bounded).unwrap();
    assert_eq!(
        limited.outcome,
        PlanarIntersectionSegments::Refused(
            nurbs_core::editing::exact_derivative::Refusal::ResourceLimit
        )
    );
    assert!(limited.work_used <= bounded.max_work);
}

#[test]
fn unresolved_contacts_do_not_generate_spatial_segments() {
    let a = ProjectedTrimmedFace {
        projection: plane(|_, _| 0.),
        loops: vec![rectangle([-1., 1.], [0., 1.], false)],
    };
    let mut b = a.clone();
    b.projection.retained_axes = [0, 2];
    b.projection.omitted_axis = 1;
    b.loops = vec![rectangle([-0.5, 0.5], [-1., 1.], false)];
    let report = build_planar_intersection_segments(&a, &b, options(10_000)).unwrap();
    let PlanarIntersectionSegments::Transverse {
        segments, possible, ..
    } = report.outcome
    else {
        panic!("{:?}", report.outcome);
    };
    assert!(segments.is_empty());
    assert!(
        possible
            .iter()
            .any(|band| band[0] <= q(-1, 2) && band[1] >= q(1, 2))
    );
}

fn exact_segment(
    xyz: [[Q; 3]; 2],
    charts: [&PlanarSurfaceProjection; 2],
) -> ExactPlanarIntersectionSegment {
    ExactPlanarIntersectionSegment {
        parameter: xyz.clone().map(|p| p[2].clone()),
        pcurves: charts.map(|chart| {
            [&xyz[0], &xyz[1]].map(|p| chart.retained_axes.map(|axis| p[axis].clone()))
        }),
        xyz,
    }
}

#[test]
fn stores_dyadic_segments_with_zero_error_and_shared_clock() {
    let a = plane(|x, _| x);
    let b = plane(|_, y| y);
    let charts = [&a, &b];
    let segment = exact_segment(
        [[q(0, 1), q(0, 1), q(0, 1)], [q(1, 2), q(1, 2), q(1, 2)]],
        charts,
    );
    let report = store_planar_intersection_segment(&segment, charts, 0., WORK).unwrap();
    let PlanarSegmentStorage::Stored {
        spatial,
        pcurves,
        squared_error,
    } = report.outcome
    else {
        panic!("{:?}", report.outcome);
    };
    assert_eq!(spatial.control_points, vec![vec![0.; 3], vec![0.5; 3]]);
    for pcurve in pcurves {
        assert_eq!(pcurve.knots, spatial.knots);
        assert_eq!(pcurve.control_points, vec![vec![0.; 2], vec![0.5; 2]]);
    }
    assert_eq!(squared_error.spatial, q(0, 1));
    assert_eq!(squared_error.lifted_pcurves, [q(0, 1), q(0, 1)]);
    assert_eq!(squared_error.correspondence, [q(0, 1), q(0, 1)]);
}

#[test]
fn qualifies_nonbinary_storage_against_exact_lifted_separation() {
    let a = plane(|x, y| x + y);
    let b = plane(|x, y| 2. * x - y + 1.);
    let charts = [&a, &b];
    let segment = exact_segment(
        [[q(-1, 3), q(1, 3), q(0, 1)], [q(1, 3), q(2, 3), q(1, 1)]],
        charts,
    );
    let exact = store_planar_intersection_segment(&segment, charts, 0., WORK).unwrap();
    let PlanarSegmentStorage::ToleranceExceeded(expected) = exact.outcome else {
        panic!("{:?}", exact.outcome);
    };
    let report = store_planar_intersection_segment(&segment, charts, 1e-14, WORK).unwrap();
    let PlanarSegmentStorage::Stored {
        spatial,
        pcurves,
        squared_error,
    } = report.outcome
    else {
        panic!("{:?}", report.outcome);
    };
    assert_eq!(squared_error, expected);
    assert!(squared_error.spatial > q(0, 1));
    assert!(squared_error.correspondence[1] > q(0, 1));
    for (i, chart) in charts.into_iter().enumerate() {
        for end in 0..2 {
            let xyz = &spatial.control_points[end];
            let [u, v] = chart.retained_axes.map(|axis| xyz[axis]);
            assert_eq!(pcurves[i].control_points[end], vec![u, v]);
            let residual = &chart.offset
                + &chart.slopes[0] * Q::from_float(u).unwrap()
                + &chart.slopes[1] * Q::from_float(v).unwrap()
                - Q::from_float(xyz[chart.omitted_axis]).unwrap();
            assert!(&residual * &residual <= squared_error.correspondence[i]);
        }
    }
}

#[test]
fn refuses_inconsistent_or_collapsed_nets_and_bounds_work() {
    use nurbs_core::editing::exact_derivative::Refusal;
    let a = plane(|x, _| x);
    let b = plane(|_, y| y);
    let charts = [&a, &b];
    let point = q(1, 1) + q(1, 1 << 54);
    let mut segment = exact_segment(
        [
            [q(1, 1), q(1, 1), q(1, 1)],
            [point.clone(), point.clone(), point],
        ],
        charts,
    );
    let collapsed = store_planar_intersection_segment(&segment, charts, 1e-14, WORK).unwrap();
    assert!(matches!(
        collapsed.outcome,
        PlanarSegmentStorage::Refused(Refusal::InvalidOutput)
    ));
    segment.pcurves[1][0][0] = q(0, 1);
    let inconsistent = store_planar_intersection_segment(&segment, charts, 1e-14, WORK).unwrap();
    assert!(matches!(
        inconsistent.outcome,
        PlanarSegmentStorage::Refused(Refusal::UnprovedIdentity)
    ));
    let limited = store_planar_intersection_segment(&segment, charts, 1e-14, 1).unwrap();
    assert!(matches!(
        limited.outcome,
        PlanarSegmentStorage::Refused(Refusal::ResourceLimit)
    ));
    assert!(limited.work_used <= 1);
}

#[test]
fn finds_coordinated_rounding_when_nearest_storage_exceeds_tolerance() {
    let a = plane(|x, y| x + y);
    let b = plane(|x, y| 2. * x - y + 1.);
    let charts = [&a, &b];
    let segment = exact_segment(
        [[q(-2, 9), q(7, 18), q(1, 6)], [q(0, 1), q(1, 2), q(1, 2)]],
        charts,
    );
    let tolerance = 2.1e-17;
    let nearest = store_planar_intersection_segment(&segment, charts, tolerance, WORK).unwrap();
    assert!(matches!(
        nearest.outcome,
        PlanarSegmentStorage::ToleranceExceeded(_)
    ));
    let optimized =
        optimize_planar_intersection_segment_storage(&segment, charts, tolerance, WORK).unwrap();
    let PlanarSegmentStorage::Stored {
        spatial,
        pcurves,
        squared_error,
    } = optimized.outcome
    else {
        panic!("{:?}", optimized.outcome);
    };
    assert_eq!(spatial.control_points[1], vec![0., 0.5, 0.5]);
    let limit = Q::from_float(tolerance).unwrap().pow(2);
    for error in std::iter::once(&squared_error.spatial)
        .chain(&squared_error.lifted_pcurves)
        .chain(&squared_error.correspondence)
    {
        assert!(error <= &limit);
    }
    let swapped_segment = exact_segment(segment.xyz.clone(), [&b, &a]);
    let swapped =
        optimize_planar_intersection_segment_storage(&swapped_segment, [&b, &a], tolerance, WORK)
            .unwrap();
    let PlanarSegmentStorage::Stored {
        spatial: swapped_xyz,
        pcurves: swapped_uv,
        ..
    } = swapped.outcome
    else {
        panic!("{:?}", swapped.outcome);
    };
    assert_eq!(swapped_xyz.control_points, spatial.control_points);
    assert_eq!(swapped_uv[0].control_points, pcurves[1].control_points);
    assert_eq!(swapped_uv[1].control_points, pcurves[0].control_points);
}

#[test]
fn adjacent_search_preserves_nonzero_segments_and_respects_budget() {
    use nurbs_core::editing::exact_derivative::Refusal;
    let a = plane(|x, _| x);
    let b = plane(|_, y| y);
    let charts = [&a, &b];
    let point = q(1, 1) + q(1, 1 << 54);
    let segment = exact_segment(
        [
            [q(1, 1), q(1, 1), q(1, 1)],
            [point.clone(), point.clone(), point],
        ],
        charts,
    );
    let optimized =
        optimize_planar_intersection_segment_storage(&segment, charts, 1e-14, WORK).unwrap();
    let PlanarSegmentStorage::Stored {
        spatial, pcurves, ..
    } = optimized.outcome
    else {
        panic!("{:?}", optimized.outcome);
    };
    assert!(
        spatial.control_points[0]
            .iter()
            .zip(&spatial.control_points[1])
            .all(|(a, b)| a <= b)
    );
    for pcurve in pcurves {
        assert_ne!(pcurve.control_points[0], pcurve.control_points[1]);
    }
    let limited = optimize_planar_intersection_segment_storage(
        &segment,
        charts,
        1e-14,
        optimized.work_used - 1,
    )
    .unwrap();
    assert!(matches!(
        limited.outcome,
        PlanarSegmentStorage::Refused(Refusal::ResourceLimit)
    ));
    assert!(limited.work_used < optimized.work_used);
}

#[test]
fn stores_all_split_intersection_edges_with_strict_chart_gaps() {
    let a = ProjectedTrimmedFace {
        projection: plane(|x, y| x + y),
        loops: vec![
            rectangle([-2., 2.], [-2., 2.], false),
            rectangle([-0.5, 0.5], [-0.5, 0.5], true),
        ],
    };
    let b = ProjectedTrimmedFace {
        projection: plane(|x, y| 2. * x - y + 1.),
        loops: vec![rectangle([-1.5, 1.5], [-1.5, 1.5], false)],
    };
    let opts = options(10_000);
    let exact = build_planar_intersection_segments(&a, &b, opts).unwrap();
    let stored = store_trimmed_planar_intersection(&a, &b, opts, 1e-14).unwrap();
    let StoredPlanarIntersection::Transverse {
        line,
        edges,
        possible,
    } = stored.outcome
    else {
        panic!("{:?}", stored.outcome);
    };
    let PlanarIntersectionSegments::Transverse {
        segments,
        possible: expected,
        ..
    } = exact.outcome
    else {
        panic!("{:?}", exact.outcome);
    };
    assert_eq!(possible, expected);
    assert_eq!(edges.len(), 2);
    for (edge, segment) in edges.iter().zip(&segments) {
        assert_eq!(edge.parameter, segment.parameter);
        assert_eq!(edge.spatial.degree, 1);
    }
    assert!(
        edges[0].spatial.control_points[1][line.clock_axis]
            < edges[1].spatial.control_points[0][line.clock_axis]
    );
    // This fixture's canonical direction has all three positive components.
    for i in 0..2 {
        assert!(
            edges[0].pcurves[i].control_points[1]
                .iter()
                .zip(&edges[1].pcurves[i].control_points[0])
                .any(|(a, b)| a < b)
        );
    }
    let mut limited_options = opts;
    limited_options.max_work = exact.work_used;
    let limited = store_trimmed_planar_intersection(&a, &b, limited_options, 1e-14).unwrap();
    assert!(matches!(
        limited.outcome,
        StoredPlanarIntersection::Refused(
            nurbs_core::editing::exact_derivative::Refusal::ResourceLimit
        )
    ));
    assert!(limited.work_used <= exact.work_used);

    let no_approximation = store_trimmed_planar_intersection(&a, &b, opts, 0.).unwrap();
    assert!(matches!(
        no_approximation.outcome,
        StoredPlanarIntersection::NoAdmissibleStorage
    ));
}

fn perpendicular_boundary_faces() -> [ProjectedTrimmedFace; 2] {
    let z = plane(|_, _| 0.);
    let mut y = z.clone();
    y.retained_axes = [0, 2];
    y.omitted_axis = 1;
    [
        ProjectedTrimmedFace {
            projection: z,
            loops: vec![rectangle([-2., 2.], [0., 1.], false)],
        },
        ProjectedTrimmedFace {
            projection: y,
            loops: vec![rectangle([0., 1.], [0., 1.], true)],
        },
    ]
}

#[test]
fn proves_a_nonbinary_endpoint_of_a_coincident_rational_span() {
    let [mut a, b] = perpendicular_boundary_faces();
    a.loops = vec![vec![
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 0.5, 1., 1., 1.],
            control_points: vec![vec![-2., 0.], vec![-1., 0.], vec![1., 0.], vec![2., 1.]],
            weights: vec![1., 1., 2., 1.],
            periodic: false,
        },
        Curve {
            degree: 1,
            knots: vec![0., 0., 0.5, 1., 1.],
            control_points: vec![vec![2., 1.], vec![-2., 1.], vec![-2., 0.]],
            weights: vec![1.; 3],
            periodic: false,
        },
    ]];
    let result = intersect_planar_face_boundaries(&a, &b, WORK).unwrap();
    let PlanarBoundaryIntersection::Transverse {
        line,
        shared_boundary,
    } = &result.outcome
    else {
        panic!("{:?}", result.outcome);
    };
    assert_eq!(line.clock_axis, 0);
    assert_eq!(shared_boundary, &[[q(0, 1), q(1, 3)]]);
    assert_eq!(
        result.outcome,
        intersect_planar_face_boundaries(&b, &a, WORK)
            .unwrap()
            .outcome
    );
}

#[test]
fn retains_singleton_shared_boundary_and_refuses_under_budget_exhaustion() {
    let [mut a, b] = perpendicular_boundary_faces();
    a.loops = vec![rectangle([-2., 0.], [0., 1.], false)];
    let full = intersect_planar_face_boundaries(&a, &b, WORK).unwrap();
    let PlanarBoundaryIntersection::Transverse {
        shared_boundary, ..
    } = full.outcome
    else {
        panic!("{:?}", full.outcome);
    };
    assert_eq!(shared_boundary, [[q(0, 1), q(0, 1)]]);
    let limited = intersect_planar_face_boundaries(&a, &b, full.work_used - 1).unwrap();
    assert_eq!(
        limited.outcome,
        PlanarBoundaryIntersection::Refused(
            nurbs_core::editing::exact_derivative::Refusal::ResourceLimit
        )
    );
    assert!(limited.work_used < full.work_used);
    a.loops = vec![rectangle([-2., -1.], [0., 1.], false)];
    let disjoint = intersect_planar_face_boundaries(&a, &b, WORK).unwrap();
    assert!(
        matches!(disjoint.outcome, PlanarBoundaryIntersection::Transverse { shared_boundary, .. } if shared_boundary.is_empty())
    );
}

fn crossing_triangle() -> Vec<Curve> {
    vec![Curve {
        degree: 1,
        knots: vec![0., 0., 1., 2., 3., 3.],
        control_points: vec![vec![0., -1.], vec![1., 2.], vec![0., 2.], vec![0., -1.]],
        weights: vec![1., 2., 3., 1.],
        periodic: false,
    }]
}

#[test]
fn proves_matching_points_of_two_transverse_rational_contours() {
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![crossing_triangle()];
    b.loops = vec![crossing_triangle()];
    let old = intersect_planar_face_boundaries(&a, &b, WORK).unwrap();
    assert!(
        matches!(old.outcome, PlanarBoundaryIntersection::Transverse { shared_boundary, .. } if shared_boundary.is_empty())
    );
    let result = intersect_planar_face_boundary_contacts(&a, &b, WORK).unwrap();
    let PlanarBoundaryIntersection::Transverse {
        shared_boundary, ..
    } = result.outcome
    else {
        panic!("{:?}", result.outcome);
    };
    assert_eq!(shared_boundary, [[q(0, 1), q(0, 1)], [q(1, 3), q(1, 3)]]);
}

#[test]
fn proves_a_quadratic_rational_tangency_without_promoting_irrational_roots() {
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![crossing_triangle()];
    let tangent = |shift: f64| {
        vec![
            Curve {
                degree: 2,
                knots: vec![0., 0., 0., 1., 1., 1.],
                control_points: vec![
                    vec![0., 1. - shift],
                    vec![0.5, -2. - shift],
                    vec![1., 4. - shift],
                ],
                weights: vec![1.; 3],
                periodic: false,
            },
            Curve {
                degree: 1,
                knots: vec![0., 0., 1., 1.],
                control_points: vec![vec![1., 4. - shift], vec![0., 1. - shift]],
                weights: vec![1.; 2],
                periodic: false,
            },
        ]
    };
    b.loops = vec![tangent(0.)];
    let shared = intersect_planar_face_boundary_contacts(&a, &b, WORK).unwrap();
    let PlanarBoundaryIntersection::Transverse {
        shared_boundary, ..
    } = shared.outcome
    else {
        panic!("{:?}", shared.outcome);
    };
    assert_eq!(shared_boundary, [[q(1, 3), q(1, 3)]]);
    let limited = intersect_planar_face_boundary_contacts(&a, &b, shared.work_used - 1).unwrap();
    assert_eq!(
        limited.outcome,
        PlanarBoundaryIntersection::Refused(
            nurbs_core::editing::exact_derivative::Refusal::ResourceLimit
        )
    );
    b.loops = vec![tangent(0.5)];
    let irrational = intersect_planar_face_boundary_contacts(&a, &b, WORK).unwrap();
    assert!(
        matches!(irrational.outcome, PlanarBoundaryIntersection::Transverse { shared_boundary, .. } if shared_boundary.is_empty())
    );
}

#[test]
fn distinguishes_open_mixed_intervals_from_common_boundary_points() {
    let [a, mut b] = perpendicular_boundary_faces();
    b.loops = vec![rectangle([-1., 1.], [-1., 1.], false)];
    let opts = options(10_000);
    let report = classify_planar_face_contacts(&a, &b, opts).unwrap();
    let PlanarFaceContactPartition::Transverse(contact) = report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert!(contact.interior.is_empty());
    assert!(contact.right_boundary_in_left_interior.is_empty());
    assert_eq!(
        contact.common_boundary,
        [[q(-1, 1), q(-1, 1)], [q(1, 1), q(1, 1)]]
    );
    let [mixed] = contact.left_boundary_in_right_interior.as_slice() else {
        panic!("{:?}", contact.left_boundary_in_right_interior);
    };
    assert_eq!(mixed.closed, [false, false]);
    assert!(mixed.parameter[0] < q(0, 1) && q(0, 1) < mixed.parameter[1]);
    assert!(
        contact
            .possible
            .iter()
            .any(|r| r[0] <= q(-1, 1) && r[1] >= q(1, 1))
    );
    let exchange = classify_planar_face_contacts(&b, &a, opts).unwrap();
    let PlanarFaceContactPartition::Transverse(reversed) = exchange.outcome else {
        panic!("{:?}", exchange.outcome);
    };
    assert_eq!(
        reversed.right_boundary_in_left_interior,
        contact.left_boundary_in_right_interior
    );
    assert_eq!(reversed.common_boundary, contact.common_boundary);
}

#[test]
fn retains_singleton_mixed_contacts_and_uncertainty_when_winding_is_limited() {
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![crossing_triangle()];
    b.loops = vec![rectangle([-1., 1.], [-1., 1.], false)];
    let report = classify_planar_face_contacts(&a, &b, options(10_000)).unwrap();
    let PlanarFaceContactPartition::Transverse(contact) = report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert_eq!(contact.left_boundary_in_right_interior.len(), 2);
    assert_eq!(
        contact.left_boundary_in_right_interior[1].parameter,
        [q(1, 3), q(1, 3)]
    );
    assert_eq!(
        contact.left_boundary_in_right_interior[1].closed,
        [true, true]
    );
    assert!(contact.common_boundary.is_empty());
    assert_eq!(contact.interior.len(), 1);
    let limited = classify_planar_face_contacts(&a, &b, options(1)).unwrap();
    let PlanarFaceContactPartition::Transverse(unresolved) = limited.outcome else {
        panic!("{:?}", limited.outcome);
    };
    assert!(limited.winding_cells <= 1);
    assert!(unresolved.interior.is_empty());
    assert!(unresolved.left_boundary_in_right_interior.is_empty());
    assert!(
        unresolved
            .possible
            .iter()
            .any(|r| r[0] <= q(0, 1) && r[1] >= q(1, 3))
    );
}

fn folded_boundary(last: f64) -> Vec<Curve> {
    vec![
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.], vec![2., 0.], vec![last, 0.]],
            weights: vec![1., 2., 1.],
            periodic: false,
        },
        Curve {
            degree: 1,
            knots: vec![0., 0., 1., 2., 3., 3.],
            control_points: vec![vec![last, 0.], vec![1., 1.], vec![0., 1.], vec![0., 0.]],
            weights: vec![1.; 4],
            periodic: false,
        },
    ]
}

#[test]
fn recovers_exact_weighted_fold_extent_past_both_endpoints() {
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![folded_boundary(0.)];
    b.loops = vec![rectangle([1., 2.], [0., 1.], false)];
    let previous = intersect_planar_face_boundary_contacts(&a, &b, WORK).unwrap();
    assert!(
        matches!(previous.outcome, PlanarBoundaryIntersection::Transverse { shared_boundary, .. } if shared_boundary.is_empty())
    );
    let refined = refine_planar_face_boundary_extents(&a, &b, 1e-8, WORK).unwrap();
    let PlanarBoundaryIntersection::Transverse {
        shared_boundary, ..
    } = refined.outcome
    else {
        panic!("{:?}", refined.outcome);
    };
    assert_eq!(shared_boundary, [[q(1, 1), q(4, 3)]]);
}

#[test]
fn certifies_inner_extent_using_a_rational_witness_near_irrational_stationary_point() {
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![folded_boundary(1.)];
    b.loops = vec![rectangle([1.25, 2.], [0., 1.], true)];
    let refined = refine_planar_face_boundary_extents(&a, &b, 1e-8, WORK).unwrap();
    let PlanarBoundaryIntersection::Transverse {
        shared_boundary, ..
    } = refined.outcome
    else {
        panic!("{:?}", refined.outcome);
    };
    let [band] = shared_boundary.as_slice() else {
        panic!("{shared_boundary:?}");
    };
    assert_eq!(band[0], q(5, 4));
    assert!(band[1] > q(149, 100) && band[1] < q(2, 1));
    let limited = refine_planar_face_boundary_extents(&a, &b, 1e-8, refined.work_used - 1).unwrap();
    assert_eq!(
        limited.outcome,
        PlanarBoundaryIntersection::Refused(
            nurbs_core::editing::exact_derivative::Refusal::ResourceLimit
        )
    );
}

#[test]
fn tightens_the_upper_boundary_of_an_irrational_rational_span_fold() {
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![folded_boundary(1.)];
    b.loops = vec![rectangle([1.25, 2.], [0., 1.], false)];
    let bounds = |width| {
        let report = bound_planar_face_boundary_intersection(&a, &b, width, WORK).unwrap();
        let BoundedPlanarBoundaryIntersection::Transverse {
            proven, possible, ..
        } = report.outcome
        else {
            panic!("{:?}", report.outcome);
        };
        assert_eq!(proven.len(), 1);
        assert_eq!(possible.len(), 1);
        assert_eq!(proven[0][0], q(5, 4));
        assert_eq!(possible[0][0], q(5, 4));
        (proven[0][1].clone(), possible[0][1].clone())
    };
    let (coarse_lo, coarse_hi) = bounds(1e-2);
    let (fine_lo, fine_hi) = bounds(1e-8);
    assert!(coarse_hi > fine_hi && fine_hi > fine_lo);
    assert!(fine_hi - fine_lo < q(1, 1_000_000));
    assert!(coarse_lo > q(3, 2));
    // This coordinate is exactly attained at normalized parameter 5/8.
    let t = q(5, 8);
    let sample = (q(8, 1) * &t - q(7, 1) * &t * &t) / (q(1, 1) + q(2, 1) * &t - q(2, 1) * &t * &t);
    assert!(sample <= bounds(1e-8).1);
}

#[test]
fn encloses_irrational_isolated_boundary_contacts_without_marking_them_exact() {
    let [a, mut b] = perpendicular_boundary_faces();
    b.loops = vec![vec![
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: vec![vec![0., 0.5], vec![0.5, -2.5], vec![1., 3.5]],
            weights: vec![1.; 3],
            periodic: false,
        },
        Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: vec![vec![1., 3.5], vec![0., 0.5]],
            weights: vec![1.; 2],
            periodic: false,
        },
    ]];
    let report = bound_planar_face_boundary_intersection(&a, &b, 1e-8, WORK).unwrap();
    let BoundedPlanarBoundaryIntersection::Transverse {
        proven, possible, ..
    } = report.outcome
    else {
        panic!("{:?}", report.outcome);
    };
    assert!(proven.is_empty());
    assert_eq!(possible.len(), 2);
    for (band, sign) in possible.iter().zip([q(-1, 1), q(1, 1)]) {
        assert!(&band[1] - &band[0] < q(1, 1_000_000));
        let polynomial = |t: &Q| (q(3, 1) * t - q(1, 1)).pow(2) - q(1, 2);
        assert!(polynomial(&band[0]) * &sign <= q(0, 1));
        assert!(polynomial(&band[1]) * &sign >= q(0, 1));
    }
    let limited =
        bound_planar_face_boundary_intersection(&a, &b, 1e-8, report.work_used - 1).unwrap();
    assert_eq!(
        limited.outcome,
        BoundedPlanarBoundaryIntersection::Refused(
            nurbs_core::editing::exact_derivative::Refusal::ResourceLimit
        )
    );
}

#[test]
fn resolves_uncertain_boundary_length_in_xyz_and_preserves_earlier_bounds() {
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![folded_boundary(1.)];
    b.loops = vec![rectangle([1.25, 2.], [0., 1.], false)];
    // Lift the x axis to the line [x, 0, x], so physical length is sqrt(2) times its clock width.
    a.projection.omitted_axis = 2;
    a.projection.retained_axes = [0, 1];
    a.projection.slopes = [q(1, 1), q(0, 1)];
    // Shear the right contour so its bottom edge follows z = x.
    for point in b
        .loops
        .iter_mut()
        .flatten()
        .flat_map(|curve| &mut curve.control_points)
    {
        point[1] += point[0];
    }
    let opts = AdaptiveBoundaryOptions {
        xyz_tolerance: 1e-7,
        max_passes: 25,
        max_work: WORK,
    };
    let first = resolve_planar_face_boundary_intersection(
        &a,
        &b,
        AdaptiveBoundaryOptions {
            max_passes: 1,
            ..opts
        },
    )
    .unwrap();
    assert_eq!(first.resolution, BoundaryResolution::PassLimit);
    assert_eq!(first.passes, 1);
    let refined = resolve_planar_face_boundary_intersection(&a, &b, opts).unwrap();
    assert_eq!(refined.resolution, BoundaryResolution::WithinTolerance);
    assert!(refined.passes > 1);
    let BoundedPlanarBoundaryIntersection::Transverse {
        proven, possible, ..
    } = &refined.bounds
    else {
        panic!("{:?}", refined.bounds);
    };
    let squared = (&possible[0][1] - &proven[0][1]).pow(2) * q(2, 1);
    assert_eq!(refined.max_unresolved_length_squared, Some(squared));
    let limited = resolve_planar_face_boundary_intersection(
        &a,
        &b,
        AdaptiveBoundaryOptions {
            max_work: first.work_used,
            ..opts
        },
    )
    .unwrap();
    assert_eq!(
        limited.resolution,
        BoundaryResolution::Refused(nurbs_core::editing::exact_derivative::Refusal::ResourceLimit)
    );
    assert_eq!(limited.bounds, first.bounds);
    assert_eq!(limited.passes, 1);
}

#[test]
fn exact_overlap_meets_the_geometric_target_on_first_pass() {
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![folded_boundary(0.)];
    b.loops = vec![rectangle([1., 2.], [0., 1.], false)];
    let report = resolve_planar_face_boundary_intersection(
        &a,
        &b,
        AdaptiveBoundaryOptions {
            xyz_tolerance: 1e-15,
            max_passes: 4,
            max_work: WORK,
        },
    )
    .unwrap();
    assert_eq!(report.resolution, BoundaryResolution::WithinTolerance);
    assert_eq!(report.passes, 1);
    assert_eq!(report.max_unresolved_length_squared, Some(q(0, 1)));
}

fn irrational_contact_loop(shift: f64, weight: f64, reverse: bool) -> Vec<Curve> {
    let mut points = vec![
        vec![0., 1. - shift],
        vec![0.5, -2. - shift],
        vec![1., 4. - shift],
    ];
    if reverse {
        points.reverse();
    }
    let close = vec![points[2].clone(), points[0].clone()];
    vec![
        Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: points,
            weights: vec![1., weight, 1.],
            periodic: false,
        },
        Curve {
            degree: 1,
            knots: vec![0., 0., 1., 1.],
            control_points: close,
            weights: vec![1.; 2],
            periodic: false,
        },
    ]
}

#[test]
fn proves_both_irrational_contact_branches_across_a_reversed_clock() {
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![irrational_contact_loop(0.5, 1., false)];
    b.loops = vec![irrational_contact_loop(0.5, 1., true)];
    for p in b
        .loops
        .iter_mut()
        .flatten()
        .flat_map(|c| &mut c.control_points)
    {
        p[1] *= 2.;
    }
    let result = match_quadratic_planar_boundary_contacts(&a, &b, WORK).unwrap();
    let AlgebraicPlanarContacts::Transverse { points, .. } = &result.outcome else {
        panic!("{:?}", result.outcome);
    };
    assert_eq!(
        points,
        &[false, true].map(|upper_root| AlgebraicBoundaryClock::Quadratic {
            linear: q(-2, 3),
            constant: q(1, 18),
            upper_root
        })
    );
    assert_eq!(
        match_quadratic_planar_boundary_contacts(&b, &a, WORK)
            .unwrap()
            .outcome,
        result.outcome
    );
    let limited = match_quadratic_planar_boundary_contacts(&a, &b, result.work_used - 1).unwrap();
    assert_eq!(
        limited.outcome,
        AlgebraicPlanarContacts::Refused(
            nurbs_core::editing::exact_derivative::Refusal::ResourceLimit
        )
    );
}

#[test]
fn compares_weighted_algebraic_images_and_rejects_nearby_unequal_roots() {
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![irrational_contact_loop(0.5, 2., false)];
    b.loops = vec![irrational_contact_loop(0.5, 2., true)];
    let result = match_quadratic_planar_boundary_contacts(&a, &b, WORK).unwrap();
    let AlgebraicPlanarContacts::Transverse { points, .. } = result.outcome else {
        panic!("{:?}", result.outcome);
    };
    assert_eq!(points.len(), 2);
    b.loops = vec![irrational_contact_loop(0.5 + 1. / 1024., 2., true)];
    let result = match_quadratic_planar_boundary_contacts(&a, &b, WORK).unwrap();
    assert!(
        matches!(result.outcome, AlgebraicPlanarContacts::Transverse { points, .. } if points.is_empty())
    );
}

#[test]
fn encloses_each_algebraic_branch_with_a_certified_xyz_diameter() {
    let [_, b] = perpendicular_boundary_faces();
    let support = intersect_planar_supports(&plane(|x, _| -x), &b.projection, WORK).unwrap();
    let SupportPlaneIntersection::Line(line) = support.outcome else {
        panic!("{:?}", support.outcome);
    };
    assert_eq!(line.clock_axis, 0);
    assert_eq!(line.direction, [q(1, 1), q(0, 1), q(-1, 1)]);
    let tolerance = 1e-25;
    let point = |upper_root| AlgebraicBoundaryClock::Quadratic {
        linear: q(-2, 3),
        constant: q(1, 18),
        upper_root,
    };
    let lower =
        enclose_algebraic_planar_boundary_point(&line, &point(false), tolerance, WORK).unwrap();
    let upper =
        enclose_algebraic_planar_boundary_point(&line, &point(true), tolerance, WORK).unwrap();
    let lower = lower.outcome.unwrap();
    let upper = upper.outcome.unwrap();
    assert!(lower.parameter[1] < upper.parameter[0]);
    for (bounds, sign) in [lower, upper].iter().zip([q(-1, 1), q(1, 1)]) {
        let [lo, hi] = &bounds.parameter;
        let at = |t: &Q| t.pow(2) - q(2, 3) * t + q(1, 18);
        assert!(at(lo) * &sign <= q(0, 1));
        assert!(at(hi) * &sign >= q(0, 1));
        assert_eq!(
            bounds.xyz,
            [[lo.clone(), hi.clone()], [q(0, 1), q(0, 1)], [-hi, -lo]]
        );
        assert_eq!(bounds.diameter_squared, (hi - lo).pow(2) * q(2, 1));
        assert!(bounds.diameter_squared <= Q::from_float(tolerance).unwrap().pow(2));
    }
    let invalid_axis = enclose_algebraic_planar_boundary_point(
        &nurbs_core::analysis::exact_plane_intersection::ExactSupportLine {
            clock_axis: 3,
            ..line
        },
        &point(true),
        tolerance,
        WORK,
    );
    assert!(invalid_axis.is_err());
}

#[test]
fn encloses_rational_clock_values_exactly_and_bounds_algebraic_work() {
    use nurbs_core::editing::exact_derivative::Refusal;
    let [a, b] = perpendicular_boundary_faces();
    let report = intersect_planar_supports(&a.projection, &b.projection, WORK).unwrap();
    let SupportPlaneIntersection::Line(line) = report.outcome else {
        panic!("{:?}", report.outcome);
    };
    let rational = AlgebraicBoundaryClock::Rational(q(1, 3));
    let exact = enclose_algebraic_planar_boundary_point(&line, &rational, 0., WORK)
        .unwrap()
        .outcome
        .unwrap();
    assert_eq!(exact.parameter, [q(1, 3), q(1, 3)]);
    assert_eq!(exact.diameter_squared, q(0, 1));
    let irrational = AlgebraicBoundaryClock::Quadratic {
        linear: q(0, 1),
        constant: q(-2, 1),
        upper_root: true,
    };
    let zero_tolerance =
        enclose_algebraic_planar_boundary_point(&line, &irrational, 0., WORK).unwrap();
    assert_eq!(zero_tolerance.outcome, Err(Refusal::NotRepresentable));
    let limited = enclose_algebraic_planar_boundary_point(&line, &irrational, 1e-8, 1).unwrap();
    assert_eq!(limited.outcome, Err(Refusal::ResourceLimit));
    assert!(limited.work_used <= 1);
}

#[test]
fn orders_close_quadratic_branches_without_rounding_them_to_the_same_clock() {
    let epsilon = q(1, 2).pow(120);
    let root = |constant, upper_root| AlgebraicBoundaryClock::Quadratic {
        linear: q(0, 1),
        constant,
        upper_root,
    };
    let shifted = AlgebraicBoundaryClock::Quadratic {
        linear: -q(2, 1) * &epsilon,
        constant: epsilon.pow(2) - q(2, 1),
        upper_root: true,
    };
    let points = [
        shifted,
        root(q(-2, 1) - &epsilon, true),
        root(q(-2, 1), true),
        root(q(-2, 1), false),
        root(q(-2, 1) - &epsilon, false),
        AlgebraicBoundaryClock::Rational(q(7, 5)),
        root(q(-2, 1), true),
        AlgebraicBoundaryClock::Rational(q(3, 2)),
    ];
    let report = order_algebraic_boundary_clocks(&points, WORK).unwrap();
    assert_eq!(
        report.outcome.unwrap(),
        vec![
            vec![4],
            vec![3],
            vec![5],
            vec![2, 6],
            vec![1],
            vec![0],
            vec![7]
        ]
    );
}

#[test]
fn groups_reducible_representations_and_refuses_invalid_roots_or_exhausted_work() {
    use nurbs_core::editing::exact_derivative::Refusal;
    let points = [
        AlgebraicBoundaryClock::Quadratic {
            linear: q(-3, 1),
            constant: q(9, 4),
            upper_root: false,
        },
        AlgebraicBoundaryClock::Rational(q(5, 2)),
        AlgebraicBoundaryClock::Rational(q(3, 2)),
        AlgebraicBoundaryClock::Quadratic {
            linear: q(-2, 1),
            constant: q(3, 4),
            upper_root: true,
        },
    ];
    let report = order_algebraic_boundary_clocks(&points, WORK).unwrap();
    assert_eq!(report.outcome.unwrap(), vec![vec![0, 2, 3], vec![1]]);
    let limited = order_algebraic_boundary_clocks(&points, report.work_used - 1).unwrap();
    assert_eq!(limited.outcome, Err(Refusal::ResourceLimit));
    assert!(limited.work_used < report.work_used);
    let invalid = [AlgebraicBoundaryClock::Quadratic {
        linear: q(0, 1),
        constant: q(1, 1),
        upper_root: true,
    }];
    let report = order_algebraic_boundary_clocks(&invalid, WORK).unwrap();
    assert_eq!(report.outcome, Err(Refusal::InvalidOutput));
}

#[test]
fn isolates_nearly_equal_algebraic_roots_and_produces_a_rational_separator() {
    let epsilon = q(1, 2).pow(125);
    let root = |constant| AlgebraicBoundaryClock::Quadratic {
        linear: q(0, 1),
        constant,
        upper_root: true,
    };
    let points = [root(q(-2, 1) - &epsilon), root(q(-2, 1)), root(q(-2, 1))];
    let report = isolate_ordered_algebraic_boundary_clocks(&points, WORK).unwrap();
    let result = report.outcome.unwrap();
    assert_eq!(result.clocks.len(), 2);
    assert_eq!(result.clocks[0].indices, [1, 2]);
    assert_eq!(result.clocks[1].indices, [0]);
    let separator = &result.witnesses[0];
    assert!(&result.clocks[0].parameter[1] < separator);
    assert!(separator < &result.clocks[1].parameter[0]);
    assert!(q(2, 1) < separator.pow(2) && separator.pow(2) < q(2, 1) + &epsilon);
    for (clock, square) in result.clocks.iter().zip([q(2, 1), q(2, 1) + &epsilon]) {
        assert!(clock.parameter[0].pow(2) <= square);
        assert!(clock.parameter[1].pow(2) >= square);
    }
    let limited = isolate_ordered_algebraic_boundary_clocks(&points, report.work_used - 1).unwrap();
    assert_eq!(
        limited.outcome,
        Err(nurbs_core::editing::exact_derivative::Refusal::ResourceLimit)
    );
}

#[test]
fn builds_exact_witnesses_between_rational_points_and_handles_empty_input() {
    let values = [q(1, 3), q(-3, 2), q(4, 3)];
    let points = values.clone().map(AlgebraicBoundaryClock::Rational);
    let result = isolate_ordered_algebraic_boundary_clocks(&points, WORK)
        .unwrap()
        .outcome
        .unwrap();
    assert_eq!(
        result
            .clocks
            .iter()
            .map(|clock| clock.parameter.clone())
            .collect::<Vec<_>>(),
        [1, 0, 2].map(|i| [values[i].clone(), values[i].clone()])
    );
    assert_eq!(result.witnesses, [q(-7, 12), q(5, 6)]);
    let empty = isolate_ordered_algebraic_boundary_clocks(&[], WORK)
        .unwrap()
        .outcome
        .unwrap();
    assert!(empty.clocks.is_empty() && empty.witnesses.is_empty());
}

#[test]
fn partitions_all_rational_and_quadratic_events_and_records_face_incidence() {
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![irrational_contact_loop(0.5, 1., false), crossing_triangle()];
    b.loops = vec![irrational_contact_loop(0.5, 1., true)];
    let report = partition_quadratic_planar_section(&a, &b, WORK).unwrap();
    let PlanarSectionOutcome::Transverse(section) = &report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert_eq!(section.events.len(), 4);
    assert_eq!(
        section.events[0].clock,
        AlgebraicBoundaryClock::Rational(q(0, 1))
    );
    assert_eq!(
        section.events[2].clock,
        AlgebraicBoundaryClock::Rational(q(1, 3))
    );
    assert_eq!(section.events[0].on_boundary, [true, false]);
    assert_eq!(section.events[1].on_boundary, [true, true]);
    assert_eq!(section.events[2].on_boundary, [true, false]);
    assert_eq!(section.events[3].on_boundary, [true, true]);
    for (gap, sample) in section.events.windows(2).zip(&section.samples) {
        assert!(&gap[0].parameter_bounds[1] < sample);
        assert!(sample < &gap[1].parameter_bounds[0]);
    }
    assert_eq!(section.samples.len(), 3);
    let limited = partition_quadratic_planar_section(&a, &b, report.work_used - 1).unwrap();
    assert_eq!(
        limited.outcome,
        PlanarSectionOutcome::Refused(
            nurbs_core::editing::exact_derivative::Refusal::ResourceLimit
        )
    );
}

#[test]
fn refuses_a_coincident_span_instead_of_claiming_a_finite_cut_list() {
    let [a, mut b] = perpendicular_boundary_faces();
    b.loops = vec![crossing_triangle()];
    let report = partition_quadratic_planar_section(&a, &b, WORK).unwrap();
    assert_eq!(
        report.outcome,
        PlanarSectionOutcome::Refused(
            nurbs_core::editing::exact_derivative::Refusal::UnprovedIdentity
        )
    );
}

fn section_winding_options(max_winding_cells: usize) -> PlanarSectionWindingOptions {
    PlanarSectionWindingOptions {
        winding_tolerance: 1e-8,
        max_winding_cells,
        max_work: WORK,
    }
}

#[test]
fn classifies_every_gap_between_exact_hole_and_face_boundaries() {
    use nurbs_core::trim::trim_domain::Location::{Inside, Outside};
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![
        rectangle([-2., 2.], [-1., 1.], false),
        rectangle([-0.5, 0.5], [-0.5, 0.5], true),
    ];
    b.loops = vec![rectangle([-1., 1.], [-1., 1.], false)];
    let options = section_winding_options(10_000);
    let report = classify_quadratic_planar_section(&a, &b, options).unwrap();
    let ClassifiedPlanarSection::Transverse { section, gaps } = report.outcome else {
        panic!("{:?}", report.outcome);
    };
    assert_eq!(section.events.len(), 6);
    assert_eq!(
        gaps,
        [
            [Inside, Outside],
            [Inside, Inside],
            [Outside, Inside],
            [Inside, Inside],
            [Inside, Outside]
        ]
    );
    let reverse = classify_quadratic_planar_section(&b, &a, options).unwrap();
    let ClassifiedPlanarSection::Transverse {
        section: reverse_section,
        gaps: reverse_gaps,
    } = reverse.outcome
    else {
        panic!("{:?}", reverse.outcome);
    };
    assert_eq!(section.line, reverse_section.line);
    assert_eq!(
        reverse_gaps,
        gaps.into_iter().map(|[a, b]| [b, a]).collect::<Vec<_>>()
    );
}

#[test]
fn classifies_the_full_gap_between_irrational_events_with_bounded_winding() {
    use nurbs_core::trim::trim_domain::Location::{Inside, Unresolved};
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![irrational_contact_loop(0.5, 2., false)];
    b.loops = vec![irrational_contact_loop(0.5, 2., true)];
    let complete =
        classify_quadratic_planar_section(&a, &b, section_winding_options(10_000)).unwrap();
    let ClassifiedPlanarSection::Transverse { section, gaps } = complete.outcome else {
        panic!("{:?}", complete.outcome);
    };
    assert_eq!(gaps, [[Inside, Inside]]);
    assert!(
        section
            .events
            .iter()
            .all(|event| matches!(event.clock, AlgebraicBoundaryClock::Quadratic { .. }))
    );
    let limited = classify_quadratic_planar_section(&a, &b, section_winding_options(1)).unwrap();
    let ClassifiedPlanarSection::Transverse {
        section: limited_section,
        gaps,
    } = limited.outcome
    else {
        panic!("{:?}", limited.outcome);
    };
    assert_eq!(limited_section, section);
    assert_eq!(gaps, [[Unresolved, Unresolved]]);
    assert!(limited.winding_cells <= 1);
}

#[test]
fn assembles_closed_components_on_both_sides_of_a_hole() {
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![
        rectangle([-2., 2.], [-1., 1.], false),
        rectangle([-0.5, 0.5], [-0.5, 0.5], true),
    ];
    b.loops = vec![rectangle([-1., 1.], [-1., 1.], false)];
    let report =
        assemble_quadratic_planar_section_material(&a, &b, section_winding_options(10_000))
            .unwrap();
    let PlanarSectionMaterial::Transverse {
        section,
        proven,
        possible,
    } = report.outcome
    else {
        panic!("{:?}", report.outcome);
    };
    assert_eq!(proven, [[1, 2], [3, 4]]);
    assert_eq!(possible, proven);
    assert_eq!(
        section.events[2].clock,
        AlgebraicBoundaryClock::Rational(q(-1, 2))
    );
    let limited =
        assemble_quadratic_planar_section_material(&a, &b, section_winding_options(1)).unwrap();
    let PlanarSectionMaterial::Transverse {
        proven, possible, ..
    } = limited.outcome
    else {
        panic!("{:?}", limited.outcome);
    };
    assert!(proven.is_empty());
    assert_eq!(possible, [[0, 5]]);
}

#[test]
fn preserves_an_isolated_tangent_contact_inside_the_other_face() {
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![irrational_contact_loop(0., 1., false)];
    b.loops = vec![rectangle([-1., 1.], [-1., 1.], false)];
    let report =
        assemble_quadratic_planar_section_material(&a, &b, section_winding_options(10_000))
            .unwrap();
    let PlanarSectionMaterial::Transverse {
        section,
        proven,
        possible,
    } = report.outcome
    else {
        panic!("{:?}", report.outcome);
    };
    assert_eq!(proven, [[1, 1]]);
    assert_eq!(possible, proven);
    assert_eq!(
        section.events[1].clock,
        AlgebraicBoundaryClock::Rational(q(1, 3))
    );
    assert_eq!(section.events[1].on_boundary, [true, false]);
}

#[test]
fn measures_the_sum_of_two_components_and_scales_to_xyz() {
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.projection.slopes = [q(1, 1), q(0, 1)];
    a.loops = vec![
        rectangle([-2., 2.], [-1., 1.], false),
        rectangle([-0.5, 0.5], [-0.5, 0.5], true),
    ];
    b.loops = vec![rectangle([-1., 1.], [-1., 1.], false)];
    for point in b
        .loops
        .iter_mut()
        .flatten()
        .flat_map(|curve| &mut curve.control_points)
    {
        point[1] += point[0];
    }
    let report =
        measure_quadratic_planar_section_material(&a, &b, section_winding_options(10_000), 1e-12)
            .unwrap();
    assert_eq!(
        report.outcome,
        PlanarSectionLength::Transverse {
            clock_length: [q(1, 1), q(1, 1)],
            xyz_length_squared: [q(2, 1), q(2, 1)],
        }
    );
    let limited =
        measure_quadratic_planar_section_material(&a, &b, section_winding_options(1), 1e-12)
            .unwrap();
    assert_eq!(
        limited.outcome,
        PlanarSectionLength::Transverse {
            clock_length: [q(0, 1), q(4, 1)],
            xyz_length_squared: [q(0, 1), q(32, 1)],
        }
    );
}

#[test]
fn bounds_the_length_between_irrational_endpoints() {
    let [mut a, mut b] = perpendicular_boundary_faces();
    a.loops = vec![irrational_contact_loop(0.5, 1., false)];
    b.loops = vec![irrational_contact_loop(0.5, 1., true)];
    let options = section_winding_options(10_000);
    let report = measure_quadratic_planar_section_material(&a, &b, options, 1e-12).unwrap();
    let PlanarSectionLength::Transverse {
        xyz_length_squared: [lo, hi],
        ..
    } = report.outcome
    else {
        panic!("{:?}", report.outcome);
    };
    assert!(lo < q(2, 9) && q(2, 9) < hi);
    assert!(hi - lo < q(1, 1_000_000_000));
    let limited = measure_quadratic_planar_section_material(
        &a,
        &b,
        PlanarSectionWindingOptions {
            max_work: report.work_used - 1,
            ..options
        },
        1e-12,
    )
    .unwrap();
    assert_eq!(
        limited.outcome,
        PlanarSectionLength::Refused(nurbs_core::editing::exact_derivative::Refusal::ResourceLimit)
    );
}
