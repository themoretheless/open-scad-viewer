//! Source fields, limits and certificate flags remain Rust-owned. No caller
//! success report or sampled pole normal supplies geometric proof credit.
use crate::{Result, Value, field};
use brep_core::{
    boundary_embedding, face_contacts, source_implicit_body as source, volume_validity,
};
use value_codec::json;

fn proof_limits(l: &Value) -> Result<source::Limits> {
    let a = &l["volume"];
    let b = &a["boundary"];
    let c = &b["contacts"];
    Ok(source::Limits {
        implicit_exact_work: field(l, "implicitExactWork")?,
        implicit_cells: field(l, "implicitCells")?,
        regularity_cells: field(l, "regularityCells")?,
        linear_cells: field(l, "linearCells")?,
        volume: volume_validity::Limits {
            boundary: boundary_embedding::Limits {
                exact_work: field(b, "exactWork")?,
                trim_pairs: field(b, "trimPairs")?,
                trim_cells: field(b, "trimCells")?,
                trim_domain_cells: field(b, "trimDomainCells")?,
                spans: field(b, "spans")?,
                contacts: face_contacts::Limits {
                    pairs: field(c, "pairs")?,
                    cells: field(c, "cells")?,
                    domain_cells: field(c, "domainCells")?,
                    cells_per_pair: field(c, "cellsPerPair")?,
                    domain_cells_per_pair: field(c, "domainCellsPerPair")?,
                },
            },
            nesting_pairs: field(a, "nestingPairs")?,
            nesting_cells: field(a, "nestingCells")?,
            nesting_domain_cells: field(a, "nestingDomainCells")?,
            orientation_cells: field(a, "orientationCells")?,
            orientation_domain_cells: field(a, "orientationDomainCells")?,
            orientation_spans: field(a, "orientationSpans")?,
        },
    })
}

pub fn sphere(v: Value, audit: bool) -> Result<Value> {
    let s = &v["source"];
    let center = field::<[f64; 3]>(s, "center")?;
    let radius = field::<f64>(s, "radius")?;
    let limits = proof_limits(&v["limits"])?;
    let tolerance = field::<f64>(&v, "toleranceUV")?;
    let attempt = if audit {
        source::verify_sphere(&field(&v, "model")?, center, radius, tolerance, limits)?
    } else {
        source::construct_sphere(center, radius, tolerance, limits)?
    };
    let (model, r) = match &attempt {
        source::Attempt::Certified(b) => (Some(b.model()), b.report()),
        source::Attempt::Refused(r) => (None, r.as_ref()),
    };
    Ok(
        json!({"model":model,"certified":r.certified,"solidGeometryCertified":r.certified,
        "scope":"original-implicit-sphere-including-complete-pole-quotients",
        "source":{"kind":"sphere","center":r.original_center,"radius":r.original_radius,"squaredConstantsRounded":false},
        "sourceBoundaryExact":r.source_boundary_exact,"continuousBound":r.certified && r.source_boundary_exact,"boundaryErrorUpper":r.boundary_error_upper,
        "topologyPreserved":r.source_topology_preserved,"fullBoundaryG1Certified":r.source_g1,"fullBoundaryG2Certified":r.source_g2,
        "idealSourceBoundaryG1Certified":r.source_g1,"idealSourceBoundaryG2Certified":r.source_g2,"seamContinuity":if r.source_g2{"G2"}else{"C0"},
        "implicitExactWork":r.implicit_exact_work,"implicitCells":r.implicit_cells,"poleEdges":r.collapsed_edges,"quotientVertexLinksComplete":r.quotient_vertex_links_complete,
        "implicitFaces":r.supports.iter().enumerate().map(|(i,p)|json!({"face":i,"exactMembership":p.exact_membership,"gradientNonzero":p.gradient_nonzero,"regularImplicitSupport":p.regular_implicit_support,"gradientLower":p.gradient_lower,"exactWork":p.exact_work,"cells":p.cells,"reason":p.reason})).collect::<Vec<_>>(),
        "regularity":r.regularity.as_ref().map(|p|json!({"allParameterChartsRegular":p.all_faces_regular,"allChartsRegularModuloPoles":p.all_faces_regular_modulo_poles,"cells":p.cells,"unresolvedFaces":p.unresolved_faces,
            "poleQuotients":p.pole_quotients.iter().enumerate().filter_map(|(i,q)|q.as_ref().map(|q|json!({"face":i,"edge":q.edge,"vertex":q.vertex,"collapsedAxis":q.collapsed_axis,"collapsedEnd":q.collapsed_end,"proven":q.proof.proven,"cells":q.proof.cells}))).collect::<Vec<_>>()})),
        "volume":r.volume.as_ref().map(|p|json!({"proven":p.proven,"boundaryEmbedded":p.boundary.proven,"exactBoundaryAgreement":p.boundary.agreement.all_equal && p.boundary.agreement.all_joins_exact,"absenceOfSelfIntersections":p.boundary.intersections.absence_proven,"allPairsClassified":p.boundary.intersections.pairs.all_pairs_classified,"trimValid":p.boundary.trim.all_valid,"allFacesInjective":p.boundary.intersections.faces.all_faces_injective,
            "faceDiagnostics":p.boundary.intersections.faces.faces.iter().filter(|f|!f.absence_proven()).map(|f|json!({"face":f.face,"reason":f.result.as_ref().map(|q|q.reason)})).collect::<Vec<_>>(),
            "pairDiagnostics":p.boundary.intersections.pairs.pairs.iter().filter(|p|p.boundary.is_none()&&!p.hull_disjoint&&!p.radial_disjoint).map(|p|json!({"faces":p.faces,"reason":p.reason})).collect::<Vec<_>>(),"orientationCells":p.orientation_cells,"orientationDomainCells":p.orientation_domain_cells})),
        "reason":r.reason}),
    )
}

