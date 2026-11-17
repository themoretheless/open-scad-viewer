use num_rational::BigRational as Q;
use nurbs_core::{
    analysis::{
        exact_affine_hull::{
            PlanarProjectionOutcome, PlanarSurfaceProjection, project_planar_surface,
        },
        exact_plane_intersection::{
            SupportPlaneIntersection, TrimmedPlanarIntersection, intersect_planar_supports,
            intersect_trimmed_planar_faces,
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
