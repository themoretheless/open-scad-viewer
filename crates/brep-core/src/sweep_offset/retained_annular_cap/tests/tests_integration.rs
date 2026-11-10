use super::*;
fn spans(c: &Curve) -> Vec<Curve> {
    assert_eq!(c.degree, 2);
    assert_eq!(c.control_points.len(), 9);
    (0..4)
        .map(|i| Curve {
            degree: 2,
            knots: vec![0., 0., 0., 1., 1., 1.],
            control_points: c.control_points[2 * i..=2 * i + 2].to_vec(),
            weights: c.weights[2 * i..=2 * i + 2].to_vec(),
            periodic: false,
        })
        .collect()
}
fn original_endpoint() -> (Vec<Curve>, Vec<Curve>) {
    let outer = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5).unwrap();
    let inner = nurbs_core::primitives::circle([0.; 3], [0., 0., -1.], 0.2).unwrap();
    let sections = nurbs_core::paths::miter_sections(
        &[outer, inner],
        &[[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 15.]],
        [1., 0., 0.],
        2.,
    )
    .unwrap();
    let last = sections.last().unwrap();
    (spans(&last[0]), spans(&last[1].reverse().unwrap()))
}
#[test]
fn original_spatial_hollow_end_curves_are_retained_with_exact_patch_boundaries() {
    let (outer, inner) = original_endpoint();
    let before = (outer.clone(), inner.clone());
    let cap = construct(&outer, &inner, 1_000_000).unwrap();
    assert_eq!(cap.patches.len(), 4);
    assert_eq!(cap.exact_boundary_uses, 16);
    assert!(cap.exact_work > 0 && cap.exact_work <= 1_000_000);
    assert!(!cap.global_embedding_certified);
    for (i, s) in cap.patches.iter().enumerate() {
        for k in 0..3 {
            assert_eq!(s.control_points[k][0], outer[i].control_points[k]);
            assert_eq!(s.control_points[k][1], inner[i].control_points[k]);
        }
        let next = &cap.patches[(i + 1) % 4];
        assert_eq!(s.control_points.last(), next.control_points.first());
        assert_eq!(s.weights.last(), next.weights.first());
        assert!(
            nurbs_core::surface_regularity::inspect(s, 1000)
                .unwrap()
                .spanwise_regular
        );
        assert!(
            nurbs_core::surface_linear_monotonicity::inspect_candidate_with_hint(s, None, 1000)
                .unwrap()
                .certified
        );
    }
    assert_eq!((outer, inner), before);
}
#[test]
fn exhausted_work_and_mismatched_correspondence_refuse_without_moving_curves() {
    let (outer, inner) = original_endpoint();
    let before = (outer.clone(), inner.clone());
    let empty = construct(&outer, &inner, 0).unwrap_err();
    assert!(
        empty.to_string().contains("unproved") && empty.to_string().contains("ResourceLimit"),
        "{empty}"
    );
    let full = construct(&outer, &inner, 1_000_000).unwrap();
    assert!(construct(&outer, &inner, full.exact_work - 1).is_err());
    let mut changed = inner.clone();
    changed[0].control_points[0][0] = f64::from_bits(changed[0].control_points[0][0].to_bits() + 1);
    assert!(construct(&outer, &changed, 1_000_000).is_err());
    assert_eq!((outer, inner), before);
}
#[test]
fn independent_positive_ring_weights_are_retained() {
    let (outer, mut inner) = original_endpoint();
    inner[0].weights[1] = f64::from_bits(inner[0].weights[1].to_bits() + 1);
    let before = inner.clone();
    let cap = construct(&outer, &inner, 1_000_000).unwrap();
    assert_eq!(
        cap.patches[0].weights[1],
        vec![outer[0].weights[1], inner[0].weights[1]]
    );
    assert_eq!(inner, before);
    assert_eq!(cap.exact_boundary_uses, 16);
}
fn original_model() -> crate::Model {
    let outer = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5).unwrap();
    let hole = nurbs_core::primitives::circle([0.; 3], [0., 0., -1.], 0.2).unwrap();
    let sections = nurbs_core::paths::miter_sections(
        &[outer, hole],
        &[[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 15.]],
        [1., 0., 0.],
        2.,
    )
    .unwrap();
    let nested = sections
        .into_iter()
        .map(|s| s.into_iter().map(|c| vec![c]).collect())
        .collect::<Vec<_>>();
    crate::rational_section_loft(&nested).unwrap()
}
#[test]
fn assembled_original_hollow_has_exact_closed_boundary_without_moving_walls() {
    let model = original_model();
    let before = model.clone();
    let old = crate::boundary_agreement::verify_exact(&model, 1_000_000).unwrap();
    assert!(!old.all_equal);
    let cap = replace(&model, 25, 1_000_000).unwrap();
    assert_eq!(cap.cap_faces, vec![25, 26, 27, 28]);
    assert!(!cap.global_embedding_certified);
    assert!(cap.exact_work > 0 && cap.exact_work <= 1_000_000);
    assert_eq!(cap.model.vertices, model.vertices);
    assert_eq!(&cap.model.edges[..model.edges.len()], &model.edges[..]);
    assert_eq!(&cap.model.faces[..25], &model.faces[..25]);
    let proof = crate::boundary_agreement::verify_exact(&cap.model, 1_000_000).unwrap();
    assert!(proof.all_equal && proof.all_joins_exact);
    cap.model.validate().unwrap();
    assert_eq!(model, before);
}
#[test]
fn both_caps_keep_opposite_shell_uses_and_explicit_face_mapping() {
    let model = original_model();
    let start = replace(&model, 24, 1_000_000).unwrap();
    assert_eq!(start.source_face_map[24], None);
    assert_eq!(start.source_face_map[25], Some(24));
    assert_eq!(start.model.faces[24].surface, model.faces[25].surface);
    let end = replace(&start.model, start.source_face_map[25].unwrap(), 1_000_000).unwrap();
    assert_eq!(end.model.faces.len(), 32);
    assert_eq!(end.model.vertices, model.vertices);
    let proof = crate::boundary_agreement::verify_exact(&end.model, 1_000_000).unwrap();
    assert!(proof.all_equal && proof.all_joins_exact);
    end.model.validate().unwrap();
    assert!(!end.global_embedding_certified);
    let volume = audit_with_planar_caps(&end.model, &[]);
    assert!(volume.proven);
    assert_eq!(volume.boundary.intersections.pairs.total_pairs, 496);
}
#[test]
fn assembly_budget_exhaustion_and_non_annular_face_leave_source_untouched() {
    let model = original_model();
    let before = model.clone();
    assert!(replace(&model, 25, 0).is_err());
    assert!(replace(&model, 0, 1_000_000).is_err());
    let cap = replace(&model, 25, 1_000_000).unwrap();
    assert!(replace(&model, 25, cap.exact_work - 1).is_err());
    assert_eq!(model, before);
}
fn audit(model: &crate::Model) -> crate::volume_validity::Report {
    audit_with_planar_caps(model, &[24])
}
fn audit_with_planar_caps(
    model: &crate::Model,
    planar_caps: &[usize],
) -> crate::volume_validity::Report {
    let limits = crate::volume_validity::Limits {
        boundary: crate::boundary_embedding::Limits {
            exact_work: 1_000_000,
            trim_pairs: 10_000,
            trim_cells: 100_000,
            trim_domain_cells: 1_000_000,
            spans: 1024,
            contacts: crate::face_contacts::Limits {
                pairs: 10_000,
                cells: 100_000,
                domain_cells: 1_000_000,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10_000,
            },
        },
        nesting_pairs: 1000,
        nesting_cells: 100_000,
        nesting_domain_cells: 1_000_000,
        orientation_cells: 100_000,
        orientation_domain_cells: 1_000_000,
        orientation_spans: 100,
    };
    let caps = crate::sweep_cap_contacts::Budgets {
        max_walls: 1024,
        max_exact_work: 1_000_000,
        max_chart_cells: 1000,
        max_trim_pairs: 100_000,
        max_trim_cells: 100_000,
        max_trim_domain_cells: 1_000_000,
    };
    // The unchanged planar first cap is an optimization. All four new
    // patches participate in generic injectivity and pair classification.
    crate::volume_validity::inspect_sweep(model, 1e-8, limits, 100_000, planar_caps, caps).unwrap()
}
#[test]
fn original_hollow_full_native_volume_proof_covers_all_406_pairs() {
    let source = original_model();
    let cap = replace(&source, 25, 1_000_000).unwrap();
    let proof = audit(&cap.model);
    assert!(proof.proven);
    assert!(proof.boundary.agreement.all_equal && proof.boundary.agreement.all_joins_exact);
    let intersection = &proof.boundary.intersections;
    assert!(intersection.faces.all_faces_injective);
    assert_eq!(intersection.faces.faces.len(), 29);
    assert!(intersection.pairs.all_pairs_classified);
    assert_eq!(intersection.pairs.total_pairs, 406);
    assert_eq!(
        intersection.pairs.pairs.len() + intersection.pairs.grouped_pairs,
        406
    );
    assert_eq!(intersection.pairs.next_pair, None);
    assert_eq!(proof.orientations.len(), 1);
    assert_eq!(proof.orientations[0].outward, Some(true));
    assert_eq!(cap.model.vertices, source.vertices);
}
#[test]
fn glued_pair_proof_requires_its_real_budget_and_refuses_overlap() {
    let cap = replace(&original_model(), 25, 1_000_000).unwrap();
    let (proof, used) =
        crate::shared_boundary::certify_opposite_with_budget(&cap.model, [16, 25], 1000);
    assert!(proof.is_some());
    assert!(used > 6 && used <= 1000);
    let (short, spent) =
        crate::shared_boundary::certify_opposite_with_budget(&cap.model, [16, 25], used - 1);
    assert!(short.is_none());
    assert!(spent <= used - 1);
    let face = nurbs_core::surface_edit::transpose(&cap.model.faces[16].surface).unwrap();
    let overlap = nurbs_core::surface_injectivity::certify_shared_natural_boundary(
        &face,
        &face,
        [true, true],
        1000,
    )
    .unwrap();
    assert!(!overlap.proven);
    assert!(overlap.spans <= 1000);
}
#[test]
fn global_cap_proof_is_not_reused_after_a_single_bit_boundary_change() {
    let cap = replace(&original_model(), 25, 1_000_000).unwrap();
    let mut changed = cap.model.clone();
    let x = &mut changed.faces[25].surface.control_points[0][0][0];
    *x = f64::from_bits(x.to_bits() + 1);
    changed.rebuild_topology_ids();
    let proof = audit(&changed);
    assert!(!proof.proven);
    assert!(!proof.boundary.agreement.all_equal);
    assert_eq!(cap.model.faces.len(), 29);
}
fn filling_limits() -> reference_filling::Limits {
    reference_filling::Limits {
        exact_work: 1_000_000,
        products: 100_000,
        trim_pairs: 10_000,
        trim_cells: 100_000,
        trim_domain_cells: 1_000_000,
        spans: 1024,
        linear_cells: 100_000,
        contacts: crate::face_contacts::Limits {
            pairs: 10_000,
            cells: 100_000,
            domain_cells: 1_000_000,
            cells_per_pair: 1000,
            domain_cells_per_pair: 10_000,
        },
    }
}
#[test]
fn original_hollow_filled_reference_covers_the_annulus_not_only_its_edges() {
    let source = original_model();
    for face in [24, 25] {
        let cap = replace(&source, face, 1_000_000).unwrap();
        let proof =
            reference_filling::inspect(&source, &cap, 1e-10, 1e-8, filling_limits()).unwrap();
        assert!(
            proof.uv_region_covered,
            "{}; region={}, injective={}, pairs={:?}",
            proof.reason,
            proof.reference_region.reason,
            proof.uv_faces.all_faces_injective,
            proof.uv_pairs.next_pair
        );
        assert!(
            proof.within_budget,
            "{}: {:?}",
            proof.reason, proof.error_upper
        );
        assert!(proof.error_upper.unwrap() < 1e-10);
        assert_eq!(proof.products, 108);
        assert_eq!(proof.uv_pairs.total_pairs, 6);
        assert_eq!(proof.uv_faces.faces.len(), 4);
        assert!(!cap.global_embedding_certified);
    }
}
#[test]
fn filled_reference_budget_and_mutation_never_reuse_a_positive_proof() {
    let source = original_model();
    let cap = replace(&source, 25, 1_000_000).unwrap();
    let full = reference_filling::inspect(&source, &cap, 1e-10, 1e-8, filling_limits()).unwrap();
    assert!(full.within_budget);
    let mut limits = filling_limits();
    limits.products = full.products - 1;
    let short = reference_filling::inspect(&source, &cap, 1e-10, 1e-8, limits).unwrap();
    assert!(short.error_upper.is_none() && !short.within_budget);
    assert!(short.products <= limits.products);
    limits = filling_limits();
    limits.exact_work = full.exact_work - 1;
    assert!(reference_filling::inspect(&source, &cap, 1e-10, 1e-8, limits).is_err());
    let mut changed = cap.clone();
    let x = &mut changed.model.faces[25].surface.control_points[0][0][0];
    *x = f64::from_bits(x.to_bits() + 1);
    assert!(reference_filling::inspect(&source, &changed, 1e-10, 1e-8, filling_limits()).is_err());
    let mut omitted = cap.clone();
    omitted.cap_faces.pop();
    assert!(reference_filling::inspect(&source, &omitted, 1e-10, 1e-8, filling_limits()).is_err());
    let strict = reference_filling::inspect(&source, &cap, 0., 1e-8, filling_limits()).unwrap();
    assert!(strict.uv_region_covered && strict.error_upper.is_some() && !strict.within_budget);
}
#[test]
fn filled_reference_requires_region_injectivity_pairs_and_matching_rational_bases() {
    let source = original_model();
    let cap = replace(&source, 25, 1_000_000).unwrap();
    for stage in 0..4 {
        let mut limits = filling_limits();
        match stage {
            0 => limits.trim_pairs = 1,
            1 => limits.trim_domain_cells = 1,
            2 => {
                limits.spans = 1;
                limits.linear_cells = 0;
            }
            _ => limits.contacts.pairs = 1,
        }
        let proof = reference_filling::inspect(&source, &cap, 1e-10, 1e-8, limits).unwrap();
        assert!(!proof.uv_region_covered && proof.error_upper.is_none() && !proof.within_budget);
    }
    let mut mismatch = source.clone();
    let wire = mismatch.faces[25].outer;
    // Uniform homogeneous scaling preserves every boundary evaluation,
    // but does not satisfy the stored world/UV basis identity required by
    // this pointwise radial proof. Keep closure exact on the entire ring.
    for c in &mut mismatch.loops[wire].coedges {
        for w in &mut c.pcurve.weights {
            *w *= 2.;
        }
    }
    let changed = replace(&mismatch, 25, 1_000_000).unwrap();
    let result = reference_filling::inspect(&mismatch, &changed, 1., 1e-8, filling_limits());
    assert!(
        result.is_err(),
        "Matching boundary point sets cannot replace matching rational bases"
    );
}
#[test]
fn original_spatial_hollow_both_filled_caps_are_bounded_against_authored_source() {
    let profiles = vec![
        nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 0.5).unwrap(),
        nurbs_core::primitives::circle([0.; 3], [0., 0., -1.], 0.2).unwrap(),
    ];
    let points = [[0., 0., 0.], [0., 0., 10.], [10., 0., 10.], [10., 10., 15.]];
    let law = |x| Curve {
        degree: 1,
        knots: vec![0., 0., 1., 1.],
        control_points: vec![vec![x, 0., 0.]; 2],
        weights: vec![1.; 2],
        periodic: false,
    };
    let scale = law(1.);
    let twist = law(0.);
    let sweep = nurbs_core::progressive_miter::Sweep::new_direct(
        &profiles,
        &points,
        &scale,
        &twist,
        nurbs_core::progressive_miter::Options {
            normal: [1., 0., 0.],
            closed: false,
            miter_limit: 2.,
            initial_steps: 1,
            max_steps: 1,
            max_deviation: 0.01,
        },
    )
    .unwrap();
    let source = original_model();
    let first = replace(&source, 24, 1_000_000).unwrap();
    let last = replace(&first.model, first.source_face_map[25].unwrap(), 1_000_000).unwrap();
    let groups = [
        first
            .cap_faces
            .iter()
            .map(|&f| last.source_face_map[f].unwrap())
            .collect(),
        last.cap_faces.clone(),
    ];
    let proof = source_filling::inspect(
        &sweep,
        &last.model,
        &groups,
        &[1, 1],
        1e-10,
        1e-8,
        1000,
        filling_limits(),
    )
    .unwrap();
    assert!(proof.source_filling_covered, "{}", proof.reason);
    assert!(
        proof.within_budget,
        "{}: {:?}",
        proof.reason, proof.filled_cap_error_upper
    );
    assert_eq!(proof.products, 288);
    assert_eq!(proof.endpoint_cells, 48);
    assert!(
        proof
            .filled_cap_error_upper
            .unwrap()
            .iter()
            .all(|&v| v < 1e-10)
    );
    let mut denied = filling_limits();
    denied.products = proof.products - 1;
    let short = source_filling::inspect(
        &sweep,
        &last.model,
        &groups,
        &[1, 1],
        1e-10,
        1e-8,
        1000,
        denied,
    )
    .unwrap();
    assert!(short.filled_cap_error_upper.is_none() && !short.within_budget);
    assert!(short.products <= denied.products);
    let mut omitted = groups.clone();
    omitted[1].pop();
    assert!(
        source_filling::inspect(
            &sweep,
            &last.model,
            &omitted,
            &[1, 1],
            1e-10,
            1e-8,
            1000,
            filling_limits()
        )
        .is_err()
    );
    let mut changed = last.model.clone();
    let x = &mut changed.faces[groups[1][0]].surface.control_points[0][0][0];
    *x = x.next_up();
    let mutated = source_filling::inspect(
        &sweep,
        &changed,
        &groups,
        &[1, 1],
        1e-10,
        1e-8,
        1000,
        filling_limits(),
    )
    .unwrap();
    assert!(mutated.filled_cap_error_upper.is_none());
    let authored = profiles
        .iter()
        .flat_map(|c| c.control_points.iter())
        .map(|p| p.clone().try_into().unwrap())
        .collect::<Vec<_>>();
    let full = sweep
        .certify_endpoint_points(&authored, 1000, 1000)
        .unwrap();
    let short = sweep
        .certify_endpoint_points(&authored, full.cells - 1, 1000)
        .unwrap();
    assert!(full.points.is_some() && short.points.is_none());
    assert_eq!(short.cells, full.cells - 1);
}
