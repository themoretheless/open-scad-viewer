use nurbs_core::{
    curve::Curve,
    periodic_trim::{self as p, AnchorLimits},
};
fn limits() -> AnchorLimits {
    AnchorLimits {
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
    }
}
fn rectangle(b: [f64; 4], reverse: bool) -> Curve {
    let [x0, x1, y0, y1] = b;
    let c = Curve::from_polyline(vec![
        vec![x0, y0],
        vec![x1, y0],
        vec![x1, y1],
        vec![x0, y1],
        vec![x0, y0],
    ])
    .unwrap();
    if reverse { c.reverse().unwrap() } else { c }
}
fn split(c: &Curve) -> p::Report {
    p::split_approximated(c, [[0., 4.], [0., 1.]], 1e-8, 0.01, 16, 32, 10000, 30000)
        .unwrap()
        .partition
        .unwrap()
}
#[test]
fn coherent_copies_preserve_multiplicity_with_independent_global_weight_scales() {
    let c = rectangle([3.5, 4.25, 0.25, 0.75], false);
    let part = split(&c);
    let mut scaled = c.clone();
    for w in &mut scaled.weights {
        *w *= 2.;
    }
    let mut scaled_part = part.clone();
    for piece in &mut scaled_part.pieces {
        for w in &mut piece.curve.weights {
            *w *= 4.;
        }
    }
    let mut preview = part.clone();
    preview.candidate_hulls_contained = false;
    let sources = [c.clone(), scaled, c];
    let parts = [part, scaled_part, preview];
    let result = p::verify_coherent_anchor_winding(
        &sources,
        &parts,
        [[0., 4.], [0., 1.]],
        [3.75, 0.5],
        limits(),
    )
    .unwrap();
    assert!(result.coherent_source_winding_preserved, "{result:?}");
    assert_eq!(result.groups, vec![vec![0, 1, 2]]);
    assert_eq!(result.representatives, vec![0]);
    assert_eq!(result.quotient.winding, Some(1));
    assert_eq!(result.winding, Some(3));
    assert_eq!(result.original_source.as_ref().unwrap().winding, Some(3));
    assert!(result.classification_cells <= limits().max_cells);
    assert!(result.quotient.isotopy.periodic_isotopy_preserved);
}
#[test]
fn unequal_group_counts_use_original_query_not_scaled_quotient_winding() {
    let outer = rectangle([3.5, 4.25, 0.25, 0.75], false);
    let inner = rectangle([3.625, 4.125, 0.375, 0.625], true);
    let sources = [outer.clone(), outer, inner];
    let parts = sources.iter().map(split).collect::<Vec<_>>();
    let result = p::verify_coherent_anchor_winding(
        &sources,
        &parts,
        [[0., 4.], [0., 1.]],
        [3.75, 0.5],
        limits(),
    )
    .unwrap();
    assert!(result.coherent_source_winding_preserved, "{result:?}");
    assert_eq!(result.groups, vec![vec![0, 1], vec![2]]);
    assert_eq!(result.quotient.winding, Some(0));
    assert_eq!(result.winding, Some(1));
}
#[test]
fn changed_candidate_family_and_unproved_quotient_never_publish_original_winding() {
    let c = rectangle([3.5, 4.25, 0.25, 0.75], false);
    let part = split(&c);
    let mut bad = part.clone();
    bad.pieces[0].translation[0] += 0.125;
    let result = p::verify_coherent_anchor_winding(
        &[c.clone(), c.clone()],
        &[part.clone(), bad],
        [[0., 4.], [0., 1.]],
        [3.75, 0.5],
        limits(),
    );
    assert!(result.is_err() || !result.unwrap().coherent_source_winding_preserved);
    let mut low = limits();
    low.isotopy.max_pairs = 1;
    let result = p::verify_coherent_anchor_winding(
        &[c.clone(), c],
        &[part.clone(), part],
        [[0., 4.], [0., 1.]],
        [3.75, 0.5],
        low,
    )
    .unwrap();
    assert!(!result.coherent_source_winding_preserved && result.winding.is_none());
}
#[test]
fn full_source_classifier_has_only_the_remaining_shared_allowance() {
    let c = rectangle([3.5, 4.25, 0.25, 0.75], false);
    let part = split(&c);
    let sources = [c.clone(), c.clone(), c];
    let parts = [part.clone(), part.clone(), part];
    let result = p::verify_coherent_anchor_winding(
        &sources,
        &parts,
        [[0., 4.], [0., 1.]],
        [3.75, 0.5],
        limits(),
    )
    .unwrap();
    assert!(result.coherent_source_winding_preserved);
    let spent = result.quotient.source.as_ref().unwrap().cells;
    assert!(spent > 0);
    let mut low = limits();
    low.max_cells = spent;
    let result =
        p::verify_coherent_anchor_winding(&sources, &parts, [[0., 4.], [0., 1.]], [3.75, 0.5], low)
            .unwrap();
    assert!(!result.coherent_source_winding_preserved && result.winding.is_none());
    assert!(result.classification_cells <= low.max_cells);
    assert!(
        p::verify_coherent_anchor_winding(
            &sources,
            &parts[..2],
            [[0., 4.], [0., 1.]],
            [3.75, 0.5],
            limits()
        )
        .is_err()
    );
}

