use brep_core::region_replacement as r;
fn network() -> r::CornerPatchNetwork {
    r::propose_corner_patch_network(0.75, [0.13, 0.2, 0.35], 5., 0.214).unwrap()
}
#[test]
fn positive_control_hulls_and_flat_radius_endpoint_jets() {
    let n = network();
    assert_eq!(n.caps.len(), 3);
    assert_eq!(n.strips.len(), 3);
    for s in &n.caps {
        assert_eq!((s.degree_u, s.degree_v), (7, 7));
        assert!(
            s.control_points
                .iter()
                .flatten()
                .flatten()
                .all(|&v| v >= -1e-12 && v <= 0.75 + 1e-12)
        );
    }
    for (axis, s) in n.strips.iter().enumerate() {
        assert!(s.weights.iter().flatten().all(|&w| w == 1.));
        for row in &s.control_points {
            for k in 0..3 {
                if k != axis {
                    assert_eq!(row[0][k], row[1][k]);
                    assert_eq!(row[1][k], row[2][k]);
                    assert_eq!(row[3][k], row[4][k]);
                    assert_eq!(row[4][k], row[5][k]);
                }
            }
        }
    }
}
#[test]
fn closed_body_has_explicit_shared_roles_and_retains_original_vertices() {
    let source = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
    let before = source.clone();
    let out = r::propose_corner_patch_body(&source, &network()).unwrap();
    assert_eq!(source, before);
    assert_eq!(
        (
            out.model.faces.len(),
            out.model.edges.len(),
            out.model.vertices.len()
        ),
        (15, 36, 23)
    );
    assert_eq!(
        brep_topology::explorer::euler_characteristic(&out.model.0),
        2
    );
    assert_eq!(out.smooth_edges.len(), 15);
    assert_eq!(out.model.shells.len(), 1);
    assert!(out.model.shells[0].closed);
    for i in 1..8 {
        let j = out
            .model
            .vertices
            .iter()
            .position(|v| v.point == source.vertices[i].point)
            .unwrap();
        assert_eq!(out.model.1.vertices[j], source.1.vertices[i]);
    }
}
#[test]
fn inconsistent_owned_surface_and_unsupported_cube_are_atomic_refusals() {
    let source = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
    let before = source.clone();
    let mut n = network();
    n.caps[0].control_points[0][0][0] += 0.01;
    assert!(r::propose_corner_patch_body(&source, &n).is_err());
    assert_eq!(source, before);
    let other = brep_core::cuboid([0.; 3], [11.; 3]).unwrap();
    assert!(r::propose_corner_patch_body(&other, &network()).is_err());
    let long = r::propose_corner_patch_network(0.75, [0.13, 0.2, 0.35], 10., 0.214).unwrap();
    assert!(r::propose_corner_patch_body(&source, &long).is_err());
    for (r, far, stop, h) in [
        (0., [0.13, 0.2, 0.35], 5., 0.214),
        (0.75, [0., 0.2, 0.35], 5., 0.214),
        (0.75, [0.13, 0.2, 0.35], 0.5, 0.214),
        (0.75, [0.13, 0.2, 0.35], 5., f64::NAN),
    ] {
        assert!(r::propose_corner_patch_network(r, far, stop, h).is_err());
    }
}

#[test]
fn binary_network_has_exact_original_edge_lifts() {
    let source = brep_core::cuboid([0.; 3], [10.; 3]).unwrap();
    let net = r::propose_binary_corner_patch_network(0.75, [0.13, 0.2, 0.35], 5., 0.214).unwrap();
    let body = r::propose_corner_patch_body(&source, &net).unwrap();
    let proof = brep_core::boundary_agreement::verify_exact(&body.model, 1_000_000).unwrap();
    assert!(
        proof.all_equal,
        "{:?}",
        proof
            .uses
            .iter()
            .filter(|u| u
                .decision
                .as_ref()
                .is_none_or(|d| format!("{:?}", d.outcome) != "Equal"))
            .collect::<Vec<_>>()
    );
    assert!(proof.all_joins_exact);
    let limits = brep_core::boundary_embedding::Limits {
        exact_work: 1_000_000,
        trim_pairs: 10_000,
        trim_cells: 100_000,
        trim_domain_cells: 1_000_000,
        spans: 1024,
        contacts: brep_core::face_contacts::Limits {
            pairs: 10_000,
            cells: 100_000,
            domain_cells: 1_000_000,
            cells_per_pair: 1000,
            domain_cells_per_pair: 10_000,
        },
    };
    let global =
        brep_core::boundary_embedding::inspect_with_linear(&body.model, 1e-8, limits, 100_000)
            .unwrap();
    assert!(global.proven);
    assert_eq!(global.intersections.pairs.total_pairs, 105);
    assert_eq!(global.intersections.pairs.next_pair, None);
    let limited = brep_core::boundary_embedding::inspect_with_linear(
        &body.model,
        1e-8,
        brep_core::boundary_embedding::Limits {
            contacts: brep_core::face_contacts::Limits {
                pairs: 1,
                ..limits.contacts
            },
            ..limits
        },
        100_000,
    )
    .unwrap();
    assert!(!limited.proven);
    assert!(limited.intersections.pairs.next_pair.is_some());
}
