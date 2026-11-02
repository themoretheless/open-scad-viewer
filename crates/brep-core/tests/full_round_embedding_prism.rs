use brep_core::{boundary_embedding, face_contacts, feature_ops::full_round};
use nurbs_core::curve::Curve;

fn wire(points: &[[f64; 2]]) -> Vec<Curve> {
    (0..points.len())
        .map(|i| {
            Curve::from_polyline(vec![
                points[i].to_vec(),
                points[(i + 1) % points.len()].to_vec(),
            ])
            .unwrap()
        })
        .collect()
}

#[test]
fn full_round_embedding_covers_nonrectangular_profile_and_all_hole_wall_pairs() {
    let outer = wire(&[[0., 0.], [1., -1.], [3., -1.], [4., 0.], [4., 8.], [0., 8.]]);
    let hole = wire(&[[1., 1.], [1., 2.], [2., 2.], [2., 1.]]);
    let source = brep_core::prism::extrude(&[outer, hole], 0., 6.).unwrap();
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
                    && f.surface
                        .control_points
                        .iter()
                        .flatten()
                        .any(|p| p[2] == 0.)
                    && f.surface
                        .control_points
                        .iter()
                        .flatten()
                        .any(|p| p[2] == 6.)
            })
            .unwrap()
    });
    let rounded = full_round(&source, selected).unwrap();
    let limits = boundary_embedding::Limits {
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
    };
    let embedded = rounded
        .tangency
        .clone()
        .qualify_embedding(1e-8, limits, 100_000)
        .unwrap();
    assert_eq!(embedded.model(), &rounded.model);
    assert!(embedded.report().intersections.absence_proven);
    let count = rounded.model.faces.len();
    assert_eq!(
        embedded.report().intersections.pairs.total_pairs,
        count * (count - 1) / 2
    );
    assert_eq!(embedded.report().intersections.pairs.next_pair, None);
    assert_eq!(
        brep_core::prism::recognize(embedded.model())
            .unwrap()
            .unwrap()
            .loops
            .len(),
        2
    );
    let volume_limits = brep_core::volume_validity::Limits {
        boundary: limits,
        nesting_pairs: 100,
        nesting_cells: 100_000,
        nesting_domain_cells: 1_000_000,
        orientation_cells: 100_000,
        orientation_domain_cells: 1_000_000,
        orientation_spans: 100,
    };
    let material = rounded
        .tangency
        .clone()
        .qualify_volume(1e-8, volume_limits, 100_000)
        .unwrap();
    assert_eq!(material.model(), &rounded.model);
    assert_eq!(material.qualification().envelopes().len(), 2);
    let volume = material.report();
    assert!(
        volume.proven,
        "boundary={} roles={:?} orientations={:?}",
        volume.boundary.proven,
        volume.nesting.as_ref().and_then(|n| n.roles_consistent),
        volume
            .orientations
            .iter()
            .map(|o| (o.expected_outward, o.outward))
            .collect::<Vec<_>>()
    );
    assert_eq!(
        volume.nesting.as_ref().unwrap().roles_consistent,
        Some(true)
    );
    assert!(
        volume
            .orientations
            .iter()
            .all(|o| o.expected_outward && o.outward == Some(true))
    );
    let mut inward = material.model().clone();
    for shell in &mut inward.shells {
        for face in &mut shell.faces {
            face.reversed = !face.reversed;
        }
    }
    let rejected =
        brep_core::volume_validity::inspect_with_linear(&inward, 1e-8, volume_limits, 100_000)
            .unwrap();
    assert!(rejected.boundary.proven);
    assert!(!rejected.proven);
    assert!(
        rejected
            .orientations
            .iter()
            .any(|o| o.expected_outward && o.outward == Some(false))
    );
    assert_eq!(material.model(), &rounded.model);
    let insufficient = brep_core::volume_validity::Limits {
        boundary: boundary_embedding::Limits {
            contacts: face_contacts::Limits {
                pairs: 1,
                ..limits.contacts
            },
            ..limits
        },
        ..volume_limits
    };
    assert!(
        rounded
            .tangency
            .clone()
            .qualify_volume(1e-8, insufficient, 100_000)
            .is_err()
    );
}
