use nurbs_core::{
    editing::{bounded_surface_differential as bounded, exact_surface_derivative as exact},
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn parabolic_cylinder() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: [(0., 0.), (0.5, 0.), (1., 1.)]
            .map(|(x, z)| vec![vec![x, 0., z], vec![x, 1., z]])
            .to_vec(),
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}

fn check_at(s: &Surface, t: f64, u: f64, world_scale: f64) {
    let r = bounded::squared_mean_curvature(s, 1e-10, WORK).unwrap();
    let patches = r.patches.unwrap_or_else(|| panic!("{:?}", r.refusal));
    let expected = 1. / (world_scale * world_scale * (1. + 4. * u * u).powi(3));
    let actual = patches[0].evaluate(t, 0.625).unwrap().point;
    let upper = r.error_upper.unwrap()[0];
    assert!((0. ..=1e-10).contains(&upper));
    assert!((expected - actual[0]).abs() <= upper + 1e-14);
    assert_eq!(&actual[1..], &[0., 0.]);
}

#[test]
fn parabolic_cylinder_matches_analytic_squared_mean_curvature() {
    let s = parabolic_cylinder();
    assert!(
        exact::squared_mean_curvature(&s, WORK)
            .unwrap()
            .patches
            .is_none()
    );
    check_at(&s, 0.25, 0.25, 1.);
    check_at(&s, 1., 1., 1.);
    let r = bounded::squared_mean_curvature(&s, 1e-30, WORK).unwrap();
    assert_eq!(r.refusal, Some(bounded::Refusal::ToleranceExceeded));
    assert!(r.patches.is_none() && r.error_upper.is_none());
}

#[test]
fn rational_reclock_preserves_the_physical_field() {
    let mut s = parabolic_cylinder();
    s.weights = vec![vec![1.; 2], vec![2.; 2], vec![4.; 2]];
    for (row, (x, z)) in s
        .control_points
        .iter_mut()
        .zip([(0., 0.), (0.25, 0.), (0.5, 0.25)])
    {
        for point in row {
            point[0] = x;
            point[2] = z;
        }
    }
    check_at(&s, 0.5, 1. / 3., 1.);
}

#[test]
fn reversing_orientation_and_scaling_respect_the_units() {
    let mut s = parabolic_cylinder();
    s.control_points.reverse();
    for value in s.control_points.iter_mut().flatten().flatten() {
        *value *= 2.;
    }
    check_at(&s, 0.75, 0.25, 2.);
}

#[test]
fn exact_zero_is_accepted_only_on_a_regular_chart() {
    let mut s = parabolic_cylinder();
    for point in s.control_points.iter_mut().flatten() {
        point[2] = 0.;
    }
    let report = exact::squared_mean_curvature(&s, WORK).unwrap();
    let out = report.patches.unwrap();
    assert!(
        out[0]
            .control_points
            .iter()
            .flatten()
            .flatten()
            .all(|x| *x == 0.)
    );
    let row = s.control_points[0].clone();
    s.control_points.fill(row);
    let report = exact::squared_mean_curvature(&s, WORK).unwrap();
    assert_eq!(report.refusal, Some(exact::Refusal::UnprovedDenominator));
    assert!(report.patches.is_none());
}

#[test]
fn insufficient_work_is_atomic() {
    let s = parabolic_cylinder();
    let report = bounded::squared_mean_curvature(&s, 1e-10, 1).unwrap();
    assert_eq!(report.refusal, Some(bounded::Refusal::ResourceLimit));
    assert!(report.patches.is_none() && report.error_upper.is_none());
}

#[test]
fn exact_rational_cylinder_has_constant_nonzero_squared_mean_curvature() {
    // Tangent half-angle parameterization of a unit circular quadrant,
    // extruded over v. All Euclidean poles and weights are binary64 exact.
    let mut s = parabolic_cylinder();
    s.control_points = [(1., 0.), (1., 1.), (0., 1.)]
        .map(|(x, y)| vec![vec![x, y, 0.], vec![x, y, 1.]])
        .to_vec();
    s.weights[2] = vec![2.; 2];
    let report = exact::squared_mean_curvature(&s, WORK).unwrap();
    let patches = report
        .patches
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    assert_eq!(patches.len(), 1);
    assert!(
        patches[0]
            .control_points
            .iter()
            .flatten()
            .all(|p| p == &[0.25, 0., 0.])
    );
}
