use super::*;
use crate::{Limits, SourceArena, ToleranceContext};
#[test]
fn transverse_derivative_retains_small_leading_tangent_and_detects_real_transverse_term() {
    for perturbation in [0., 1e-12] {
        let points: Vec<Vec<[f64; 4]>> = (0..3)
            .map(|u| {
                (0..2)
                    .map(|v| {
                        [
                            u as f64 / 2.,
                            if u == 2 {
                                v as f64
                            } else if u == 1 && v == 1 {
                                perturbation
                            } else {
                                0.
                            },
                            0.,
                            if u == 1 && v == 1 { 3. } else { (v + 1) as f64 },
                        ]
                    })
                    .collect()
            })
            .collect();
        let arena = SourceArena::authored(
            "original-transverse-derivative",
            1,
            points
                .iter()
                .flatten()
                .flatten()
                .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
                .collect(),
        )
        .unwrap();
        let leaves = (0..3)
            .map(|u| {
                (0..2)
                    .map(|v| std::array::from_fn(|k| arena.leaf((u * 2 + v) * 4 + k).unwrap()))
                    .collect()
            })
            .collect::<Vec<_>>();
        let tolerance = ToleranceContext::default_valid();
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let tangent =
            rational_surface_derivative_numerator_row_bounds(&mut ctx, &leaves, 0, 1, 1)
                .unwrap();
        assert!(tangent.signs.unwrap()[0].iter().any(|s| *s != Sign::Zero));
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let d =
            rational_surface_derivative_transverse_row_bounds(&mut ctx, &leaves, 1, 0, 1, 1)
                .unwrap();
        let signs = d.signs.unwrap();
        let bounds = d.coefficient_bounds.unwrap();
        let scale = d.coefficient_scale.unwrap();
        assert!(scale[0] > 0.);
        assert_eq!(
            signs[0].iter().all(|s| *s == Sign::Zero),
            perturbation == 0.
        );
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let du =
            rational_surface_derivative_transverse_row_bounds(&mut ctx, &leaves, 1, 0, 0, 0)
                .unwrap();
        assert_eq!(
            du.signs.unwrap()[0].iter().all(|s| *s == Sign::Zero),
            perturbation == 0.
        );
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let endpoint =
            rational_surface_derivative_transverse_row_bounds(&mut ctx, &leaves, 1, 0, 1, 4)
                .unwrap();
        let scale = endpoint.coefficient_scale.unwrap();
        assert!(
            endpoint.coefficient_bounds.unwrap()[0]
                .iter()
                .all(|b| b[0] <= scale[1] && b[1] >= scale[0]),
            "source transverse endpoint coefficient is exactly one"
        );
        for column in 0..2 {
            let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
            let cell = rational_surface_derivative_transverse_coefficient_bounds(
                &mut ctx,
                &leaves,
                1,
                0,
                1,
                [1, column],
            )
            .unwrap();
            assert_eq!(cell.signs.unwrap()[0][0], signs[0][column]);
            assert_eq!(cell.coefficient_bounds.unwrap()[0][0], bounds[0][column]);
        }
        let mut ctx = PredicateContext::new(
            &arena,
            &tolerance,
            Limits {
                max_work: 1,
                ..Limits::default()
            },
            None,
        );
        let d =
            rational_surface_derivative_transverse_row_bounds(&mut ctx, &leaves, 1, 0, 1, 1)
                .unwrap();
        assert!(d.signs.is_none() && d.coefficient_bounds.is_none());
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let d =
            rational_surface_derivative_transverse_row_bounds(&mut ctx, &leaves, 0, 2, 1, 1)
                .unwrap();
        assert!(d.signs.is_none() && d.coefficient_bounds.is_none());
    }
}
#[test]
fn rational_derivative_rows_preserve_quadratic_factor_and_refuse_partial_work() {
    // S=(u,2u²v/(1+v),0): V weights vary, but Q_v=(0,2u²,0).
    let points: Vec<Vec<[f64; 4]>> = (0..3)
        .map(|u| {
            (0..2)
                .map(|v| {
                    [
                        u as f64 / 2.,
                        if u == 2 { v as f64 } else { 0. },
                        0.,
                        (v + 1) as f64,
                    ]
                })
                .collect()
        })
        .collect();
    let arena = SourceArena::authored(
        "rational-derivative-factor",
        1,
        points
            .iter()
            .flatten()
            .flatten()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .unwrap();
    let leaves = (0..3)
        .map(|u| {
            (0..2)
                .map(|v| std::array::from_fn(|k| arena.leaf((u * 2 + v) * 4 + k).unwrap()))
                .collect()
        })
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    for coordinate in 0..3 {
        for row in 0..5 {
            let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
            let d = rational_surface_derivative_numerator_row_bounds(
                &mut ctx, &leaves, coordinate, 1, row,
            )
            .unwrap();
            let signs = d.signs.unwrap();
            let bounds = d.coefficient_bounds.unwrap();
            let scale = d.coefficient_scale.unwrap();
            assert!(scale[0] > 0.);
            if coordinate == 1 && row == 4 {
                assert!(
                    bounds[0]
                        .iter()
                        .all(|b| b[0] <= 2. * scale[1] && b[1] >= 2. * scale[0])
                );
            }
            assert_eq!(bounds.len(), 1);
            assert_eq!(bounds[0].len(), 2);
            assert!(signs[0].iter().all(|s| *s
                == if coordinate == 1 && row >= 2 {
                    Sign::Positive
                } else {
                    Sign::Zero
                }));
            for column in 0..2 {
                let mut ctx =
                    PredicateContext::new(&arena, &tolerance, Limits::default(), None);
                let cell = rational_surface_derivative_numerator_coefficient_bounds(
                    &mut ctx,
                    &leaves,
                    coordinate,
                    1,
                    [row, column],
                )
                .unwrap();
                assert_eq!(cell.signs.unwrap()[0][0], signs[0][column]);
                assert_eq!(cell.coefficient_bounds.unwrap()[0][0], bounds[0][column]);
            }
        }
    }
    for row in 0..4 {
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let d = rational_surface_derivative_numerator_row_bounds(&mut ctx, &leaves, 0, 0, row)
            .unwrap();
        assert_eq!(d.coefficient_bounds.unwrap()[0].len(), 3);
        assert!(d.signs.unwrap()[0].iter().all(|s| *s == Sign::Positive));
    }
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: 1,
            ..Limits::default()
        },
        None,
    );
    let d =
        rational_surface_derivative_numerator_row_bounds(&mut ctx, &leaves, 1, 1, 2).unwrap();
    assert!(d.signs.is_none() && d.coefficient_bounds.is_none() && d.work_used <= 1);
    let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
    assert!(
        rational_surface_derivative_numerator_row_bounds(&mut ctx, &leaves, 1, 1, 5).is_err()
    );
}

