//! Read-only original-source audit. Reports do not authorize body edits.
use crate::{Result, Value, field, require_exact_fields};
use nurbs_core::{
    surface::Surface,
    surface_self_intersection::{self as audit, Status},
};
use value_codec::json;

pub(crate) fn run(v: Value) -> Result<Value> {
    require_exact_fields(
        &v,
        &["op", "surface", "maxSpans"],
        "surface self-intersection audit",
    )?;
    let surface: Surface = field(&v, "surface")?;
    let max_spans: usize = field(&v, "maxSpans")?;
    let report = audit::inspect(&surface, max_spans)?;
    let exact =
        report.exact_witness.as_ref().map(|pair| {
            pair.iter()
                .map(|uv| {
                    uv.iter().map(|x| json!({
            "numerator":x.numer().to_string(),"denominator":x.denom().to_string()
        })).collect::<Vec<_>>()
                })
                .collect::<Vec<_>>()
        });
    let status = match report.status {
        Status::Absent => "absent",
        Status::Present => "present",
        Status::Unresolved => "unresolved",
    };
    let crossing = report.certified_crossing.as_ref().map(|c| {
        json!({
            "method":"original-planar-krawczyk",
            "parameterBoxes":c.parameter_boxes,"positionBounds":c.position_bounds,"cells":c.cells
        })
    });
    Ok(json!({
        "schema":"nurbs-surface-self-intersection/1",
        "status":status,
        "absenceProven":report.status==Status::Absent,
        "presenceProven":report.status==Status::Present,
        "allContactBranchesEnumerated":false,
        "witness":report.witness,"exactWitness":exact,
        "certifiedCrossing":crossing,
        "witnessCoordinates":"normalized-original-active-domain",
        "reflectedAxis":report.reflected_axis,
        "injectivity":{"proven":report.injectivity.proven,
            "spans":report.injectivity.spans,"reason":report.injectivity.reason},
        "maxSpans":max_spans
    }))
}