pub fn affine_sphere(v: Value) -> Result<Value> {
    let s = &v["source"];
    let authored = source::AffineSphere {
        center: field(s, "center")?,
        linear: field(s, "linear")?,
        radius: field(s, "radius")?,
    };
    let limits = &v["limits"];
    let attempt = source::verify_affine_sphere(
        &field(&v, "model")?,
        authored,
        field(&v, "toleranceUV")?,
        field(limits, "maxPoints")?,
        proof_limits(limits)?,
    )?;
    let (model, points, reason, normalized) = match &attempt {
        source::AffineSphereAttempt::Certified(body) => (
            Some(body.model()),
            body.mapped_points(),
            None,
            Some(body.normalized_proof()),
        ),
        source::AffineSphereAttempt::Refused {
            reason,
            mapped_points,
            normalized,
        } => (None, *mapped_points, Some(*reason), normalized.as_deref()),
    };
    let certified = model.is_some();
    Ok(json!({
        "model": model, "certified": certified, "solidGeometryCertified": certified,
        "scope": "original-affine-sphere-including-complete-pole-quotients",
        "source": {"kind": "affineSphere", "center": authored.center, "linear": authored.linear, "radius": authored.radius},
        "sourceBoundaryExact": certified, "continuousBound": certified,
        "boundaryErrorUpper": certified.then_some(0.), "topologyPreserved": certified,
        "fullBoundaryG1Certified": certified, "fullBoundaryG2Certified": certified,
        "idealSourceBoundaryG1Certified": certified, "idealSourceBoundaryG2Certified": certified,
        "mappedPoints": points, "reason": reason,
        "normalizedProof": normalized.map(|report| json!({
            "reason": report.reason, "implicitExactWork": report.implicit_exact_work,
            "implicitCells": report.implicit_cells, "poleEdges": report.collapsed_edges,
            "quotientVertexLinksComplete": report.quotient_vertex_links_complete
        }))
    }))
}
