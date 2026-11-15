use nurbs_core::{
    editing::{bounded_surface_differential as bounded, exact_surface_derivative as exact},
    surface::Surface,
};

const WORK: u64 = 1_000_000;

fn saddle() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 1.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}

fn check(s: &Surface, t: f64, v: f64, u: f64, scale: f64) {
    let r = bounded::squared_principal_curvature_gap(s, 1e-10, WORK).unwrap();
    let patches = r.patches.unwrap_or_else(|| panic!("{:?}", r.refusal));
    let upper = r.error_upper.unwrap()[0];
    assert!((0. ..=1e-10).contains(&upper));
    let d = 1. + u * u + v * v;
    let expected = 4. * (d + u * u * v * v) / (scale * scale * d.powi(3));
    let p = patches[0].evaluate(t, v).unwrap().point;
    assert!((p[0] - expected).abs() <= upper + 1e-14);
    assert_eq!(&p[1..], &[0., 0.]);
}

#[test]
fn saddle_resolves_nonzero_gap_even_where_the_mean_curvature_is_zero() {
    let s = saddle();
    assert!(
        exact::squared_principal_curvature_gap(&s, WORK)
            .unwrap()
            .patches
            .is_none()
    );
    check(&s, 0., 0., 0., 1.);
    check(&s, 0.25, 0.5, 0.25, 1.);
    check(&s, 1., 1., 1., 1.);
}

#[test]
fn exact_cylindrical_patch_has_unit_squared_gap() {
    let mut s = saddle();
    s.degree_u = 2;
    s.knots_u = vec![0., 0., 0., 1., 1., 1.];
    s.control_points = [(1., 0.), (1., 1.), (0., 1.)]
        .map(|(x, y)| vec![vec![x, y, 0.], vec![x, y, 1.]])
        .to_vec();
    s.weights = vec![vec![1.; 2], vec![1.; 2], vec![2.; 2]];
    let r = exact::squared_principal_curvature_gap(&s, WORK).unwrap();
    let patches = r.patches.unwrap_or_else(|| panic!("{:?}", r.refusal));
    assert!(
        patches[0]
            .control_points
            .iter()
            .flatten()
            .all(|p| p == &[1., 0., 0.])
    );
}

#[test]
fn reversals_and_scaling_preserve_geometric_meaning() {
    let mut s = saddle();
    s.control_points.reverse();
    for x in s.control_points.iter_mut().flatten().flatten() {
        *x *= 2.;
    }
    check(&s, 0.75, 0.5, 0.25, 2.);
}

#[test]
fn rational_reclock_preserves_the_gap() {
    let mut s = saddle();
    s.weights[1] = vec![2.; 2];
    s.control_points[1][0][0] = 0.5;
    s.control_points[1][1][0] = 0.5;
    s.control_points[1][1][2] = 0.5;
    check(&s, 0.5, 0.25, 1. / 3., 1.);
}

#[test]
fn regular_umbilic_is_valid_but_singular_surface_is_not() {
    let mut s = saddle();
    s.control_points[1][1][2] = 0.;
    let r = exact::squared_principal_curvature_gap(&s, WORK).unwrap();
    assert!(
        r.patches.unwrap()[0]
            .control_points
            .iter()
            .flatten()
            .all(|p| p == &[0., 0., 0.])
    );
    s.control_points[1] = s.control_points[0].clone();
    let r = bounded::squared_principal_curvature_gap(&s, 1., WORK).unwrap();
    assert_eq!(r.refusal, Some(bounded::Refusal::UnprovedDenominator));
    assert!(r.patches.is_none() && r.error_upper.is_none());
}
