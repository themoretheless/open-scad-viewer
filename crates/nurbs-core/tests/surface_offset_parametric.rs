use nurbs_core::{
    surface::{Axis, Surface},
    surface_edit,
    surface_offset::{CandidateStatus as S, certify_parametric_candidate as certify},
};

fn source() -> Surface {
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
fn map() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![-4., -4., 2., 2.],
        knots_v: vec![10., 10., 10., 14., 14., 14.],
        control_points: [0.1, 0.8]
            .map(|u| vec![vec![u, 0.1, 0.], vec![u + 0.06, 0.5, 0.], vec![u, 0.9, 0.]])
            .to_vec(),
        weights: vec![vec![1., 1.05, 1.]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn cap(m: &Surface) -> Surface {
    let mut c = m.clone();
    for p in c.control_points.iter_mut().flatten() {
        p[2] = 0.1;
    }
    c
}
#[test]
fn curved_rational_uv_sides_and_independent_candidate_basis_are_admitted() {
    let s = source();
    let m = map();
    let c = surface_edit::elevate(&cap(&m), Axis::U, 2).unwrap();
    let before = (s.clone(), m.clone(), c.clone());
    let result = certify(&s, &c, &m, 0.1, 0.005, 16384).unwrap();
    assert_eq!(result.status, S::Within);
    assert!(result.error_upper.unwrap() <= 0.005);
    assert!(result.cells > 1);
    // Literal map boundary has a curved image, not a straight quadrilateral.
    let first = m.evaluate(-4., 10.).unwrap().point;
    let mid = m.evaluate(-4., 12.).unwrap().point;
    assert!(mid[0] > first[0] + 0.01);
    assert_eq!((s, m, c), before);
}
#[test]
fn a_local_fold_away_from_positive_midpoint_cannot_authorize_a_map() {
    let mut m = map();
    m.degree_u = 2;
    m.knots_u = vec![-4., -4., -4., 2., 2., 2.];
    m.control_points = [0.2, 0.1, 0.9]
        .map(|u| [0.1, 0.5, 0.9].map(|v| vec![u, v, 0.]).to_vec())
        .to_vec();
    m.weights = vec![vec![1.; 3]; 3];
    assert!(m.evaluate(-1., 12.).unwrap().first_derivatives().unwrap().0[0] > 0.);
    assert!(m.evaluate(-4., 12.).unwrap().first_derivatives().unwrap().0[0] < 0.);
    assert!(certify(&source(), &cap(&m), &m, 0.1, 0.005, 16384).is_err());
}
#[test]
fn cap_interior_drift_refuses_while_map_and_boundary_curves_are_retained() {
    let m = map();
    let mut c = surface_edit::elevate(&cap(&m), Axis::U, 2).unwrap();
    let before = c.clone();
    c.control_points[1][1][2] += 0.1;
    let n = c.control_points.len();
    let v = c.control_points[0].len();
    for i in 0..n {
        for j in 0..v {
            if i == 0 || i == n - 1 || j == 0 || j == v - 1 {
                assert_eq!(c.control_points[i][j], before.control_points[i][j]);
            }
        }
    }
    let result = certify(&source(), &c, &m, 0.1, 0.005, 16384).unwrap();
    assert_eq!(result.status, S::Exceeds);
}
#[test]
fn work_exhaustion_and_invalid_original_correspondences_never_admit() {
    let s = source();
    let m = map();
    let c = cap(&m);
    let result = certify(&s, &c, &m, 0.1, 1e-12, 1).unwrap();
    assert_eq!(result.status, S::Unresolved);
    assert_eq!(result.cells, 1);
    assert!(result.error_upper.is_none());
    let mut outside = m.clone();
    outside.control_points[0][0][0] = -0.1;
    let mut clock = m.clone();
    clock.knots_u = vec![0., 0., 1., 1.];
    let mut nonuv = m.clone();
    nonuv.control_points[0][0][2] = 0.1;
    let mut reversed = m.clone();
    reversed.control_points.reverse();
    reversed.weights.reverse();
    for bad in [outside, clock, nonuv, reversed] {
        assert!(certify(&s, &c, &bad, 0.1, 0.005, 16384).is_err());
    }
}
