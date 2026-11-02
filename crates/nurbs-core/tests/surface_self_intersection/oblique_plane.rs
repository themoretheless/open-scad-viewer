use super::within_span::loop_surface;
use super::*;
use nurbs_core::{
    curve::Curve,
    intersection::{
        intersect_curve_curve_certified_report as pair,
        intersect_self_curve_certified_report as audit,
    },
};
fn oblique() -> Surface {
    let mut s = loop_surface();
    for p in s.control_points.iter_mut().flatten() {
        p[2] += p[0] + 2. * p[1];
    }
    s
}
fn curve(s: &Surface) -> Curve {
    Curve {
        degree: s.degree_u,
        knots: s.knots_u.clone(),
        control_points: s.control_points.iter().map(|row| row[0].clone()).collect(),
        weights: s.weights.iter().map(|row| row[0]).collect(),
        periodic: false,
    }
}
#[test]
fn exact_oblique_plane_loop_has_original_three_dimensional_proof() {
    let s = oblique();
    let r = surface_self_intersection::inspect(&s, 10000).unwrap();
    assert_eq!(r.status, Status::Present);
    let c = r.certified_crossing.unwrap();
    for (i, t) in [0.25, 0.75].into_iter().enumerate() {
        assert!(c.parameter_boxes[i][0][0] <= t && t <= c.parameter_boxes[i][0][1]);
    }
    for (i, x) in [0., 0., 1.].into_iter().enumerate() {
        assert!(c.position_bounds[i][0] <= x && x <= c.position_bounds[i][1]);
    }
}
#[test]
fn oblique_self_curve_coverage_is_complete_without_a_constant_coordinate() {
    let c = curve(&oblique());
    assert!((0..3).all(|axis| {
        c.control_points
            .iter()
            .any(|p| p[axis] != c.control_points[0][axis])
    }));
    let r = audit(&c, 1e-7, 10000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.crossings.len(), 1);
}
#[test]
fn one_ulp_nonplanarity_cannot_be_rounded_into_a_plane_certificate() {
    let mut c = curve(&oblique());
    c.control_points[1][2] = c.control_points[1][2].next_up();
    let r = audit(&c, 1e-7, 10000).unwrap();
    assert!(!r.complete);
    assert!(r.crossings.is_empty());
    assert!(!r.unresolved.is_empty());
}
#[test]
fn separate_curves_must_share_the_same_exact_plane() {
    let a = curve(&oblique());
    let mut b = a.clone();
    for p in &mut b.control_points {
        p[2] += 1.;
    }
    let r = pair(&a, &b, 1e-7, 10000).unwrap();
    assert!(!r.complete);
    assert!(r.crossings.is_empty());
}

#[test]
fn separate_nonlinear_curves_can_share_an_exact_oblique_plane() {
    let a = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![-1., 0., -1.], vec![0., 0., 0.], vec![1., 0., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    let b = Curve {
        degree: 2,
        knots: a.knots.clone(),
        control_points: vec![
            vec![-0.5, -1., -2.5],
            vec![0.5, 0., 0.5],
            vec![-0.5, 1., 1.5],
        ],
        weights: vec![1.; 3],
        periodic: false,
    };
    let r = pair(&a, &b, 1e-7, 1000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.crossings.len(), 1);
    for interval in r.crossings[0].parameters {
        assert!(interval[0] <= 0.5 && 0.5 <= interval[1]);
    }
    for interval in r.crossings[0].position_bounds {
        assert!(interval[0] <= 0. && 0. <= interval[1]);
    }
}
