use brep_core::{boundary_embedding, face_contacts, feature_ops::full_round};

fn limits() -> boundary_embedding::Limits {
    boundary_embedding::Limits {
        exact_work: 1_000_000,
        trim_pairs: 10_000,
        trim_cells: 100_000,
        trim_domain_cells: 1_000_000,
        spans: 1024,
        contacts: face_contacts::Limits {
            pairs: 10_000,
            cells: 100_000,
            domain_cells: 1_000_000,
            cells_per_pair: 1000,
            domain_cells_per_pair: 10_000,
        },
    }
}

#[test]
fn full_round_original_boundary_embedding_has_complete_face_and_pair_proofs() {
    let source = brep_core::cuboid([0.; 3], [4., 8., 6.]).unwrap();
    let selected = [(0, 0.), (1, 8.), (0, 4.)].map(|(axis, value)| {
        source
            .faces
            .iter()
            .position(|f| {
                f.surface
                    .control_points
                    .iter()
                    .flatten()
                    .all(|p| p[axis] == value)
            })
            .unwrap()
    });
    let rounded = full_round(&source, selected).unwrap();
    let embedded = rounded
        .tangency
        .clone()
        .qualify_embedding(1e-8, limits(), 100_000)
        .unwrap();
    assert_eq!(embedded.model(), &rounded.model);
    assert_eq!(embedded.qualification().envelopes().len(), 2);
    let report = embedded.report();
    assert!(
        report.proven,
        "agreement={} joins={} trim={} injective={} pairs={} pending={:?} unresolved={:?}",
        report.agreement.all_equal,
        report.agreement.all_joins_exact,
        report.trim.all_valid,
        report.intersections.faces.all_faces_injective,
        report.intersections.pairs.all_pairs_classified,
        report.intersections.pairs.next_pair,
        report
            .intersections
            .pairs
            .pairs
            .iter()
            .filter(|p| !p.hull_disjoint && p.boundary.is_none())
            .map(|p| (p.faces, p.reason))
            .collect::<Vec<_>>()
    );
    let count = rounded.model.faces.len();
    assert_eq!(
        report.intersections.pairs.total_pairs,
        count * (count - 1) / 2
    );
    assert_eq!(report.intersections.pairs.next_pair, None);
    let limited = boundary_embedding::inspect_with_linear(
        &rounded.model,
        1e-8,
        boundary_embedding::Limits {
            contacts: face_contacts::Limits {
                pairs: 1,
                ..limits().contacts
            },
            ..limits()
        },
        100_000,
    )
    .unwrap();
    assert!(!limited.proven);
    assert!(limited.intersections.pairs.next_pair.is_some());
    assert!(
        rounded
            .tangency
            .clone()
            .qualify_embedding(
                1e-8,
                boundary_embedding::Limits {
                    contacts: face_contacts::Limits {
                        pairs: 1,
                        ..limits().contacts
                    },
                    ..limits()
                },
                100_000
            )
            .is_err()
    );
}
