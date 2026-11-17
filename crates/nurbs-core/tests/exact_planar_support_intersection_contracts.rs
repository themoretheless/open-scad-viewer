use num_rational::BigRational as Q;
use nurbs_core::{
    analysis::{
        exact_affine_hull::{
            PlanarProjectionOutcome, PlanarSurfaceProjection, project_planar_surface,
        },
        exact_plane_intersection::{SupportPlaneIntersection, intersect_planar_supports},
    },
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
