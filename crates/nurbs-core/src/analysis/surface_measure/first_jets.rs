//! First original rational jets with coefficient-owned quotient numerators.
//! Hu*W-H*Wu retains algebraic cancellation lost by Hu-Wu*(H/W).
use crate::numerics::bernstein_tensor::{Net, derivative, evaluate, minus, product};
use crate::{Result, distance_bounds::Interval as I, surface::Surface};
type V3 = [I; 3];
/// Relative position and derivatives in normalized ORIGINAL knot-cell axes.
/// Higher degree cells retain the general stable-jet verifier.
pub(crate) fn calculate(s: &Surface, span: [usize; 2], domain: [[f64; 2]; 2]) -> Result<[V3; 3]> {
    if s.degree_u > 5 || s.degree_v > 5 {
        let j = super::jets::calculate_partial_stable(s, span, domain, [None; 2])?;
        return Ok([j[0][0], j[1][0], j[0][1]]);
    }
    let full = [
        [s.knots_u[span[0]], s.knots_u[span[0] + 1]],
        [s.knots_v[span[1]], s.knots_v[span[1] + 1]],
    ];
    let parameter = [
        I::new(domain[0][0], domain[0][1])?
            .sub(I::point(full[0][0]))?
            .div(I::point(full[0][1]).sub(I::point(full[0][0]))?)?
            .intersect(0., 1.)?,
        I::new(domain[1][0], domain[1][1])?
            .sub(I::point(full[1][0]))?
            .div(I::point(full[1][1]).sub(I::point(full[1][0]))?)?
            .intersect(0., 1.)?,
    ];
    let net = crate::curve_surface_composition::surface_net(s, span)?;
    let origin = &s.control_points[span[0] - s.degree_u][span[1] - s.degree_v];
    let w: Net = net
        .iter()
        .map(|r| r.iter().map(|p| p[3]).collect())
        .collect();
    let dw = [derivative(&w, 0)?, derivative(&w, 1)?];
    let denominator = evaluate(&w, parameter)?;
    let denominator2 = denominator.mul(denominator)?;
    let mut out = [[I::point(0.); 3]; 3];
    for k in 0..3 {
        let h: Net = net
            .iter()
            .map(|r| {
                r.iter()
                    .map(|p| p[k].sub(I::point(origin[k]).mul(p[3])?))
                    .collect()
            })
            .collect::<Result<_>>()?;
        let value = evaluate(&h, parameter)?.div(denominator)?;
        // Positive rational B-spline basis also gives the ORIGINAL active
        // Cartesian convex hull; no rounded extraction pole is a premise.
        let mut lo = f64::INFINITY;
        let mut hi = f64::NEG_INFINITY;
        for u in span[0] - s.degree_u..=span[0] {
            for v in span[1] - s.degree_v..=span[1] {
                let p = I::point(s.control_points[u][v][k]).sub(I::point(origin[k]))?;
                lo = lo.min(p.lo);
                hi = hi.max(p.hi);
            }
        }
        out[0][k] = value.intersect(lo, hi)?;
        for axis in 0..2 {
            let dh = derivative(&h, axis)?;
            let numerator = minus(product(&dh, &w)?, product(&h, &dw[axis])?)?;
            out[axis + 1][k] = evaluate(&numerator, parameter)?.div(denominator2)?;
        }
    }
    Ok(out)
}
