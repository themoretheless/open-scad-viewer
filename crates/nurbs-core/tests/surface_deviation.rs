use nurbs_core::{
    continuity::deviation::positional_upper,
    surface::{Axis, Surface},
    surface_edit,
};
fn panel() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 4., 0.]],
            vec![vec![4., 0., 0.], vec![4., 4., 0.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 6.]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn translation_bound_encloses_exact_corresponding_error() {
    let a = panel();
    assert_eq!(positional_upper(&a, &a).unwrap(), 0.);
    let mut b = a.clone();
    for row in &mut b.control_points {
        for p in row {
            p[0] += 3.;
            p[2] += 4.;
        }
    }
    let bound = positional_upper(&a, &b).unwrap();
    assert!(bound >= 5. && bound.is_finite());
    assert!(positional_upper(&b, &a).unwrap() >= 5.);
}
#[test]
fn narrow_peak_is_not_missed_between_display_samples() {
    let a = panel();
    let mut b = a.clone();
    b.knots_u = vec![2., 2., 2.0001, 2.0002, 7., 7.];
    b.weights = vec![vec![1.; 2]; 4];
    b.control_points = vec![
        vec![vec![0., 0., 0.], vec![0., 4., 0.]],
        vec![vec![0., 0., 1.], vec![0., 4., 1.]],
        vec![vec![0., 0., 0.], vec![0., 4., 0.]],
        vec![vec![0., 0., 0.], vec![0., 4., 0.]],
    ];
    let mut flat = b.clone();
    for row in &mut flat.control_points {
        for p in row {
            p[2] = 0.;
        }
    }
    flat.knots_u = vec![2., 2., 7., 7.];
    flat.control_points = vec![flat.control_points[0].clone(); 2];
    flat.weights = vec![vec![1.; 2]; 2];
    let upper = positional_upper(&flat, &b).unwrap();
    assert!(upper >= 1. && upper < 1.00001, "{upper}");
}
#[test]
fn different_bases_and_invalid_domains_are_handled() {
    let a = panel();
    let b = surface_edit::elevate(
        &surface_edit::insert(&a, Axis::U, 3., 1).unwrap(),
        Axis::V,
        2,
    )
    .unwrap();
    let upper = positional_upper(&a, &b).unwrap();
    assert!(upper < 1e-9, "{upper}");
    let mut wrong = a.clone();
    wrong.knots_u = vec![0., 0., 1., 1.];
    assert!(positional_upper(&a, &wrong).is_err());
    let mut invalid = a.clone();
    invalid.weights[0][0] = 0.;
    assert!(positional_upper(&a, &invalid).is_err());
}
