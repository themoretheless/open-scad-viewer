use super::*;
use crate::ss_intersection::{GraphCurveTolerances, construct_graph_curves};
fn fixture(
    swapped: bool,
    independent: bool,
) -> (QualifiedQuadraticGraphSS, curves::QualifiedGraphCurves) {
    let a = crate::polynomial::graph(
        [0., 1., 0., 1.],
        &[vec![0.4375, -1., 1.], vec![-1., 0., 0.], vec![1., 0., 0.]],
    )
    .unwrap();
    let mut b = crate::polynomial::graph([0., 1., 0., 1.], &[vec![0., 0.], vec![0., 0.]]).unwrap();
    if independent {
        b.knots_u = vec![-3., -3., 5., 5.];
        b.knots_v = vec![7., 7., 11., 11.];
    }
    let source = if swapped {
        FiberSources::new(&b, &a)
    } else {
        FiberSources::new(&a, &b)
    }
    .unwrap();
    let o = qualify_quadratic_graph_ss(&source, Default::default()).unwrap();
    let c = o
        .construct_graph_curves_with_dyadic_proposals(
            GraphCurveTolerances {
                world: 1e-7,
                uv: 1e-9,
            },
            Default::default(),
        )
        .unwrap();
    (o, c)
}
#[test]
fn original_zero_traces_keep_both_native_frames_directions_and_weight_scaling() {
    for swapped in [false, true] {
        for independent in [false, true] {
            let (o, c) = fixture(swapped, independent);
            assert_eq!(
                o.prove_closed_height_sides(4096).unwrap().0,
                [!swapped, swapped]
            );
            for e in c.edges() {
                for side in 0..2 {
                    for reverse in [false, true] {
                        let mut curve = if reverse {
                            e.pcurves[side].reverse().unwrap()
                        } else {
                            e.pcurves[side].clone()
                        };
                        for w in &mut curve.weights {
                            *w *= 8.;
                        }
                        let before = curve.clone();
                        let operations = o.prove_zero_trace(&curve, side, 4096).unwrap_or_else(|e|panic!("{e:?}: swapped={swapped} independent={independent} side={side} reverse={reverse} curve={curve:?}"));
                        assert_eq!(
                            o.prove_zero_trace(&curve, side, operations).unwrap(),
                            operations
                        );
                        assert!(o.prove_zero_trace(&curve, side, operations - 1).is_err());
                        assert_eq!(curve, before);
                    }
                }
            }
            let (_, operations) = o.prove_closed_height_sides(4096).unwrap();
            assert!(o.prove_closed_height_sides(operations).is_ok());
            assert!(o.prove_closed_height_sides(operations - 1).is_err());
        }
    }
}
#[test]
fn endpoints_or_one_ulp_residuals_never_replace_full_zero_coefficients() {
    let (o, c) = fixture(false, false);
    let original = c.edges()[0].pcurves[0].clone();
    for mutation in 0..3 {
        let mut bad = original.clone();
        match mutation {
            0 => bad.control_points[1][0] = bad.control_points[1][0].next_up(),
            1 => bad.weights[1] = bad.weights[1].next_up(),
            _ => {
                bad.degree = 1;
                bad.knots = vec![0., 0., 1., 1.];
                bad.control_points.remove(1);
                bad.weights.remove(1);
            }
        }
        // Both endpoint stations remain exactly the original saved stations.
        assert_eq!(bad.control_points[0], original.control_points[0]);
        assert_eq!(bad.control_points.last(), original.control_points.last());
        assert!(o.prove_zero_trace(&bad, 0, 4096).is_err());
    }
    o.reverify().unwrap();
    assert!(o.prove_zero_trace(&original, 2, 4096).is_err());
}

#[test]
fn explicit_proposal_mode_preserves_the_ordinary_constructor_and_sealed_replay() {
    let (o, nice) = fixture(false, true);
    let legacy = construct_graph_curves(
        &o,
        GraphCurveTolerances {
            world: 1e-7,
            uv: 1e-9,
        },
        Default::default(),
    )
    .unwrap();
    assert!(!legacy.uses_dyadic_proposals());
    assert!(nice.uses_dyadic_proposals());
    legacy.reverify().unwrap();
    nice.reverify().unwrap();
    for owner in [&legacy, &nice] {
        let limits = curves::GraphCurveBuildLimits {
            max_operations: owner.operations(),
        };
        let rebuild = if owner.uses_dyadic_proposals() {
            o.construct_graph_curves_with_dyadic_proposals(
                GraphCurveTolerances {
                    world: 1e-7,
                    uv: 1e-9,
                },
                limits,
            )
        } else {
            construct_graph_curves(
                &o,
                GraphCurveTolerances {
                    world: 1e-7,
                    uv: 1e-9,
                },
                limits,
            )
        }
        .unwrap();
        assert_eq!(owner.vertices(), rebuild.vertices());
        assert_eq!(owner.edges(), rebuild.edges());
        let short = curves::GraphCurveBuildLimits {
            max_operations: limits.max_operations - 1,
        };
        assert!(
            if owner.uses_dyadic_proposals() {
                o.construct_graph_curves_with_dyadic_proposals(
                    GraphCurveTolerances {
                        world: 1e-7,
                        uv: 1e-9,
                    },
                    short,
                )
            } else {
                construct_graph_curves(
                    &o,
                    GraphCurveTolerances {
                        world: 1e-7,
                        uv: 1e-9,
                    },
                    short,
                )
            }
            .is_err()
        );
    }
}
