use num_rational::BigRational as Q;
use nurbs_core::{
    analysis::{
        exact_affine_hull::{
            PlanarProjectionOutcome, PlanarSurfaceProjection, project_planar_surface,
        },
        exact_plane_intersection::{
            ExactPlanarIntersectionSegment, PlanarIntersectionSegments, PlanarSegmentStorage,
            StoredPlanarIntersection, SupportPlaneIntersection, TrimmedPlanarIntersection,
            build_planar_intersection_segments, intersect_planar_supports,
            intersect_trimmed_planar_faces, optimize_planar_intersection_segment_storage,
            store_planar_intersection_segment, store_trimmed_planar_intersection,
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
