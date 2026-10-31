use nurbs_core::{analysis::affine_support, curve::Curve, surface::Surface};

fn exact_integer_rank(points: &[[i64; 3]]) -> usize {
    let vectors: Vec<[i128; 3]> = points
        .iter()
        .skip(1)
        .map(|p| std::array::from_fn(|k| i128::from(p[k]) - i128::from(points[0][k])))
        .collect();
    let mut rank = usize::from(vectors.iter().any(|v| *v != [0; 3]));
    for a in &vectors {
        for b in &vectors {
            let cross = [
                a[1] * b[2] - a[2] * b[1],
                a[2] * b[0] - a[0] * b[2],
                a[0] * b[1] - a[1] * b[0],
            ];
            if cross != [0; 3] {
                rank = 2;
            }
            for c in &vectors {
                if (0..3).map(|k| cross[k] * c[k]).sum::<i128>() != 0 {
                    return 3;
                }
            }
        }
    }
    rank
}
fn curve(points: &[[i64; 3]]) -> Curve {
    Curve::from_polyline(
        points
            .iter()
            .map(|p| p.iter().map(|&x| x as f64).collect())
            .collect(),
    )
    .unwrap()
}
fn patch() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 1.], vec![0., 1., 3.]],
            vec![vec![1., 0., 2.], vec![1., 1., 4.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 4.]],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn all_four_dimensions_match_an_independent_integer_determinant_oracle() {
    let cases = [
        vec![[0, 0, 0]; 5],
        vec![[0, 0, 0], [1, 2, 3], [2, 4, 6], [-1, -2, -3]],
        vec![[0, 0, 0], [1, 0, 1], [0, 1, 2], [1, 1, 3], [-1, 2, 3]],
        vec![[0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1]],
    ];
    for offset in [0_i64, 1 << 28, -(1 << 28)] {
        for base in &cases {
            let points: Vec<_> = base
                .iter()
                .map(|p| {
                    [
                        2 * p[0] + p[1] + offset,
                        p[0] + p[1] - offset,
                        p[2] + offset,
                    ]
                })
                .collect();
            let source = curve(&points);
            let before = source.control_points.clone();
            let report = affine_support::curve(&source, 1_000_000).unwrap();
            assert!(report.complete, "{report:?}");
            assert_eq!(report.dimension, exact_integer_rank(&points));
            assert_eq!(report.witness_indices.len(), report.dimension + 1);
            assert_eq!(report.reason, None);
            assert!(report.work_used <= 1_000_000);
            assert_eq!(source.control_points, before);
        }
    }
}

#[test]
fn varied_small_integer_clouds_preserve_exact_rank() {
    let mut seed = 17_u64;
    for _ in 0..64 {
        let mut points = Vec::new();
        for _ in 0..6 {
            points.push(std::array::from_fn(|_| {
                seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
                ((seed >> 32) % 9) as i64 - 4
            }));
        }
        let report = affine_support::curve(&curve(&points), 1_000_000).unwrap();
        assert!(report.complete);
        assert_eq!(report.dimension, exact_integer_rank(&points));
    }
}

#[test]
fn rational_surface_retains_its_exact_plane_without_mutation() {
    let source = patch();
    let before = (source.control_points.clone(), source.weights.clone());
    let report = affine_support::surface(&source, 1_000_000).unwrap();
    assert!(report.complete);
    assert_eq!(report.dimension, 2);
    assert_eq!(report.control_count, 4);
    for u in [0., 0.17, 0.5, 1.] {
        for v in [0., 0.31, 0.8, 1.] {
            let p = source.evaluate(u, v).unwrap().point;
            assert!((p[2] - p[0] - 2. * p[1] - 1.).abs() < 1e-13);
        }
    }
    assert_eq!((source.control_points, source.weights), before);
}

#[test]
fn a_single_ulp_is_not_accepted_as_exact_planarity() {
    let mut source = patch();
    source.control_points[1][1][2] = f64::from_bits(4_f64.to_bits() + 1);
    let report = affine_support::surface(&source, 1_000_000).unwrap();
    assert!(report.complete);
    assert_eq!(report.dimension, 3);
}

#[test]
fn exhausted_work_returns_an_explicit_incomplete_lower_bound() {
    let report = affine_support::surface(&patch(), 1).unwrap();
    assert!(!report.complete);
    assert_eq!(report.reason, Some(cad_predicates::Reason::ResourceLimit));
    assert!(report.work_used <= 1);
    assert_eq!(report.witness_indices.len(), report.dimension + 1);
}

#[test]
fn work_is_shared_across_all_predicate_contexts() {
    let points: Vec<_> = (0..30).map(|x| [x, 2 * x, 3 * x]).collect();
    let source = curve(&points);
    let high = affine_support::curve(&source, 1_000_000).unwrap();
    assert!(high.complete);
    assert!(high.work_used > 2);
    let budget = high.work_used / 2;
    let low = affine_support::curve(&source, budget).unwrap();
    assert!(!low.complete);
    assert_eq!(low.reason, Some(cad_predicates::Reason::ResourceLimit));
    assert!(low.work_used <= budget);
}

#[test]
fn invalid_definitions_dimensions_and_budgets_are_rejected() {
    assert!(affine_support::surface(&patch(), 0).is_err());
    assert!(affine_support::surface(&patch(), 1_000_001).is_err());
    let mut bad = patch();
    bad.knots_u.clear();
    assert!(affine_support::surface(&bad, 1000).is_err());
    let mut outside = curve(&[[0, 0, 0], [1, 1, 1]]);
    outside.control_points[1][0] = (1_u64 << 45) as f64;
    assert!(affine_support::curve(&outside, 1000).is_err());
    let two = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
    assert!(affine_support::curve(&two, 1000).is_err());
}

#[test]
fn subnormal_spatial_controls_never_gain_a_false_planar_proof() {
    let t = f64::from_bits(1);
    let source = Curve::from_polyline(vec![
        vec![0., 0., 0.],
        vec![t, 0., 0.],
        vec![0., t, 0.],
        vec![0., 0., t],
    ])
    .unwrap();
    let report = affine_support::curve(&source, 1_000_000).unwrap();
    if report.complete {
        assert_eq!(report.dimension, 3);
    } else {
        assert!(report.reason.is_some());
    }
}
