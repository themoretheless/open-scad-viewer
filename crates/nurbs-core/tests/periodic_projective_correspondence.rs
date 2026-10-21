use nurbs_core::{curve::Curve, periodic_trim::verify_projective_curve_correspondence as verify};
fn curve(points: &[[f64; 2]], weights: Vec<f64>) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: [vec![-2.; degree + 1], vec![6.; degree + 1]].concat(),
        control_points: points.iter().map(|p| p.to_vec()).collect(),
        weights,
        periodic: false,
    }
}
#[test]
fn nonlinear_fractional_parameter_is_not_same_parameter_identity() {
    let a = curve(&[[0., 0.], [1., 2.], [3., 0.]], vec![1., 1., 1.]);
    let mut b = a.clone();
    b.weights = vec![1., 2., 4.];
    b.knots = vec![10., 10., 10., 14., 14., 14.];
    let proof = verify(&a, &b, [2., 1.], false, 1_000_000).unwrap().unwrap();
    proof.reverify(1_000_000).unwrap();
    assert_eq!(proof.orientation(), 1);
    assert!(
        verify(&a, &b, [1., 1.], false, 1_000_000)
            .unwrap()
            .is_none()
    );
    // Independent rational evaluation uses a nonbinary parameter map, not samples as authority.
    for t in [0., 0.125, 0.5, 0.75, 1.] {
        let phi = 2. * t / (1. + t);
        let pa = a.evaluate(-2. + 8. * phi).unwrap().point;
        let pb = b.evaluate(10. + 4. * t).unwrap().point;
        assert!(pa.iter().zip(pb).all(|(x, y)| (x - y).abs() < 1e-12));
    }
    let mut changed = b.clone();
    changed.control_points[1][0] = changed.control_points[1][0].next_up();
    assert!(
        verify(&a, &changed, [2., 1.], false, 1_000_000)
            .unwrap()
            .is_none()
    );
    assert!(proof.reverify(0).is_err());
    assert!(verify(&a, &b, [2., 1.], false, 0).unwrap().is_none());
}
#[test]
fn unequal_degree_positive_rational_ratio_and_reverse_are_exact() {
    // A is a line. A(phi) with alpha=3,beta=2 has endpoint weights2,3.
    // Its exact degree elevation has middle weight2.5 and coordinate0.6.
    // Use x endpoint5 so elevated coordinate3 is exactly binary64.
    let a = curve(&[[0., 0.], [5., 10.]], vec![1., 1.]);
    let b = curve(&[[0., 0.], [3., 6.], [5., 10.]], vec![2., 2.5, 3.]);
    let proof = verify(&a, &b, [3., 2.], false, 1_000_000).unwrap().unwrap();
    proof.reverify(1_000_000).unwrap();
    let reverse = b.reverse().unwrap();
    let proof = verify(&a, &reverse, [2., 3.], true, 1_000_000)
        .unwrap()
        .unwrap();
    assert_eq!(proof.orientation(), -1);
    proof.reverify(1_000_000).unwrap();
    assert!(
        verify(&a, &reverse, [3., 2.], true, 1_000_000)
            .unwrap()
            .is_none()
    );
}
#[test]
fn invalid_factors_and_nonclamped_sources_have_no_authority() {
    let a = curve(&[[0., 0.], [1., 1.]], vec![1., 1.]);
    for factors in [[0., 1.], [-1., 1.], [1., f64::INFINITY]] {
        assert!(verify(&a, &a, factors, false, 1000).is_err());
    }
    let mut poly = Curve::from_polyline(vec![vec![0., 0.], vec![1., 1.], vec![2., 0.]]).unwrap();
    poly.knots = vec![-1., 0., 0.5, 1., 2.];
    assert!(
        verify(&poly, &poly, [1., 1.], false, 1_000_000)
            .unwrap()
            .is_none()
    );
}
fn line(a: f64, b: f64, domain: [f64; 2], weights: [f64; 2]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![domain[0], domain[0], domain[1], domain[1]],
        control_points: vec![vec![a, 0.], vec![b, 0.]],
        weights: weights.to_vec(),
        periodic: false,
    }
}
#[test]
fn seam_subintervals_share_one_global_nonlinear_homotopy_and_work_budget() {
    use nurbs_core::periodic_trim::{
        Piece, Report, verify_projective_partition_homotopy as homotopy,
    };
    let a = line(0., 4., [0., 1.], [1., 1.]);
    let b = line(0., 4., [0., 1.], [1., 3.]);
    let piece = |curve: Curve, translation: f64, tile: i32| Piece {
        source_domain: curve.domain(),
        curve,
        translation: [translation, 0.],
        tile: [tile, 0],
    };
    let pa = Report {
        pieces: vec![
            piece(line(0., 3., [0., 0.75], [1., 1.]), 0., 0),
            piece(line(0., 1., [0.75, 1.], [1., 1.]), 3., 1),
        ],
        candidate_hulls_contained: false,
    };
    let pb = Report {
        pieces: vec![
            piece(line(0., 3., [0., 0.5], [1., 2.]), 0., 0),
            piece(line(0., 1., [0.5, 1.], [2., 3.]), 3., 1),
        ],
        candidate_hulls_contained: false,
    };
    let proof = homotopy(&a, &b, &pa, &pb, [3., 1.], false, 1_000_000)
        .unwrap()
        .unwrap();
    proof.reverify(1_000_000).unwrap();
    assert_eq!(proof.partitions()[0].pieces.len(), 2);
    assert!(proof.reverify(proof.work_used() - 1).is_err());
    let reversed_source = b.reverse().unwrap();
    let mut reversed_parts = pb.clone();
    reversed_parts.pieces.reverse();
    for piece in &mut reversed_parts.pieces {
        let old = piece.source_domain;
        piece.source_domain = [1. - old[1], 1. - old[0]];
        piece.curve = piece.curve.reverse().unwrap();
        piece.curve.knots = vec![
            piece.source_domain[0],
            piece.source_domain[0],
            piece.source_domain[1],
            piece.source_domain[1],
        ];
    }
    let reversed = homotopy(
        &a,
        &reversed_source,
        &pa,
        &reversed_parts,
        [1., 3.],
        true,
        1_000_000,
    )
    .unwrap()
    .unwrap();
    reversed.reverify(1_000_000).unwrap();
    assert_eq!(reversed.orientation(), -1);
    let mut unsynchronized = pb.clone();
    unsynchronized.pieces[0].curve.weights = vec![1., 1.];
    assert!(
        homotopy(&a, &b, &pa, &unsynchronized, [3., 1.], false, 1_000_000)
            .unwrap()
            .is_none()
    );
    let mut changed = pa.clone();
    changed.pieces[0].source_domain[1] = 0.75_f64.next_up();
    changed.pieces[0].curve.knots[2] = 0.75_f64.next_up();
    changed.pieces[0].curve.knots[3] = 0.75_f64.next_up();
    changed.pieces[1].source_domain[0] = 0.75_f64.next_up();
    changed.pieces[1].curve.knots[0] = 0.75_f64.next_up();
    changed.pieces[1].curve.knots[1] = 0.75_f64.next_up();
    assert!(
        homotopy(&a, &b, &changed, &pb, [3., 1.], false, 1_000_000)
            .unwrap()
            .is_none()
    );
    let mut wrong_tile = pb.clone();
    wrong_tile.pieces[1].tile[0] = 2;
    assert!(
        homotopy(&a, &b, &pa, &wrong_tile, [3., 1.], false, 1_000_000)
            .unwrap()
            .is_none()
    );
    let mut gap = pb.clone();
    gap.pieces.pop();
    assert!(homotopy(&a, &b, &pa, &gap, [3., 1.], false, 1_000_000).is_err());
    // Candidate homotopy is distinct from image equality: points stay on the
    // same segment after weight mutation, but their synchronized motion differs.
}
#[test]
fn nonlinear_aliases_preserve_full_original_winding_through_qualified_quotient() {
    use nurbs_core::periodic_trim::{self as p, Piece, Report};
    let a = curve(
        &[
            [0.5, 0.5],
            [0.75, 0.5],
            [0.5, 0.75],
            [0.25, 0.5],
            [0.5, 0.5],
        ],
        vec![1.; 5],
    );
    let mut b = a.clone();
    b.weights = vec![1., 2., 4., 8., 16.];
    let part = |c: &Curve| Report {
        pieces: vec![Piece {
            source_domain: c.domain(),
            translation: [0.; 2],
            tile: [0; 2],
            curve: c.clone(),
        }],
        candidate_hulls_contained: false,
    };
    let pa = part(&a);
    let pb = part(&b);
    let correspondence =
        p::verify_projective_partition_homotopy(&a, &b, &pa, &pb, [2., 1.], false, 1_000_000)
            .unwrap()
            .unwrap();
    let limits = p::AnchorLimits {
        isotopy: p::IsotopyLimits {
            tolerance_uv: 0.01,
            max_deviation_cells: 40000,
            max_jet_cells: 256,
            max_pairs: 20000,
        },
        tolerance_uv: 1e-8,
        max_images: 256,
        max_cells: 30000,
        max_checks: 10000,
    };
    let result = p::verify_projective_coherent_anchor_winding(
        &[a.clone(), b.clone()],
        &[pa.clone(), pb.clone()],
        std::slice::from_ref(&correspondence),
        [[0., 2.], [0., 2.]],
        [0.5, 0.55],
        limits,
        1_000_000,
    )
    .unwrap();
    assert!(
        result.transfer.coherent_source_winding_preserved,
        "{:?}",
        result.transfer
    );
    assert_eq!(result.transfer.groups, vec![vec![0, 1]]);
    assert_eq!(result.transfer.winding, Some(2));
    assert_eq!(result.transfer.quotient.winding, Some(1));
    assert_eq!(result.correspondences().len(), 1);
    assert!(
        p::verify_projective_coherent_anchor_winding(
            &[a.clone(), b.clone()],
            &[pa.clone(), pb.clone()],
            &[correspondence.clone(), correspondence.clone()],
            [[0., 2.], [0., 2.]],
            [0.5, 0.55],
            limits,
            correspondence.work_used()
        )
        .is_err()
    );
    let chart_limits = p::ApproximateChartLimits {
        isotopy: limits.isotopy,
        max_edges: 128,
        max_images: limits.max_images,
        max_cells: limits.max_cells,
        max_checks: limits.max_checks,
        tolerance_uv: limits.tolerance_uv,
    };
    let weighted = p::assemble_projective_coherent_weighted_chart(
        &[a.clone(), b.clone()],
        &[pa.clone(), pb.clone()],
        std::slice::from_ref(&correspondence),
        [[0., 2.], [0., 2.]],
        [0.5, 0.55],
        chart_limits,
        1_000_000,
    )
    .unwrap();
    assert!(
        weighted.assembly.candidate_winding_preserved,
        "{:?}",
        weighted.assembly
    );
    let graph = weighted.assembly.graph.unwrap();
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].coefficient, 2);
    assert_eq!(graph.edges[0].curve, a);
    assert_eq!(graph.edges[0].contributions.len(), 2);
    assert_eq!(
        graph.edges[0]
            .contributions
            .iter()
            .map(|c| c.source)
            .collect::<Vec<_>>(),
        vec![Some([0, 0]), Some([1, 0])]
    );
    let reversed_b = b.reverse().unwrap();
    let reversed_part = part(&reversed_b);
    let reversed_receipt = p::verify_projective_partition_homotopy(
        &a,
        &reversed_b,
        &pa,
        &reversed_part,
        [1., 2.],
        true,
        1_000_000,
    )
    .unwrap()
    .unwrap();
    let cancelled = p::assemble_projective_coherent_weighted_chart(
        &[a.clone(), reversed_b],
        &[pa.clone(), reversed_part],
        &[reversed_receipt],
        [[0., 2.], [0., 2.]],
        [0.5, 0.55],
        chart_limits,
        1_000_000,
    )
    .unwrap();
    assert!(
        cancelled.assembly.candidate_winding_preserved,
        "{:?}",
        cancelled.assembly
    );
    assert_eq!(cancelled.assembly.transfer.winding, Some(0));
    let cancelled_graph = cancelled.assembly.graph.unwrap();
    assert!(cancelled_graph.edges.is_empty());
    assert_eq!(cancelled_graph.cancellations.len(), 1);
    assert_eq!(
        cancelled_graph.cancellations[0]
            .iter()
            .map(|c| c.orientation)
            .collect::<Vec<_>>(),
        vec![1, -1]
    );
    let mut changed = pb;
    changed.pieces[0].curve.control_points[1][0] += 0.001;
    let mismatched = p::verify_projective_coherent_anchor_winding(
        &[a, b],
        &[pa, changed],
        &[correspondence],
        [[0., 2.], [0., 2.]],
        [0.5, 0.55],
        limits,
        1_000_000,
    )
    .unwrap();
    assert!(!mismatched.transfer.coherent_source_winding_preserved);
}

#[test]
fn extreme_finite_products_never_turn_a_wrong_map_into_identity() {
    for weight in [1e-12, 1e12] {
        let a = line(0., 1., [0., 1.], [weight, weight]);
        let mut b = a.clone();
        b.control_points[1][0] = 1.0_f64.next_up();
        assert!(
            verify(&a, &b, [1., 1.], false, 1_000_000)
                .unwrap()
                .is_none()
        );
        for factors in [[f64::MIN_POSITIVE, 1.], [1e300, 1.]] {
            assert!(verify(&a, &b, factors, false, 1_000_000).unwrap().is_none());
        }
    }
}
