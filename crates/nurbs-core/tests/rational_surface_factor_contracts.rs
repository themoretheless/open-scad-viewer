use nurbs_core::{
    curve::Curve,
    editing::rational_surface_factor::{Refusal, cancel},
    surface::{Axis, Surface},
};
fn surface(x: &[f64], weights: &[f64]) -> Surface {
    let p = x.len() - 1;
    Surface {
        degree_u: p,
        degree_v: 1,
        knots_u: [vec![-2.; p + 1], vec![6.; p + 1]].concat(),
        knots_v: vec![3., 3., 7., 7.],
        control_points: x
            .iter()
            .map(|&x| [0., 3.].iter().map(|&y| vec![x, y, 2. * x + y]).collect())
            .collect(),
        weights: weights.iter().map(|&w| vec![w, 3. * w]).collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn simple() -> Surface {
    surface(&[0., 0.5, 1.], &[1., 2., 4.])
}
fn transpose(s: &Surface) -> Surface {
    Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v.clone(),
        knots_v: s.knots_u.clone(),
        control_points: (0..s.control_points[0].len())
            .map(|j| s.control_points.iter().map(|r| r[j].clone()).collect())
            .collect(),
        weights: (0..s.weights[0].len())
            .map(|j| s.weights.iter().map(|r| r[j]).collect())
            .collect(),
        periodic_u: s.periodic_v,
        periodic_v: s.periodic_u,
    }
}
fn dump(s: &Surface) -> String {
    format!(
        "{{\"degreeU\":{},\"degreeV\":{},\"knotsU\":{:?},\"knotsV\":{:?},\"controlPoints\":{:?},\"weights\":{:?}}}",
        s.degree_u, s.degree_v, s.knots_u, s.knots_v, s.control_points, s.weights
    )
}
fn accepted(source: &Surface, axis: Axis) -> Surface {
    let r = cancel(source, axis, 1_000_000).unwrap();
    assert!(r.simplified, "{:?}", r.refusal);
    assert_eq!(r.refusal, None);
    assert!(r.work_used <= 1_000_000);
    assert_eq!(
        r.rows_verified,
        match axis {
            Axis::U => source.control_points[0].len(),
            Axis::V => source.control_points.len(),
        }
    );
    println!(
        "surface-factor-oracle={{\"source\":{},\"output\":{}}}",
        dump(source),
        dump(&r.surface)
    );
    r.surface
}
#[test]
fn both_axes_cancel_the_same_factor_and_keep_nonconstant_tensor_clocks() {
    let s = simple();
    let u = accepted(&s, Axis::U);
    assert_eq!(u.degree_u, 1);
    assert_eq!(u.knots_u, vec![-2., -2., 6., 6.]);
    assert_eq!(u.knots_v, s.knots_v);
    assert_eq!(u.weights, vec![vec![0.25, 0.75], vec![0.5, 1.5]]);
    let v = accepted(&transpose(&s), Axis::V);
    assert_eq!(v, transpose(&u));
    for i in 0..=16 {
        for j in 0..=16 {
            let t = i as f64 / 16.;
            let q = j as f64 / 16.;
            let p = u.evaluate(-2. + 8. * t, 3. + 4. * q).unwrap().point;
            let x = 2. * t / (1. + t);
            let y = 9. * q / (1. + 2. * q);
            for (actual, expected) in p.into_iter().zip([x, y, 2. * x + y]) {
                assert!((actual - expected).abs() < 1e-12);
            }
        }
    }
    assert_eq!(s, simple());
}
#[test]
fn independently_reducible_rows_do_not_prove_a_surface_factor() {
    let mut s = surface(&[0., 0., 0.], &[1., 2., 4.]);
    for (i, w) in [4., 2., 1.].iter().enumerate() {
        s.weights[i][1] = *w;
    }
    for j in 0..2 {
        let c = Curve {
            degree: 2,
            knots: s.knots_u.clone(),
            control_points: s.control_points.iter().map(|r| r[j].clone()).collect(),
            weights: s.weights.iter().map(|r| r[j]).collect(),
            periodic: false,
        };
        assert!(
            nurbs_core::editing::rational_factor::cancel(&c, 1_000_000)
                .unwrap()
                .simplified
        );
    }
    let r = cancel(&s, Axis::U, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::NoCommonFactor));
    assert!(!r.simplified);
    assert_eq!(r.surface, s);
    assert!((s.evaluate(-2., 5.).unwrap().point[1] - 2.4).abs() < 1e-12);
}
#[test]
fn one_ulp_in_a_later_row_destroys_the_common_factor() {
    let mut s = simple();
    s.control_points[1][1][0] = s.control_points[1][1][0].next_up();
    let r = cancel(&s, Axis::U, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::NoCommonFactor));
    assert!(!r.simplified);
    assert_eq!(r.surface, s);
}
#[test]
fn positive_storage_can_require_elevation_of_the_primitive_quotient() {
    let s = surface(&[0., 0., 2., 4.5, 3.], &[6., 3.75, 1.5, 3., 12.]);
    let r = accepted(&s, Axis::U);
    assert_eq!(r.degree_u, 3);
    assert_eq!(r.control_points[2][0][0], 6.);
    assert!(r.weights.iter().flatten().all(|&w| w > 0.));
}
#[test]
fn negative_monic_factor_has_one_global_positive_output_gauge() {
    let s = surface(&[0., 0.5, 1.], &[4., 2., 1.]);
    let r = accepted(&s, Axis::U);
    assert_eq!(r.degree_u, 1);
    assert!(r.weights.iter().flatten().all(|&w| w > 0.));
}
#[test]
fn nonrepresentable_primitive_never_becomes_an_approximate_surface() {
    let s = surface(&[0., 0.25, 0.5, 1.], &[3., 8., 16., 24.]);
    let r = cancel(&s, Axis::U, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::NotRepresentable));
    assert!(!r.simplified);
    assert_eq!(r.surface, s);
}
#[test]
fn shared_budget_covers_all_rows_and_final_literal_proof() {
    let s = simple();
    let full = cancel(&s, Axis::U, 1_000_000).unwrap();
    assert!(full.simplified);
    for budget in [1, 100, full.work_used - 1] {
        let r = cancel(&s, Axis::U, budget).unwrap();
        assert!(!r.simplified);
        assert_eq!(r.surface, s);
        assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
        assert!(r.work_used <= budget);
    }
    let exact = cancel(&s, Axis::U, full.work_used).unwrap();
    assert!(exact.simplified);
    assert_eq!(exact.surface, full.surface);
}
#[test]
fn untouched_periodic_storage_and_invalid_full_tail() {
    let mut s = simple();
    s.degree_v = 2;
    s.knots_v = vec![-2., -1., 0., 1., 2., 3., 4., 5.];
    s.periodic_v = true;
    for i in 0..3 {
        let x = s.control_points[i][0][0];
        s.control_points[i] = [0., 2., -2., 0., 2.]
            .iter()
            .map(|&y| vec![x, y, 2. * x + y])
            .collect();
        s.weights[i] = vec![[1., 2., 4.][i]; 5];
    }
    s.validate().unwrap();
    let r = accepted(&s, Axis::U);
    assert!(r.periodic_v);
    assert_eq!(r.knots_v, s.knots_v);
    assert!(cancel(&s, Axis::V, 1_000_000).is_err());
    let mut bad = s.clone();
    bad.weights[2][4] = f64::NAN;
    assert!(cancel(&bad, Axis::U, 1_000_000).is_err());
    for budget in [0, 1_000_001] {
        assert!(cancel(&s, Axis::U, budget).is_err());
    }
    let multi = nurbs_core::surface_edit::insert(&simple(), Axis::U, 2., 1).unwrap();
    assert!(cancel(&multi, Axis::U, 1_000_000).is_err());
}
#[test]
fn maximum_degree_constant_axis_keeps_existing_limits() {
    let s = surface(
        &[3.; 26],
        &(0..26).map(|i| 2f64.powi(i)).collect::<Vec<_>>(),
    );
    let r = accepted(&s, Axis::U);
    assert_eq!(r.degree_u, 1);
}
