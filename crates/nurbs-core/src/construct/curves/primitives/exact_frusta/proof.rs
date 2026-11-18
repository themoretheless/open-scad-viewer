//! Full literal tensor binding, including the single collapsed cone end.
use super::super::exact_frame::{cross, dot};
use super::{PResult, Q, Refusal, Surface, Template, V_KNOTS, Work};
use num_traits::Zero;
pub(super) fn verify(
    center: [f64; 3],
    axes: [[f64; 3]; 3],
    radii: [f64; 2],
    reference: &Template,
    s: &Surface,
    w: &mut Work,
) -> PResult<()> {
    let c = reference.circle();
    if s.degree_u != 2
        || s.degree_v != 1
        || s.periodic_u
        || s.periodic_v
        || s.knots_u != c.knots
        || s.knots_v != V_KNOTS
        || s.control_points.len() != 9
        || s.weights.len() != 9
        || s.control_points
            .iter()
            .any(|r| r.len() != 2 || r.iter().any(|p| p.len() != 3))
        || s.weights
            .iter()
            .any(|r| r.len() != 2 || r.iter().any(|&x| !x.is_finite() || x <= 0.))
    {
        return Err(Refusal::UnprovedIdentity);
    }
    let radii = radii
        .into_iter()
        .map(|x| w.lift(x))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    if radii.iter().any(|x| *x < Q::zero()) || radii.iter().all(|x| x.is_zero()) {
        return Err(Refusal::UnprovedIdentity);
    }
    let center = center
        .into_iter()
        .map(|x| w.lift(x))
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let axes = axes
        .into_iter()
        .map(|a| {
            a.into_iter()
                .map(|x| w.lift(x))
                .collect::<std::result::Result<Vec<_>, _>>()
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let cofactors = [
        cross(&axes[1], &axes[2], w)?,
        cross(&axes[2], &axes[0], w)?,
        cross(&axes[0], &axes[1], w)?,
    ];
    let det = dot(&axes[0], &cofactors[0], w)?;
    if det.is_zero() {
        return Err(Refusal::DependentAxes);
    }
    let base = w.lift(c.weights[0])?;
    let first = w.lift(s.weights[0][0])?;
    let gauge = w.op(&first, &base, |a, b| a / b)?;
    for i in 0..9 {
        for j in 0..2 {
            let weight = w.lift(s.weights[i][j])?;
            let expected = w.lift(c.weights[i])?;
            if w.op(&expected, &gauge, |a, b| a * b)? != weight {
                return Err(Refusal::UnprovedIdentity);
            }
            let mut t = Vec::new();
            for axis in 0..3 {
                let p = w.lift(s.control_points[i][j][axis])?;
                let h = w.op(&p, &weight, |a, b| a * b)?;
                let o = w.op(&center[axis], &weight, |a, b| a * b)?;
                t.push(w.op(&h, &o, |a, b| a - b)?);
            }
            let x = w.lift(c.control_points[i][0])?;
            let y = w.lift(c.control_points[i][1])?;
            let local = [
                w.op(&x, &radii[j], |a, b| a * b)?,
                w.op(&y, &radii[j], |a, b| a * b)?,
                w.lift(j as f64)?,
            ];
            let d = w.op(&det, &weight, |a, b| a * b)?;
            for axis in 0..3 {
                let actual = dot(&cofactors[axis], &t, w)?;
                if actual != w.op(&d, &local[axis], |a, b| a * b)? {
                    return Err(Refusal::UnprovedIdentity);
                }
            }
        }
    }
    // Actual tensor is one global gauge times X=X_circle*r(v),
    // Y=Y_circle*r(v), Z=W_circle*v, D=W_circle. Thus all four cells
    // satisfy X²+Y²=[r0*D+(r1-r0)*Z]², with the original v clock.
    // Nonnegative endpoints, not both zero, imply r(v)>0 on the open
    // interval. Recover v from Z/D and longitude from (X,Y)/r(v).
    // Fresh circle traversal therefore proves full injective side coverage
    // modulo the longitude seam and the ONE allowed zero-radius endpoint.
    // The cone apex is intrinsically nonsmooth, unlike an ellipsoid's smooth
    // image at a singular parameter pole. No caps/B-rep/solid are inferred.
    Ok(())
}
