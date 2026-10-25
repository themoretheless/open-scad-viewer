//! Exact expression recipe replay. Closed incidence is not solid admission.
use super::{Result, Value, field};
use brep_core::source_profile_body as native;
use value_codec::json;
fn shell_limits(v: &Value) -> Result<native::ShellReplayLimits> {
    let b = &v["boundary"];
    let s = &b["seam"];
    Ok(native::ShellReplayLimits {
        boundary: native::TransitionBoundaryReplayLimits {
            seam: native::RootSeamReplayLimits {
                cut_spans: field(s, "cutSpans")?,
                mapping_cells: field(s, "mappingCells")?,
                max_sine_squared: field(s, "maxSineSquared")?,
                angular_cells: field(s, "angularCells")?,
                seam_spans: field(s, "seamSpans")?,
            },
            controls: field(b, "controls")?,
            side_cells: field(b, "sideCells")?,
            side_spans: field(b, "sideSpans")?,
        },
        projection: field(v, "projection")?,
        cut_spans_per_section: field(v, "cutSpansPerSection")?,
        mapping_cells_per_section: field(v, "mappingCellsPerSection")?,
        fragments_per_section: field(v, "fragmentsPerSection")?,
        controls: field(v, "controls")?,
        join_cells: field(v, "joinCells")?,
        join_spans: field(v, "joinSpans")?,
        exterior_drivers: field(v, "exteriorDrivers")?,
        collapsed_ends: field(v, "collapsedEnds")?,
        subdivisions: field(v, "subdivisions")?,
        quotient_cells: field(v, "quotientCells")?,
        retained_cells: field(v, "retainedCells")?,
        retained_spans: field(v, "retainedSpans")?,
        transition_cells: field(v, "transitionCells")?,
        transition_spans: field(v, "transitionSpans")?,
        exact_work_per_rail: field(v, "exactWorkPerRail")?,
        drivers_per_rail: field(v, "driversPerRail")?,
        fragments_per_rail: field(v, "fragmentsPerRail")?,
        outer_exact_work: field(v, "outerExactWork")?,
        outer_drivers: field(v, "outerDrivers")?,
        shell_uses: field(v, "shellUses")?,
    })
}
pub fn restore(v: Value) -> Result<Value> {
    restore_inner(v, false)
}
pub fn inspect(v: Value) -> Result<Value> {
    restore_inner(v, true)
}
fn restore_inner(v: Value, inspect: bool) -> Result<Value> {
    let proposal: Value = field(&v, "proposalDefinition")?;
    let boundary: Value = field(&v, "boundaryDefinition")?;
    let source_limits: Value = field(&v, "sourceLimits")?;
    let p = &v["profileLimits"];
    let profile_limits = native::Limits {
        controls: field(p, "controls")?,
        fragments: field(p, "fragments")?,
        position_tolerance: field(p, "positionTolerance")?,
    };
    let shell_work = shell_limits(&v["shellLimits"])?;
    let mut request = json!({"proposalDefinition":proposal,"boundaryDefinition":boundary,"sourceLimits":source_limits,"profileLimits":p,"shellLimits":v["shellLimits"]});
    if inspect {
        request["inspectionLimits"] = v["inspectionLimits"].clone();
    }
    if !v["displayLimits"].is_null() {
        request["displayLimits"] = v["displayLimits"].clone();
    }
    if !v["meshLimits"].is_null() {
        request["meshLimits"] = v["meshLimits"].clone();
    }
    if !v["boundaryMeshLimits"].is_null() {request["boundaryMeshLimits"]=v["boundaryMeshLimits"].clone();}
    if !v["alignedChordCheck"].is_null() {request["alignedChordCheck"]=v["alignedChordCheck"].clone();}
    if !v["radiusCheck"].is_null() {request["radiusCheck"]=v["radiusCheck"].clone();}
    if !v["supportTangencyCheck"].is_null() {request["supportTangencyCheck"]=v["supportTangencyCheck"].clone();}
    if !v["neighborTangencyCheck"].is_null() {request["neighborTangencyCheck"]=v["neighborTangencyCheck"].clone();}
    let response = |reason: &str, result: Option<Value>| {
        json!({"version":1,"method":if inspect {"exact-expression-shell-inspection"}else{"exact-expression-shell-replay"},
        "proposalDefinition":proposal,"boundaryDefinition":boundary,
        "request":request,
        "incidenceQualified":result.is_some(),"embeddingQualified":result.as_ref().is_some_and(|s|s["inspection"]["embeddingQualified"].as_bool()==Some(true)),"materialQualified":false,
        "reason":reason,"shell":result})
    };
    let source = brep_core::source_body_restore::restore(
        proposal["originalBody"].clone(),
        super::cad_source_body::limits(&source_limits)?,
    )?;
    let Some(body) = source.body() else {
        return Ok(response(source.reason(), None));
    };
    let profiles = native::restore(body, proposal.clone(), profile_limits)?;
    let Some(profiles) = profiles.proposal else {
        return Ok(response(profiles.reason, None));
    };
    let replay = native::with_restored_expression_shell(
        &profiles,
        boundary.clone(),
        shell_work,
        |shell| {
            let faces = shell
                .faces()
                .iter()
                .enumerate()
                .map(|(id, face)| {
                    let (kind, source) = match face.kind() {
                        native::ShellFaceKind::Original(source) => ("original", Some(source)),
                        native::ShellFaceKind::Retained(side) => ("retained", Some(side)),
                        native::ShellFaceKind::Transition => ("transition", None),
                    };
                    json!({"id":id,"kind":kind,"source":source,"loops":face.loops().len()})
                })
                .collect::<Vec<_>>();
            let mut result = json!({"faces":faces,"faceCount":shell.faces().len(),"pairCount":shell.pairs().len(),
                "vertexCount":shell.vertices(),"poleCount":shell.poles().len(),"components":shell.components()});
            if !v["supportTangencyCheck"].is_null() {
                let limits=&v["supportTangencyCheck"];
                let checked=shell.assembly().patch().rails().face().qualify_support_tangency(
                    field(limits,"controls")?,field(limits,"cells")?,field(limits,"spans")?)?;
                let boundaries=checked.boundaries.iter().enumerate().map(|(index,report)| {
                    json!({"index":index,"kind":if index<2 {"section"}else{"rail"},"high":index%2==1,
                        "qualified":report.complete_regular && report.pending.is_empty(),
                        "regular":report.regular,"pending":report.pending,
                        "cells":report.cells,"spans":report.spans,"reason":report.reason})
                }).collect::<Vec<_>>();
                result["supportTangencyCheck"]=json!({"version":1,"qualified":checked.certificate.is_some(),
                    "basis":"owned-full-Hermite-original-profile-support", "boundaries":boundaries,
                    "cells":checked.cells,"spans":checked.spans,"reason":checked.reason,
                    "neighborBodyQualified":false,"solidQualified":false});
            }
            if !v["neighborTangencyCheck"].is_null() {
                let limits=&v["neighborTangencyCheck"];
                let checked=shell.qualify_transition_neighbor_tangency(field(limits,"maxSineSquared")?,
                    field(limits,"controls")?,field(limits,"cells")?,field(limits,"spans")?)?;
                result["neighborTangencyCheck"]=json!({"version":1,
                    "qualified":checked.certificate.is_some(),
                    "basis":"owned-full-Hermite-actual-shell-neighbors",
                    "upperSineSquared":checked.certificate.as_ref().map(|c|c.upper_sine_squared()),
                    "cells":checked.cells,"spans":checked.spans,"reason":checked.reason,
                    "solidQualified":false});
            }
            if !v["radiusCheck"].is_null() {
                let r=&v["radiusCheck"];
                let centers:nurbs_core::curve::Curve=field(r,"centers")?;
                let radius:nurbs_core::curve::Curve=field(r,"radius")?;
                let tolerance:f64=field(r,"toleranceMm")?;
                let checked=shell.assembly().patch().rails().face().qualify_radius_coefficients(
                    &centers,&radius,field(r,"lawDomains")?,tolerance,
                    field(r,"controls")?,field(r,"spans")?,field(r,"coefficientWork")?)?;
                let bound=checked.bound.as_ref();
                result["radiusCheck"]=json!({"version":1,"qualified":checked.certificate.is_some(),
                    "violationQualified":bound.and_then(|b|b.error_lower).is_some_and(|e|e>tolerance),
                    "errorLowerMm":bound.and_then(|b|b.error_lower),
                    "errorUpperMm":bound.and_then(|b|b.error_upper),
                    "spans":bound.map_or(0,|b|b.spans),
                    "coefficientWork":bound.map_or(0,|b|b.coefficient_work),
                    "reason":checked.reason,"solidQualified":false});
            }
            if !v["displayLimits"].is_null() {
                let d = &v["displayLimits"];
                let display = shell.inspect_display_enclosures(native::ExpressionDisplayLimits {
                    divisions: field(d, "divisions")?,
                    cells: field(d, "cells")?,
                    controls_per_enclosure: field(d, "controlsPerEnclosure")?,
                    spans: field(d, "spans")?,
                    tolerance_uv: if d["toleranceUv"].is_null() {1e-8} else {field(d,"toleranceUv")?},
                    domain_cells: if d["domainCells"].is_null() {0} else {field(d,"domainCells")?},
                    domain_cells_per_cell: if d["domainCellsPerCell"].is_null() {0} else {field(d,"domainCellsPerCell")?},
                })?;
                result["supportEnclosures"] = json!({"trimQualified":false,
                    "enclosed":display.enclosed,"pending":display.pending,"spans":display.spans,
                    "inside":display.inside,"outside":display.outside,"unresolvedTrim":display.unresolved_trim,
                    "domainCells":display.domain_cells,
                    "cells":display.cells.iter().map(|cell|json!({"face":cell.face,
                        "domain":cell.domain,"bounds":cell.bounds,"location":match cell.location {
                            nurbs_core::trim_domain::Location::Inside=>"inside",
                            nurbs_core::trim_domain::Location::Outside=>"outside",
                            _=>"unresolved",
                        }})).collect::<Vec<_>>()});
            }
            if !v["meshLimits"].is_null() {
                let m = &v["meshLimits"];
                let mesh = shell.prepare_display_mesh(native::ExpressionMeshLimits {
                    max_deviation_mm: field(m, "maxDeviationMm")?,
                    cells: field(m, "cells")?,
                    controls_per_enclosure: field(m, "controlsPerEnclosure")?,
                    spans: field(m, "spans")?,
                    tolerance_uv: field(m, "toleranceUv")?,
                    domain_cells: field(m, "domainCells")?,
                    domain_cells_per_cell: field(m, "domainCellsPerCell")?,
                    max_depth: field(m, "maxDepth")?,
                })?;
                result["displayMesh"] = json!({"version":1,"watertightQualified":false,
                    "splitCells":mesh.split_cells,"leafCount":mesh.tiles.len()+mesh.pending.len()+mesh.outside.len(),
                    "complete":mesh.pending.is_empty(),"cells":mesh.cells,"spans":mesh.spans,
                    "domainCells":mesh.domain_cells,"maxDeviationUpperMm":mesh.max_deviation_upper_mm,
                    "trianglesPerTile":[[0,1,2],[0,2,3]],
                    "tiles":mesh.tiles.iter().map(|tile|json!({"face":tile.face,"domain":tile.domain,
                        "corners":tile.corners,"deviationUpperMm":tile.deviation_upper_mm})).collect::<Vec<_>>(),
                    "pending":mesh.pending.iter().map(|cell|json!({"face":cell.face,"domain":cell.domain,
                        "bounds":cell.bounds,"reason":cell.reason})).collect::<Vec<_>>(),
                    "outside":mesh.outside.iter().map(|(face,domain)|json!({"face":face,"domain":domain})).collect::<Vec<_>>()});
            }
            if !v["boundaryMeshLimits"].is_null() {
                let m=&v["boundaryMeshLimits"];
                let mut graph=shell.prepare_boundary_graph(brep_core::source_shared_edge::BoundaryMeshLimits {
                    max_deviation_mm:field(m,"maxDeviationMm")?,cells:field(m,"cells")?,
                    spans:field(m,"spans")?,max_depth:field(m,"maxDepth")?})?;
                let candidates=shell.propose_boundary_face_triangles(&graph);
                let incidence=candidates.inspect_incidence();
                let refined=if !m["refinement"].is_null() {
                    let r=&m["refinement"];
                    let mut report=shell.refine_face_candidates(&mut graph,&candidates,native::TriangleRefinementLimits {
                        max_deviation_mm:field(r,"maxDeviationMm")?,cells:field(r,"cells")?,spans:field(r,"spans")?,
                        spans_per_query:field(r,"spansPerQuery")?,controls:field(r,"controls")?,domain_cells:field(r,"domainCells")?,
                        domain_cells_per_triangle:field(r,"domainCellsPerTriangle")?,tolerance_uv:field(r,"toleranceUv")?,max_depth:field(r,"maxDepth")?})?;
                    let regional=shell.qualify_pending_transition_regions(&graph,&mut report,field(r,"regionalSpans")?)?;
                    let closed=report.pending_faces.is_empty() && report.inspect_leaf_incidence().closed_oriented();
                    Some(json!({"version":1,"limits":r,"solidQualified":false,"closedOrientedIncidence":closed,
                        "initialTriangles":report.initial_triangles,"initialChecked":report.initial_checked,
                        "splitTriangles":report.split_triangles,"cells":report.cells,"spans":report.spans,"domainCells":report.domain_cells,
                        "regional":{"spans":regional.spans,"queries":regional.queries,"accepted":regional.accepted},
                        "accepted":report.accepted.iter().map(|t|json!({"face":t.triangle.face,"vertices":t.triangle.vertices,
                            "uv":t.triangle.uv,"deviationUpperMm":t.deviation_upper_mm})).collect::<Vec<_>>(),
                        "pending":report.pending.iter().map(|t|json!({"face":t.triangle.face,"vertices":t.triangle.vertices,
                            "uv":t.triangle.uv,"reason":t.reason})).collect::<Vec<_>>(),
                        "numericFailures":report.numeric_failures.iter().map(|t|json!({"face":t.face,"vertices":t.vertices,
                            "uv":t.uv,"stage":t.stage,"code":t.code,"message":t.message})).collect::<Vec<_>>() }))
                } else {None};
                result["boundaryMesh"]=json!({"version":1,"materialQualified":false,"surfaceToleranceQualified":false,
                    "closedOrientedIncidence":candidates.pending.is_empty() && incidence.closed_oriented(),
                    "openEdges":incidence.open_edges,"nonmanifoldEdges":incidence.nonmanifold_edges,
                    "orientationConflicts":incidence.orientation_conflicts,"invalidVertexLinks":incidence.invalid_vertex_links,
                    "vertices":graph.vertices.iter().map(|v|json!({"bounds":v.bounds,"point":v.point})).collect::<Vec<_>>(),
                    "segments":graph.segments.iter().map(|s|json!({"pair":s.pair,"vertices":s.vertices,
                        "uv":s.uv,"deviationUpperMm":s.deviation_upper_mm})).collect::<Vec<_>>(),
                    "pending":graph.pending.iter().map(|p|json!({"pair":p.pair,"vertices":p.vertices,"reason":p.reason})).collect::<Vec<_>>(),
                    "candidateTriangles":candidates.triangles.iter().map(|t|json!({"face":t.face,"vertices":t.vertices,"uv":t.uv})).collect::<Vec<_>>(),
                    "pendingFaces":candidates.pending.iter().map(|p|json!({"face":p.face,"reason":p.reason})).collect::<Vec<_>>()});
                if let Some(refined)=refined {result["boundaryMesh"]["refinement"]=refined;}
            }
            if !v["alignedChordCheck"].is_null() {result["alignedChordCheck"]=Value::Null;}
            if inspect {
                let limits = &v["inspectionLimits"];
                let projected = &limits["projectedRegion"];
                let region = shell.assembly().patch().face().qualify_profile_xy_region(
                    field(projected, "toleranceUv")?,
                    field(projected, "exactWork")?,
                    field(projected, "driverCells")?,
                )?;
                result["inspection"] = json!({"embeddingQualified":false,"volumeQualified":false,"reason":region.reason});
                if let Some(region) = region.certificate {
                    let c = &limits["contacts"];
                    let contacts = shell.inspect_reconstructed_contacts(
                        native::ContactAuditLimits {
                            cells: field(c, "cells")?,
                            cells_per_pair: field(c, "cellsPerPair")?,
                            controls_per_enclosure: field(c, "controlsPerEnclosure")?,
                            spans: field(c, "spans")?,
                            domain_cells: field(c, "domainCells")?,
                            domain_cells_per_pair: field(c, "domainCellsPerPair")?,
                            exact_work: field(c, "exactWork")?,
                            driver_cells: field(c, "driverCells")?,
                            tolerance_uv: field(c, "toleranceUv")?,
                        },
                        &region,
                        field(limits, "linearCells")?,
                        field(limits, "linearCellsPerPair")?,
                        field(limits, "refinementCellsPerPair")?,
                    )?;
                    let geometry = shell.qualify_embedding(&contacts);
                    let mut inspection = json!({"embeddingQualified":geometry.is_some(),"volumeQualified":false,"contacts":contacts.diagnostics(),"reason":"expression-contacts-unproven"});
                    if let Some(geometry) = geometry {
                        if !v["alignedChordCheck"].is_null() {
                            let limits=&v["alignedChordCheck"];
                            let checked=geometry.qualify_trimmed_minimum_aligned_chords(
                                field(limits,"minimumMm")?,field(limits,"maxSineSquared")?,
                                field(limits,"toleranceUv")?,field(limits,"controls")?,
                                field(limits,"cells")?,field(limits,"spans")?,field(limits,"domainCells")?)?;
                            result["alignedChordCheck"]=json!({"version":1,
                                "qualified":checked.certificate.is_some(),"report":checked.report,
                                "wholeThicknessQualified":false,"solidQualified":false});
                        }

                        let l = &limits["volume"];
                        let volume = geometry.qualify_volume(native::ExpressionVolumeLimits {
                            axis: field(l, "axis")?,
                            origin: field(l, "origin")?,
                            absolute_error: field(l, "absoluteError")?,
                            tolerance_uv: field(l, "toleranceUv")?,
                            controls_per_enclosure: field(l, "controlsPerEnclosure")?,
                            cells: field(l, "cells")?,
                            spans: field(l, "spans")?,
                            domain_cells: field(l, "domainCells")?,
                            domain_cells_per_rectangle: field(l, "domainCellsPerRectangle")?,
                        })?;
                        inspection["volumeQualified"] = json!(volume.certificate.is_some());
                        inspection["reason"] = json!(volume.report.reason);
                        inspection["volume"] = json!({"complete":volume.report.complete,"signedBounds":volume.report.signed_bounds,
                            "positiveBounds":volume.certificate.as_ref().map(|p|p.positive_bounds()),
                            "cells":volume.report.cells,"spans":volume.report.spans,"domainCells":volume.report.domain_cells,
                            "uncertainFace":volume.report.uncertain_face,"uncertainDomain":volume.report.uncertain_domain,"reason":volume.report.reason});
                    }
                    result["inspection"] = inspection;
                }
            }
            Ok(result)
        },
    )?;
    Ok(response(replay.reason, replay.result))
}
