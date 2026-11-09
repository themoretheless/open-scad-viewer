//! Bind literal tensor coefficients to the product of freshly proved circles.
use super::super::exact_frame::{cross, dot};
use super::{PResult, Refusal, Surface, Template, V_KNOTS, Work};
use num_traits::Zero;
pub(super) fn verify(
    center: [f64; 3],
    axes: [[f64; 3]; 3],
    reference: &Template,
    s: &Surface,
    w: &mut Work,
) -> PResult<()> {
    let c = reference.circle();
    if s.degree_u != 2
        || s.degree_v != 2
        || s.periodic_u
        || s.periodic_v
        || s.knots_u != c.knots
        || s.knots_v != V_KNOTS
        || s.control_points.len() != 9
        || s.weights.len() != 9
        || s.control_points
            .iter()
            .any(|row| row.len() != 5 || row.iter().any(|p| p.len() != 3))
        || s.weights
            .iter()
            .any(|row| row.len() != 5 || row.iter().any(|&x| !x.is_finite() || x <= 0.))
    {
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
    let base = w.op(&base, &base, |a, b| a * b)?;
    let first = w.lift(s.weights[0][0])?;
    let gauge = w.op(&first, &base, |a, b| a / b)?;
    for i in 0..9 {
        for j in 0..5 {
            let weight = w.lift(s.weights[i][j])?;
            let a = w.lift(c.weights[i])?;
            let b = w.lift(c.weights[j])?;
            let factor = w.op(&a, &b, |a, b| a * b)?;
            if w.op(&factor, &gauge, |a, b| a * b)? != weight {
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
            let r = w.lift(c.control_points[j][1])?;
            let z = w.lift(-c.control_points[j][0])?;
            let local = [w.op(&x, &r, |a, b| a * b)?, w.op(&y, &r, |a, b| a * b)?, z];
            let d = w.op(&det, &weight, |a, b| a * b)?;
            for axis in 0..3 {
                let actual = dot(&cofactors[axis], &t, w)?;
                if actual != w.op(&d, &local[axis], |a, b| a * b)? {
                    return Err(Refusal::UnprovedIdentity);
                }
            }
        }
    }
    // The entire literal homogeneous tensor is now one global gauge times:
    // X=X_long*R_lat, Y=Y_long*R_lat, Z=W_long*Z_lat, D=W_long*W_lat.
    // Reference proves X_long²+Y_long²=W_long² and R_lat²+Z_lat²=W_lat².
    // Thus X²+Y²+Z²=D² coefficientwise on all eight tensor cells.
    // Positive reference weights and its first two canonical quarters prove
    // R_lat>=0, latitude south->north once, longitude one full turn. Therefore
    // the whole sphere is covered, with only the intentional seam/pole quotient.
    // Exact det!=0 transfers coverage and regular implicit support to the
    // affine ellipsoid. This supplies no B-rep/trim/solid incidence theorem.
    Ok(())
}
