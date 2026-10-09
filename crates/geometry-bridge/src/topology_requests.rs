use super::*;

pub(super) fn require_exact_fields(value: &Value, expected: &[&str], label: &str) -> Result<()> {
    let object = value
        .as_object()
        .ok_or_else(|| input(format!("{label} must be an object")))?;
    if object.len() != expected.len() || !expected.iter().all(|field| object.contains_key(*field)) {
        return Err(input(format!(
            "{label} contains missing or unauthorized fields"
        )));
    }
    Ok(())
}

pub(super) fn close_topology_role(value: &str) -> Result<brep_core::BodyRole> {
    Ok(match value {
        "wire" => brep_core::BodyRole::Wire,
        "face" => brep_core::BodyRole::Face,
        "sheet-shell" => brep_core::BodyRole::SheetShell,
        "open-shell" => brep_core::BodyRole::OpenShell,
        "solid" => brep_core::BodyRole::Solid,
        "compound" => brep_core::BodyRole::Compound,
        _ => return Err(input("Unknown close-topology body role")),
    })
}

pub(super) fn close_topology_audit_value(value: &Value) -> Result<Value> {
    require_exact_fields(
        value,
        &["op", "parts", "sharedFaces", "radialRings", "vertexFans"],
        "close topology request",
    )?;
    let parts_value = value["parts"]
        .as_array()
        .ok_or_else(|| input("parts must be an array"))?;
    let mut parts = Vec::with_capacity(parts_value.len());
    for part in parts_value {
        require_exact_fields(part, &["role", "model"], "close topology part")?;
        parts.push(brep_core::ComplexPart {
            role: close_topology_role(
                part["role"]
                    .as_str()
                    .ok_or_else(|| input("part role must be a string"))?,
            )?,
            model: value_codec::from_value(part["model"].clone())
                .map_err(|e| input(e.to_string()))?,
        });
    }
    let mut shared_faces = Vec::new();
    for relation in value["sharedFaces"]
        .as_array()
        .ok_or_else(|| input("sharedFaces must be an array"))?
    {
        let uses = relation
            .as_array()
            .ok_or_else(|| input("shared face must be an array"))?;
        shared_faces.push(brep_core::SharedFace {
            uses: uses
                .iter()
                .map(|use_| {
                    require_exact_fields(use_, &["part", "face", "reversed"], "shared face use")?;
                    Ok(brep_core::FaceRef {
                        part: field(use_, "part")?,
                        face: field(use_, "face")?,
                        reversed: field(use_, "reversed")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        });
    }
    let mut radial_rings = Vec::new();
    for relation in value["radialRings"]
        .as_array()
        .ok_or_else(|| input("radialRings must be an array"))?
    {
        let uses = relation
            .as_array()
            .ok_or_else(|| input("radial ring must be an array"))?;
        radial_rings.push(brep_core::EdgeRadialRing {
            uses: uses
                .iter()
                .map(|use_| {
                    require_exact_fields(
                        use_,
                        &["part", "face", "edge", "reversed"],
                        "radial use",
                    )?;
                    Ok(brep_core::EdgeUseRef {
                        part: field(use_, "part")?,
                        face: field(use_, "face")?,
                        edge: field(use_, "edge")?,
                        reversed: field(use_, "reversed")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        });
    }
    let mut vertex_fans = Vec::new();
    for fan in value["vertexFans"]
        .as_array()
        .ok_or_else(|| input("vertexFans must be an array"))?
    {
        require_exact_fields(fan, &["part", "vertex", "closed", "uses"], "vertex fan")?;
        let uses = fan["uses"]
            .as_array()
            .ok_or_else(|| input("fan uses must be an array"))?;
        vertex_fans.push(brep_core::VertexFan {
            vertex: (field(fan, "part")?, field(fan, "vertex")?),
            closed: field(fan, "closed")?,
            uses: uses
                .iter()
                .map(|use_| {
                    require_exact_fields(use_, &["part", "face", "vertex"], "vertex fan use")?;
                    Ok(brep_core::VertexUseRef {
                        part: field(use_, "part")?,
                        face: field(use_, "face")?,
                        vertex: field(use_, "vertex")?,
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        });
    }
    let audited = brep_core::MixedDimensionalBrep::new(
        parts,
        shared_faces,
        radial_rings,
        vertex_fans,
        vec![],
    )?
    .audit()?;
    let certificate = audited.certificate();
    let boundary_faces = audited.boundary_faces();
    Ok(json!({
        "certificate": {
            "capability": certificate.capability,
            "complete": certificate.complete,
            "partCount": certificate.part_count,
            "solidCellCount": certificate.solid_cell_count,
            "sheetCount": certificate.sheet_count,
            "openShellCount": certificate.open_shell_count,
            "sharedFaceCount": certificate.shared_face_count,
            "nonManifoldEdgeCount": certificate.non_manifold_edge_count,
            "vertexFanCount": certificate.vertex_fan_count,
            "boundaryFaceCount": certificate.boundary_face_count,
            "maxRadialValence": certificate.max_radial_valence,
            "namingComplete": certificate.naming_complete,
            "notes": certificate.notes
        },
        "boundaryFaces": boundary_faces.iter().map(|face| json!({
            "part":face.part,"face":face.face,"reversed":face.reversed
        })).collect::<Vec<_>>(),
        "decomposition": audited.manifold_decomposition().into_iter().map(|part| json!({
            "role":part.role.as_str(),"model":part.model
        })).collect::<Vec<_>>()
    }))
}

pub(super) fn curved_graph_boolean_value(
    model: brep_core::Model,
    certificate: brep_core::CurvedGraphBooleanCertificate,
) -> Result<Value> {
    let axis = match certificate.axis {
        brep_core::nurbs_ss_g6::ExactIsoAxis::U => "U",
        brep_core::nurbs_ss_g6::ExactIsoAxis::V => "V",
    };
    Ok(json!({
        "model": encode(model)?,
        "certificate": {
            "capability": certificate.capability,
            "status": certificate.status,
            "operation": certificate.operation,
            "axis": axis,
            "fixedParameter": certificate.fixed_parameter,
            "lineage": {
                "sourceFace": certificate.source_face,
                "retainedFace": certificate.retained_face,
                "deletedRegion": certificate.deleted_region,
                "generatedIntersectionEdge": certificate.intersection_edge
            },
            "tensorCells": certificate.tensor_cells,
            "exactCorrespondence": certificate.exact_correspondence,
            "sew": {
                "matched": certificate.sew.matched,
                "complete": certificate.sew.complete,
                "displacementBudgetOk": certificate.sew.displacement_budget_ok
            },
            "audit": {
                "ok": certificate.audit.ok,
                "bodyCount": certificate.audit.body_count,
                "shellCount": certificate.audit.shell_count,
                "selfIntersectionPairsCandidate": certificate.audit.self_intersection_pairs_candidate,
                "selfIntersectionComplete": certificate.audit.self_intersection_complete,
                "selfIntersectionPairsChecked": certificate.audit.self_intersection_pairs_checked,
                "notes": certificate.audit.notes
            },
            "changeSet": encode(certificate.change_set)?,
            "namingComplete": certificate.naming_complete,
            "noFallback": certificate.no_fallback,
            "separationProof": certificate.separation_proof,
            "homogeneousRootProof": certificate.homogeneous_root_proof,
            "denominatorLowerBound": certificate.denominator_lower_bound,
            "weightConditionNumber": certificate.weight_condition_number,
            "resourceBound": certificate.resource_bound
        }
    }))
}

pub(super) fn contained_graph_boolean_value(
    model: brep_core::Model,
    certificate: brep_core::ContainedGraphBooleanCertificate,
) -> Result<Value> {
    Ok(json!({
        "model": encode(model)?,
        "certificate": {
            "capability": certificate.capability,
            "status": certificate.status,
            "operation": certificate.operation,
            "relation": certificate.relation,
            "strictUvMargin": certificate.strict_uv_margin,
            "floorClearance": certificate.floor_clearance,
            "roofClearance": certificate.roof_clearance,
            "cavityProof": certificate.cavity_proof,
            "separationProof": certificate.separation_proof,
            "audit": {
                "ok": certificate.audit.ok,
                "bodyCount": certificate.audit.body_count,
                "shellCount": certificate.audit.shell_count,
                "notes": certificate.audit.notes
            },
            "changeSet": encode(certificate.change_set)?,
            "namingComplete": certificate.naming_complete,
            "noFallback": certificate.no_fallback
        }
    }))
}

pub(super) fn general_nurbs_boolean_value(
    model: brep_core::Model,
    certificate: brep_core::GeneralNurbsBooleanCertificate,
) -> Result<Value> {
    Ok(json!({
        "model": encode(model)?,
        "certificate": {
            "capability": certificate.capability,
            "authority": certificate.authority,
            "status": certificate.status,
            "operation": certificate.operation,
            "operandOrder": certificate.operand_order,
            "exactRegionMembership": certificate.exact_region_membership,
            "partitionCells": certificate.partition_cells,
            "cavityCount": certificate.cavity_count,
            "branchGraph": {
                "components": certificate.branch_graph.certificate.component_count,
                "fragments": certificate.branch_graph.certificate.fragment_count,
                "candidateSpanPairs": certificate.branch_graph.certificate.candidate_span_pairs,
                "sourceSpanCount": certificate.branch_graph.certificate.source_span_count,
                "denominatorLowerBound": certificate.branch_graph.certificate.denominator_lower_bound,
                "complete": certificate.branch_graph.permits_topology_authorship()
            },
            "uv": {
                "tensorCells": certificate.uv.tensor_cell_count,
                "branches": certificate.uv.branch_count,
                "materialCells": certificate.uv.material_cell_count,
                "holeCells": certificate.uv.hole_cell_count,
                "complete": certificate.uv.permits_trim_classification()
            },
            "exactCurvePcurveCount": certificate.exact_curve_pcurve_count,
            "ssReportsComplete": certificate.ss_reports_complete,
            "ssFacePairs": certificate.ss_face_pairs,
            "sew": {
                "matched": certificate.sew.matched,
                "complete": certificate.sew.complete,
                "displacementBudgetOk": certificate.sew.displacement_budget_ok
            },
            "audit": {
                "ok": certificate.audit.ok,
                "bodyCount": certificate.audit.body_count,
                "shellCount": certificate.audit.shell_count,
                "selfIntersectionPairsCandidate": certificate.audit.self_intersection_pairs_candidate,
                "selfIntersectionComplete": certificate.audit.self_intersection_complete,
                "selfIntersectionPairsChecked": certificate.audit.self_intersection_pairs_checked,
                "notes": certificate.audit.notes
            },
            "changeSet": encode(certificate.change_set)?,
            "naming": {
                "split": certificate.naming.split,
                "retained": certificate.naming.retained,
                "deleted": certificate.naming.deleted,
                "generated": certificate.naming.generated,
                "operationStable": certificate.naming.operation_stable
            },
            "resultComponents": certificate.result_components,
            "resultFaces": certificate.result_faces,
            "noFallback": certificate.no_fallback
        }
    }))
}
