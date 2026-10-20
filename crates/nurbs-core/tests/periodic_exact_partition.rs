use nurbs_core::{
    curve::Curve,
    periodic_trim::{self as p, Piece, Report},
};
fn curve() -> Curve {
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 0.5, 1., 1., 1.],
        control_points: vec![vec![0., 0.], vec![3., 6.], vec![9., 6.], vec![12., 0.]],
        weights: vec![1.; 4],
        periodic: false,
    }
}
fn candidate(points: &[[f64; 2]], domain: [f64; 2]) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: [vec![domain[0]; degree + 1], vec![domain[1]; degree + 1]].concat(),
        control_points: points.iter().map(|p| p.to_vec()).collect(),
        weights: vec![1.; points.len()],
        periodic: false,
    }
}
fn part(c: Curve, translation: [f64; 2]) -> Piece {
    Piece {
        source_domain: c.domain(),
        curve: c,
        translation,
        tile: [0; 2],
    }
}
#[test]
fn original_c1_bspline_equals_differently_graded_exact_candidate_pieces() {
    let a = curve();
    let partition = Report {
        pieces: vec![
            part(
                candidate(&[[0., 0.], [2., 4.], [4., 6.], [6., 6.]], [0., 0.5]),
                [0.; 2],
            ),
            part(
                candidate(&[[6., 6.], [8., 6.], [10., 4.], [12., 0.]], [0.5, 1.]),
                [0.; 2],
            ),
        ],
        candidate_hulls_contained: false,
    };
    let proof = p::verify_exact_partition_identity(&a, &partition, 1_000_000)
        .unwrap()
        .unwrap();
    proof.reverify(1_000_000).unwrap();
    assert!(proof.reverify(proof.work_used() - 1).is_err());
    for piece in &partition.pieces {
        for i in 0..=16 {
            let t = piece.source_domain[0]
                + (piece.source_domain[1] - piece.source_domain[0]) * i as f64 / 16.;
            let aa = a.evaluate(t).unwrap().point;
            let bb = piece.curve.evaluate(t).unwrap().point;
            assert!(aa.iter().zip(bb).all(|(x, y)| (x - y).abs() < 1e-12));
        }
    }
    let mut bad = partition.clone();
    bad.pieces[0].curve.control_points[1][0] = 2.0_f64.next_up();
    assert!(
        p::verify_exact_partition_identity(&a, &bad, 1_000_000)
            .unwrap()
            .is_none()
    );
    let mut bad = partition.clone();
    bad.pieces[0].translation[0] = f64::EPSILON;
    assert!(
        p::verify_exact_partition_identity(&a, &bad, 1_000_000)
            .unwrap()
            .is_none()
    );
    let mut missing = partition;
    missing.pieces.pop();
    assert!(p::verify_exact_partition_identity(&a, &missing, 1_000_000).is_err());
}
#[test]
fn exact_original_restriction_uses_actual_domains_and_translation() {
    let a = Curve::from_polyline(vec![vec![4., 0.], vec![8., 4.]]).unwrap();
    let partition = Report {
        pieces: vec![
            part(candidate(&[[0., 0.], [1., 1.]], [0., 0.25]), [4., 0.]),
            part(candidate(&[[5., 1.], [8., 4.]], [0.25, 1.]), [0., 0.]),
        ],
        candidate_hulls_contained: false,
    };
    let proof = p::verify_exact_partition_identity(&a, &partition, 1_000_000)
        .unwrap()
        .unwrap();
    proof.reverify(1_000_000).unwrap();
    assert!(
        p::verify_exact_partition_identity(&a, &partition, 0)
            .unwrap()
            .is_none()
    );
}
#[test]
fn partial_original_knot_cells_and_rational_weights_are_compared_exactly() {
    let a = curve();
    let pieces = Report {
        pieces: vec![
            part(
                candidate(&[[0., 0.], [1.5, 3.], [3., 4.5]], [0., 0.25]),
                [0.; 2],
            ),
            part(
                candidate(&[[3., 4.5], [4.5, 6.], [6., 6.]], [0.25, 0.5]),
                [0.; 2],
            ),
            part(
                candidate(&[[6., 6.], [9., 6.], [12., 0.]], [0.5, 1.]),
                [0.; 2],
            ),
        ],
        candidate_hulls_contained: false,
    };
    p::verify_exact_partition_identity(&a, &pieces, 1_000_000)
        .unwrap()
        .unwrap()
        .reverify(1_000_000)
        .unwrap();
    let mut rational = a;
    rational.control_points[2][0] = 8.;
    rational.weights = vec![1., 2., 3., 1.];
    let mut left = candidate(&[[0., 0.], [3., 6.], [6., 6.]], [0., 0.5]);
    left.weights = vec![1., 2., 2.5];
    let mut right = candidate(&[[6., 6.], [8., 6.], [12., 0.]], [0.5, 1.]);
    right.weights = vec![2.5, 3., 1.];
    let mut partitions = Report {
        pieces: vec![part(left, [0.; 2]), part(right, [0.; 2])],
        candidate_hulls_contained: false,
    };
    p::verify_exact_partition_identity(&rational, &partitions, 1_000_000)
        .unwrap()
        .unwrap()
        .reverify(1_000_000)
        .unwrap();
    partitions.pieces[0].curve.weights[2] = 2.5_f64.next_up();
    assert!(
        p::verify_exact_partition_identity(&rational, &partitions, 1_000_000)
            .unwrap()
            .is_none()
    );
}
#[test]
fn nonclamped_originals_require_valid_closed_domain_end_cells() {
    let mut a = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.]]).unwrap();
    let candidate = a.clone();
    a.knots = vec![-1., 0., 1., 2.];
    let partition = Report {
        pieces: vec![part(candidate, [0.; 2])],
        candidate_hulls_contained: false,
    };
    p::verify_exact_partition_identity(&a, &partition, 1_000_000)
        .unwrap()
        .unwrap()
        .reverify(1_000_000)
        .unwrap();
    let empty_end = Curve {
        degree: 1,
        knots: vec![-1., 0., 0., 1., 2.],
        control_points: vec![vec![3., 3.], vec![0., 0.], vec![1., 1.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    assert!(
        p::verify_exact_partition_identity(&empty_end, &partition, 1_000_000)
            .unwrap()
            .is_none()
    );
}
