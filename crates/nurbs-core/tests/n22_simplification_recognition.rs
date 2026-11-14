//! N22: greedy knot removal, degree reduction round trips and
//! coefficient-certified analytic primitive recognition.
use nurbs_core::analysis::analytic_recognition::{
    AnalyticPrimitive, certify_primitive, recognize_analytic_surface,
};
use nurbs_core::curve::Curve;
use nurbs_core::foundation::approximate_edits::{
    reduce_curve_degree_report, reduce_surface_degree_report, remove_curve_knots_report,
    remove_surface_knots_greedy_report,
};
use nurbs_core::primitives;
use nurbs_core::surface::{Axis, Surface};
use nurbs_core::surface_edit;

fn cubic() -> Curve {
    Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 2., 0.],
            vec![3., -1., 1.],
            vec![4., 1., 0.],
        ],
        weights: vec![1., 2., 0.5, 1.],
        periodic: false,
    }
}

fn distance(a: &Curve, b: &Curve) -> f64 {
    (0..=200)
        .map(|i| {
            let t = i as f64 / 200.;
            let p = a.evaluate(t).unwrap().point;
            let q = b.evaluate(t).unwrap().point;
            p.iter().zip(&q).map(|(x, y)| (x - y).powi(2)).sum::<f64>().sqrt()
        })
        .fold(0., f64::max)
}

#[test]
fn insert_then_greedy_remove_recovers_original_curve() {
    let source = cubic();
    let mut inserted = source.clone();
    for (knot, times) in [(0.25, 1), (0.5, 2), (0.8, 3)] {
        inserted = inserted.insert(knot, times).unwrap();
    }
    assert_eq!(inserted.knots.len(), source.knots.len() + 6);
    let (edit, removed) = remove_curve_knots_report(&inserted, 1e-9, None).unwrap();
    assert!(edit.certificate.accepted);
    assert_eq!(removed.len(), 6, "every inserted knot is removable");
    assert_eq!(edit.curve.knots, source.knots);
    assert!(edit.certificate.error_upper <= 1e-9);
    assert!(distance(&edit.curve, &source) <= 1e-9);
}

/// Deterministic pseudo-random generator for reproducible "random" curves.
fn lcg(state: &mut u64) -> f64 {
    *state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
    ((*state >> 11) as f64) / ((1u64 << 53) as f64)
}

#[test]
fn greedy_removal_respects_tolerance_on_random_curves() {
    let mut seed = 0x5eed_u64;
    for case in 0..6 {
        let n = 9;
        let degree = 3;
        let mut knots = vec![0.; degree + 1];
        for i in 1..n - degree {
            knots.push(i as f64 / (n - degree) as f64);
        }
        knots.extend(vec![1.; degree + 1]);
        let control_points = (0..n)
            .map(|_| (0..3).map(|_| lcg(&mut seed) * 4. - 2.).collect())
            .collect();
        let weights = (0..n).map(|_| 0.5 + lcg(&mut seed)).collect();
        let source = Curve { degree, knots, control_points, weights, periodic: false };
        let tolerance = [1e-3, 1e-2, 0.1][case % 3];
        let (edit, removed) = remove_curve_knots_report(&source, tolerance, None).unwrap();
        assert_eq!(edit.curve.knots.len() + removed.len(), source.knots.len());
        assert!(edit.certificate.error_upper <= tolerance);
        // Independent sampled check never exceeds the certified bound.
        assert!(distance(&edit.curve, &source) <= edit.certificate.error_upper + 1e-12);
    }
}

#[test]
fn elevate_then_reduce_recovers_curve() {
    let source = cubic().insert(0.4, 1).unwrap();
    let elevated = source.elevate(5).unwrap();
    let edit = reduce_curve_degree_report(&elevated, 3, 1e-9, None).unwrap();
    assert!(edit.certificate.accepted);
    assert_eq!(edit.curve.degree, 3);
    assert!(distance(&edit.curve, &source) <= 1e-9);
}