#[test]
fn coherent_weighted_chart_uses_full_original_multiplicity() {
    let c = rectangle([3.5, 4.25, 0.25, 0.75], false);
    let part = split(&c);
    let mut sources = [c.clone(), c.clone(), c];
    let mut parts = [part.clone(), part.clone(), part];
    for w in &mut sources[1].weights {
        *w *= 2.;
    }
    for piece in &mut parts[1].pieces {
        for w in &mut piece.curve.weights {
            *w *= 4.;
        }
    }
    let l = limits();
    let limits = p::ApproximateChartLimits {
        isotopy: l.isotopy,
        max_edges: 128,
        max_images: l.max_images,
        max_cells: l.max_cells,
        max_checks: l.max_checks,
        tolerance_uv: l.tolerance_uv,
    };
    let result = p::assemble_coherent_weighted_chart(
        &sources,
        &parts,
        [[0., 4.], [0., 1.]],
        [3.75, 0.5],
        limits,
    )
    .unwrap();
    assert!(result.candidate_winding_preserved, "{result:?}");
    let graph = result.graph.unwrap();
    assert!(graph.edges.iter().all(|e| e.coefficient == 3));
    for (q, w) in [([3.75, 0.5], 3), ([0.125, 0.5], 3), ([1., 0.5], 0)] {
        assert_eq!(
            p::inspect_weighted_winding(&graph, [[0., 4.], [0., 1.]], q, 1e-8, 256, 30000)
                .unwrap()
                .winding,
            Some(w)
        );
    }
}

fn negate_reverse(c: &Curve) -> Curve {
    let mut c = c.clone();
    c.control_points.reverse();
    c.weights.reverse();
    c.knots = c.knots.into_iter().rev().map(|k| -k).collect();
    c
}
fn reverse_partition(p: &p::Report) -> p::Report {
    p::Report {
        candidate_hulls_contained: p.candidate_hulls_contained,
        pieces: p
            .pieces
            .iter()
            .rev()
            .map(|x| p::Piece {
                source_domain: [-x.source_domain[1], -x.source_domain[0]],
                translation: x.translation,
                tile: x.tile,
                curve: negate_reverse(&x.curve),
            })
            .collect(),
    }
}
#[test]
fn reverse_homotopies_with_nonbinary_seam_parameters_cancel_exactly() {
    let c = rectangle([3.5, 4.25, 0.25, 0.75], false);
    let part = split(&c);
    let sources = [c.clone(), negate_reverse(&c)];
    let parts = [part.clone(), reverse_partition(&part)];
    let l = limits();
    let limits = p::ApproximateChartLimits {
        isotopy: l.isotopy,
        max_edges: 128,
        max_images: l.max_images,
        max_cells: l.max_cells,
        max_checks: l.max_checks,
        tolerance_uv: l.tolerance_uv,
    };
    let r = p::assemble_coherent_weighted_chart(
        &sources,
        &parts,
        [[0., 4.], [0., 1.]],
        [3.75, 0.5],
        limits,
    )
    .unwrap();
    assert!(r.candidate_winding_preserved, "{r:?}");
    assert_eq!(r.transfer.source_orientations, vec![1, -1]);
    assert_eq!(r.transfer.groups, vec![vec![0, 1]]);
    assert_eq!(r.transfer.winding, Some(0));
    let graph = r.graph.unwrap();
    assert!(graph.edges.is_empty() && graph.loops.is_empty());
    assert!(graph.cancellations.iter().flatten().count() > 0);
}
#[test]
fn affine_source_and_candidate_domains_preserve_signed_copy_coherence() {
    let c = rectangle([0.25, 0.75, 0.25, 0.75], false);
    let part = p::split_candidates(&c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap();
    let mut scaled = c.clone();
    for k in &mut scaled.knots {
        *k = 8. + 2. * *k;
    }
    let mut scaled_part = part.clone();
    for x in &mut scaled_part.pieces {
        x.source_domain = x.source_domain.map(|u| 8. + 2. * u);
        for k in &mut x.curve.knots {
            *k = 8. + 2. * *k;
        }
    }
    let reverse = negate_reverse(&scaled);
    let reverse_part = reverse_partition(&scaled_part);
    let l = limits();
    let limits = p::ApproximateChartLimits {
        isotopy: l.isotopy,
        max_edges: 128,
        max_images: l.max_images,
        max_cells: l.max_cells,
        max_checks: l.max_checks,
        tolerance_uv: l.tolerance_uv,
    };
    let r = p::assemble_coherent_weighted_chart(
        &[c, scaled, reverse],
        &[part, scaled_part, reverse_part],
        [[0., 1.]; 2],
        [0.5, 0.5],
        limits,
    )
    .unwrap();
    assert!(r.candidate_winding_preserved, "{r:?}");
    assert_eq!(r.transfer.source_orientations, vec![1, 1, -1]);
    assert_eq!(r.transfer.groups, vec![vec![0, 1, 2]]);
    assert_eq!(r.transfer.winding, Some(1));
    let graph = r.graph.unwrap();
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.edges[0].coefficient, 1);
}
#[test]
fn nonaffine_domain_perturbation_is_not_a_coherent_parameter_map() {
    let c = rectangle([0.25, 0.75, 0.25, 0.75], false);
    let part = p::split_candidates(&c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap();
    let mut changed = c.clone();
    changed.knots[2] += 0.125;
    let r = p::verify_coherent_anchor_winding(
        &[c, changed],
        &[part.clone(), part],
        [[0., 1.]; 2],
        [0.5, 0.5],
        limits(),
    );
    assert!(r.is_err() || !r.unwrap().coherent_source_winding_preserved);
}
