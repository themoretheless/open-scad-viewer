use super::*;

#[test]
fn certified_event_order_and_classification() {
    let cert = classify_chart_events(
        ChartKind::PlanePoly,
        vec![
            ChartEvent {
                parameter: 0.25,
                kind: "enter",
                edge: 0,
            },
            ChartEvent {
                parameter: 0.75,
                kind: "exit",
                edge: 1,
            },
        ],
        &[
            (0.1, CellLabel::Outside),
            (0.5, CellLabel::Inside),
            (0.9, CellLabel::Outside),
        ],
    )
    .unwrap();
    assert!(cert.complete);
    assert_eq!(cert.cells.len(), 3);
}

#[test]
fn sample_on_event_without_boundary_label_refuses() {
    assert!(
        classify_chart_events(
            ChartKind::AnalyticCircle,
            vec![ChartEvent {
                parameter: 0.5,
                kind: "root",
                edge: 0,
            }],
            &[(0.5, CellLabel::Inside)],
        )
        .is_err()
    );
}

#[test]
fn endpoint_only_exact_sew_wrapper_fails_closed() {
    let key = sew_edge_key([0., 0., 0.], [1., 0., 0.], 1e-9).unwrap();
    let pending = [
        SewLedgerEntry {
            key: key.clone(),
            face_a: 0,
            face_b: 1,
            orientation_agree: true,
            displacement: None,
        },
        SewLedgerEntry {
            key: key.clone(),
            face_a: 1,
            face_b: 0,
            orientation_agree: false,
            displacement: None,
        },
    ];
    let base = SewSnapshot::default();
    assert_eq!(
        sew_atomic(base, &pending).unwrap_err().code,
        "BREP_SEW_CORRESPONDENCE_REQUIRED"
    );
}

fn proven_line() -> (ToleranceContext, SewEdgeKey, BoundaryCorrespondence, Curve) {
    let context = ToleranceContext::default_valid();
    let curve = Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 0., 0.]]).unwrap();
    let uses = [
        BoundaryUse {
            face: 0,
            wire: 0,
            cyclic_index: 0,
            reversed: false,
        },
        BoundaryUse {
            face: 1,
            wire: 1,
            cyclic_index: 0,
            reversed: true,
        },
    ];
    let proof = prove_boundary_correspondence(
        &context,
        BoundaryCorrespondenceInput {
            curve_a: &curve,
            curve_b: &curve,
            endpoints_a: [[0., 0., 0.], [1., 0., 0.]],
            endpoints_b: [[1., 0., 0.], [0., 0., 0.]],
            orientation: ParameterOrientation::Reversed,
            seam_shift: 0,
            shell: 7,
            uses,
        },
    )
    .unwrap();
    let mut key = sew_edge_key([0., 0., 0.], [1., 0., 0.], 1e-9).unwrap();
    key.shell = Some(7);
    key.definition = Some(authority_key(&proof.authority));
    (context, key, proof, curve)
}

