use super::*;
use nurbs_intersect::Coverage;
use nurbs_intersect::coverage_verifier::verify_complete_report;

fn bicubic(z: f64, shift: f64) -> Surface {
    let row = |y: f64| {
        (0..4)
            .map(|i| vec![i as f64 + shift, y, z])
            .collect::<Vec<_>>()
    };
    Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![row(0.), row(1.), row(2.), row(3.)],
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    }
}

fn planar_yz(x: f64) -> Surface {
    let mut control_points = Vec::new();
    for iz in 0..4 {
        let z = iz as f64;
        control_points.push((0..4).map(|iy| vec![x, iy as f64, z]).collect());
    }
    Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points,
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    }
}

fn planar_yz_wide(x: f64) -> Surface {
    let mut surface = planar_yz(x);
    for (i, row) in surface.control_points.iter_mut().take(4).enumerate() {
        for (j, point) in row.iter_mut().take(4).enumerate() {
            *point = vec![x, -1. + j as f64 * 5. / 3., -1. + i as f64 * 5. / 3.];
        }
    }
    surface
}

fn bumped_xy() -> Surface {
    let mut s = bicubic(0., 0.);
    s.control_points[1][1][2] = 0.02;
    s
}

#[test]
fn nurbs_boolean_imprint_on_planar_transverse() {
    let xy = bicubic(0., 0.);
    let yz = planar_yz(1.5);
    let error = nurbs_boolean_transverse_bicubic(&xy, &yz, Options::default()).unwrap_err();
    assert_eq!(error.code, "BREP_NURBS_SS_REFUSED");
    assert!(error.message.contains("finite /2 contacting-solid author"));
}

#[test]
fn nurbs_boolean_refuses_out_of_matrix() {
    let mut s = bicubic(0., 0.);
    s.weights[0][0] = 2.;
    assert_eq!(
        nurbs_boolean_transverse_bicubic(&s, &bicubic(0., 20.), Options::default())
            .unwrap_err()
            .code,
        "BREP_NURBS_SS_REFUSED"
    );
}

#[test]
fn nurbs_solid_boolean_disjoint_cuboids_refuse_v2() {
    let a = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
    let b0 = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 10.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    assert_eq!(
        nurbs_boolean_imprint_solids(&a, &b, "union")
            .unwrap_err()
            .code,
        "BREP_NURBS_SS_REFUSED"
    );
}

#[test]
fn nurbs_solid_boolean_contact_authors_topology() {
    let a = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
    let b0 = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 1.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let (model, cert) = nurbs_boolean_imprint_solids(&a, &b, "union").unwrap();
    assert!(cert.permits_topology_change());
    assert_eq!(cert.capability, "nurbs-boolean-bezier-le3/2");
    assert!(cert.audit.ok && cert.sew.complete);
    assert!(!cert.arrangements.is_empty());
    model.validate().unwrap();
}

#[test]
fn bc02_nonplanar_bump_plane_publishes_certified_curve() {
    let a = bumped_xy();
    let b = planar_yz_wide(1.5);
    let report = narrow_transverse_bicubic(&a, &b, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Complete);
    verify_complete_report(&report, false).unwrap();
    assert!(matches!(report.components[0], G6Component::Curve { .. }));

    let error = nurbs_boolean_transverse_bicubic(&a, &b, Options::default()).unwrap_err();
    assert_eq!(error.code, "BREP_NURBS_SS_REFUSED");
    assert!(error.message.contains("finite /2 contacting-solid author"));
}

#[test]
fn ep03_containment_remains_unavailable() {
    let a = crate::cuboid([0., 0., 0.], [4., 4., 4.]).unwrap();
    let b0 = crate::cuboid([0., 0., 0.], [1., 1., 1.]).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 1.],
            [0., 1., 0., 1.],
            [0., 0., 1., 1.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    assert!(nurbs_boolean_imprint_solids(&a, &b, "difference").is_err());
}

