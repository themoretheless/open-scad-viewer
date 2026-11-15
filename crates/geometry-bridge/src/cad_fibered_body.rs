//! Transport for the native original-volume owner. No caller certificate credit.
use crate::{Result, Value, field};
use brep_core::{FiberedBodyAttempt, FiberedBodyLimits, LoftCap};
use nurbs_core::{affine_volume::RationalPiece, curve::Curve, fibered_volume};
use value_codec::json;
pub fn run(v: Value, audit: bool) -> Result<Value> {
    let s = &v["source"];
    let profiles = field::<Vec<Vec<Curve>>>(s, "profiles")?;
    let origin = field::<[f64; 3]>(s, "origin")?;
    let motion = field::<Vec<Value>>(s, "motion")?
        .iter()
        .map(|p| {
            Ok(RationalPiece {
                controls: field(p, "controls")?,
                weights: field(p, "weights")?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    let material = LoftCap {
        surface: field(&v["material"], "surface")?,
        trims: field(&v["material"], "trims")?,
    };
    let proposal = fibered_volume::Proposal {
        source_normal: field(&v["proposal"], "sourceNormal")?,
        world_rows: field(&v["proposal"], "worldRows")?,
    };
    let l = &v["limits"];
    let limits = FiberedBodyLimits {
        volume: fibered_volume::Limits {
            cells: field(&l["volume"], "cells")?,
            exact_work: field(&l["volume"], "exactWork")?,
        },
        material_pairs: field(l, "materialPairs")?,
        material_cells: field(l, "materialCells")?,
        material_domain_cells: field(l, "materialDomainCells")?,
        profile_cells: field(l, "profileCells")?,
        source_trace_work: field(l, "sourceTraceWork")?,
        binding_rows: field(l, "bindingRows")?,
        agreement_work: field(l, "agreementWork")?,
        regularity_cells: field(l, "regularityCells")?,
        normal_cells: field(l, "normalCells")?,
        smoothness_work: field(l, "smoothnessWork")?,
        order: field(l, "order")?,
    };
    let tolerance = field::<f64>(&v, "toleranceUV")?;
    let attempt = if audit {
        brep_core::verify_fibered_body(
            brep_core::FiberedSource {
                profiles: &profiles,
                material: &material,
                origin,
                motion: &motion,
            },
            proposal,
            tolerance,
            limits,
            &field(&v, "model")?,
        )?
    } else {
        brep_core::construct_fibered_body(
            brep_core::FiberedSource {
                profiles: &profiles,
                material: &material,
                origin,
                motion: &motion,
            },
            proposal,
            tolerance,
            limits,
        )?
    };
    let (model, r) = match &attempt {
        FiberedBodyAttempt::Certified(b) => (Some(b.model()), b.report()),
        FiberedBodyAttempt::Refused(r) => (None, r.as_ref()),
    };
    Ok(json!({
        "model":model,"certified":r.certified,"solidGeometryCertified":r.certified,
        "scope":"original-rational-affine-fibers-parallel-height-planes-and-filled-planar-caps",
        "sourceBoundaryExact":r.certified && r.boundary.source_exact,
        "continuousBound":r.certified && r.boundary.source_exact,"boundaryErrorUpper":r.boundary.error_upper,
        "topologyPreserved":r.certified && r.boundary.topology_preserved,
        "absenceOfSelfIntersections":r.certified && r.boundary.absence_of_self_intersections,
        "fullBoundaryG1Certified":r.certified && r.smoothness.source_g1,"fullBoundaryG2Certified":r.certified && r.smoothness.source_g2,
        "idealSourceBoundaryG1Certified":r.certified && r.smoothness.source_g1,"idealSourceBoundaryG2Certified":r.certified && r.smoothness.source_g2,
        "seamContinuity":r.smoothness.seam_continuity,"vertexLinksComplete":r.boundary.vertex_links_complete,
        "originalVolume":r.audits.volume.as_ref().map(|p|json!({"embeddingCertified":p.embedding_certified,
            "parameterTopologyPreserved":p.parameter_topology_preserved,"spanwiseInteriorRegular":p.spanwise_interior_regular,
            "knownSingularParameterEnds":p.known_singular_parameter_ends,"heightDirection":p.height_direction,
            "endpointHeightFlatOrders":p.endpoint_height_flat_orders,"cells":p.cells,"exactWork":p.exact_work,
            "frames":p.frames.iter().map(|f|json!({"piece":f.piece,"cells":f.cells,"restrictedDeterminantAbsLower":f.restricted_determinant_abs_lower})).collect::<Vec<_>>(),"reason":p.reason})),
        "material":r.audits.material.as_ref().map(|p|json!({"valid":p.valid,"winding":p.winding,"pairs":p.pairs,"cells":p.cells,"domainCells":p.domain_cells,"reason":p.reason})),
        "actualBoundaryRegular":r.certified && r.audits.regularity.as_ref().is_some_and(|p|p.all_faces_regular),
        "exactBoundaryAgreement":r.certified && r.audits.agreement.as_ref().is_some_and(|p|p.all_equal && p.all_joins_exact),
        "work":{"profileCells":r.work.profile_cells,"sourceTraceWork":r.work.source_trace_work,"bindingRows":r.work.binding_rows,
            "agreementWork":r.audits.agreement.as_ref().map_or(0,|p|p.work),"regularityCells":r.audits.regularity.as_ref().map_or(0,|p|p.cells),
            "normalCells":r.work.normal_cells,"smoothnessWork":r.work.smoothness_work},
        "inspectedProfileJoins":r.work.inspected_profile_joins,"inspectedMotionJoins":r.work.inspected_motion_joins,"reason":r.reason
    }))
}