#[test]
fn correspondence_proof_sews_and_mutations_refuse() {
    let (context, key, proof, curve) = proven_line();
    let entry = CorrespondenceLedgerEntry {
        key: key.clone(),
        correspondence: proof.clone(),
    };
    let (_, certificate) =
        exact_sew_correspondences(&SewSnapshot::default(), &context, &[entry]).unwrap();
    assert!(certificate.complete);

    let mut reversed = proof.clone();
    reversed.orientation = ParameterOrientation::Same;
    assert!(
        exact_sew_correspondences(
            &SewSnapshot::default(),
            &context,
            &[CorrespondenceLedgerEntry {
                key: key.clone(),
                correspondence: reversed,
            }],
        )
        .is_err()
    );

    let mut foreign_spec = context.specification().clone();
    foreign_spec.policy = "foreign-sew-context".into();
    let foreign = ToleranceContext::new(foreign_spec).unwrap();
    assert!(
        exact_sew_correspondences(
            &SewSnapshot::default(),
            &foreign,
            &[CorrespondenceLedgerEntry {
                key: key.clone(),
                correspondence: proof.clone(),
            }],
        )
        .is_err()
    );

    let mut different = curve.clone();
    different.weights[0] = 2.;
    assert!(
        prove_boundary_correspondence(
            &context,
            BoundaryCorrespondenceInput {
                curve_a: &curve,
                curve_b: &different,
                endpoints_a: [[0., 0., 0.], [1., 0., 0.]],
                endpoints_b: [[1., 0., 0.], [0., 0., 0.]],
                orientation: ParameterOrientation::Reversed,
                seam_shift: 0,
                shell: 7,
                uses: proof.uses.clone(),
            },
        )
        .is_err()
    );

    let mut seam = proof;
    seam.seam_shift = 1;
    assert!(
        exact_sew_correspondences(
            &SewSnapshot::default(),
            &context,
            &[CorrespondenceLedgerEntry {
                key,
                correspondence: seam,
            }],
        )
        .is_err()
    );

    let (_, key, proof, _) = proven_line();
    let excessive = (0..=4096)
        .map(|_| CorrespondenceLedgerEntry {
            key: key.clone(),
            correspondence: proof.clone(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        exact_sew_correspondences(&SewSnapshot::default(), &context, &excessive)
            .unwrap_err()
            .code,
        "BREP_SEW_RESOURCE_LIMIT"
    );
}

fn curved_graph_and_plane() -> (Surface, Surface) {
    let mut graph = Surface {
        degree_u: 2,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| (0..4).map(|j| vec![i as f64 * 1.5, j as f64, 0.]).collect())
            .collect(),
        weights: vec![vec![1.; 4]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    graph.control_points[1][1][2] = 0.2;
    let plane = Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| vec![1.5, -1. + j as f64 * 5. / 3., -1. + i as f64 * 5. / 3.])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    (graph, plane)
}

#[test]
fn exact_curve_pcurve_correspondence_covers_curved_and_planar_supports() {
    let context = ToleranceContext::default_valid();
    let (graph, plane) = curved_graph_and_plane();
    let seam =
        crate::nurbs_ss_g6::certify_exact_planar_iso_intersection(&graph, &plane, &context)
            .unwrap();
    let owner = BoundaryUse {
        face: 3,
        wire: 5,
        cyclic_index: 1,
        reversed: false,
    };
    let curved = prove_curve_pcurve_correspondence(
        &context,
        &seam.curve,
        &graph,
        &seam.uv_traces[0],
        ParameterOrientation::Same,
        owner.clone(),
    )
    .unwrap();
    assert_eq!(curved.support, CurvePcurveSupport::CurvedIsoU);
    assert!(curved.permits_exact_correspondence());
    let planar = prove_curve_pcurve_correspondence(
        &context,
        &seam.curve,
        &plane,
        &seam.uv_traces[1],
        ParameterOrientation::Same,
        owner,
    )
    .unwrap();
    assert_eq!(planar.support, CurvePcurveSupport::AffinePlanar);
    assert!(planar.permits_exact_correspondence());
}

#[test]
fn curve_pcurve_orientation_context_definition_and_mutations_refuse() {
    let context = ToleranceContext::default_valid();
    let (graph, plane) = curved_graph_and_plane();
    let seam =
        crate::nurbs_ss_g6::certify_exact_planar_iso_intersection(&graph, &plane, &context)
            .unwrap();
    let owner = BoundaryUse {
        face: 3,
        wire: 5,
        cyclic_index: 1,
        reversed: false,
    };
    assert!(
        prove_curve_pcurve_correspondence(
            &context,
            &seam.curve,
            &graph,
            &seam.uv_traces[0],
            ParameterOrientation::Reversed,
            owner.clone()
        )
        .is_err()
    );
    let mut foreign_spec = context.specification().clone();
    foreign_spec.policy = "foreign-pcurve-context".into();
    let foreign = ToleranceContext::new(foreign_spec).unwrap();
    let mut certificate = prove_curve_pcurve_correspondence(
        &context,
        &seam.curve,
        &plane,
        &seam.uv_traces[1],
        ParameterOrientation::Same,
        owner,
    )
    .unwrap();
    certificate.context = foreign.spec_identity();
    assert!(!certificate.permits_exact_correspondence());
    let mut certificate = prove_curve_pcurve_correspondence(
        &context,
        &seam.curve,
        &plane,
        &seam.uv_traces[1],
        ParameterOrientation::Same,
        certificate.owner.clone(),
    )
    .unwrap();
    certificate.pcurve.control_points[1][0] += 1e-9;
    assert!(!certificate.permits_exact_correspondence());
    certificate.no_snapping = false;
    assert!(!certificate.permits_exact_correspondence());

    let mut periodic = seam.uv_traces[1].clone();
    periodic.periodic = true;
    assert!(
        prove_curve_pcurve_correspondence(
            &context,
            &seam.curve,
            &plane,
            &periodic,
            ParameterOrientation::Same,
            certificate.owner
        )
        .is_err()
    );
}

#[test]
fn planar_geometry_with_nonlinear_rational_chart_cannot_gain_affine_authority() {
    let context = ToleranceContext::default_valid();
    let (graph, mut plane) = curved_graph_and_plane();
    let seam = crate::nurbs_ss_g6::certify_exact_planar_iso_intersection(
        &graph, &plane, &context).unwrap();
    plane.weights[1][1] = 3.;
    let owner = BoundaryUse { face: 3, wire: 5, cyclic_index: 1, reversed: false };
    assert!(prove_curve_pcurve_correspondence(&context, &seam.curve, &plane,
        &seam.uv_traces[1], ParameterOrientation::Same, owner).is_err());
}

#[test]
fn sew_gap_refuses_without_mutating_base() {
    let key = sew_edge_key([0., 0., 0.], [1., 0., 0.], 1e-9).unwrap();
    let pending = [SewLedgerEntry {
        key,
        face_a: 0,
        face_b: 1,
        orientation_agree: true,
        displacement: None,
    }];
    let base = SewSnapshot::default();
    let err = sew_atomic(base.clone(), &pending).unwrap_err();
    assert_eq!(err.code, "BREP_SEW_CORRESPONDENCE_REQUIRED");
    assert!(base.edges.is_empty());
}

#[test]
fn sew_duplicate_and_orientation_refuse() {
    let key = sew_edge_key([0., 0., 0.], [0., 1., 0.], 1e-9).unwrap();
    let dup = [
        SewLedgerEntry {
            key: key.clone(),
            face_a: 0,
            face_b: 1,
            orientation_agree: true,
            displacement: None,
        },
        SewLedgerEntry {
            key: key.clone(),
            face_a: 2,
            face_b: 3,
            orientation_agree: false,
            displacement: None,
        },
        SewLedgerEntry {
            key: key.clone(),
            face_a: 4,
            face_b: 5,
            orientation_agree: true,
            displacement: None,
        },
    ];
    assert_eq!(
        sew_atomic(SewSnapshot::default(), &dup).unwrap_err().code,
        "BREP_SEW_CORRESPONDENCE_REQUIRED"
    );
    let bad_orient = [
        SewLedgerEntry {
            key: key.clone(),
            face_a: 0,
            face_b: 1,
            orientation_agree: true,
            displacement: None,
        },
        SewLedgerEntry {
            key,
            face_a: 1,
            face_b: 0,
            orientation_agree: true,
            displacement: None,
        },
    ];
    assert_eq!(
        sew_atomic(SewSnapshot::default(), &bad_orient)
            .unwrap_err()
            .code,
        "BREP_SEW_CORRESPONDENCE_REQUIRED"
    );
}

#[test]
fn cuboid_face_outer_loop_classifies_complete() {
    let model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
    let cert = classify_face_outer_loop(&model, 0, ChartKind::PlanePoly).unwrap();
    assert!(cert.complete);
    assert!(!cert.events.is_empty());
}

#[test]
fn cylinder_closed_edge_sew_or_typed_refuse() {
    let model = crate::cylinder(2., 4.).unwrap();
    // Analytic cylinder shares edges across faces; expect Complete sew or a
    // typed incidence refuse — never silent heal.
    match sew_closed_model_edges(&model) {
        Ok(cert) => assert!(cert.complete),
        Err(err) => assert!(
            err.code == "BREP_SEW_GAP"
                || err.code == "BREP_SEW_DUPLICATE"
                || err.code == "BREP_SEW_ORIENTATION"
                || err.code == "BREP_SEW_INVALID"
        ),
    }
}

fn heal_fixture() -> (Model, ToleranceContext, BoundaryCorrespondence) {
    let model = crate::cuboid([0.; 3], [2.; 3]).unwrap();
    let context = model.tolerance_context().unwrap();
    let proof = prove_model_edge_correspondence(&model, &context, 0).unwrap();
    (model, context, proof)
}

#[test]
fn heal_plan_refuses_budget_context_and_missing_lineage() {
    let (model, context, proof) = heal_fixture();
    let vertex = model.edges[0].vertices[0];
    let mut to = model.vertices[vertex].point;
    to[0] += context.spatial_bounds().absolute_mm * 0.25;
    let operation = HealOperation::EndpointSnap {
        vertex,
        expected_id: model.1.vertices[vertex],
        to,
        correspondence: proof.clone(),
    }
    .bind_native_proof();
    assert_eq!(
        AuthorizedHealPlan::new(
            &model,
            &context,
            vec![operation.clone()],
            2. * context.spatial_bounds().absolute_mm,
            2. * context.spatial_bounds().absolute_mm
        )
        .unwrap_err()
        .code,
        "BREP_HEAL_BUDGET_EXCEEDED"
    );
    let mut foreign_spec = context.specification().clone();
    foreign_spec.policy = "foreign-heal".into();
    let foreign = ToleranceContext::new(foreign_spec).unwrap();
    assert_eq!(
        AuthorizedHealPlan::new(
            &model,
            &foreign,
            vec![operation],
            foreign.spatial_bounds().absolute_mm,
            foreign.spatial_bounds().absolute_mm
        )
        .unwrap_err()
        .code,
        "BREP_HEAL_CONTEXT_MISMATCH"
    );
    let bad = HealOperation::EndpointSnap {
        vertex,
        expected_id: TopoId::derive(TopoKind::Vertex, "bad", "bad", "bad", b"bad"),
        to,
        correspondence: proof,
    }
    .bind_native_proof();
    assert_eq!(
        AuthorizedHealPlan::new(
            &model,
            &context,
            vec![bad],
            context.spatial_bounds().absolute_mm,
            context.spatial_bounds().absolute_mm
        )
        .unwrap_err()
        .code,
        "BREP_HEAL_LINEAGE_MISMATCH"
    );
}

#[test]
#[cfg(feature = "codec")]
fn heal_transaction_cancel_rollback_and_idempotence() {
    let (model, context, proof) = heal_fixture();
    let vertex = model.edges[0].vertices[0];
    let to = model.vertices[vertex].point;
    let plan = AuthorizedHealPlan::new(
        &model,
        &context,
        vec![
            HealOperation::EndpointSnap {
                vertex,
                expected_id: model.1.vertices[vertex],
                to,
                correspondence: proof,
            }
            .bind_native_proof(),
        ],
        context.spatial_bounds().absolute_mm,
        context.spatial_bounds().absolute_mm,
    )
    .unwrap();
    let snapshot = crate::transactions::ModelSnapshot::new(model.clone()).unwrap();
    let cancellation = crate::transactions::HealCancellation::default();
    let mut transaction = crate::transactions::AuthorizedHealTransaction::begin(
        snapshot.clone(),
        cancellation.clone(),
    );
    cancellation.cancel();
    assert_eq!(
        transaction.apply(&plan).unwrap_err().code,
        "BREP_HEAL_CANCELLED"
    );

    let mut transaction = crate::transactions::AuthorizedHealTransaction::begin(
        snapshot,
        crate::transactions::HealCancellation::default(),
    );
    transaction.apply(&plan).unwrap();
    let once = transaction.staged().unwrap().clone();
    transaction.rollback();
    assert!(transaction.staged().is_none());
    let twice = apply_authorized_heal(&once, &plan).unwrap();
    assert_eq!(
        value_codec::Serialize::to_value(&once),
        value_codec::Serialize::to_value(&twice)
    );
}

#[test]
#[cfg(feature = "codec")]
fn heal_positive_endpoint_snap_returns_complete_native_certificate() {
    let (model, context, proof) = heal_fixture();
    let vertex = model.edges[0].vertices[0];
    let mut to = model.vertices[vertex].point;
    to[0] += context.spatial_bounds().absolute_mm
        / (1 + model
            .edges
            .iter()
            .filter(|edge| edge.vertices.contains(&vertex))
            .count()) as f64
        * 0.25;
    let plan = AuthorizedHealPlan::new(
        &model,
        &context,
        vec![
            HealOperation::EndpointSnap {
                vertex,
                expected_id: model.1.vertices[vertex],
                to,
                correspondence: proof,
            }
            .bind_native_proof(),
        ],
        context.spatial_bounds().absolute_mm,
        context.spatial_bounds().absolute_mm,
    )
    .unwrap();
    let mut transaction = crate::transactions::AuthorizedHealTransaction::begin(
        crate::transactions::ModelSnapshot::new(model.clone()).unwrap(),
        crate::transactions::HealCancellation::default(),
    );
    transaction.apply(&plan).unwrap();
    let result = transaction.commit().unwrap();
    assert_eq!(result.status, "Complete");
    assert!(result.displacement[0].actual_mm > 0.);
    assert!(result.cumulative_displacement_mm <= context.spatial_bounds().absolute_mm);
    assert!(result.sew.complete && result.audit.ok && result.naming_complete);
    assert_ne!(
        value_codec::Serialize::to_value(&model),
        value_codec::Serialize::to_value(&result.model)
    );
}

#[test]
fn heal_budget_exact_boundary_accepts_and_boundary_ulp_refuses() {
    let (model, context, proof) = heal_fixture();
    let vertex = model.edges[0].vertices[0];
    let writes = 1 + model
        .edges
        .iter()
        .filter(|edge| edge.vertices.contains(&vertex))
        .count();
    let exact = context.spatial_bounds().absolute_mm / writes as f64;
    let operation = |delta: f64| {
        let mut to = model.vertices[vertex].point;
        to[1] += delta;
        HealOperation::EndpointSnap {
            vertex,
            expected_id: model.1.vertices[vertex],
            to,
            correspondence: proof.clone(),
        }
        .bind_native_proof()
    };
    AuthorizedHealPlan::new(
        &model,
        &context,
        vec![operation(exact)],
        context.spatial_bounds().absolute_mm,
        context.spatial_bounds().absolute_mm,
    )
    .unwrap();
    let above = f64::from_bits(exact.to_bits() + 1);
    assert_eq!(
        AuthorizedHealPlan::new(
            &model,
            &context,
            vec![operation(above)],
            context.spatial_bounds().absolute_mm,
            context.spatial_bounds().absolute_mm,
        )
        .unwrap_err()
        .code,
        "BREP_HEAL_BUDGET_EXCEEDED"
    );
}

#[test]
fn heal_positive_one_to_one_rational_refit_is_complete() {
    let (model, context, proof) = heal_fixture();
    let edge_index = 0;
    let mut replacement = model.edges[edge_index].curve.clone();
    replacement.control_points[1][2] += context.spatial_bounds().absolute_mm * 0.25;
    let plan = AuthorizedHealPlan::new(
        &model,
        &context,
        vec![
            HealOperation::CurveRefit {
                edge: edge_index,
                expected_id: model.1.edges[edge_index],
                replacement,
                correspondence: proof,
            }
            .bind_native_proof(),
        ],
        context.spatial_bounds().absolute_mm,
        context.spatial_bounds().absolute_mm,
    )
    .unwrap();
    let mut transaction = crate::transactions::AuthorizedHealTransaction::begin(
        crate::transactions::ModelSnapshot::new(model).unwrap(),
        crate::transactions::HealCancellation::default(),
    );
    transaction.apply(&plan).unwrap();
    let result = transaction.commit().unwrap();
    assert!(result.displacement[0].actual_mm > 0.);
    assert!(result.audit.ok && result.naming_complete);
}

#[test]
fn heal_refuses_unbound_stale_and_overlapping_recipes() {
    let (model, context, proof) = heal_fixture();
    let vertex = model.edges[0].vertices[0];
    let to = model.vertices[vertex].point;
    let unbound = HealOperation::EndpointSnap {
        vertex,
        expected_id: model.1.vertices[vertex],
        to,
        correspondence: proof.clone(),
    };
    assert_eq!(
        AuthorizedHealPlan::new(
            &model,
            &context,
            vec![unbound],
            context.spatial_bounds().absolute_mm,
            context.spatial_bounds().absolute_mm,
        )
        .unwrap_err()
        .code,
        "BREP_HEAL_PROOF_RECIPE_MISMATCH"
    );
    let mut stale = HealOperation::EndpointSnap {
        vertex,
        expected_id: model.1.vertices[vertex],
        to,
        correspondence: proof.clone(),
    }
    .bind_native_proof();
    if let HealOperation::EndpointSnap { to, .. } = &mut stale {
        to[2] = f64::from_bits(to[2].to_bits() + 1);
    }
    assert_eq!(
        AuthorizedHealPlan::new(
            &model,
            &context,
            vec![stale],
            context.spatial_bounds().absolute_mm,
            context.spatial_bounds().absolute_mm,
        )
        .unwrap_err()
        .code,
        "BREP_HEAL_PROOF_RECIPE_MISMATCH"
    );
    let snap = HealOperation::EndpointSnap {
        vertex,
        expected_id: model.1.vertices[vertex],
        to,
        correspondence: proof.clone(),
    }
    .bind_native_proof();
    let refit = HealOperation::CurveRefit {
        edge: 0,
        expected_id: model.1.edges[0],
        replacement: model.edges[0].curve.clone(),
        correspondence: proof,
    }
    .bind_native_proof();
    assert_eq!(
        AuthorizedHealPlan::new(
            &model,
            &context,
            vec![snap, refit],
            context.spatial_bounds().absolute_mm,
            context.spatial_bounds().absolute_mm,
        )
        .unwrap_err()
        .code,
        "BREP_HEAL_WRITESET_OVERLAP"
    );
    let unrelated = HealOperation::CurveRefit {
        edge: 1,
        expected_id: model.1.edges[1],
        replacement: model.edges[1].curve.clone(),
        correspondence: prove_model_edge_correspondence(&model, &context, 0).unwrap(),
    }
    .bind_native_proof();
    assert_eq!(
        AuthorizedHealPlan::new(
            &model,
            &context,
            vec![unrelated],
            context.spatial_bounds().absolute_mm,
            context.spatial_bounds().absolute_mm,
        )
        .unwrap_err()
        .code,
        "BREP_HEAL_PROOF_UNRELATED"
    );
}

#[test]
fn heal_refit_requires_exact_rational_cardinality_degree_knots_and_weights() {
    let (model, context, proof) = heal_fixture();
    let assert_refused = |replacement: Curve| {
        AuthorizedHealPlan::new(
            &model,
            &context,
            vec![
                HealOperation::CurveRefit {
                    edge: 0,
                    expected_id: model.1.edges[0],
                    replacement,
                    correspondence: proof.clone(),
                }
                .bind_native_proof(),
            ],
            context.spatial_bounds().absolute_mm,
            context.spatial_bounds().absolute_mm,
        )
        .unwrap_err()
        .code
    };
    let mut cardinality = model.edges[0].curve.clone();
    cardinality
        .control_points
        .push(cardinality.control_points[0].clone());
    cardinality.weights.push(1.);
    let code = assert_refused(cardinality);
    assert!(code.starts_with("NURBS_") || code == "BREP_HEAL_REFIT_REFUSED");
    let mut degree = model.edges[0].curve.clone();
    degree.degree += 1;
    let code = assert_refused(degree);
    assert!(code.starts_with("NURBS_") || code == "BREP_HEAL_REFIT_REFUSED");
    let mut knot = model.edges[0].curve.clone();
    knot.knots[0] = f64::from_bits(knot.knots[0].to_bits() + 1);
    let code = assert_refused(knot);
    assert!(code.starts_with("NURBS_") || code == "BREP_HEAL_REFIT_REFUSED");
    let mut weight = model.edges[0].curve.clone();
    weight.weights[0] = f64::from_bits(weight.weights[0].to_bits() + 1);
    assert_eq!(assert_refused(weight), "BREP_HEAL_REFIT_REFUSED");
    let split = HealOperation::EdgeSplit {
        edge: 0,
        expected_id: model.1.edges[0],
        parameter: model.edges[0].curve.domain().iter().sum::<f64>() * 0.5,
        correspondence: proof,
    }
    .bind_native_proof();
    assert_eq!(
        AuthorizedHealPlan::new(
            &model,
            &context,
            vec![split],
            context.spatial_bounds().absolute_mm,
            context.spatial_bounds().absolute_mm,
        )
        .unwrap_err()
        .code,
        "BREP_HEAL_SPLIT_REFUSED"
    );
}
