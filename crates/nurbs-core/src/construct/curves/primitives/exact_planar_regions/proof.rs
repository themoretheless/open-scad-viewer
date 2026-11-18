//! Literal plane and exact-minor coordinates bind the whole radial tensor.
use super::super::exact_frame::{cross, dot};
use super::{PResult, Q, Refusal, Surface, Template, V_KNOTS, Work};
use num_traits::Zero;
pub(super) fn verify(
    center: [f64; 3],
    axes: [[f64; 3]; 2],
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
    if radii[0] < Q::zero() || radii[1] <= radii[0] {
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
    let normal = cross(&axes[0], &axes[1], w)?;
    let Some((k, (a, b))) = [(1, 2), (2, 0), (0, 1)]
        .into_iter()
        .enumerate()
        .find(|(k, _)| !normal[*k].is_zero())
    else {
        return Err(Refusal::DependentAxes);
    };
    let determinant = &normal[k];
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
            if !dot(&normal, &t, w)?.is_zero() {
                return Err(Refusal::UnprovedIdentity);
            }
            let l = w.op(&axes[1][b], &t[a], |a, b| a * b)?;
            let r = w.op(&axes[1][a], &t[b], |a, b| a * b)?;
            let x = w.op(&l, &r, |a, b| a - b)?;
            let l = w.op(&axes[0][a], &t[b], |a, b| a * b)?;
            let r = w.op(&axes[0][b], &t[a], |a, b| a * b)?;
            let y = w.op(&l, &r, |a, b| a - b)?;
            let x0 = w.lift(c.control_points[i][0])?;
            let y0 = w.lift(c.control_points[i][1])?;
            let local = [
                w.op(&x0, &radii[j], |a, b| a * b)?,
                w.op(&y0, &radii[j], |a, b| a * b)?,
            ];
            let d = w.op(determinant, &weight, |a, b| a * b)?;
            for (actual, value) in [&x, &y].into_iter().zip(&local) {
                if *actual != w.op(&d, value, |a, b| a * b)? {
                    return Err(Refusal::UnprovedIdentity);
                }
            }
        }
    }
    // Whole actual tensor is one global gauge of X=X_circle*r(v),
    // Y=Y_circle*r(v), D=W_circle, in the exact authored plane. Circle
    // traversal and strictly increasing nonnegative radial law prove full
    // injective disk/annulus coverage modulo longitude seam and (inner=0)
    // center collapse. The selected nonzero minor proves the plane embedding
    // without epsilon tests or squaring a tiny normal. Its intrinsic support
    // is regular even at the collapsed disk center; no sharp cone apex is
    // present. This does not prove B-rep seam/vertex incidence or solid closure.
    Ok(())
}