#[test]
fn bezier_le3_solid_boolean_all_operations_contact() {
    let outer = crate::cuboid([0., 0., 0.], [3., 3., 3.]).unwrap();
    let inner0 = crate::cuboid([0., 0., 0.], [3., 3., 3.]).unwrap();
    let inner = crate::transform::affine(
        &inner0,
        [
            [1., 0., 0., 1.],
            [0., 1., 0., 0.5],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    for operation in ["union", "difference", "intersection"] {
        let (model, cert) = nurbs_boolean_imprint_solids(&outer, &inner, operation).unwrap();
        assert!(cert.permits_topology_change(), "{operation}");
        model.validate().unwrap();
        assert_eq!(cert.operation, operation);
    }
}

#[test]
fn bezier_le3_solid_boolean_refuses_any_out_of_matrix_face() {
    let mut a = crate::cuboid([0., 0., 0.], [4., 4., 4.]).unwrap();
    let b = crate::cuboid([1., 1., 1.], [2., 2., 2.]).unwrap();
    a.faces[0].surface = a.faces[0]
        .surface
        .edit_axis(Axis::U, |curve| curve.elevate(4))
        .unwrap();
    a.validate().unwrap();
    assert_eq!(
        nurbs_boolean_imprint_solids(&a, &b, "union")
            .unwrap_err()
            .code,
        "BREP_NURBS_SS_REFUSED"
    );
}

fn elevate_model_faces(mut model: Model) -> Model {
    for face in &mut model.faces {
        face.surface = face
            .surface
            .edit_axis(Axis::U, |curve| curve.elevate(3))
            .unwrap()
            .edit_axis(Axis::V, |curve| curve.elevate(3))
            .unwrap();
    }
    model.validate().unwrap();
    model
}

#[test]
fn elevated_planar_contact_survives_transform_and_scale() {
    let a0 = crate::cuboid([0., 0., 0.], [2., 3., 2.]).unwrap();
    let b0 = crate::cuboid([0., 0., 0.], [2., 3., 2.]).unwrap();
    let matrix = [
        [2., 0., 0., 10.],
        [0., 1.5, 0., -4.],
        [0., 0., 0.5, 3.],
        [0., 0., 0., 1.],
    ];
    let a = elevate_model_faces(crate::transform::affine(&a0, matrix).unwrap());
    let shifted = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 1.],
            [0., 1., 0., 0.5],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let b = elevate_model_faces(crate::transform::affine(&shifted, matrix).unwrap());
    let (result, cert) = nurbs_boolean_imprint_solids(&a, &b, "intersection").unwrap();
    assert!(cert.permits_topology_change());
    assert!(
        result
            .faces
            .iter()
            .all(|face| is_bezier_le3_positive(&face.surface))
    );
    result.validate().unwrap();
    crate::operations::boolean(&a, &b, "union")
        .unwrap()
        .validate()
        .unwrap();
}

#[test]
fn tangent_and_gray_band_contacts_refuse() {
    let a = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
    for shift in [2., 2. - 4e-7] {
        let b = crate::transform::affine(
            &crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap(),
            [
                [1., 0., 0., shift],
                [0., 1., 0., 0.],
                [0., 0., 1., 0.],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap();
        assert!(nurbs_boolean_imprint_solids(&a, &b, "union").is_err());
    }
}

#[test]
fn unequal_extrusion_spans_refuse_without_stepped_fallback() {
    let a = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
    let b = crate::cuboid([1., 0.5, 0.5], [3., 2.5, 2.5]).unwrap();
    let error = nurbs_boolean_imprint_solids(&a, &b, "union").unwrap_err();
    assert_eq!(error.code, "BREP_PROFILE_IMPRINT_REFUSED");
    assert!(error.message.contains("stepped fallback is forbidden"));
}

#[test]
fn aggregate_certificate_mutations_revoke_authority() {
    let a = crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
    let b = crate::transform::affine(
        &a,
        [
            [1., 0., 0., 1.],
            [0., 1., 0., 0.5],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let (_, cert) = nurbs_boolean_imprint_solids(&a, &b, "difference").unwrap();
    assert!(cert.permits_topology_change());
    let mut mutated = cert.clone();
    mutated.no_healing = false;
    assert!(!mutated.permits_topology_change());
    let mut mutated = cert.clone();
    mutated.arrangements[0].complete = false;
    assert!(!mutated.permits_topology_change());
    let mut mutated = cert.clone();
    mutated.evidence.claims.clear();
    assert!(!mutated.permits_topology_change());
    let mut mutated = cert;
    mutated.change_set.changes.clear();
    assert!(!mutated.permits_topology_change());
}

#[test]
fn nonplanar_contact_typed_refuses_without_curved_prism_fallback() {
    let mut a = elevate_model_faces(crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap());
    a.faces[0].surface.control_points[1][1][2] += 0.01;
    a.validate().unwrap();
    let b = crate::transform::affine(
        &crate::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap(),
        [
            [1., 0., 0., 1.],
            [0., 1., 0., 0.5],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let error = nurbs_boolean_imprint_solids(&a, &b, "union").unwrap_err();
    assert_eq!(error.code, "BREP_NURBS_SS_REFUSED");
}

#[test]
fn v3_graph_patch_u_v_degree_2_3_intersection_and_difference() {
    for (du, dv, cutter) in [
        (2, 2, crate::cuboid([0.5, -1., -1.], [2., 2., 3.]).unwrap()),
        (2, 3, crate::cuboid([-1., 0.5, -1.], [2., 2., 3.]).unwrap()),
        (3, 2, crate::cuboid([0.5, -1., -1.], [2., 2., 3.]).unwrap()),
        (3, 3, crate::cuboid([-1., 0.5, -1.], [2., 2., 3.]).unwrap()),
    ] {
        let graph = canonical_bezier_graph_solid(du, dv).unwrap();
        for operation in ["intersection", "difference"] {
            let (result, certificate) =
                nurbs_boolean_graph_patch_v3(&graph, &cutter, operation).unwrap();
            assert!(
                certificate.permits_topology_change(),
                "{du}x{dv} {operation}"
            );
            assert_eq!(certificate.tensor_cells, 3);
            assert!(
                certificate
                    .audit
                    .notes
                    .contains(&"v3_graph_patch_self_intersection_exclusion_ok")
            );
            result.validate().unwrap();
            assert_eq!(result.bodies.len(), 1);
            assert_eq!(result.faces.len(), 6);
            assert_eq!(
                result.vertices.len() as isize - result.edges.len() as isize
                    + result.faces.len() as isize,
                2
            );
            let expected_total =
                1. + 0.25 * ((du - 1) * (dv - 1)) as f64 / ((du + 1) * (dv + 1)) as f64;
            let mass = crate::analysis::mass_properties(&result, 1e-9, 300_000).unwrap();
            assert!(
                (mass.signed_volume_mm3.abs() - expected_total * 0.5).abs() < 1e-7,
                "{du}x{dv} {operation}: {:?}",
                mass.signed_volume_mm3
            );
        }
    }
}

#[test]
fn v3_graph_patch_transform_reflection_swap_and_refusals() {
    let graph = canonical_bezier_graph_solid(3, 2).unwrap();
    let cutter = crate::cuboid([0.5, -1., -1.], [2., 2., 3.]).unwrap();
    let matrix = [
        [0., -1., 0., 4.],
        [-1., 0., 0., 3.],
        [0., 0., 1., 2.],
        [0., 0., 0., 1.],
    ];
    let graph = crate::transform::affine(&graph, matrix).unwrap();
    let cutter = crate::transform::affine(&cutter, matrix).unwrap();
    let (_, certificate) =
        nurbs_boolean_graph_patch_v3(&cutter, &graph, "intersection").unwrap();
    assert!(certificate.permits_topology_change());
    let mut mutation = certificate.clone();
    mutation.tensor_cells = 2;
    assert!(!mutation.permits_topology_change());
    let mut mutation = certificate.clone();
    mutation.sew.complete = false;
    assert!(!mutation.permits_topology_change());
    let mut mutation = certificate.clone();
    mutation.naming_complete = false;
    assert!(!mutation.permits_topology_change());
    let mut mutation = certificate.clone();
    mutation.change_set.changes.clear();
    assert!(!mutation.permits_topology_change());
    assert!(nurbs_boolean_graph_patch_v3(&graph, &cutter, "union").is_err());
    assert!(nurbs_boolean_graph_patch_v3(&cutter, &graph, "difference").is_err());

    let mut rational = graph.clone();
    let face = canonical_graph_frame(&rational).unwrap().0;
    rational.faces[face].surface.weights[1][1] = 2.;
    assert!(nurbs_boolean_graph_patch_v3(&rational, &cutter, "intersection").is_err());
    let mut multispan = graph.clone();
    let face = canonical_graph_frame(&multispan).unwrap().0;
    multispan.faces[face].surface = multispan.faces[face]
        .surface
        .edit_axis(Axis::U, |curve| curve.insert(0.25, 1))
        .unwrap();
    assert!(nurbs_boolean_graph_patch_v3(&multispan, &cutter, "intersection").is_err());
    let boundary = crate::transform::affine(
        &crate::cuboid([0., -1., -1.], [2., 2., 3.]).unwrap(),
        matrix,
    )
    .unwrap();
    assert!(nurbs_boolean_graph_patch_v3(&graph, &boundary, "intersection").is_err());
}

#[test]
fn v4_unequal_span_and_containment_operations_are_independently_certified() {
    let graph = canonical_bezier_graph_solid(3, 3).unwrap();
    let unequal = crate::cuboid([0.5, -2., -3.], [2., 3., 4.]).unwrap();
    for operation in ["intersection", "difference"] {
        let (result, certificate) =
            nurbs_boolean_graph_patch_unequal_v4(&graph, &unequal, operation).unwrap();
        assert_eq!(certificate.capability, NURBS_BOOLEAN_CAPABILITY_V4);
        assert!(certificate.permits_topology_change());
        assert!(
            certificate
                .change_set
                .changes
                .iter()
                .any(|change| { change.provenance.operation == NURBS_BOOLEAN_CAPABILITY_V4 })
        );
        result.validate().unwrap();
    }

    let inner = crate::cuboid([0.2, 0.2, 0.2], [0.8, 0.8, 0.8]).unwrap();
    for operation in ["union", "intersection", "difference"] {
        let (result, certificate) =
            nurbs_boolean_graph_containment_v4(&graph, &inner, operation).unwrap();
        assert!(certificate.permits_topology_change(), "{operation}");
        assert_eq!(certificate.operation, operation);
        assert!(certificate.strict_uv_margin > 0.);
        assert!(certificate.floor_clearance > 0.);
        assert!(certificate.roof_clearance > 0.);
        assert_eq!(
            result.bodies[0].inner_shells.len(),
            usize::from(operation == "difference")
        );
    }
    let (_, certificate) =
        nurbs_boolean_graph_containment_v4(&graph, &inner, "difference").unwrap();
    let mut mutation = certificate.clone();
    mutation.cavity_proof = false;
    assert!(!mutation.permits_topology_change());
    let mut mutation = certificate;
    mutation.roof_clearance = 0.;
    assert!(!mutation.permits_topology_change());
}

#[test]
fn v4_containment_boundary_and_reversed_difference_refuse() {
    let graph = canonical_bezier_graph_solid(2, 3).unwrap();
    let boundary = crate::cuboid([0., 0.2, 0.2], [0.8, 0.8, 0.8]).unwrap();
    assert!(nurbs_boolean_graph_containment_v4(&graph, &boundary, "difference").is_err());
    let inner = crate::cuboid([0.2, 0.2, 0.2], [0.8, 0.8, 0.8]).unwrap();
    assert!(nurbs_boolean_graph_containment_v4(&inner, &graph, "difference").is_err());
}

#[test]
fn v5_rational_graph_has_homogeneous_root_denominator_and_resource_proofs() {
    for (du, dv, cutter) in [
        (2, 2, crate::cuboid([0.5, -1., -1.], [2., 2., 3.]).unwrap()),
        (3, 3, crate::cuboid([0.5, -2., -1.], [2., 3., 4.]).unwrap()),
    ] {
        let graph = canonical_rational_graph_solid(du, dv).unwrap();
        for operation in ["intersection", "difference"] {
            let (result, certificate) =
                nurbs_boolean_rational_graph_patch_v5(&graph, &cutter, operation).unwrap();
            assert_eq!(certificate.capability, NURBS_BOOLEAN_CAPABILITY_V5);
            assert!(certificate.permits_topology_change());
            assert!(certificate.homogeneous_root_proof);
            assert!(certificate.denominator_lower_bound >= 0.25);
            assert!(certificate.weight_condition_number <= 8.);
            assert!(certificate.resource_bound <= 16);
            assert!(
                result.faces[1]
                    .surface
                    .weights
                    .iter()
                    .flatten()
                    .any(|weight| (*weight - 1.).abs() > 1e-15)
            );
        }
    }
}

#[test]
fn v5_weight_root_resource_tangency_and_certificate_mutations_refuse() {
    let cutter = crate::cuboid([0.5, -1., -1.], [2., 2., 3.]).unwrap();
    let graph = canonical_rational_graph_solid(3, 3).unwrap();
    let (_, certificate) =
        nurbs_boolean_rational_graph_patch_v5(&graph, &cutter, "intersection").unwrap();
    for mutation in [
        {
            let mut value = certificate.clone();
            value.homogeneous_root_proof = false;
            value
        },
        {
            let mut value = certificate.clone();
            value.denominator_lower_bound = 0.;
            value
        },
        {
            let mut value = certificate.clone();
            value.resource_bound = 17;
            value
        },
    ] {
        assert!(!mutation.permits_topology_change());
    }

    let mut low_weight = graph.clone();
    let face = canonical_graph_frame_cell(&low_weight, true, false)
        .unwrap()
        .0;
    low_weight.faces[face].surface.weights[0][1] = 0.125;
    assert!(
        nurbs_boolean_rational_graph_patch_v5(&low_weight, &cutter, "intersection").is_err()
    );

    let mut nonfactorable = graph;
    let face = canonical_graph_frame_cell(&nonfactorable, true, false)
        .unwrap()
        .0;
    nonfactorable.faces[face].surface.weights[1][1] += 0.125;
    assert!(
        nurbs_boolean_rational_graph_patch_v5(&nonfactorable, &cutter, "intersection").is_err()
    );

    let tangent = crate::cuboid([0., -1., -1.], [2., 2., 3.]).unwrap();
    assert!(
        nurbs_boolean_rational_graph_patch_v5(
            &canonical_rational_graph_solid(3, 3).unwrap(),
            &tangent,
            "intersection"
        )
        .is_err()
    );
    assert!(
        nurbs_boolean_rational_graph_patch_v5(
            &canonical_rational_graph_solid(3, 3).unwrap(),
            &cutter,
            "union"
        )
        .is_err()
    );
}

#[test]
fn general_multispan_boolean_authors_exact_region_operations() {
    for spans in [(2, 1), (1, 2), (2, 2)] {
        let graph = canonical_multispan_graph_solid(spans.0, spans.1).unwrap();
        let cutter = crate::cuboid([0.25, -1., -1.], [0.75, 2., 3.]).unwrap();
        let graph_snapshot = value_codec::Serialize::to_value(&graph);
        let cutter_snapshot = value_codec::Serialize::to_value(&cutter);
        for (operation, bodies, faces) in [("intersection", 1, 6), ("difference", 2, 12)] {
            let (result, certificate) =
                author_general_nurbs_boolean(&graph, &cutter, operation).unwrap();
            assert_eq!(certificate.capability, NURBS_BOOLEAN_SS_CAPABILITY);
            assert!(certificate.permits_topology_change());
            assert_eq!(certificate.branch_graph.components.len(), 2);
            assert_eq!(certificate.result_components, bodies);
            assert_eq!(result.faces.len(), faces);
            assert!(certificate.exact_curve_pcurve_count >= 4);
            assert!(certificate.no_fallback && certificate.naming.operation_stable);
            let top_domains = (0..bodies)
                .map(|component| {
                    let top = &result.faces[component * 6 + 1].surface;
                    [
                        top.knots_u[top.degree_u],
                        top.knots_u[top.control_points.len()],
                    ]
                })
                .collect::<Vec<_>>();
            assert_eq!(
                top_domains,
                if operation == "intersection" {
                    vec![[0.25, 0.75]]
                } else {
                    vec![[0., 0.25], [0.75, 1.]]
                }
            );
            if operation == "intersection" {
                let (permuted, _) =
                    author_general_nurbs_boolean(&cutter, &graph, operation).unwrap();
                assert_eq!(result.1.faces, permuted.1.faces);
                assert_eq!(result.1.edges, permuted.1.edges);
            }
        }
        assert_eq!(value_codec::Serialize::to_value(&graph), graph_snapshot);
        assert_eq!(value_codec::Serialize::to_value(&cutter), cutter_snapshot);
        for (a, b, operation, order) in [
            (&graph, &cutter, "union", "source-tool"),
            (&cutter, &graph, "difference", "tool-source"),
        ] {
            let (result, certificate) = author_general_nurbs_boolean(a, b, operation).unwrap();
            assert_eq!(certificate.capability, NURBS_BOOLEAN_SS_CAPABILITY);
            assert_eq!(certificate.operand_order, order);
            assert!(certificate.exact_region_membership);
            assert_eq!(
                certificate.partition_cells,
                if operation == "union" { 3 } else { 4 }
            );
            assert_eq!(certificate.result_components, certificate.partition_cells);
            assert_eq!(certificate.result_faces, certificate.partition_cells * 6);
            assert!(certificate.sew.complete && certificate.audit.ok);
            assert!(certificate.change_set.validate().is_ok());
            assert!(certificate.permits_topology_change());
            assert!(result.persistent_naming_complete());
        }
        assert_eq!(value_codec::Serialize::to_value(&graph), graph_snapshot);
        assert_eq!(value_codec::Serialize::to_value(&cutter), cutter_snapshot);
    }
}

#[test]
fn general_multispan_boolean_naming_is_rigid_transform_stable() {
    let graph = canonical_multispan_graph_solid(2, 2).unwrap();
    let cutter = crate::cuboid([0.25, -1., -1.], [0.75, 2., 3.]).unwrap();
    let (base, _) = author_general_nurbs_boolean(&graph, &cutter, "difference").unwrap();
    let matrix = [
        [0., -1., 0., 4.],
        [1., 0., 0., -2.],
        [0., 0., 1., 3.],
        [0., 0., 0., 1.],
    ];
    let moved_graph = crate::transform::affine(&graph, matrix).unwrap();
    let moved_cutter = crate::transform::affine(&cutter, matrix).unwrap();
    let (moved, _) =
        author_general_nurbs_boolean(&moved_graph, &moved_cutter, "difference").unwrap();
    assert_eq!(base.1.vertices, moved.1.vertices);
    assert_eq!(base.1.edges, moved.1.edges);
    assert_eq!(base.1.faces, moved.1.faces);
    assert_eq!(base.1.bodies, moved.1.bodies);
}

#[test]
fn general_multispan_boolean_has_independent_regularized_volume_oracle() {
    let graph = canonical_multispan_graph_solid(2, 2).unwrap();
    let cutter = crate::cuboid([0.25, -1., -1.], [0.75, 2., 3.]).unwrap();
    let volume = |model: &Model| {
        crate::analysis::mass_properties(model, 1e-9, 500_000)
            .unwrap()
            .signed_volume_mm3
            .abs()
    };
    let intersection = author_general_nurbs_boolean(&graph, &cutter, "intersection")
        .unwrap()
        .0;
    let source_difference = author_general_nurbs_boolean(&graph, &cutter, "difference")
        .unwrap()
        .0;
    let union = author_general_nurbs_boolean(&graph, &cutter, "union")
        .unwrap()
        .0;
    let reversed = author_general_nurbs_boolean(&cutter, &graph, "difference")
        .unwrap()
        .0;
    let (vg, vc, vi, vd, vu, vr) = (
        volume(&graph),
        volume(&cutter),
        volume(&intersection),
        volume(&source_difference),
        volume(&union),
        volume(&reversed),
    );
    for residual in [vg - vi - vd, vc - vi - vr, vu - vg - vc + vi] {
        assert!(
            residual.abs() < 1e-7,
            "regularized volume residual {residual:e}"
        );
    }
}
