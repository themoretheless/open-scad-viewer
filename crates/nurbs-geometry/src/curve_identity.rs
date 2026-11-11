//! Exact identity of actual binary64 curve definitions or denominator laws.
use crate::{Result,curve::Curve};
use cad_predicates::{AuthoredScalar,Limits,PredicateContext,SourceArena,ToleranceContext};
/// Compare complete parameter laws, or the literal scalar denominator laws
/// encoded as [stored_weight,0,0,1]. No rounded homogeneous products are used.
pub fn compare_definitions(source:&Curve,candidate:&Curve,knot:f64,max_work:u64,denominators:bool)->Result<cad_predicates::BezierIdentityDecision> {
    crate::check(source.domain()==candidate.domain() && source.control_points[0].len()==candidate.control_points[0].len(),"Literal curve identity requires equal domains and dimensions")?;
    let p=source.degree;let n=source.control_points.len();
    let mut values=Vec::new();
    let mut offsets=Vec::new();
    for curve in [source,candidate] {
        let controls=values.len();
        for (point,&w) in curve.control_points.iter().zip(&curve.weights) {
            values.extend(if denominators {[w,0.,0.,1.]}else{[point[0],point[1],*point.get(2).unwrap_or(&0.),w]});
        }
        let knots=values.len();values.extend(&curve.knots);offsets.push((controls,knots));
    }
    let translation=values.len();values.extend([0.;3]);
    values.extend([p as f64,source.control_points[0].len() as f64,n as f64,candidate.control_points.len() as f64,candidate.degree as f64,knot]);
    // These are literal source and proposed output definitions. We do not
    // relabel a rounded construction as an exact original-source operation:
    // the full identity is proved independently for the ACTUAL output bits.
    let arena=SourceArena::authored("literal-curve-definitions",1,values.into_iter().map(|x|AuthoredScalar::Binary64Bits(x.to_bits())).collect()).map_err(|e|crate::input(e.to_string()))?;
    let tolerance=ToleranceContext::default_valid();
    let mut ctx=PredicateContext::new(&arena,&tolerance,Limits{max_work,..Limits::default()},None);
    let controls = |offset: usize, count: usize| {
        (0..count)
            .map(|i| crate::leaves_at(&arena, offset + 4 * i))
            .collect::<crate::Result<Vec<_>>>()
    };
    let knots = |offset: usize, count: usize| {
        (0..count)
            .map(|i| arena.leaf(offset + i).map_err(|e| crate::input(e.to_string())))
            .collect::<crate::Result<Vec<_>>>()
    };
    let a = controls(offsets[0].0, n)?;
    let ak = knots(offsets[0].1, source.knots.len())?;
    let b = controls(offsets[1].0, candidate.control_points.len())?;
    let bk = knots(offsets[1].1, candidate.knots.len())?;
    let t = crate::leaves_at(&arena, translation)?;
    let decision=cad_predicates::rational_bspline_restriction_identity(&mut ctx,p,&a,&ak,candidate.degree,&b,&bk,t).map_err(|e|crate::input(e.to_string()))?;
    Ok(decision)
}
