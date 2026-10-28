//! Rust owns construction, every proof flag and refusals. Construction ignores
//! a caller model; audit recomputes every obligation for supplied geometry.
//! Caller success fields are ignored. No geometry decision is delegated to TS.
use super::*;
fn atlas(p: &nurbs_core::progressive_sweep::IdealVolumeReport) -> Value {
    json!({"embeddingCertified":p.embedding_certified,"volumeRegularityCertified":p.regularity_certified,
        "parameterTopologyPreserved":p.parameter_topology_preserved,"cells":p.cells,"exactWork":p.exact_work,
        "boxes":p.boxes,"projection":p.projection,"reason":p.reason})
}
pub(super) fn construct(
    sweep: &MultiSweep<'_>,
    loop_sizes: &[usize],
    sections: usize,
    tolerance_uv: f64,
    error_budget: f64,
    ideal: owned::IdealLimits,
    v: &Value,
    audit: bool,
) -> Result<Value> {
    let material = LoftCap {
        surface: field(&v["material"], "surface")?,
        trims: field(&v["material"], "trims")?,
    };
    let l = &v["limits"];
    let m = &l["material"];
    let s = &v["smoothing"];
    let a = &s["atlas"];
    let order = field::<usize>(s, "order")?;
    let limits = owned::SmoothClosedLimits {
        original_smoothness: optional::<Value>(v, "originalSmoothness")?
            .map(|l| {
                Ok::<_, crate::Error>(
                    nurbs_core::progressive_sweep::OriginalBoundarySmoothnessLimits {
                        order: field(&l, "order")?,
                        cells: field(&l, "maxCells")?,
                        exact_work: field(&l, "maxExactWork")?,
                        products: field(&l, "maxProducts")?,
                    },
                )
            })
            .transpose()?,
        source: owned::OriginalClosedLimits {
            source_cells: field(l, "sourceCells")?,
            source_products: field(l, "sourceProducts")?,
            source_exact_work: field(l, "sourceExactWork")?,
            ideal,
            wall_faces: field(l, "maxWallFaces")?,
            material: owned::MaterialLimits {
                pairs: field(m, "maxPairs")?,
                cells: field(m, "maxCells")?,
                domain_cells: field(m, "maxDomainCells")?,
            },
        },
        candidate: brep_core::AffineStationLimits {
            quantum: field(s, "quantum")?,
            tolerance: field(s, "maxDisplacement")?,
            work: field(s, "maxWork")?,
            matrix_cells: field(s, "maxMatrixCells")?,
        },
        smoothness: brep_core::sweep_smoothness::complete::Limits {
            order,
            exact_work: field(s, "maxExactWork")?,
            agreement_work: field(s, "maxAgreementWork")?,
            regularity_cells: field(s, "maxRegularityCells")?,
        },
        atlas: owned::IdealLimits {
            projection: field(a, "projection")?,
            cells: field(a, "maxCells")?,
            exact_work: field(a, "maxExactWork")?,
        },
        binding_work: field(s, "maxBindingWork")?,
        orientation_cells: field(s, "maxOrientationCells")?,
    };
    let sharp = optional::<Vec<usize>>(v, "sharpStations")?.unwrap_or_default();
    let attempt = if audit {
        owned::verify_smooth_closed_with_ideal_atlas(
            sweep,
            loop_sizes,
            sections,
            &material,
            tolerance_uv,
            error_budget,
            &sharp,
            limits,
            &field(v, "model")?,
        )?
    } else {
        owned::construct_smooth_closed_with_ideal_atlas(
            sweep,
            loop_sizes,
            sections,
            &material,
            tolerance_uv,
            error_budget,
            &sharp,
            limits,
        )?
    };
    let (model, r) = match &attempt {
        owned::SmoothClosedAttempt::Certified(b) => (Some(b.model()), b.report()),
        owned::SmoothClosedAttempt::Refused(r) => (None, r.as_ref()),
    };
    let g1 = r.certified && r.smoothness.as_ref().is_some_and(|s| s.g1_certified);
    let g2 = r.certified && r.smoothness.as_ref().is_some_and(|s| s.g2_certified);
    let original_g1 = r.certified && r.original.g1_certified();
    let original_g2 = r.certified && r.original.g2_certified();
    Ok(
        json!({"model":model,"certified":r.certified,"solidGeometryCertified":r.certified,
            "continuousBound":r.continuous_bound,"withinBudget":r.within_budget,"boundaryErrorUpper":r.boundary_error_upper,
            "wallDisplacementUpper":r.displacement_upper,"topologyPreserved":r.topology_preserved,"shellRolesCertified":r.shell_roles_certified,
            "smoothnessCertified":g1,"fullBoundaryG1Certified":g1,"fullBoundaryG2Certified":g2,
            "seamContinuity":if g2 {"G2"} else if g1 {"G1"} else {"C0"},"order":order,
            "scope":"original-closed-sweep-and-owned-affine-solid","physicalCapCount":0,"filledCapErrorUpper":null,
            "idealSourceSmoothnessCertified":original_g1,"idealSourceBoundaryG1Certified":original_g1,
            "idealSourceBoundaryG2Certified":original_g2,"reason":r.reason,
            "original":{"certified":r.original.certified,"cells":r.original.cells,"products":r.original.products,
                "exactWork":r.original.exact_work,"boundaryErrorUpper":r.original.boundary_error_upper,
                "decompositionErrorUpper":r.original.decomposition_error_upper,"reason":r.original.reason,
                "volume":r.original.volume.as_ref().map(atlas),"wall":r.original.wall,
                "smoothness":r.original.smoothness.as_ref().map(|p|json!({"order":p.order,
                    "differentialCompatibilityCertified":p.differential_compatibility_certified,
                    "frameCertified":p.frame_certified,"lawsCertified":p.laws_certified,"profilesCertified":p.profiles_certified,
                    "inspectedProfileJoins":p.inspected_profile_joins,"cells":p.cells,"exactWork":p.exact_work,"products":p.products,"reason":p.reason})),
                "material":{"certified":r.original.material.certified,"boundSpans":r.original.material.bound_spans,
                    "exactWork":r.original.material.exact_work,"trim":r.original.material.trim.as_ref().map(|t|json!({"valid":t.valid,
                        "winding":t.winding,"reason":t.reason,"pairs":t.pairs,"cells":t.cells,"domainCells":t.domain_cells}))}},
            "candidate":{"work":r.candidate_work,"matrixCells":r.matrix_cells,"reason":r.candidate_reason},
            "retainedBinding":r.binding.as_ref().map(|b|json!({"certified":b.certified,"inspectedFaces":b.faces,"exactWork":b.exact_work,
                "reparameterizedFaces":b.reparameterized_faces(),"reason":b.reason})),
            "retainedVolume":r.volume.as_ref().map(|p|{let mut v=atlas(p);v["proofMethod"]=json!("coefficient-owned-filled-volume-atlas");v}),
            "orientation":r.orientation.as_ref().map(|p|json!({"sign":p.sign,"determinant":p.determinant,"cells":r.orientation_cells,"reason":p.reason})),
            "smoothness":r.smoothness.as_ref().map(|s|json!({"extractionComplete":s.extraction_complete,"vertexLinksComplete":s.vertex_links_complete,
                "edgeIds":s.edge_ids,"unresolvedEdges":s.unresolved_edges,"exactWork":s.exact_work,"parameterWork":s.parameter_work,
                "edgeCorrespondenceCertified":s.edge_correspondence.as_ref().is_some_and(|p|p.all_equal&&p.all_joins_exact),
                "agreementWork":s.edge_correspondence.as_ref().map_or(0,|p|p.work),"allFacesRegular":s.regularity.all_faces_regular,
                "regularityCells":s.regularity.cells,"fullBoundaryG1Certified":s.g1_certified,"fullBoundaryG2Certified":s.g2_certified,
                "seams":s.proof.seams.iter().enumerate().map(|(i,p)|json!({"edge":s.edge_ids.get(i),"certified":p.certified,
                    "exactIdentity":p.exact_identity,"regularityCertified":p.regularity_certified,"exactWork":p.work,"reason":p.reason})).collect::<Vec<_>>()}))
        }),
    )
}
