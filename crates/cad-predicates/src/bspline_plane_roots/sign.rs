//! Exact sign of one authored plane equation at another plane's unique root.
use super::*;
#[derive(Clone, Debug)]
pub struct OriginalPlaneRootSignDecision {
    pub sign: Result<Sign, Reason>,
    pub work_used: u64,
    pub context: ContextIdentity,
}
/// Sign of n₂·C(t)-d₂ at the unique n₁·C(t)-d₁ root in a closed normalized
/// original-span box. Controls, knots and both plane equations use authored
/// binary64 leaves. Positive source weights make the numerator sign equal to
/// the rational plane residual sign. No rounded evaluation at an approximate
/// root is used. Nonunique boxes and exhausted resources refuse proof.
pub fn rational_bspline_plane_root_sign(
    ctx: &mut PredicateContext<'_>,
    controls: &[[LeafRef; 4]],
    knots: &[LeafRef],
    degree: usize,
    span: usize,
    planes: [[LeafRef; 4]; 2],
    interval: [f64; 2],
) -> Result<OriginalPlaneRootSignDecision, InputError> {
    let values = super::common::prepare(ctx, controls, knots, degree, span, planes, interval)?;
    let sign = (|| {
        let p =
            super::common::polynomials(ctx, &values, controls.len(), knots.len(), degree, span)?;
        let sign = crate::planar_bezier_roots::unique_root_sign(&p[0], &p[1], interval, ctx)?;
        ctx.charge(0)?;
        Ok(sign)
    })();
    Ok(OriginalPlaneRootSignDecision {
        sign,
        work_used: ctx.work_used(),
        context: ctx.identity(),
    })
}