#[test]
fn exact_normal_component_bounds_preserve_common_scale_and_refuse_exhaustion() {
    let points = [
        [[0., 0., 0., 1.], [0., 1., 3., 1.]],
        [[1., 0., 2., 1.], [1., 1., 5., 1.]],
    ];
    let arena = SourceArena::authored(
        "normal-bounds-plane",
        1,
        points
            .iter()
            .flatten()
            .flatten()
            .map(|x: &f64| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .unwrap();
    let leaves = (0..2)
        .map(|u| {
            (0..2)
                .map(|v| std::array::from_fn(|k| arena.leaf((u * 2 + v) * 4 + k).unwrap()))
                .collect()
        })
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut bounds = Vec::new();
    for axes in [[1, 2], [2, 0], [0, 1]] {
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let d = rational_surface_projected_jacobian_coefficient_bounds(
            &mut ctx,
            &leaves,
            axes,
            [0, 0],
        )
        .unwrap();
        let selected = d.coefficient_bounds.unwrap()[0][0];
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let whole =
            rational_surface_projected_jacobian_bounds(&mut ctx, &leaves, axes).unwrap();
        let net = whole.coefficient_bounds.unwrap();
        assert_eq!(net.len(), 3);
        assert!(net.iter().all(|row| row.len() == 3));
        assert_eq!(net[0][0], selected);
        let mut ctx = PredicateContext::new(&arena, &tolerance, Limits::default(), None);
        let row =
            rational_surface_projected_jacobian_row_bounds(&mut ctx, &leaves, axes, 0).unwrap();
        assert_eq!(row.coefficient_bounds.unwrap(), vec![net[0].clone()]);
        assert!(whole.signs.unwrap().iter().flatten().all(|s| *s
            == if axes == [0, 1] {
                Sign::Positive
            } else {
                Sign::Negative
            }));
        bounds.push(selected);
    }
    assert!(bounds[2][0] > 0.);
    for (axis, ratio) in [(0, 2.), (1, 3.)] {
        assert!(bounds[axis][0] + ratio * bounds[2][0] <= 0.);
        assert!(bounds[axis][1] + ratio * bounds[2][1] >= 0.);
    }
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: 1,
            ..Limits::default()
        },
        None,
    );
    let d = rational_surface_projected_jacobian_coefficient_bounds(
        &mut ctx,
        &leaves,
        [0, 1],
        [0, 0],
    )
    .unwrap();
    assert!(d.signs.is_none() && d.coefficient_bounds.is_none());
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: 1,
            ..Limits::default()
        },
        None,
    );
    let d = rational_surface_projected_jacobian_bounds(&mut ctx, &leaves, [0, 1]).unwrap();
    assert!(d.signs.is_none() && d.coefficient_bounds.is_none());
}

