//! Finite graph contacts, explicit target/curve/boundary work and opaque proofs.
use crate::{Result, Value, field, input, require_exact_fields};
use cad_predicates::{BezierIdentity, MAX_WORK, Sign};
use nurbs_core::{
    curve::Curve,
    intersection::certified_curve_surface::{
        OriginalQuadricGraphParameterization, OriginalQuadricGraphUvProof,
        original_quadric_graph_contacts,
    },
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
            "quadric",
            "constantProducts",
            "tolerance",
            "maxSpans",
            "maxExactWork",
        ],
        "original quadric graph contacts",
    )?;
    let c: Curve = field(&v, "curve")?;
    let s: Surface = field(&v, "patch")?;
    let q: [[f64; 4]; 4] = field(&v, "quadric")?;
    let products: Vec<[f64; 2]> = field(&v, "constantProducts")?;
    let tolerance: f64 = field(&v, "tolerance")?;
    let max_spans: usize = field(&v, "maxSpans")?;
    let max_work: u64 = field(&v, "maxExactWork")?;
    if max_work > MAX_WORK {
        return Err(input("Quadric graph predicate budget exceeds MAX_WORK"));
    }
    let r = original_quadric_graph_contacts(&c, &s, q, &products, tolerance, max_spans, max_work)?;
    let (status, reason) = match r.target_membership.outcome {
        BezierIdentity::Equal => ("equal", None),
        BezierIdentity::Different => ("different", None),
        BezierIdentity::Indeterminate(reason) => ("indeterminate", Some(format!("{reason:?}"))),
    };
    let target_work = r.target_membership.work_used;
    let carrier = r
        .carrier
        .as_ref()
        .map(|c| super::quadric_carrier::encode_report(c, max_spans, max_work - target_work));
    let contacts:Vec<_>=r.contacts.iter().map(|c|json!({"method":"original-quadric-graph","uvMethod":match c.uv_proof{OriginalQuadricGraphUvProof::ExactSingleton=>"exact-singleton",OriginalQuadricGraphUvProof::OriginalAffineControlBounds=>"original-affine-control-bounds",OriginalQuadricGraphUvProof::QuadricCoordinateSign=>"quadric-coordinate-sign"},"parameterBox":c.parameter_box,"positionBounds":c.position_bounds,"sourceSpans":c.source_spans,"multiplicities":c.multiplicities,"normalizedOriginalSpanParameters":c.normalized_original_span_parameters})).collect();
    let boundary:Vec<_>=r.boundary_proofs.iter().map(|p|{
        let (sign,reason)=match p.decision.sign{Ok(s)=>(Some(match s{Sign::Positive=>1,Sign::Negative=>-1,Sign::Zero=>0}),None),Err(r)=>(None,Some(format!("{r:?}")))};
        json!({"span":p.span,"normalizedRoot":p.normalized_root,"edge":p.edge,"coordinateSign":sign,"reason":reason,"workUsed":p.work_delta})
    }).collect();
    let presence = !contacts.is_empty();
    let absence = r.complete && !presence;
    Ok(
        json!({"schema":"nurbs-original-quadric-graph-contacts/1","status":if presence{"present-on-patch"}else if absence{"absent-on-patch"}else{"unresolved"},
        "presenceProven":presence,"patchAbsenceProven":absence,"curvePatchCoverageComplete":r.complete,"targetGraphProven":r.target_graph_proven,"targetParameterization":match r.target_parameterization{OriginalQuadricGraphParameterization::PolynomialAffine=>"polynomial-affine",OriginalQuadricGraphParameterization::SeparableProjective=>"separable-projective",OriginalQuadricGraphParameterization::SeparableRational=>"separable-rational",OriginalQuadricGraphParameterization::CoupledBilinear=>"coupled-bilinear",OriginalQuadricGraphParameterization::CoupledBernstein=>"coupled-bernstein",OriginalQuadricGraphParameterization::HomogeneousAffine=>"homogeneous-affine",OriginalQuadricGraphParameterization::HomogeneousSeparable=>"homogeneous-separable",OriginalQuadricGraphParameterization::TriangularRational=>"triangular-rational"},
        "targetMembership":{"status":status,"reason":reason,"workUsed":target_work},"carrier":carrier,
        "contacts":contacts,"ambiguousParameters":r.ambiguous_parameters,"parameterCoordinates":["curve-natural-t","patch-natural-u","patch-natural-v"],
        "boundaryProofs":boundary,"boundaryExactWork":r.boundary_exact_work,"exactWork":r.exact_work,"maxExactWork":max_work,"maxSpans":max_spans,
        "proofContextSerialized":false,"finiteSurfaceUvAdmission":presence,"allSurfaceBranchesEnumerated":false,"trimAdmission":false,"bodyAdmission":false}),
    )
}
