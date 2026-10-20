use brep_core::profile_certificate::regularize_periodic_winding;
use nurbs_core::{
    curve::Curve,
    periodic_trim::{self as periodic, ChartAssemblyLimits},
};
fn limits() -> ChartAssemblyLimits {
    ChartAssemblyLimits {
        max_segments: 128,
        max_links: 64,
        max_edges: 128,
        max_images: 256,
        max_cells: 30000,
        tolerance_uv: 1e-8,
    }
}
fn circle() -> Curve {
    let mut c = nurbs_core::primitives::circle([0., 0., 0.], [0., 0., 1.], 0.125).unwrap();
    for p in &mut c.control_points {
        p[0] += 0.375;
        p[1] += 0.5;
        p.pop();
    }
    c
}
fn partitions(source: &[Curve]) -> Vec<periodic::Report> {
    source
        .iter()
        .map(|c| periodic::split_candidates(c, [[0., 1.]; 2], &[], &[[0, 0]], 1).unwrap())
        .collect()
}
#[test]
fn repeated_rational_cycles_keep_multiplicity_and_one_original_boundary() {
    let c = circle();
    let mut source = vec![c.clone(), c.clone(), c.clone()];
    let p = partitions(&source);
    let result =
        regularize_periodic_winding(&source, &p, [[0., 1.]; 2], [0.375, 0.5], limits(), 1e-7)
            .unwrap();
    result.reverify_source().unwrap();
    assert_eq!(result.regions().coefficients(), &[3]);
    assert_eq!(result.regions().winding(), &[[0, 3]]);
    assert_eq!(result.regions().components().len(), 1);
    assert_eq!(result.regions().components()[0].wires(), &[vec![c.clone()]]);
    let graph = result.assembly().graph.as_ref().unwrap();
    assert_eq!(graph.edges.len(), 1);
    assert_eq!(graph.original_edges, 3);
    let mut original = result
        .original_cycles()
        .iter()
        .flatten()
        .copied()
        .collect::<Vec<_>>();
    original.sort();
    assert_eq!(original, (0..graph.loops.len()).collect::<Vec<_>>());
    source[0].control_points[0][0] = 100.;
    assert_eq!(result.source(), &[c.clone(), c.clone(), c]);
    result.reverify_source().unwrap();
}
#[test]
fn exact_opposites_cancel_and_negative_fill_is_material_left() {
    let c = circle();
    let reversed = c.reverse().unwrap();
    for source in [
        vec![c.clone(), reversed.clone()],
        vec![c.clone(), reversed.clone(), reversed.clone()],
    ] {
        let result = regularize_periodic_winding(
            &source,
            &partitions(&source),
            [[0., 1.]; 2],
            [0.375, 0.5],
            limits(),
            1e-7,
        )
        .unwrap();
        result.reverify_source().unwrap();
        if source.len() == 2 {
            assert!(result.regions().components().is_empty());
            assert!(result.cycle_edges().is_empty());
            assert_eq!(
                result
                    .assembly()
                    .graph
                    .as_ref()
                    .unwrap()
                    .cancellations
                    .len(),
                1
            );
        } else {
            assert_eq!(result.regions().winding(), &[[0, -1]]);
            assert_eq!(result.regions().components()[0].reversed(), &[true]);
            assert_eq!(
                result.regions().components()[0]
                    .verified_profile()
                    .nesting_depth(),
                &[0]
            );
        }
    }
}
#[test]
fn duplicated_seam_crossing_regions_keep_both_chart_components() {
    let c = Curve::from_polyline(vec![
        vec![0.75, 0.25],
        vec![1.25, 0.25],
        vec![1.25, 0.75],
        vec![0.75, 0.75],
        vec![0.75, 0.25],
    ])
    .unwrap();
    let p =
        periodic::split_candidates(&c, [[0., 1.]; 2], &[0.5, 2.5], &[[0, 0], [1, 0], [0, 0]], 3)
            .unwrap();
    let result = regularize_periodic_winding(
        &[c.clone(), c],
        &[p.clone(), p],
        [[0., 1.]; 2],
        [0.875, 0.5],
        limits(),
        1e-7,
    )
    .unwrap();
    result.reverify_source().unwrap();
    assert_eq!(result.regions().components().len(), 2);
    assert_eq!(result.regions().coefficients(), &[2, 2]);
    assert!(result.regions().winding().iter().all(|w| *w == [0, 2]));
    let graph = result.assembly().graph.as_ref().unwrap();
    for (ids, wire) in result.cycle_edges().iter().zip(result.regions().source()) {
        assert_eq!(ids.len(), wire.len());
        for (&i, c) in ids.iter().zip(wire) {
            assert_eq!(&graph.edges[i].curve, c);
        }
    }
    // Independent planar rectangle membership, away from boundary/seams.
    for q in [[0.125, 0.5], [0.875, 0.5], [0.5, 0.5], [0.125, 0.125]] {
        let expected = (q[0] < 0.25 || q[0] > 0.75) && q[1] > 0.25 && q[1] < 0.75;
        let actual = result.regions().components().iter().any(|component| {
            let points = component.wires()[0]
                .iter()
                .flat_map(|c| &c.control_points)
                .collect::<Vec<_>>();
            (0..2).all(|k| {
                points.iter().map(|p| p[k]).fold(f64::INFINITY, f64::min) < q[k]
                    && q[k]
                        < points
                            .iter()
                            .map(|p| p[k])
                            .fold(f64::NEG_INFINITY, f64::max)
            })
        });
        assert_eq!(actual, expected, "q={q:?}");
    }
}
#[test]
fn malformed_partition_incomplete_flow_budget_and_invalid_tolerance_refuse() {
    let c = circle();
    let source = vec![c.clone(), c.clone(), c];
    let mut p = partitions(&source);
    p[0].pieces[0].translation[0] += 0.125;
    assert!(
        regularize_periodic_winding(&source, &p, [[0., 1.]; 2], [0.375, 0.5], limits(), 1e-7)
            .is_err()
    );
    let p = partitions(&source);
    let mut low = limits();
    low.max_edges = 1;
    assert!(
        regularize_periodic_winding(&source, &p, [[0., 1.]; 2], [0.375, 0.5], low, 1e-7).is_err()
    );
    assert!(
        regularize_periodic_winding(&source, &p, [[0., 1.]; 2], [0.375, 0.5], limits(), f64::NAN)
            .is_err()
    );
    assert!(
        regularize_periodic_winding(
            &source,
            &p[..2],
            [[0., 1.]; 2],
            [0.375, 0.5],
            limits(),
            1e-7
        )
        .is_err()
    );
}

