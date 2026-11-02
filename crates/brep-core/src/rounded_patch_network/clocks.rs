//! Fixed-control rational Bezier clock proposals. Geometry authority always
//! comes from the caller's fresh complete actual world/UV lift proof.
use super::*;
use num_rational::BigRational as Q;
use num_traits::ToPrimitive;
fn q(v: f64) -> Q {
    Q::from_float(v).unwrap()
}
/// If alpha_i = gamma beta_i lambda^i, substitution by
/// t -> lambda t/(1-t+lambda t) gives identical rational Bezier images.
/// The first two positive weight ratios propose lambda for any degree. Neither
/// endpoint weights nor pole coincidence alone admit the resulting whole use.
pub(crate) fn prepared_clock(
    c: &Curve,
    reference: &Curve,
    pc: &Curve,
    reversed: bool,
) -> Result<Curve> {
    if c.degree != reference.degree
        || pc.degree != 1
        || pc.control_points.len() != 2
        || pc.weights[0] != pc.weights[1]
        || !clamped_single(&c.knots, c.degree, c.control_points.len())
        || !clamped_single(
            &reference.knots,
            reference.degree,
            reference.control_points.len(),
        )
    {
        return Ok(pc.clone());
    }
    let n = c.control_points.len();
    let alpha = if reversed {
        [c.weights[n - 1], c.weights[n - 2]]
    } else {
        [c.weights[0], c.weights[1]]
    };
    let weights = [
        q(reference.weights[1]) * q(alpha[0]),
        q(reference.weights[0]) * q(alpha[1]),
    ];
    let scale = weights[0].clone().max(weights[1].clone());
    let mut proposal = pc.clone();
    proposal.weights = weights
        .iter()
        .map(|w| {
            (w / &scale)
                .to_f64()
                .ok_or_else(|| refuse("BREP_PATCH_NETWORK_UNPROVEN", "UV clock proposal overflow"))
        })
        .collect::<Result<Vec<_>>>()?;
    proposal.validate()?;
    Ok(proposal)
}
/// Only selects the prepared proof route, never declares boundary ownership.
/// Exact proportional gauges with identical poles keep the original exact path.
pub(crate) fn needs_prepared(a: &Curve, b: &Curve, reversed: bool) -> bool {
    if a.degree != b.degree || a.control_points.len() != b.control_points.len() {
        return true;
    }
    let n = a.control_points.len();
    let index = |i| if reversed { n - 1 - i } else { i };
    if (0..n).any(|i| a.control_points[index(i)] != b.control_points[i]) {
        return true;
    }
    let alpha = q(a.weights[index(0)]);
    let beta = q(b.weights[0]);
    (0..n).any(|i| q(a.weights[index(i)]) * &beta != q(b.weights[i]) * &alpha)
}
