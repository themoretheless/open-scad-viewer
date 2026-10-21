use nurbs_core::{
    curve::Curve,
    periodic_trim::{self as p, verify_projective_curve_correspondence as verify},
};
fn pair(degree: usize, c0: bool) -> (Curve, Curve) {
    let repeats = if c0 { degree } else { 1 };
    let knots = |x, y| {
        [
            vec![0.; degree + 1],
            vec![x; repeats],
            vec![y; repeats],
            vec![1.; degree + 1],
        ]
        .concat()
    };
    let ak = knots(0.5, 0.75);
    let bk = knots(0.25, 0.5);
    let n = ak.len() - degree - 1;
    let a = Curve {
        degree,
        knots: ak,
        control_points: (0..n)
            .map(|i| vec![i as f64, (i % 3) as f64 - 1.])
            .collect(),
        weights: (0..n)
            .map(|i| {
                if degree >= 20 {
                    1.
                } else {
                    [1., 0.5, 2., 3.][i % 4]
                }
            })
            .collect(),
        periodic: false,
    };
    let mut b = a.clone();
    b.knots = bk;
    for i in 0..n {
        b.weights[i] = a.weights[i]
            * (i + 1..=i + degree)
                .map(|j| 1. + 2. * b.knots[j])
                .product::<f64>();
    }
    if degree >= 20 {
        for w in &mut b.weights {
            *w *= 0.125;
        }
    }
    (a, b)
}
#[test]
fn complete_nonuniform_bspline_basis_preserves_projective_position_and_chain_rule() {
    for (degree, c0) in [
        (1, false),
        (2, false),
        (3, false),
        (2, true),
        (3, true),
        (25, false),
    ] {
        let (a, b) = pair(degree, c0);
        let proof = verify(&a, &b, [3., 1.], false, 1_000_000).unwrap().unwrap();
        proof.reverify(1_000_000).unwrap();
        for t in [0., 0.0625, 0.125, 0.25, 0.375, 0.5, 0.625, 0.875, 1.] {
            let phi = 3. * t / (1. + 2. * t);
            let aa = a.evaluate(phi).unwrap();
            let bb = b.evaluate(t).unwrap();
            assert!(
                aa.point
                    .iter()
                    .zip(&bb.point)
                    .all(|(x, y)| (x - y).abs() < 1e-11),
                "p={degree},c0={c0},t={t}"
            );
            if ![0., 0.25, 0.5, 1.].contains(&t) {
                let factor = 3. / (1. + 2. * t).powi(2);
                assert!(
                    aa.d1
                        .unwrap()
                        .iter()
                        .zip(bb.d1.unwrap())
                        .all(|(x, y)| (x * factor - y).abs() < 1e-9)
                );
            }
        }
        let mut changed = b.clone();
        changed.weights[1] = changed.weights[1].next_up();
        assert!(
            verify(&a, &changed, [3., 1.], false, 1_000_000)
                .unwrap()
                .is_none()
        );
        let mut changed = b.clone();
        for k in &mut changed.knots[degree + 1..degree + 1 + if c0 { degree } else { 1 }] {
            *k = k.next_up();
        }
        assert!(
            verify(&a, &changed, [3., 1.], false, 1_000_000)
                .unwrap()
                .is_none()
        );
        let mut changed = b.clone();
        changed.control_points[1][0] = changed.control_points[1][0].next_up();
        assert!(
            verify(&a, &changed, [3., 1.], false, 1_000_000)
                .unwrap()
                .is_none()
        );
        let reversed = b.reverse().unwrap();
        let reverse = verify(&a, &reversed, [1., 3.], true, 1_000_000)
            .unwrap()
            .unwrap();
        reverse.reverify(1_000_000).unwrap();
        assert_eq!(reverse.orientation(), -1);
    }
}
#[test]
fn independently_scaled_actual_domains_and_weight_gauges_keep_exact_basis_correspondence() {
    let (mut a, mut b) = pair(2, false);
    for k in &mut a.knots {
        *k = -4. + 8. * *k;
    }
    for k in &mut b.knots {
        *k = 10. + 4. * *k;
    }
    for w in &mut a.weights {
        *w *= 0.5;
    }
    for w in &mut b.weights {
        *w *= 8.;
    }
    let proof = verify(&a, &b, [3., 1.], false, 1_000_000).unwrap().unwrap();
    proof.reverify(1_000_000).unwrap();
    assert!(proof.reverify(proof.work_used() - 1).is_err());
    assert!(
        verify(&a, &b, [1., 1.], false, 1_000_000)
            .unwrap()
            .is_none()
    );
    assert!(verify(&a, &b, [3., 1.], false, 0).unwrap().is_none());
}
fn triangle() -> (Curve, Curve, p::Report, p::Report) {
    let a = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 0.75, 1., 1.],
        control_points: vec![
            vec![0.5, 0.5],
            vec![0.75, 0.5],
            vec![0.5, 0.75],
            vec![0.5, 0.5],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    let mut b = a.clone();
    b.knots = vec![0., 0., 0.25, 0.5, 1., 1.];
    b.weights = vec![1., 1.5, 2., 3.];
    let pa = p::split_candidates(&a, [[0., 2.]; 2], &[0.5, 0.75], &[[0, 0]; 3], 3).unwrap();
    let pb = p::split_candidates(&b, [[0., 2.]; 2], &[0.25, 0.5], &[[0, 0]; 3], 3).unwrap();
    (a, b, pa, pb)
}
#[test]
fn multispan_original_sources_reach_nonlinear_homotopy_and_weighted_graph() {
    let (a, b, pa, pb) = triangle();
    let receipt =
        p::verify_projective_partition_homotopy(&a, &b, &pa, &pb, [3., 1.], false, 1_000_000)
            .unwrap()
            .unwrap();
    receipt.reverify(1_000_000).unwrap();
    let limits = p::ApproximateChartLimits {
        isotopy: p::IsotopyLimits {
            tolerance_uv: 0.01,
            max_deviation_cells: 40000,
            max_jet_cells: 256,
            max_pairs: 20000,
        },
        max_edges: 128,
        max_images: 256,
        max_cells: 30000,
        max_checks: 10000,
        tolerance_uv: 1e-8,
    };
    let result = p::assemble_projective_coherent_weighted_chart(
        &[a, b],
        &[pa.clone(), pb],
        &[receipt],
        [[0., 2.]; 2],
        [0.55, 0.55],
        limits,
        1_000_000,
    )
    .unwrap();
    assert!(
        result.assembly.candidate_winding_preserved,
        "{:?}",
        result.assembly
    );
    assert_eq!(result.assembly.transfer.winding, Some(2));
    let graph = result.assembly.graph.unwrap();
    assert_eq!(graph.edges.len(), 3);
    for (i, edge) in graph.edges.iter().enumerate() {
        assert_eq!(edge.coefficient, 2);
        assert_eq!(edge.curve, pa.pieces[i].curve);
        assert_eq!(
            edge.contributions
                .iter()
                .map(|c| c.source)
                .collect::<Vec<_>>(),
            vec![Some([0, i]), Some([1, i])]
        );
    }
}
#[test]
fn directional_anchor_clearance_covers_the_entire_homotopy_and_shared_budget() {
    let (a, _, pa, _) = triangle();
    let limits = p::AnchorLimits {
        isotopy: p::IsotopyLimits {
            tolerance_uv: 0.1,
            max_deviation_cells: 40000,
            max_jet_cells: 256,
            max_pairs: 20000,
        },
        tolerance_uv: 1e-8,
        max_images: 256,
        max_cells: 30000,
        max_checks: 10000,
    };
    let source = [a.clone()];
    let partitions = [pa.clone()];
    let positive =
        p::verify_anchor_winding(&source, &partitions, [[0., 2.]; 2], [0.55, 0.55], limits)
            .unwrap();
    assert_eq!(positive.winding, Some(1), "{positive:?}");
    let mut low = limits;
    low.max_checks = positive.checks - 1;
    let limited =
        p::verify_anchor_winding(&source, &partitions, [[0., 2.]; 2], [0.55, 0.55], low).unwrap();
    assert!(limited.winding.is_none());
    assert_eq!(limited.reason, Some("anchor-check-budget"));
    let mut moving = pa;
    for piece in &mut moving.pieces {
        for pole in &mut piece.curve.control_points {
            pole[0] += 0.02;
        }
    }
    let crossing =
        p::verify_anchor_winding(&[a], &[moving], [[0., 2.]; 2], [0.505, 0.505], limits).unwrap();
    assert!(crossing.isotopy.periodic_isotopy_preserved, "{crossing:?}");
    assert!(crossing.winding.is_none());
    assert_eq!(crossing.reason, Some("anchor-homotopy-clearance-unproven"));
}
#[test]
fn tiny_independent_original_domains_do_not_underflow_knot_correspondence() {
    let (mut a, mut b) = pair(2, false);
    for k in &mut a.knots {
        *k *= 2.0_f64.powi(-500);
    }
    for k in &mut b.knots {
        *k *= 2.0_f64.powi(-700);
    }
    let proof = verify(&a, &b, [3., 1.], false, 1_000_000).unwrap().unwrap();
    proof.reverify(1_000_000).unwrap();
    let mut changed = b.clone();
    changed.knots[3] = changed.knots[3].next_up();
    assert!(
        verify(&a, &changed, [3., 1.], false, 1_000_000)
            .unwrap()
            .is_none()
    );
}

#[test]
fn rounded_nonbinary_domain_scaling_cannot_inherit_exact_knot_phase() {
    let (mut a, mut b) = pair(2, false);
    for k in &mut a.knots {
        *k *= 1e-150;
    }
    for k in &mut b.knots {
        *k *= 1e-200;
    }
    // .75*Ahi is rounded in this actual binary64 definition. There is no
    // exact global knot correspondence to silently replace it with.
    assert!(
        verify(&a, &b, [3., 1.], false, 1_000_000)
            .unwrap()
            .is_none()
    );
}
