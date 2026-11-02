use nurbs_core::{
    surface::{Axis, Surface},
    surface_offset::{CandidateStatus as S, certify_quadrilateral_candidate as certify},
};

const CORNERS: [[[f64; 2]; 2]; 2] = [[[3., -2.], [3.3, 0.]], [[6., -1.8], [6.2, -0.2]]];
fn patch(corners: [[[f64; 2]; 2]; 2], z: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: corners
            .map(|row| row.map(|p| vec![p[0], p[1], z]).to_vec())
            .to_vec(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn source() -> Surface {
    patch([[[2., -3.], [2., 1.]], [[7., -3.], [7., 1.]]], 0.)
}
fn candidate() -> Surface {
    let mut s = patch(CORNERS, 0.1);
    s.knots_u = vec![-4., -4., 2., 2.];
    s.knots_v = vec![10., 10., 14., 14.];
    s
}
#[test]
fn nonparallel_uv_sides_and_nonunit_domains_have_complete_offset_bounds() {
    let s = source();
    let mut c = candidate();
    // A projective gauge is not an authored parameter correspondence.
    c.weights = vec![vec![7.; 2]; 2];
    let before = (s.clone(), c.clone());
    let r = certify(&s, &c, CORNERS, 0.1, 0.002, 8192).unwrap();
    assert_eq!(r.status, S::Within);
    assert!(r.error_upper.unwrap() <= 0.002);
    assert!(r.cells > 1 && r.cells <= 8192);
    assert_eq!((s, c), before);
}
#[test]
fn interior_bulge_is_not_authorized_by_unchanged_boundary_curves() {
    let s = source();
    let c = nurbs_core::surface_edit::elevate(&candidate(), Axis::U, 2).unwrap();
    let mut c = nurbs_core::surface_edit::elevate(&c, Axis::V, 2).unwrap();
    let original = c.clone();
    c.control_points[1][1][2] += 0.08;
    let n = c.control_points.len();
    let m = c.control_points[0].len();
    for i in 0..n {
        for j in 0..m {
            if i == 0 || i == n - 1 || j == 0 || j == m - 1 {
                assert_eq!(c.control_points[i][j], original.control_points[i][j]);
            }
        }
    }
    let r = certify(&s, &c, CORNERS, 0.1, 0.002, 8192).unwrap();
    assert_eq!(r.status, S::Exceeds);
    assert!(r.error_upper.is_none());
}
#[test]
fn incomplete_budget_retains_unresolved_status() {
    let r = certify(&source(), &candidate(), CORNERS, 0.1, 1e-12, 1).unwrap();
    assert_eq!(r.status, S::Unresolved);
    assert_eq!(r.cells, 1);
    assert!(r.error_upper.is_none());
}
#[test]
fn oblique_original_normals_and_negative_offsets_keep_both_uv_chain_terms() {
    let mut s = source();
    for p in s.control_points.iter_mut().flatten() {
        p[2] = 0.5 * p[0] + 0.25 * p[1];
    }
    let mut c = candidate();
    let length = 1.3125_f64.sqrt();
    let distance = -0.1;
    let normal = [-0.5 / length, -0.25 / length, 1. / length];
    for p in c.control_points.iter_mut().flatten() {
        p[2] = 0.5 * p[0] + 0.25 * p[1];
        for k in 0..3 {
            p[k] += distance * normal[k];
        }
    }
    let r = certify(&s, &c, CORNERS, distance, 0.002, 8192).unwrap();
    assert_eq!(r.status, S::Within);
    assert!(r.error_upper.unwrap() <= 0.002);
    let wrong = certify(&s, &c, CORNERS, -distance, 0.002, 8192).unwrap();
    assert_eq!(wrong.status, S::Exceeds);
}
#[test]
fn folded_reversed_outside_and_nonfinite_maps_refuse() {
    let s = source();
    let c = candidate();
    let mut folded = CORNERS;
    folded[1].swap(0, 1);
    let mut reversed = CORNERS;
    reversed.swap(0, 1);
    let mut outside = CORNERS;
    outside[0][0][0] = 1.;
    let mut nan = CORNERS;
    nan[0][0][1] = f64::NAN;
    for corners in [folded, reversed, outside, nan, [[[3., -2.]; 2]; 2]] {
        assert!(certify(&s, &c, corners, 0.1, 0.002, 8192).is_err());
    }
    assert!(certify(&s, &c, CORNERS, 0.1, 0.002, 0).is_err());
}
