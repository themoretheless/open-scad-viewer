//! Read-only contacts from original affine-patch expressions, including multiplicity.
use crate::{Result, Value, field, input, require_exact_fields};
use cad_predicates::{MAX_WORK, OriginalPlaneRootsOutcome};
use nurbs_core::{
    curve::Curve,
    intersection::certified_curve_surface::{OriginalPatchUvProof, original_affine_patch_contacts},
    surface::Surface,
};
use value_codec::json;
pub(crate) fn run(v: Value) -> Result<Value> {
    require_exact_fields(
        &v,
        &[
            "op",
            "curve",
            "patch",
            "tolerance",
            "maxSpans",
            "maxExactWork",
        ],
        "original affine patch contacts",
    )?;
    let c: Curve = field(&v, "curve")?;
    let s: Surface = field(&v, "patch")?;
    let tolerance: f64 = field(&v, "tolerance")?;
    let max_spans: usize = field(&v, "maxSpans")?;
    let max_exact_work: u64 = field(&v, "maxExactWork")?;
    if max_exact_work > MAX_WORK {
        return Err(input("Exact contact budget exceeds predicate MAX_WORK"));
    }
    let r = original_affine_patch_contacts(&c, &s, tolerance, max_spans, max_exact_work)?;
    let contacts:Vec<_>=r.contacts.iter().map(|c|json!({"method":"original-patch-plane-polynomial","uvMethod":match c.uv_proof {OriginalPatchUvProof::SeparableProjective=>"separable-projective",OriginalPatchUvProof::InjectiveBilinearKrawczyk=>"injective-bilinear-krawczyk",OriginalPatchUvProof::PositiveBilinearGlobalInverse=>"positive-bilinear-global-inverse",OriginalPatchUvProof::ExactBoundaryProjective=>"exact-boundary-projective",OriginalPatchUvProof::OriginalAffineCoordinateBounds=>"original-affine-coordinate-bounds"},"parameterBox":c.parameter_box,"positionBounds":c.position_bounds,"sourceSpans":c.source_spans,"multiplicities":c.multiplicities})).collect();
    let proofs:Vec<_>=r.scalar_proofs.iter().map(|(span,p)|{
  let (status,roots,reason)=match &p.outcome{
   OriginalPlaneRootsOutcome::Roots(roots)=>("roots",roots.iter().map(|r|json!({"interval":r.interval,"multiplicity":r.multiplicity})).collect::<Vec<_>>(),None),
   OriginalPlaneRootsOutcome::Coincident=>("coincident",vec![],None),
   OriginalPlaneRootsOutcome::Indeterminate(reason)=>("indeterminate",vec![],Some(format!("{reason:?}"))),
  };
  json!({"span":span,"status":status,"normalizedOriginalSpanRoots":roots,"reason":reason,"workUsed":p.work_used})
 }).collect();
    let target_injective =
        r.target_weights_separable || r.target_uv_injectivity.as_ref().is_some_and(|p| p.proven);
    let presence = !contacts.is_empty();
    let absence = r.complete && !presence;
    Ok(
        json!({"schema":"nurbs-original-affine-patch-contacts/1","status":if presence{"present"}else if absence{"absent-on-patch"}else{"unresolved"},
         "presenceProven":presence,"patchAbsenceProven":absence,"curvePatchCoverageComplete":r.complete,
         "parameterCoordinates":["curve-natural-t","patch-natural-u","patch-natural-v"],"contacts":contacts,
         "targetWeightsSeparable":r.target_weights_separable,"targetUvInjectivityProven":target_injective,"scalarProofs":proofs,"proofContextSerialized":false,"unresolvedSpans":r.unresolved_spans,"ambiguousParameters":r.ambiguous_parameters,
         "spans":r.spans,"maxSpans":max_spans,"exactWork":r.exact_work,"maxExactWork":max_exact_work,
         "allSurfaceBranchesEnumerated":false,"trimAdmission":false,"bodyAdmission":false
        }),
    )
}
