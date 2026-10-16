//! Transport only. Every bound and admission decision is computed by Rust.
use crate::{Result, Value, field};
use brep_core::{boundary_certificate as certificate, volume_validity};
use value_codec::json;
pub(super) fn volume_limits(v: &Value) -> Result<volume_validity::Limits> {
    Ok(volume_validity::Limits {
        boundary: brep_core::boundary_embedding::Limits {
            exact_work: field(v, "maxExactWork")?,
            trim_pairs: field(v, "maxTrimPairs")?,
            trim_cells: field(v, "maxTrimCells")?,
            trim_domain_cells: field(v, "maxTrimDomainCells")?,
            spans: field(v, "maxSpans")?,
            contacts: brep_core::face_contacts::Limits {
                pairs: field(v, "maxPairs")?,
                cells: field(v, "maxCells")?,
                domain_cells: field(v, "maxDomainCells")?,
                cells_per_pair: field(v, "cellsPerPair")?,
                domain_cells_per_pair: field(v, "domainCellsPerPair")?,
            },
        },
        nesting_pairs: field(v, "nestingPairs")?,
        nesting_cells: field(v, "nestingCells")?,
        nesting_domain_cells: field(v, "nestingDomainCells")?,
        orientation_cells: field(v, "orientationCells")?,
        orientation_domain_cells: field(v, "orientationDomainCells")?,
        orientation_spans: field(v, "orientationSpans")?,
    })
}
fn volume(r: &Option<volume_validity::Report>) -> Value {
    r.as_ref().map_or(Value::Null, |r| {
        json!({"proven":r.proven,
        "exactBoundary":r.boundary.agreement.all_equal&&r.boundary.agreement.all_joins_exact,
        "trimRegionsValid":r.boundary.trim.all_valid,
        "allFacesInjective":r.boundary.intersections.faces.all_faces_injective,
        "allPairsClassified":r.boundary.intersections.pairs.all_pairs_classified,
        "nextPair":r.boundary.intersections.pairs.next_pair,
        "nestingConsistent":r.nesting.as_ref().and_then(|n|n.roles_consistent),
        "orientations":r.orientations.iter().map(|o|json!({"shell":o.shell,"outward":o.outward,
            "expectedOutward":o.expected_outward})).collect::<Vec<_>>()})
    })
}
fn regularity(r: &Option<brep_core::boundary_embedding::regularity::Report>) -> Value {
    r.as_ref().map_or(Value::Null,|r|json!({"proven":r.all_faces_regular_modulo_poles,
        "ordinaryRegularity":r.all_faces_regular,"cells":r.cells,"unresolvedFaces":r.unresolved_faces}))
}
pub fn inspect(v: Value) -> Result<Value> {
    let source = field(&v, "reference")?;
    let retained = field(&v, "retained")?;
    let limits = field::<Value>(&v, "limits")?;
    let r = certificate::inspect(
        &source,
        &retained,
        field(&v, "toleranceUv")?,
        field(&v, "maxDeviation")?,
        certificate::Limits {
            correspondence: nurbs_core::surface_correspondence::Limits {
                cells: field(&limits, "correspondenceCells")?,
                products: field(&limits, "correspondenceProducts")?,
            },
            regularity_cells: field(&limits, "regularityCells")?,
            linear_cells_per_boundary: field(&limits, "linearCellsPerBoundary")?,
            reference_volume: volume_limits(&limits["referenceVolume"])?,
            retained_volume: volume_limits(&limits["retainedVolume"])?,
        },
    )?;
    Ok(
        json!({"certified":r.certified,"continuousBound":r.continuous_bound,"withinBudget":r.within_budget,
        "errorUpper":r.error_upper,"scope":"authored-rational-boundary-common-uv",
        "sourceMotionLawCertified":false,"smoothnessCertified":false,"seamContinuity":"C0",
        "correspondenceCells":r.correspondence_cells,"correspondenceProducts":r.correspondence_products,
        "faces":r.face_bounds.iter().enumerate().map(|(face,b)|json!({"face":face,
            "errorUpper":b.error_upper,"withinBudget":b.within_budget,"cells":b.cells,
            "products":b.products,"reason":b.reason})).collect::<Vec<_>>(),
        "referenceRegularity":regularity(&r.reference_regularity),"retainedRegularity":regularity(&r.retained_regularity),
        "referenceVolume":volume(&r.reference_volume),"retainedVolume":volume(&r.retained_volume),"reason":r.reason}),
    )
}
