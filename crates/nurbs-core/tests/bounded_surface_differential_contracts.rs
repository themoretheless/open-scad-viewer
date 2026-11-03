use nurbs_core::{
    editing::{
        bounded_surface_differential::{self as b, Axis, Field, Refusal},
        exact_surface_derivative,
    },
    surface::Surface,
};
fn rational() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 1.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 5.]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[cfg(feature = "codec")]
fn name(f: Field) -> String {
    match f {
        Field::Partial(Axis::U) => "partial_u".into(),
        Field::Partial(Axis::V) => "partial_v".into(),
        Field::Mixed([u, v]) => format!("mixed_{u}_{v}"),
        Field::AreaNormal => "area_normal".into(),
        Field::Metric => "metric".into(),
    }
}
fn accepted(s: &Surface, field: Field, tolerance: f64) -> b::Report {
    let before = s.clone();
    let r = b::surface(s, field, tolerance, 1_000_000).unwrap();
    assert!(r.refusal.is_none(), "{:?},work={}", r.refusal, r.work_used);
    assert_eq!(*s, before);
    let patches = r.patches.as_ref().unwrap();
    let bounds = r.error_upper.as_ref().unwrap();
    let mut domains = Vec::new();
    for ku in s.degree_u..s.control_points.len() {
        for kv in s.degree_v..s.control_points[0].len() {
            if s.knots_u[ku] < s.knots_u[ku + 1] && s.knots_v[kv] < s.knots_v[kv + 1] {
                domains.push((
                    [s.knots_u[ku], s.knots_u[ku + 1]],
                    [s.knots_v[kv], s.knots_v[kv + 1]],
                ));
            }
        }
    }
    assert_eq!(patches.len(), domains.len());
    assert_eq!(bounds.len(), patches.len());
    for ((p, b), (u, v)) in patches.iter().zip(bounds).zip(domains) {
        p.validate().unwrap();
        assert_eq!(
            [p.knots_u[p.degree_u], p.knots_u[p.control_points.len()]],
            u
        );
        assert_eq!(
            [p.knots_v[p.degree_v], p.knots_v[p.control_points[0].len()]],
            v
        );
        assert!(!p.periodic_u && !p.periodic_v);
        assert!(b.is_finite() && *b >= 0. && *b <= tolerance);
    }
    #[cfg(feature = "codec")]
    println!(
        "bounded-surface-differential-oracle={}",
        value_codec::json!({"source":s,"field":name(field),"tolerance":tolerance,"patches":patches,"bounds":bounds})
    );
    r
}
#[test]
fn rational_partials_admit_inexact_storage_with_whole_cell_bounds() {
    let s = rational();
    assert_eq!(
        exact_surface_derivative::surface(&s, Axis::U, 1_000_000)
            .unwrap()
            .refusal,
        Some(exact_surface_derivative::Refusal::NotRepresentable)
    );
    for a in [Axis::U, Axis::V] {
        let r = accepted(&s, Field::Partial(a), 1e-8);
        assert!(r.error_upper.unwrap()[0] > 0.);
    }
}
#[test]
fn area_normal_and_metric_share_one_complete_tensor_gauge() {
    for f in [Field::AreaNormal, Field::Metric] {
        accepted(&rational(), f, 1e-8);
    }
}
#[test]
fn higher_and_mixed_partials_are_built_before_rounding() {
    for orders in [[2, 0], [1, 1], [0, 2], [3, 0], [2, 1], [1, 2], [0, 3]] {
        accepted(&rational(), Field::Mixed(orders), 1e-8);
    }
}
#[test]
fn exact_polynomial_plane_retains_zero_error_at_subnormal_tolerance() {
    let mut s = rational();
    s.weights = vec![vec![1.; 2]; 2];
    for p in s.control_points.iter_mut().flatten() {
        p[2] = 0.;
    }
    for f in [Field::Partial(Axis::U), Field::AreaNormal, Field::Metric] {
        let r = accepted(&s, f, f64::from_bits(1));
        assert_eq!(r.error_upper.unwrap(), vec![0.]);
    }
}
#[test]
fn original_absolute_parameter_units_are_retained() {
    let mut s = rational();
    s.knots_u = vec![-2., -2., 2., 2.];
    s.knots_v = vec![3., 3., 5., 5.];
    accepted(&s, Field::Partial(Axis::U), 1e-8);
    accepted(&s, Field::Mixed([1, 1]), 1e-8);
}
#[test]
fn too_small_tolerance_is_an_atomic_refusal() {
    let r = b::surface(&rational(), Field::Partial(Axis::U), 1e-30, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::ToleranceExceeded));
    assert!(r.patches.is_none() && r.error_upper.is_none());
}
#[test]
fn final_rounding_work_is_part_of_the_shared_budget() {
    let s = rational();
    let full = b::surface(&s, Field::Partial(Axis::U), 1e-8, 1_000_000).unwrap();
    assert!(full.patches.is_some());
    let r = b::surface(&s, Field::Partial(Axis::U), 1e-8, full.work_used - 1).unwrap();
    assert_eq!(r.refusal, Some(Refusal::ResourceLimit));
    assert!(r.patches.is_none() && r.error_upper.is_none());
}
#[test]
fn invalid_orders_tolerances_budgets_and_sources_are_errors() {
    let s = rational();
    for orders in [[0, 0], [4, 0], [usize::MAX, 1]] {
        assert!(b::surface(&s, Field::Mixed(orders), 1e-8, 1_000_000).is_err());
    }
    for tol in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(b::surface(&s, Field::Metric, tol, 1_000_000).is_err());
    }
    for work in [0, 1_000_001] {
        assert!(b::surface(&s, Field::Metric, 1e-8, work).is_err());
    }
    let mut invalid = s;
    invalid.weights[0][0] = 0.;
    assert!(b::surface(&invalid, Field::Metric, 1e-8, 1_000_000).is_err());
}
#[test]
fn nonuniform_cells_keep_u_major_v_minor_complete_cover() {
    let gu = [0., 0.5, 2., 3.];
    let gv = [-2., -1., 1., 2.];
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 3., 3., 3.],
        knots_v: vec![-2., -2., -2., 0., 2., 2., 2.],
        control_points: gu
            .iter()
            .map(|&u| gv.iter().map(|&v| vec![u, v, u * v]).collect())
            .collect(),
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    for f in [Field::Partial(Axis::U), Field::Mixed([1, 1]), Field::Metric] {
        assert_eq!(accepted(&s, f, 1e-8).patches.unwrap().len(), 4);
    }
}
#[test]
fn both_periodic_axes_keep_unwrapped_one_sided_traces() {
    let a = [[0., 0., 0.], [2., 0., 0.], [0., 2., 0.], [0., 0., 0.]];
    let z = [[0., 0., 0.], [0., 0., 2.], [1., 1., 1.], [0., 0., 0.]];
    let w = [1., 2., 4., 1.];
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1., 0., 1., 2., 3., 4.],
        knots_v: vec![-1., 0., 1., 2., 3., 4.],
        control_points: a
            .iter()
            .map(|a| {
                z.iter()
                    .map(|b| (0..3).map(|k| a[k] + b[k]).collect())
                    .collect()
            })
            .collect(),
        weights: w
            .iter()
            .map(|a| w.iter().map(|b| a * b).collect())
            .collect(),
        periodic_u: true,
        periodic_v: true,
    };
    assert_eq!(
        accepted(&s, Field::Partial(Axis::U), 1e-8)
            .patches
            .unwrap()
            .len(),
        9
    );
}
#[test]
fn unclamped_tensor_retains_all_nine_original_cells() {
    let g = [-0.5, 0.5, 1.5, 2.5, 3.5];
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![-2., -1., 0., 1., 2., 3., 4., 5.],
        knots_v: vec![-2., -1., 0., 1., 2., 3., 4., 5.],
        control_points: g
            .iter()
            .map(|&u| g.iter().map(|&v| vec![u, v, u * v]).collect())
            .collect(),
        weights: vec![vec![1.; 5]; 5],
        periodic_u: false,
        periodic_v: false,
    };
    assert_eq!(accepted(&s, Field::Metric, 1e-8).patches.unwrap().len(), 9);
}
#[test]
fn late_tolerance_refusal_discards_earlier_exact_patch() {
    let x = [0., 1., 2.];
    let w = [1., 1., 3.];
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 2., 2.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: x
            .iter()
            .map(|&x| vec![vec![x, 0., 0.], vec![x, 1., 0.]])
            .collect(),
        weights: w.iter().map(|&w| vec![w; 2]).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let r = b::surface(&s, Field::Partial(Axis::U), 1e-30, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::ToleranceExceeded));
    assert!(r.patches.is_none() && r.error_upper.is_none());
}
#[test]
fn degenerate_source_has_zero_fields_without_regularity_credit() {
    let mut s = rational();
    for p in s.control_points.iter_mut().flatten() {
        *p = vec![1., 2., 3.];
    }
    for f in [Field::AreaNormal, Field::Metric] {
        assert_eq!(
            accepted(&s, f, f64::from_bits(1)).error_upper.unwrap(),
            vec![0.]
        );
    }
}
#[test]
fn tolerance_cannot_relax_geometry_limits() {
    let mut s = rational();
    s.weights = vec![vec![1.; 2]; 2];
    for p in s.control_points.iter_mut().flatten() {
        p[0] *= 1e9;
        p[2] = 0.;
    }
    let r = b::surface(&s, Field::Metric, 1e30, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::InvalidOutput));
    assert!(r.patches.is_none() && r.error_upper.is_none());
}
#[test]
fn output_weight_conditioning_limits_are_not_relaxed() {
    let mut s = rational();
    s.weights = vec![vec![1.; 2], vec![1e6; 2]];
    for p in s.control_points.iter_mut().flatten() {
        p[0] *= 1e-18;
        p[2] = 0.;
    }
    let r = b::surface(&s, Field::Mixed([3, 0]), 1e-6, 1_000_000).unwrap();
    assert_eq!(r.refusal, Some(Refusal::InvalidOutput));
    assert!(r.patches.is_none() && r.error_upper.is_none());
}
#[test]
fn parameter_transposition_reverses_oriented_area_field() {
    let s = rational();
    let t = Surface {
        degree_u: s.degree_v,
        degree_v: s.degree_u,
        knots_u: s.knots_v.clone(),
        knots_v: s.knots_u.clone(),
        control_points: (0..2)
            .map(|j| (0..2).map(|i| s.control_points[i][j].clone()).collect())
            .collect(),
        weights: (0..2)
            .map(|j| (0..2).map(|i| s.weights[i][j]).collect())
            .collect(),
        periodic_u: false,
        periodic_v: false,
    };
    let a = accepted(&s, Field::AreaNormal, 1e-8).patches.unwrap();
    let b = accepted(&t, Field::AreaNormal, 1e-8).patches.unwrap();
    for u in [0., 0.5, 1.] {
        for v in [0., 0.5, 1.] {
            let x = a[0].evaluate(u, v).unwrap().point;
            let y = b[0].evaluate(v, u).unwrap().point;
            for k in 0..3 {
                assert!((x[k] + y[k]).abs() < 1e-8);
            }
        }
    }
}