fn check(points: Vec<Vec<[f64; 4]>>, axes: [usize; 2], work: u64) -> Decision {
    check_row(points, axes, work, None)
}
fn check_row(
    points: Vec<Vec<[f64; 4]>>,
    axes: [usize; 2],
    work: u64,
    row: Option<usize>,
) -> Decision {
    check_cell(points, axes, work, row, None)
}
fn check_cell(
    points: Vec<Vec<[f64; 4]>>,
    axes: [usize; 2],
    work: u64,
    row: Option<usize>,
    column: Option<usize>,
) -> Decision {
    let arena = SourceArena::authored(
        "projected-jacobian-test",
        1,
        points
            .iter()
            .flatten()
            .flatten()
            .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
            .collect(),
    )
    .unwrap();
    let leaves = points
        .iter()
        .enumerate()
        .map(|(i, row)| {
            row.iter()
                .enumerate()
                .map(|(j, _)| {
                    std::array::from_fn(|a| arena.leaf((i * row.len() + j) * 4 + a).unwrap())
                })
                .collect()
        })
        .collect::<Vec<_>>();
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: work,
            ..Limits::default()
        },
        None,
    );
    match (row, column) {
        (Some(row), Some(column)) => rational_surface_projected_jacobian_coefficient(
            &mut ctx,
            &leaves,
            axes,
            [row, column],
        )
        .unwrap(),
        (Some(row), None) => {
            rational_surface_projected_jacobian_row(&mut ctx, &leaves, axes, row).unwrap()
        }
        (None, None) => rational_surface_projected_jacobian(&mut ctx, &leaves, axes).unwrap(),
        _ => unreachable!(),
    }
}
fn has_orientation(d: &Decision, wanted: Sign) -> bool {
    d.signs.as_ref().is_some_and(|rows| {
        rows.iter()
            .flatten()
            .all(|s| *s == Sign::Zero || *s == wanted)
            && rows.iter().flatten().any(|s| *s == wanted)
    })
}
fn factorized_check(
    points: Vec<Vec<[f64; 3]>>,
    rows: [f64; 2],
    columns: [f64; 2],
    axes: [usize; 2],
    work: u64,
    index: Option<[usize; 2]>,
) -> Decision {
    let values: Vec<_> = points
        .iter()
        .flatten()
        .flatten()
        .copied()
        .chain(rows)
        .chain(columns)
        .map(|x| AuthoredScalar::Binary64Bits(x.to_bits()))
        .collect();
    let arena = SourceArena::authored("factorized-jacobian", 1, values).unwrap();
    let points: Vec<Vec<_>> = points
        .iter()
        .enumerate()
        .map(|(u, row)| {
            row.iter()
                .enumerate()
                .map(|(v, _)| std::array::from_fn(|a| arena.leaf((u * 2 + v) * 3 + a).unwrap()))
                .collect()
        })
        .collect();
    let rows = [arena.leaf(12).unwrap(), arena.leaf(13).unwrap()];
    let columns = [arena.leaf(14).unwrap(), arena.leaf(15).unwrap()];
    let tolerance = ToleranceContext::default_valid();
    let mut ctx = PredicateContext::new(
        &arena,
        &tolerance,
        Limits {
            max_work: work,
            ..Limits::default()
        },
        None,
    );
    factorized_surface_projected_jacobian(&mut ctx, &points, &rows, &columns, axes, index)
        .unwrap()
}
#[test]
fn factorized_weights_preserve_exact_orientation_and_refuse_partial_work() {
    let points = vec![
        vec![[0., 0., 0.], [0., 1., 0.]],
        vec![[1., 0., 0.], [1., 1., 0.]],
    ];
    // 1.1*1.1 is not an exact binary64 weight; this path never flattens it.
    let full = factorized_check(
        points.clone(),
        [1., 1.1],
        [1., 1.1],
        [0, 1],
        1_000_000,
        None,
    );
    assert!(has_orientation(&full, Sign::Positive));
    let signs = full.signs.unwrap();
    for scale in [0.0009765625, 1024.] {
        let transformed: Vec<Vec<[f64; 3]>> = points
            .iter()
            .map(|row| {
                row.iter()
                    .map(|p| [p[0] * scale + 128., p[1] * scale - 256., p[2]])
                    .collect()
            })
            .collect();
        let d = factorized_check(
            transformed,
            [1. * scale, 1.1 * scale],
            [1. / scale, 1.1 / scale],
            [0, 1],
            1_000_000,
            None,
        );
        assert_eq!(d.signs.unwrap(), signs);
    }
    for (u, row) in signs.iter().enumerate() {
        for (v, sign) in row.iter().enumerate() {
            let cell = factorized_check(
                points.clone(),
                [1., 1.1],
                [1., 1.1],
                [0, 1],
                1_000_000,
                Some([u, v]),
            );
            assert_eq!(cell.signs.unwrap(), vec![vec![*sign]]);
        }
    }
    assert!(has_orientation(
        &factorized_check(
            points.clone(),
            [1., 1.1],
            [1., 1.1],
            [1, 0],
            1_000_000,
            None
        ),
        Sign::Negative
    ));
    for work in [0, 1, 16] {
        let denied = factorized_check(points.clone(), [1., 1.1], [1., 1.1], [0, 1], work, None);
        assert!(denied.signs.is_none());
        assert!(denied.reason.is_some());
    }
    let denied = factorized_check(points.clone(), [1., -1.], [1., 1.], [0, 1], 1_000_000, None);
    assert!(denied.signs.is_none());
    let folded = vec![
        vec![[0., 0., 0.], [1., 1., 0.]],
        vec![[1., 0., 0.], [0., 1., 0.]],
    ];
    let d = factorized_check(folded, [1., 1.1], [1., 1.1], [0, 1], 1_000_000, None);
    let signs = d.signs.unwrap();
    assert!(signs.iter().flatten().any(|s| *s == Sign::Positive));
    assert!(signs.iter().flatten().any(|s| *s == Sign::Negative));
}
#[test]
fn independently_bounded_rows_match_whole_original_chart() {
    let p = vec![
        vec![[0., 0., 0., 1.], [0., 1., 0., 2.], [0., 2., 0., 1.]],
        vec![[0.5, -1., 0., 2.], [0.5, 2., 0., 1.], [0.5, 1., 0., 2.]],
        vec![[1., 0., 0., 1.], [1., 1., 0., 2.], [1., 2., 0., 1.]],
    ];
    let whole = check(p.clone(), [0, 1], 1_000_000).signs.unwrap();
    for (row, expected) in whole.into_iter().enumerate() {
        assert_eq!(
            check_row(p.clone(), [0, 1], 1_000_000, Some(row))
                .signs
                .unwrap(),
            vec![expected.clone()]
        );
        for (column, sign) in expected.into_iter().enumerate() {
            assert_eq!(
                check_cell(p.clone(), [0, 1], 1_000_000, Some(row), Some(column))
                    .signs
                    .unwrap(),
                vec![vec![sign]]
            );
        }
    }
}
#[test]
fn plane_orientation_and_axis_reversal() {
    let p = vec![
        vec![[0., 0., 0., 1.], [0., 1., 0., 1.]],
        vec![[1., 0., 0., 1.], [1., 1., 0., 1.]],
    ];
    assert!(has_orientation(
        &check(p.clone(), [0, 1], 1_000_000),
        Sign::Positive
    ));
    assert!(has_orientation(
        &check(p, [1, 0], 1_000_000),
        Sign::Negative
    ));
}
#[test]
fn collapsed_boundary_and_flat_endpoint_preserve_interior_sign() {
    // (u, u*(2v-v²)): Jacobian 2u*(1-v), zero only on chart boundaries.
    let p = vec![
        vec![[0., 0., 0., 1.]; 3],
        vec![[1., 0., 0., 1.], [1., 1., 0., 1.], [1., 1., 0., 1.]],
    ];
    let d = check(p, [0, 1], 1_000_000);
    assert!(has_orientation(&d, Sign::Positive));
    assert!(d.signs.unwrap().iter().flatten().any(|s| *s == Sign::Zero));
}
#[test]
fn interior_fold_has_both_signs() {
    // (u, u*(v-v²)): orientation reverses at v=1/2.
    let p = vec![
        vec![[0., 0., 0., 1.]; 3],
        vec![[1., 0., 0., 1.], [1., 0.5, 0., 1.], [1., 0., 0., 1.]],
    ];
    let d = check(p, [0, 1], 1_000_000);
    let signs = d.signs.unwrap();
    assert!(signs.iter().flatten().any(|s| *s == Sign::Positive));
    assert!(signs.iter().flatten().any(|s| *s == Sign::Negative));
}
#[test]
fn rational_weights_and_work_refusal() {
    let p = vec![
        vec![[0., 0., 0., 1.], [0., 1., 0., 2.]],
        vec![[1., 0., 0., 1.], [1., 1., 0., 2.]],
    ];
    assert!(has_orientation(
        &check(p.clone(), [0, 1], 1_000_000),
        Sign::Positive
    ));
    let d = check(p, [0, 1], 1);
    assert!(d.signs.is_none());
    assert!(d.reason.is_some());
}
#[test]
fn rank_deficient_projection_has_no_strict_sign() {
    let p = vec![
        vec![[0., 0., 0., 1.], [0., 0., 1., 1.]],
        vec![[1., 0., 0., 1.], [1., 0., 1., 1.]],
    ];
    let d = check(p, [0, 1], 1_000_000);
    assert!(d.signs.unwrap().iter().flatten().all(|s| *s == Sign::Zero));
}
