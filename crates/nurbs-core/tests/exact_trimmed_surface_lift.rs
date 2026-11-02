use cad_predicates::{BezierIdentity as B, Reason};
use nurbs_core::{
    curve::Curve, curve_surface_agreement::verify_exact_trimmed as verify, surface::Surface,
};
fn curve(points: Vec<Vec<f64>>, weights: Vec<f64>, d: [f64; 2]) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: [vec![d[0]; degree + 1], vec![d[1]; degree + 1]].concat(),
        control_points: points,
        weights,
        periodic: false,
    }
}
fn plane(size: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., size, size],
        knots_v: vec![0., 0., size, size],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., size, 0.]],
            vec![vec![size, 0., 0.], vec![size, size, 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn rational_curve_and_uv_keep_independent_partial_clocks_and_reverse_world_once() {
    let c = curve(
        vec![vec![0., 0., 0.], vec![1., 2., 0.], vec![2., 0., 0.]],
        vec![1., 2., 1.],
        [-2., 6.],
    );
    let p = curve(
        vec![vec![0., 0.], vec![1., 2.], vec![2., 0.]],
        vec![1., 2., 1.],
        [10., 18.],
    );
    let s = plane(4.);
    assert_eq!(
        verify(&c, &p, &s, false, [0., 4.], [12., 16.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
    assert_eq!(
        verify(&c, &p, &s, true, [0., 4.], [16., 12.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
    assert_eq!(
        verify(&c, &p, &s, true, [4., 0.], [12., 16.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
    assert_eq!(
        verify(&c, &p, &s, true, [0., 4.], [12., 16.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Different
    );
    let mut bad = c.clone();
    bad.control_points[1][2] = 2f64.powi(-45);
    assert_eq!(
        verify(&bad, &p, &s, false, [0., 4.], [12., 16.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Different
    );
    assert!(matches!(
        verify(&c, &p, &s, false, [0., 4.], [12., 16.], 1)
            .unwrap()
            .unwrap()
            .outcome,
        B::Indeterminate(Reason::ResourceLimit)
    ));
}
#[test]
fn restricted_uv_hull_can_be_inside_chart_even_when_original_poles_are_outside() {
    let c = curve(
        vec![vec![0., 0., 0.], vec![1., 0., 0.]],
        vec![1.; 2],
        [0., 1.],
    );
    let p = curve(vec![vec![-1., 0.], vec![3., 0.]], vec![1.; 2], [2., 6.]);
    let s = plane(1.);
    assert_eq!(
        verify(&c, &p, &s, false, [0., 1.], [3., 4.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
    assert!(matches!(
        verify(&c, &p, &s, false, [0., 1.], [2., 4.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Indeterminate(Reason::MissingProof)
    ));
    assert!(verify(&c, &p, &s, false, [0., 1.], [1., 4.], 1000000).is_err());
}
#[test]
fn rational_surface_boundary_matches_the_original_trimmed_world_curve() {
    let mut s = plane(1.);
    s.control_points[1][0][0] = 2.;
    s.control_points[1][1][0] = 2.;
    s.weights[1] = vec![3.; 2];
    let c = curve(
        vec![vec![0., 0., 0.], vec![2., 0., 0.]],
        vec![1., 3.],
        [0., 1.],
    );
    let p = curve(vec![vec![0., 0.], vec![1., 0.]], vec![1.; 2], [0., 1.]);
    assert_eq!(
        verify(&c, &p, &s, false, [0.125, 0.75], [0.125, 0.75], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
}

#[test]
fn physical_trim_clocks_preserve_exact_non_dyadic_normalized_ends() {
    let c = curve(
        vec![vec![0., 0., 0.], vec![9., 0., 0.]],
        vec![1., 4.],
        [-2., 4.],
    );
    let p = curve(vec![vec![0., 0.], vec![9., 0.]], vec![1., 4.], [-4., 8.]);
    assert_eq!(
        verify(&c, &p, &plane(10.), false, [0., 2.], [0., 4.], 1000000)
            .unwrap()
            .unwrap()
            .outcome,
        B::Equal
    );
}
