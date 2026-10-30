//! Raw original rational source -> Rust owner, with no sampled frame adapter.
//! Supplied success fields are ignored. Audit never repairs supplied geometry.
use super::*;
use nurbs_core::affine_volume::{Pose, RationalPiece};

pub(super) fn run(v: Value, audit: bool) -> Result<Value> {
    let s = &v["source"];
    let profiles = field::<Vec<Vec<Curve>>>(s, "profiles")?;
    let origin = field::<[f64; 3]>(s, "origin")?;
    let motion = field::<Vec<Value>>(s, "motion")?
        .iter()
        .map(|p| {
            Ok(RationalPiece {
                controls: field::<Vec<Pose>>(p, "controls")?,
                weights: field(p, "weights")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let material = LoftCap {
        surface: field(&v["material"], "surface")?,
        trims: field(&v["material"], "trims")?,
    };
    let caps = field::<Vec<[usize; 2]>>(&v, "capProfiles")?;
    let tolerance_uv = field::<f64>(&v, "toleranceUV")?;
    let l = &v["limits"];
    let m = &l["material"];
    let a = &l["volume"];
    let g = &l["smoothness"];
    let limits = owned::RationalClosedLimits {
        source_cells: field(l, "sourceCells")?,
        source_exact_work: field(l, "sourceExactWork")?,
        construction_rows: field(l, "constructionRows")?,
        max_faces: field(l, "maxFaces")?,
        binding_work: field(l, "maxBindingWork")?,
        orientation_cells: field(l, "maxOrientationCells")?,
        cap_cells: field(l, "maxCapCells")?,
        material: owned::MaterialLimits {
            pairs: field(m, "maxPairs")?,
            cells: field(m, "maxCells")?,
            domain_cells: field(m, "maxDomainCells")?,
        },
        volume: owned::IdealLimits {
            projection: field(a, "projection")?,
            cells: field(a, "maxCells")?,
            exact_work: field(a, "maxExactWork")?,
        },
        smoothness: brep_core::sweep_smoothness::complete::Limits {
            order: field(g, "order")?,
            exact_work: field(g, "maxExactWork")?,
            agreement_work: field(g, "maxAgreementWork")?,
            regularity_cells: field(g, "maxRegularityCells")?,
        },
    };
    let supplied = if audit {
        Some(field::<brep_core::Model>(&v, "model")?)
    } else {
        None
    };
    let attempt = if let Some(model) = supplied.as_ref() {
        owned::verify_rational_closed_material(
            &profiles,
            &material,
            origin,
            &motion,
            &caps,
            tolerance_uv,
            limits,
            model,
        )?
    } else {
        owned::construct_rational_closed_material(
            &profiles,
            &material,
            origin,
            &motion,
            &caps,
            tolerance_uv,
            limits,
        )?
    };
    let joint=match &attempt {
        owned::RationalClosedAttempt::Certified(body)=>[0,1,2].map(|order|body.joint_certificate(order).is_ok()),
        owned::RationalClosedAttempt::Refused(_)=>[false;3],
    };
    let (model, r) = match &attempt {
        owned::RationalClosedAttempt::Certified(b) => (Some(b.model()), b.report()),
        owned::RationalClosedAttempt::Refused(r) => (None, r.as_ref()),
    };
    let g1 = r.certified && r.source_g1;
    let g2 = r.certified && r.source_g2;
    Ok(json!({
        "model":model,"jointContinuousGlobalCertified":joint[0],"jointContinuousGlobalG1Certified":joint[1],"jointContinuousGlobalG2Certified":joint[2],"certified":r.certified,"solidGeometryCertified":r.certified,
        "scope":"original-rational-frames-and-filled-annular-caps",
        "continuousBound":r.certified && r.source_boundary_exact,"boundaryErrorUpper":r.boundary_error_upper,
        "sourceBoundaryExact":r.source_boundary_exact,"topologyPreserved":r.topology_preserved,
        "shellRolesCertified":r.certified,"seamContinuity":r.seam_continuity,
        "fullBoundaryG1Certified":g1,"fullBoundaryG2Certified":g2,
        "idealSourceBoundaryG1Certified":g1,"idealSourceBoundaryG2Certified":g2,
        "physicalCapCount":r.caps.len(),"physicalCapsCertified":r.certified && !r.caps.is_empty(),
        "filledCapErrorUpper":if r.certified && !r.caps.is_empty(){Some(0.)}else{None},
        "sourceWork":{"cells":r.source_cells,"exactWork":r.source_exact_work,"constructionRows":r.construction_rows,"capCells":r.cap_cells,"orientationCells":r.orientation_cells},
        "originalVolume":r.volume.as_ref().map(|p|json!({"embeddingCertified":p.embedding_certified,"volumeRegularityCertified":p.regularity_certified,"parameterTopologyPreserved":p.parameter_topology_preserved,"cells":p.cells,"exactWork":p.exact_work,"boxes":p.boxes,"projection":p.projection,"reason":p.reason})),
        "proofCoordinates":r.proof_coordinates.as_ref().map(|c|json!({"rows":c.rows,"offset":c.offset,"determinant":c.determinant,
            "columnFactor":c.column_factor.as_ref().map(|f|json!({"base":f.base,"dependent":f.dependent,"scale":f.scale,"constant":f.constant}))})),
        "material":r.material.as_ref().map(|p|json!({"valid":p.valid,"winding":p.winding,"pairs":p.pairs,"cells":p.cells,"domainCells":p.domain_cells,"reason":p.reason})),
        "retainedBinding":r.binding.as_ref().map(|p|json!({"certified":p.certified,"inspectedFaces":p.faces,"exactWork":p.exact_work,"reparameterizedFaces":p.reparameterized_faces(),"reason":p.reason})),
        "caps":r.caps.iter().map(|p|json!({"ring":p.ring,"span":p.span,"faces":p.faces,"sourceUDomain":p.source_u_domain,"sourceVBounds":p.source_v_bounds,
            "parameterMonotonicityCertified":p.parameter_monotonicity.spanwise_regular,"cells":p.parameter_monotonicity.cells})).collect::<Vec<_>>(),
        "smoothness":r.smoothness.as_ref().map(|p|json!({"extractionComplete":p.extraction_complete,"vertexLinksComplete":p.vertex_links_complete,"edgeIds":p.edge_ids,"unresolvedEdges":p.unresolved_edges,"exactWork":p.exact_work,
            "edgeCorrespondenceCertified":p.edge_correspondence.as_ref().is_some_and(|p|p.all_equal && p.all_joins_exact),"agreementWork":p.edge_correspondence.as_ref().map_or(0,|p|p.work),
            "allFacesRegular":p.regularity.all_faces_regular,"regularityCells":p.regularity.cells,"fullBoundaryG1Certified":p.g1_certified,"fullBoundaryG2Certified":p.g2_certified})),
        "c0":if r.seam_continuity=="C0" {Some(json!({"allFacesRegular":r.c0_regularity.as_ref().is_some_and(|p|p.all_faces_regular),"edgeCorrespondenceCertified":r.c0_agreement.as_ref().is_some_and(|p|p.all_equal && p.all_joins_exact)}))}else{None},
        "reason":r.reason
    }))
}
