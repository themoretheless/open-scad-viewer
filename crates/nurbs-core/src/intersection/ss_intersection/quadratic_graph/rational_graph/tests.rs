use super::*;
use crate::ss_intersection::{GraphCurveTolerances, construct_graph_curves};
fn rational_patch(radius2: f64, tilted: bool) -> Surface {
    let x = [0., 0.0625, 0.5, 0.9375, 1.];
    // Exact Bernstein representation of 30*u^2 times W, divided by
    // W's degree-four Bernstein weights [1,4,5,4,1]. W=1+12u-12u^2.
    let square = [0., 0., 1., 26.25, 30.];
    let vheight = [0., -15., 0.];
    Surface {
        degree_u: 4,
        degree_v: 2,
        knots_u: vec![10., 10., 10., 10., 10., 18., 18., 18., 18., 18.],
        knots_v: vec![7., 7., 7., 11., 11., 11.],
        control_points: (0..5)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        let y = j as f64 * 0.5;
                        vec![
                            x[i],
                            y,
                            square[i] - 30. * x[i] + vheight[j] + 15. - 30. * radius2
                                + if tilted {
                                    0.5 * x[i] - 0.25 * y + 0.125
                                } else {
                                    0.
                                },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: [1., 4., 5., 4., 1.]
            .into_iter()
            .map(|v| vec![v; 3])
            .collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
fn plane(source: &Surface, tilted: bool) -> Surface {
    let mut p = source.clone();
    for row in &mut p.control_points {
        for xyz in row {
            xyz[2] = if tilted {
                0.5 * xyz[0] - 0.25 * xyz[1] + 0.125
            } else {
                0.
            };
        }
    }
    // Different actual native clocks retain the same normalized XY frame.
    p.knots_u = vec![-2.; 5];
    p.knots_u.extend(vec![2.; 5]);
    p.knots_v = vec![3.; 3];
    p.knots_v.extend(vec![7.; 3]);
    p
}
fn limits() -> QuadraticGraphSSLimits {
    QuadraticGraphSSLimits {
        max_operations: 4096,
    }
}
#[test]
fn original_unequal_weights_have_complete_closed_tangent_and_empty_censuses() {
    for (radius2, kind) in [
        (0.0625, QuadraticGraphSSKind::ClosedEllipse),
        (0., QuadraticGraphSSKind::IsolatedTangentPoint),
        (-0.0625, QuadraticGraphSSKind::Empty),
    ] {
        let a = rational_patch(radius2, true);
        let b = plane(&a, true);
        let saved = [a.clone(), b.clone()];
        let source = FiberSources::new(&a, &b).unwrap();
        let owner = qualify_quadratic_graph_ss(&source, limits()).unwrap();
        assert_eq!(owner.kind(), kind);
        assert!(owner.covers_entire_surfaces());
        assert_eq!(owner.source().first(), &saved[0]);
        assert_eq!(owner.source().second(), &saved[1]);
        owner.reverify().unwrap();
        if kind == QuadraticGraphSSKind::ClosedEllipse {
            assert_eq!(owner.branches().len(), 1);
            assert!(owner.branches()[0].closed);
            assert_eq!(owner.edges().len(), 4);
            let curves = construct_graph_curves(
                &owner,
                GraphCurveTolerances {
                    world: 1e-7,
                    uv: 1e-9,
                },
                Default::default(),
            )
            .unwrap();
            curves.reverify().unwrap();
            for edge in 0..4 {
                for t in [0., 0.125, 0.5, 0.875, 1.] {
                    let range = owner.edge_enclosure(edge, [t, t], 4096).unwrap();
                    let u = (range.first_uv[0][0] + range.first_uv[0][1]) * 0.5;
                    let v = (range.first_uv[1][0] + range.first_uv[1][1]) * 0.5;
                    let p = a.evaluate(u, v).unwrap().point;
                    let x = (u - 10.) / 8.;
                    let y = (v - 7.) / 4.;
                    // Independent original-support and analytic-oracle checks.
                    assert!(((x - 0.5).powi(2) + (y - 0.5).powi(2) - radius2).abs() < 1e-12);
                    assert!((p[2] - (0.5 * x - 0.25 * y + 0.125)).abs() < 1e-11);
                    for k in 0..3 {
                        assert!(
                            range.world[k][0] - 1e-11 <= p[k] && p[k] <= range.world[k][1] + 1e-11
                        );
                    }
                }
            }
        }
        let swapped = FiberSources::new(&b, &a).unwrap();
        assert_eq!(
            qualify_quadratic_graph_ss(&swapped, limits())
                .unwrap()
                .kind(),
            kind
        );
        let exact = QuadraticGraphSSLimits {
            max_operations: owner.operations(),
        };
        assert!(qualify_quadratic_graph_ss(&source, exact).is_ok());
        assert!(
            qualify_quadratic_graph_ss(
                &source,
                QuadraticGraphSSLimits {
                    max_operations: exact.max_operations - 1
                }
            )
            .is_err()
        );
    }
}
#[test]
fn high_order_residuals_and_changed_original_weights_never_inherit_cancellation() {
    let a = rational_patch(0.0625, true);
    let b = plane(&a, true);
    let original = FiberSources::new(&a, &b).unwrap();
    let owner = qualify_quadratic_graph_ss(&original, limits()).unwrap();
    for mutation in 0..4 {
        let mut bad = a.clone();
        match mutation {
            0 => bad.control_points[4][2][2] = bad.control_points[4][2][2].next_up(),
            1 => bad.weights[2][1] = bad.weights[2][1].next_up(),
            2 => bad.control_points[2][1][0] = bad.control_points[2][1][0].next_up(),
            _ => bad.weights[1][1] = 0.,
        }
        let Ok(source) = FiberSources::new(&bad, &b) else {
            continue;
        };
        assert!(qualify_quadratic_graph_ss(&source, limits()).is_err());
        let mut forged = owner.clone();
        forged.source = source;
        assert!(forged.reverify().is_err());
    }
    // A Taylor prefix must not authorize a rational quotient with a late tail.
    let den = vec![vec![1.], vec![1.]];
    let numerator = vec![vec![1.], vec![1.], vec![0.], vec![1.]];
    assert!(quotient(&numerator, &den, &mut Work::new(4096).unwrap()).is_err());
}
#[test]
fn common_weight_scaling_and_coincidence_preserve_actual_source_definitions() {
    let a = rational_patch(0.0625, false);
    let mut b = a.clone();
    for v in b.weights.iter_mut().flatten() {
        *v *= 8.;
    }
    let source = FiberSources::new(&a, &b).unwrap();
    let owner = qualify_quadratic_graph_ss(&source, limits()).unwrap();
    assert_eq!(owner.kind(), QuadraticGraphSSKind::CoincidentPatch);
    assert!(owner.edges().is_empty());
    owner.reverify().unwrap();
    assert_eq!(owner.source().second().weights, b.weights);
}

fn transpose(mut s: Surface) -> Surface {
    let old = s.clone();
    s.degree_u = old.degree_v;
    s.degree_v = old.degree_u;
    s.knots_u = old.knots_v;
    s.knots_v = old.knots_u;
    s.control_points = (0..=s.degree_u)
        .map(|i| {
            (0..=s.degree_v)
                .map(|j| {
                    let mut p = old.control_points[j][i].clone();
                    p.swap(0, 1);
                    p
                })
                .collect()
        })
        .collect();
    s.weights = (0..=s.degree_u)
        .map(|i| (0..=s.degree_v).map(|j| old.weights[j][i]).collect())
        .collect();
    s
}
#[test]
fn original_rational_factors_keep_junctions_multiplicities_components_and_both_axes() {
    for (vy, constant, kind, branches) in [
        ([0., 15., 0.], 0., QuadraticGraphSSKind::CrossingLines, 2),
        ([0.; 3], 7.5, QuadraticGraphSSKind::TangentDoubleLine, 1),
        ([0.; 3], 5.625, QuadraticGraphSSKind::ParallelLineFactors, 2),
    ] {
        let mut a = rational_patch(0., false);
        let square = [0., 0., 1., 26.25, 30.];
        for i in 0..5 {
            for j in 0..3 {
                a.control_points[i][j][2] =
                    square[i] - 30. * a.control_points[i][j][0] + vy[j] + constant;
            }
        }
        let b = plane(&a, false);
        for swap_axes in [false, true] {
            let [a, b] = if swap_axes {
                [transpose(a.clone()), transpose(b.clone())]
            } else {
                [a.clone(), b.clone()]
            };
            let source = FiberSources::new(&a, &b).unwrap();
            let owner = qualify_quadratic_graph_ss(&source, limits()).unwrap();
            assert_eq!(owner.kind(), kind);
            assert_eq!(owner.branches().len(), branches);
            if kind == QuadraticGraphSSKind::CrossingLines {
                assert_eq!(owner.vertices().iter().filter(|v| v.singular).count(), 1);
            }
            let multiplicity = if kind == QuadraticGraphSSKind::TangentDoubleLine {
                2
            } else {
                1
            };
            assert!(owner.edges().iter().all(|e| e.multiplicity == multiplicity));
            construct_graph_curves(
                &owner,
                GraphCurveTolerances {
                    world: 1e-7,
                    uv: 1e-9,
                },
                Default::default(),
            )
            .unwrap()
            .reverify()
            .unwrap();
            owner.reverify().unwrap();
        }
    }
}

mod coupled;
