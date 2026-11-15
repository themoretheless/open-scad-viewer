use nurbs_core::{
    editing::{
        exact_surface_knot_insertion::{SplitRefusal, split_tensor_surface},
        exact_surface_merge::merge,
    },
    surface::{Axis, Surface},
};

const WORK: u64 = 1_000_000;

fn quadratic() -> Surface {
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![2., 2., 2., 6., 6., 6.],
        knots_v: vec![7., 7., 7., 9., 9., 9.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64 * 0.5,
                            j as f64 * 0.5,
                            f64::from(u8::from(i == 2)) + f64::from(u8::from(j == 2)),
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn both_axes_share_a_literal_seam_and_round_trip_through_exact_merge() {
    let source = quadratic();
    for (axis, split) in [(Axis::U, 4.), (Axis::V, 8.)] {
        let r = split_tensor_surface(&source, axis, split, WORK).unwrap();
        let [a, b] = r.parts.unwrap_or_else(|| panic!("{:?}", r.refusal));
        match axis {
            Axis::U => {
                assert_eq!(a.knots_u, [2., 2., 2., 4., 4., 4.]);
                assert_eq!(b.knots_u, [4., 4., 4., 6., 6., 6.]);
                assert_eq!(a.knots_v, source.knots_v);
                assert_eq!(a.control_points.last(), b.control_points.first());
                assert_eq!(a.weights.last(), b.weights.first());
            }
            Axis::V => {
                assert_eq!(a.knots_v, [7., 7., 7., 8., 8., 8.]);
                assert_eq!(b.knots_v, [8., 8., 8., 9., 9., 9.]);
                assert_eq!(b.knots_u, source.knots_u);
                for (u, v) in a.control_points.iter().zip(&b.control_points) {
                    assert_eq!(u.last(), v.first());
                }
                for (u, v) in a.weights.iter().zip(&b.weights) {
                    assert_eq!(u.last(), v.first());
                }
            }
        }
        let merged = merge(&a, &b, axis).unwrap().surface.unwrap();
        for (u, v) in [(2., 7.), (2.75, 7.25), (4., 8.), (5.75, 8.5), (6., 9.)] {
            let actual = merged.evaluate(u, v).unwrap().point;
            let before = source.evaluate(u, v).unwrap().point;
            assert!(
                actual
                    .iter()
                    .zip(before)
                    .all(|(x, y)| (x - y).abs() < 1e-14)
            );
        }
    }
}

#[test]
fn structural_split_at_an_existing_full_knot_is_cheap_and_exact() {
    let mut source = quadratic();
    source.degree_u = 1;
    source.knots_u = vec![2., 2., 4., 6., 6.];
    let r = split_tensor_surface(&source, Axis::U, 4., 18).unwrap();
    assert_eq!(r.work_used, 18);
    let [a, b] = r.parts.unwrap();
    assert_eq!(a.control_points[1], source.control_points[1]);
    assert_eq!(b.control_points[0], source.control_points[1]);
    let short = split_tensor_surface(&source, Axis::U, 4., 17).unwrap();
    assert_eq!(short.refusal, Some(SplitRefusal::ResourceLimit));
    assert!(short.parts.is_none());
}

#[test]
fn nonbinary_seam_proposal_returns_no_rounded_parts() {
    let mut source = quadratic();
    source.degree_u = 1;
    source.knots_u = vec![0., 0., 1., 1.];
    source.control_points.remove(1);
    source.weights = vec![vec![1.; 3], vec![2.; 3]];
    let result = split_tensor_surface(&source, Axis::U, 0.25, WORK).unwrap();
    assert!(matches!(result.refusal, Some(SplitRefusal::Insertion(_))));
    assert!(result.parts.is_none());
}

#[test]
fn accepts_only_interior_parameters_and_refuses_short_work() {
    let source = quadratic();
    for invalid in [2., 6., f64::NAN] {
        assert!(split_tensor_surface(&source, Axis::U, invalid, WORK).is_err());
    }
    let r = split_tensor_surface(&source, Axis::U, 4., 1).unwrap();
    assert!(r.parts.is_none() && r.refusal.is_some());
}

#[test]
fn rational_split_has_one_common_gauge_on_both_sides() {
    let mut source = quadratic();
    source.degree_u = 1;
    source.knots_u = vec![0., 0., 1., 1.];
    source.control_points.remove(1);
    source.weights = vec![vec![1.; 3], vec![3.; 3]];
    let result = split_tensor_surface(&source, Axis::U, 0.5, WORK).unwrap();
    let [left, right] = result
        .parts
        .unwrap_or_else(|| panic!("{:?}", result.refusal));
    let scale = left.weights[0][0];
    assert_eq!(left.weights[1], right.weights[0]);
    assert_eq!(right.control_points[0], left.control_points[1]);
    assert_eq!(left.weights[1][0], 2. * scale);
    assert_eq!(right.weights[1][0], 3. * scale);
    let point = left.evaluate(0.5, 8.).unwrap().point;
    assert!((point[0] - 0.75).abs() < 1e-14);
}

#[test]
fn global_refinement_tiles_both_uv_axes_and_round_trips() {
    use nurbs_core::editing::{
        exact_surface_knot_insertion::tile_tensor_surface, exact_surface_merge::merge_surface_grid,
    };
    let s = quadratic();
    let report = tile_tensor_surface(&s, &[3., 4.], &[8.], WORK).unwrap();
    let tiles = report
        .patches
        .unwrap_or_else(|| panic!("{:?}", report.refusal));
    assert_eq!((report.columns, report.rows, tiles.len()), (3, 2, 6));
    assert_eq!(tiles[1].knots_u, [3., 3., 3., 4., 4., 4.]);
    assert_eq!(tiles[3].knots_v, [8., 8., 8., 9., 9., 9.]);
    assert_eq!(
        tiles[0].control_points.last(),
        tiles[1].control_points.first()
    );
    for (a, b) in tiles[0].weights.iter().zip(&tiles[3].weights) {
        assert_eq!(a.last(), b.first());
    }
    let merged = merge_surface_grid(&tiles, 3, WORK)
        .unwrap()
        .surface
        .unwrap();
    for (u, v) in [(2., 7.), (3.5, 7.5), (4., 8.), (6., 9.)] {
        let a = s.evaluate(u, v).unwrap().point;
        let b = merged.evaluate(u, v).unwrap().point;
        assert!(a.iter().zip(b).all(|(x, y)| (x - y).abs() < 1e-14));
    }
}

#[test]
fn an_unrepresentable_late_cut_or_short_copy_budget_withholds_every_tile() {
    use nurbs_core::editing::exact_surface_knot_insertion::tile_tensor_surface;
    let s = quadratic();
    let full = tile_tensor_surface(&s, &[4.], &[8.], WORK).unwrap();
    let short = tile_tensor_surface(&s, &[4.], &[8.], full.work_used - 1).unwrap();
    assert_eq!(short.refusal, Some(SplitRefusal::ResourceLimit));
    assert!(short.patches.is_none());
    assert!(tile_tensor_surface(&s, &[4., 3.], &[8.], WORK).is_err());
    let mut rational = s;
    rational.degree_v = 1;
    rational.knots_v = vec![0., 0., 1., 1.];
    for (points, weights) in rational
        .control_points
        .iter_mut()
        .zip(&mut rational.weights)
    {
        points.remove(1);
        *weights = vec![1., 2.];
    }
    let refused = tile_tensor_surface(&rational, &[4.], &[0.25], WORK).unwrap();
    assert!(matches!(refused.refusal, Some(SplitRefusal::Insertion(_))));
    assert!(refused.patches.is_none());
}