fn patch() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.3], vec![0., 2., 0.]],
            vec![vec![1., 0., 0.4], vec![1., 1., 1.], vec![1., 2., 0.2]],
            vec![vec![2., 0., 0.], vec![2., 1., -0.3], vec![2., 2., 0.]],
        ],
        weights: vec![vec![1., 1.5, 1.], vec![0.8, 1.2, 1.], vec![1., 1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn surface_insert_then_greedy_remove_recovers_original() {
    let source = patch();
    let inserted = surface_edit::insert(&source, Axis::U, 0.3, 2).unwrap();
    let inserted = surface_edit::insert(&inserted, Axis::V, 0.6, 1).unwrap();
    let (edit, [ru, rv]) = remove_surface_knots_greedy_report(&inserted, 1e-8, None).unwrap();
    assert!(edit.certificate.accepted);
    assert_eq!((ru.len(), rv.len()), (2, 1));
    assert_eq!(edit.surface.knots_u, source.knots_u);
    assert_eq!(edit.surface.knots_v, source.knots_v);
    assert!(edit.certificate.error_upper <= 1e-8);
}

#[test]
fn surface_elevate_then_reduce_recovers_original() {
    for axis in [Axis::U, Axis::V] {
        let elevated = surface_edit::elevate(&patch(), axis, 4).unwrap();
        let edit = reduce_surface_degree_report(&elevated, axis, 2, 1e-8, None).unwrap();
        assert!(edit.certificate.accepted);
        assert_eq!((edit.surface.degree_u, edit.surface.degree_v), (2, 2));
    }
}

fn close(a: [f64; 3], b: [f64; 3], tol: f64) -> bool {
    a.iter().zip(&b).all(|(x, y)| (x - y).abs() <= tol)
}

fn parallel(a: [f64; 3], b: [f64; 3]) -> bool {
    (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).abs() > 1. - 1e-6
}

#[test]
fn recognizes_rational_primitives_with_parameters() {
    let tol = 1e-6;
    let cyl = primitives::cylinder([1., 2., 3.], 2.5, 4.).unwrap();
    match recognize_analytic_surface(&cyl, tol).unwrap().unwrap().primitive {
        AnalyticPrimitive::Cylinder { axis, radius, point } => {
            assert!((radius - 2.5).abs() < 1e-6);
            assert!(parallel(axis, [0., 0., 1.]));
            assert!((point[0] - 1.).abs() < 1e-6 && (point[1] - 2.).abs() < 1e-6);
        }
        other => panic!("expected cylinder, got {other:?}"),
    }
    let sph = primitives::sphere([0., -1., 2.], 3.).unwrap();
    match recognize_analytic_surface(&sph, tol).unwrap().unwrap().primitive {
        AnalyticPrimitive::Sphere { center, radius } => {
            assert!((radius - 3.).abs() < 1e-6);
            assert!(close(center, [0., -1., 2.], 1e-6));
        }
        other => panic!("expected sphere, got {other:?}"),
    }
    // Tall frustum: the sample proposal (PCA axis) needs the axis to be the
    // dominant spread direction; certification itself has no such limit.
    let cone = primitives::cone_frustum([0.; 3], 2., 1., 6.).unwrap();
    match recognize_analytic_surface(&cone, tol).unwrap().unwrap().primitive {
        AnalyticPrimitive::Cone { apex, axis, half_angle } => {
            assert!(close(apex, [0., 0., 12.], 1e-5), "{apex:?}");
            assert!(parallel(axis, [0., 0., 1.]));
            assert!((half_angle - (1. / 6_f64).atan()).abs() < 1e-6);
        }
        other => panic!("expected cone, got {other:?}"),
    }
    let torus = primitives::torus([0.; 3], 3., 1., 1.).unwrap();
    match recognize_analytic_surface(&torus, tol).unwrap().unwrap().primitive {
        AnalyticPrimitive::Torus { major_radius, minor_radius, axis, center } => {
            assert!((major_radius - 3.).abs() < 1e-6 && (minor_radius - 1.).abs() < 1e-6);
            assert!(parallel(axis, [0., 0., 1.]));
            assert!(close(center, [0.; 3], 1e-6));
        }
        other => panic!("expected torus, got {other:?}"),
    }
    let plane = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 1.], vec![0., 2., 1.]],
            vec![vec![3., 0., 1.], vec![3., 2., 1.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 1.]],
        periodic_u: false,
        periodic_v: false,
    };
    match recognize_analytic_surface(&plane, tol).unwrap().unwrap().primitive {
        AnalyticPrimitive::Plane { normal, offset } => {
            assert!(parallel(normal, [0., 0., 1.]));
            assert!((offset.abs() - 1.).abs() < 1e-9);
        }
        other => panic!("expected plane, got {other:?}"),
    }
}

#[test]
fn rejects_near_misses() {
    let tol = 1e-5;
    // Elliptic cylinder 1 : 1.001 deviates by ~5e-4 from every circular cylinder.
    let ellip = primitives::elliptic_cylinder([0.; 3], 1., 1.001, 2.).unwrap();
    assert!(recognize_analytic_surface(&ellip, tol).unwrap().is_none());
    // A single control point bumped by 1e-3 out of a cylinder.
    let mut bumped = primitives::cylinder([0.; 3], 1., 2.).unwrap();
    bumped.control_points[2][0][0] += 1e-3;
    assert!(recognize_analytic_surface(&bumped, tol).unwrap().is_none());
    // A genuinely curved freeform patch.
    assert!(recognize_analytic_surface(&patch(), tol).unwrap().is_none());
    // Certifying the true cylinder against a wrong radius is refused by the bound.
    let cyl = primitives::cylinder([0.; 3], 1., 2.).unwrap();
    let wrong = AnalyticPrimitive::Cylinder { point: [0.; 3], axis: [0., 0., 1.], radius: 1.0001 };
    assert!(certify_primitive(&cyl, &wrong).unwrap().unwrap() > tol);
    // Squat frustum certified directly against its exact cone.
    let squat = primitives::cone_frustum([0.; 3], 2., 1., 2.).unwrap();
    let exact = AnalyticPrimitive::Cone { apex: [0., 0., 4.], axis: [0., 0., -1.], half_angle: 0.5_f64.atan() };
    assert!(certify_primitive(&squat, &exact).unwrap().unwrap() < 1e-9);
    let tilted = AnalyticPrimitive::Cone { apex: [0., 0., 4.], axis: [0., 0., -1.], half_angle: 0.5_f64.atan() + 1e-4 };
    assert!(certify_primitive(&squat, &tilted).unwrap().unwrap() > tol);
    let right = AnalyticPrimitive::Cylinder { point: [0.; 3], axis: [0., 0., 1.], radius: 1. };
    assert!(certify_primitive(&cyl, &right).unwrap().unwrap() < 1e-9);
}

