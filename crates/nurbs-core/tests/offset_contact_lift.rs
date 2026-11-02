use nurbs_core::{
    curve::Curve,
    surface::Surface,
    surface_offset::contact_curve::lift::{Limits, certify},
};
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn line(points: Vec<Vec<f64>>) -> Curve {
    Curve {
        degree: 1,
        knots: vec![2., 2., 7., 7.],
        control_points: points,
        weights: vec![1.; 2],
        periodic: false,
    }
}
fn uv() -> Curve {
    line(vec![vec![0.1, 0.2], vec![0.9, 0.2]])
}
fn world() -> Curve {
    line(vec![vec![0.1, 0.2, 0.2], vec![0.9, 0.2, 0.2]])
}
fn limits() -> Limits {
    Limits {
        cells: 8192,
        spans: 128,
    }
}
#[test]
fn direct_normal_offset_lift_covers_nonunit_clock_and_actual_nets() {
    let r = certify(&world(), &uv(), &plane(), 0.2, 1e-8, limits()).unwrap();
    assert!(r.lift_correspondence_proven && r.error_upper_mm.unwrap() <= 1e-8);
    assert!(r.visited <= limits().cells);
}
#[test]
fn tiny_uv_error_does_not_authorize_large_physical_lift_error() {
    let mut s = plane();
    for p in s.control_points.iter_mut().flatten() {
        p[0] *= 1e6;
    }
    let mut w = world();
    for p in &mut w.control_points {
        p[0] *= 1e6;
    }
    let mut u = uv();
    for p in &mut u.control_points {
        p[0] += 1e-8;
    }
    let r = certify(&w, &u, &s, 0.2, 1e-5, limits()).unwrap();
    assert!(!r.lift_correspondence_proven);
    assert_eq!(r.reason, "actual-world-UV-lift-mismatch");
}
#[test]
fn matching_endpoints_cannot_hide_interior_lift_bulge_or_support_escape() {
    let mut u = uv();
    u.degree = 2;
    u.knots = vec![2., 2., 2., 7., 7., 7.];
    u.control_points = vec![vec![0.1, 0.2], vec![0.5, 0.3], vec![0.9, 0.2]];
    u.weights = vec![1.; 3];
    let r = certify(&world(), &u, &plane(), 0.2, 1e-6, limits()).unwrap();
    assert!(!r.lift_correspondence_proven);
    u.control_points[0][0] = -0.01;
    let r = certify(&world(), &u, &plane(), 0.2, 1e-6, limits()).unwrap();
    assert!(!r.lift_correspondence_proven);
    assert_eq!(r.reason, "whole-UV-original-support-unproved");
    assert!(
        certify(
            &world(),
            &uv(),
            &plane(),
            0.2,
            1e-6,
            Limits {
                cells: 0,
                spans: 128
            }
        )
        .is_err()
    );
    let mut u = uv();
    u.knots = vec![0., 0., 1., 1.];
    assert!(certify(&world(), &u, &plane(), 0.2, 1e-6, limits()).is_err());
}
#[test]
fn c1_source_knot_uses_all_incident_jets_without_requiring_c2() {
    let mut s = plane();
    s.degree_u = 2;
    s.knots_u = vec![0., 0., 0., 0.5, 1., 1., 1.];
    s.control_points = [0., 0.1, 0.8, 1.]
        .map(|x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
        .to_vec();
    s.weights = vec![vec![1.; 2]; 4];
    let u = Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![0., 0.2], vec![1., 0.2]],
        weights: vec![1.; 2],
        periodic: false,
    };
    let w = Curve {
        degree: 2,
        knots: s.knots_u.clone(),
        control_points: [0., 0.1, 0.8, 1.].map(|x| vec![x, 0.2, 0.2]).to_vec(),
        weights: vec![1.; 4],
        periodic: false,
    };
    let r = certify(&w, &u, &s, 0.2, 1e-6, limits()).unwrap();
    assert!(
        r.lift_correspondence_proven,
        "{} visited={}",
        r.reason, r.visited
    );
    let limited = certify(
        &w,
        &u,
        &s,
        0.2,
        1e-6,
        Limits {
            cells: 1,
            spans: 128,
        },
    )
    .unwrap();
    assert!(!limited.lift_correspondence_proven && limited.visited == 1);
}