fn approximate_limits() -> periodic::ApproximateChartLimits {
    periodic::ApproximateChartLimits {
        isotopy: periodic::IsotopyLimits {
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
    }
}
#[test]
fn approximate_periodic_regions_keep_source_topology_and_exact_candidate_profiles() {
    use brep_core::profile_certificate::regularize_approximated_periodic_winding;
    let c = Curve::from_polyline(vec![
        vec![0.75, 0.25],
        vec![1.125, 0.25],
        vec![1.125, 0.75],
        vec![0.75, 0.75],
        vec![0.75, 0.25],
    ])
    .unwrap();
    for source in [c.clone(), c.reverse().unwrap()] {
        let split =
            periodic::split_approximated(&source, [[0., 1.]; 2], 1e-8, 0.01, 16, 32, 10000, 30000)
                .unwrap();
        assert!(!split.approximated_events.is_empty());
        let partition = split.partition.unwrap();
        let result = regularize_approximated_periodic_winding(
            &[source.clone()],
            &[partition],
            [[0., 1.]; 2],
            [0.875, 0.5],
            approximate_limits(),
            1e-7,
        )
        .unwrap();
        result.reverify_source().unwrap();
        assert_eq!(result.source(), &[source]);
        assert_eq!(result.candidate_regions().components().len(), 2);
        assert!(result.assembly().transfer.winding.is_some());
        for component in result.candidate_regions().components() {
            assert_eq!(component.verified_profile().nesting_depth(), &[0]);
        }
        for (q, w) in [([0.0625, 0.5], 1), ([0.875, 0.5], 1), ([0.5, 0.5], 0)] {
            let actual = periodic::inspect_weighted_winding(
                result.assembly().graph.as_ref().unwrap(),
                [[0., 1.]; 2],
                q,
                1e-8,
                256,
                30000,
            )
            .unwrap()
            .winding
            .unwrap();
            assert_eq!(actual.abs(), w);
        }
    }
}
#[test]
fn approximate_periodic_regions_refuse_stale_source_and_incomplete_isotopy() {
    use brep_core::profile_certificate::regularize_approximated_periodic_winding;
    let source = Curve::from_polyline(vec![
        vec![0.75, 0.25],
        vec![1.125, 0.25],
        vec![1.125, 0.75],
        vec![0.75, 0.75],
        vec![0.75, 0.25],
    ])
    .unwrap();
    let p = periodic::split_approximated(&source, [[0., 1.]; 2], 1e-8, 0.01, 16, 32, 10000, 30000)
        .unwrap()
        .partition
        .unwrap();
    let mut bad = p.clone();
    bad.pieces[0].translation[0] += 0.125;
    assert!(
        regularize_approximated_periodic_winding(
            &[source.clone()],
            &[bad],
            [[0., 1.]; 2],
            [0.875, 0.5],
            approximate_limits(),
            1e-7
        )
        .is_err()
    );
    let mut low = approximate_limits();
    low.isotopy.max_pairs = 1;
    assert!(
        regularize_approximated_periodic_winding(
            &[source],
            &[p],
            [[0., 1.]; 2],
            [0.875, 0.5],
            low,
            1e-7
        )
        .is_err()
    );
}

#[test]
fn coherent_copies_reach_owned_candidate_profiles_without_duplicate_geometry() {
    use brep_core::profile_certificate::regularize_coherent_periodic_winding;
    let c = Curve::from_polyline(vec![
        vec![0.75, 0.25],
        vec![1.125, 0.25],
        vec![1.125, 0.75],
        vec![0.75, 0.75],
        vec![0.75, 0.25],
    ])
    .unwrap();
    let p = periodic::split_approximated(&c, [[0., 1.]; 2], 1e-8, 0.01, 16, 32, 10000, 30000)
        .unwrap()
        .partition
        .unwrap();
    let mut source = [c.clone(), c.clone(), c];
    let mut parts = [p.clone(), p.clone(), p];
    for w in &mut source[1].weights {
        *w *= 2.;
    }
    for piece in &mut parts[1].pieces {
        for w in &mut piece.curve.weights {
            *w *= 4.;
        }
    }
    let result = regularize_coherent_periodic_winding(
        &source,
        &parts,
        [[0., 1.]; 2],
        [0.875, 0.5],
        approximate_limits(),
        1e-7,
    )
    .unwrap();
    result.reverify_source().unwrap();
    assert_eq!(result.assembly().transfer.winding, Some(3));
    assert_eq!(result.assembly().transfer.quotient.winding, Some(1));
    assert_eq!(result.candidate_regions().coefficients(), &[3, 3]);
    assert_eq!(result.candidate_regions().components().len(), 2);
    assert!(
        result
            .assembly()
            .graph
            .as_ref()
            .unwrap()
            .edges
            .iter()
            .all(|e| e.coefficient == 3)
    );
}
#[test]
fn coherent_nonuniform_multiplicity_preserves_original_fill_not_quotient_hole() {
    use brep_core::profile_certificate::regularize_coherent_periodic_winding;
    let outer = Curve::from_polyline(vec![
        vec![0.25, 0.25],
        vec![0.75, 0.25],
        vec![0.75, 0.75],
        vec![0.25, 0.75],
        vec![0.25, 0.25],
    ])
    .unwrap();
    let inner = Curve::from_polyline(vec![
        vec![0.375, 0.375],
        vec![0.625, 0.375],
        vec![0.625, 0.625],
        vec![0.375, 0.625],
        vec![0.375, 0.375],
    ])
    .unwrap()
    .reverse()
    .unwrap();
    let sources = [outer.clone(), outer.clone(), inner];
    let p = partitions(&sources);
    let result = regularize_coherent_periodic_winding(
        &sources,
        &p,
        [[0., 1.]; 2],
        [0.5, 0.5],
        approximate_limits(),
        1e-7,
    )
    .unwrap();
    result.reverify_source().unwrap();
    assert_eq!(result.assembly().transfer.quotient.winding, Some(0));
    assert_eq!(result.assembly().transfer.winding, Some(1));
    assert_eq!(result.candidate_regions().components().len(), 1);
    assert_eq!(
        result.candidate_regions().components()[0].wires(),
        &[vec![outer]]
    );
}

#[test]
fn reverse_coherent_seam_paths_produce_an_empty_owned_uv_region() {
    use brep_core::profile_certificate::regularize_coherent_periodic_winding;
    fn reverse(c: &Curve) -> Curve {
        let mut b = c.clone();
        b.control_points.reverse();
        b.weights.reverse();
        b.knots = b.knots.into_iter().rev().map(|u| -u).collect();
        b
    }
    let c = Curve::from_polyline(vec![
        vec![0.75, 0.25],
        vec![1.125, 0.25],
        vec![1.125, 0.75],
        vec![0.75, 0.75],
        vec![0.75, 0.25],
    ])
    .unwrap();
    let p = periodic::split_approximated(&c, [[0., 1.]; 2], 1e-8, 0.01, 16, 32, 10000, 30000)
        .unwrap()
        .partition
        .unwrap();
    let rp = periodic::Report {
        candidate_hulls_contained: p.candidate_hulls_contained,
        pieces: p
            .pieces
            .iter()
            .rev()
            .map(|x| periodic::Piece {
                source_domain: [-x.source_domain[1], -x.source_domain[0]],
                translation: x.translation,
                tile: x.tile,
                curve: reverse(&x.curve),
            })
            .collect(),
    };
    let result = regularize_coherent_periodic_winding(
        &[c.clone(), reverse(&c)],
        &[p, rp],
        [[0., 1.]; 2],
        [0.875, 0.5],
        approximate_limits(),
        1e-7,
    )
    .unwrap();
    result.reverify_source().unwrap();
    assert_eq!(result.assembly().transfer.source_orientations, vec![1, -1]);
    assert_eq!(result.assembly().transfer.winding, Some(0));
    assert!(result.candidate_regions().components().is_empty());
}
#[test]
fn affine_reparameterized_copies_reach_single_candidate_region() {
    use brep_core::profile_certificate::regularize_coherent_periodic_winding;
    let c = Curve::from_polyline(vec![
        vec![0.25, 0.25],
        vec![0.75, 0.25],
        vec![0.75, 0.75],
        vec![0.25, 0.75],
        vec![0.25, 0.25],
    ])
    .unwrap();
    let p = partitions(&[c.clone()]).pop().unwrap();
    let mut shifted = c.clone();
    for k in &mut shifted.knots {
        *k = 8. + 2. * *k;
    }
    let mut sp = p.clone();
    for x in &mut sp.pieces {
        x.source_domain = x.source_domain.map(|u| 8. + 2. * u);
        for k in &mut x.curve.knots {
            *k = 8. + 2. * *k;
        }
    }
    let result = regularize_coherent_periodic_winding(
        &[c.clone(), shifted],
        &[p, sp],
        [[0., 1.]; 2],
        [0.5, 0.5],
        approximate_limits(),
        1e-7,
    )
    .unwrap();
    result.reverify_source().unwrap();
    assert_eq!(result.assembly().transfer.source_orientations, vec![1, 1]);
    assert_eq!(result.candidate_regions().coefficients(), &[2]);
    assert_eq!(
        result.candidate_regions().components()[0].wires(),
        &[vec![c]]
    );
}
