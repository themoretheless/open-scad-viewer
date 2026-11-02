//! Read-only original curve/quadric carrier roots; no finite surface admission.
use crate::{Result, Value, field, input, require_exact_fields};
use cad_predicates::{MAX_WORK, OriginalQuadricRootsOutcome};
use nurbs_core::{
    curve::Curve, intersection::certified_curve_surface::original_quadric_carrier_roots,
};
use value_codec::json;
pub(crate) fn run(v: Value) -> Result<Value> {
    require_exact_fields(
        &v,
        &[
            "op",
            "curve",
            "quadric",
            "constantProducts",
            "tolerance",
            "maxSpans",
            "maxExactWork",
        ],
        "original quadric carrier roots",
    )?;
    let curve: Curve = field(&v, "curve")?;
    let quadric: [[f64; 4]; 4] = field(&v, "quadric")?;
    let products: Vec<[f64; 2]> = field(&v, "constantProducts")?;
    let tolerance: f64 = field(&v, "tolerance")?;
    let max_spans: usize = field(&v, "maxSpans")?;
    let max_work: u64 = field(&v, "maxExactWork")?;
    if max_work > MAX_WORK {
        return Err(input("Exact quadric budget exceeds predicate MAX_WORK"));
    }
    let r =
        original_quadric_carrier_roots(&curve, quadric, &products, tolerance, max_spans, max_work)?;
    let roots:Vec<_>=r.roots.iter().map(|p|json!({"method":"original-quadric-polynomial","parameter":p.parameter,"positionBounds":p.position_bounds,"sourceSpans":p.source_spans,"multiplicities":p.multiplicities})).collect();
    let proofs:Vec<_>=r.scalar_proofs.iter().map(|(span,p)| {
        let (status,roots,reason)=match &p.outcome {
            OriginalQuadricRootsOutcome::Roots(roots)=>("roots",roots.iter().map(|r|json!({"interval":r.interval,"multiplicity":r.multiplicity})).collect::<Vec<_>>(),None),
            OriginalQuadricRootsOutcome::Coincident=>("coincident",vec![],None),
            OriginalQuadricRootsOutcome::Indeterminate(reason)=>("indeterminate",vec![],Some(format!("{reason:?}"))),
        };
        json!({"span":span,"status":status,"normalizedOriginalSpanRoots":roots,"reason":reason,"workUsed":p.work_used})
    }).collect();
    let presence = !roots.is_empty();
    let absence = r.complete && !presence;
    Ok(
        json!({"schema":"nurbs-original-quadric-carrier-roots/1","status":if presence{"present-on-carrier"}else if absence{"absent-on-carrier"}else{"unresolved"},
        "presenceProven":presence,"carrierAbsenceProven":absence,"curveCarrierCoverageComplete":r.complete,
        "parameterCoordinates":["curve-natural-t"],"roots":roots,"scalarProofs":proofs,
        "unresolvedSpans":r.unresolved_spans,"spans":r.spans,"maxSpans":max_spans,"exactWork":r.exact_work,"maxExactWork":max_work,
        "proofContextSerialized":false,"finiteSurfaceUvAdmission":false,"allSurfaceBranchesEnumerated":false,"trimAdmission":false,"bodyAdmission":false}),
    )
}
