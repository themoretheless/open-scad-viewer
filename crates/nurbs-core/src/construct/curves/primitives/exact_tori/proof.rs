//! Literal basis and homogeneous coefficients bind the complete torus theorem.
use super::super::exact_frame::{cross, dot};
use super::{PResult, Q, Refusal, Surface, Template, Work};
use num_traits::Zero;
pub(super) fn verify(
    center: [f64; 3],
    axes: [[f64; 3]; 3],
    major: f64,
    minor: f64,
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
        || s.knots_v != c.knots
        || s.control_points.len() != 9
        || s.weights.len() != 9
        || s.control_points
            .iter()
            .any(|r| r.len() != 9 || r.iter().any(|p| p.len() != 3))
        || s.weights
            .iter()
            .any(|r| r.len() != 9 || r.iter().any(|&x| !x.is_finite() || x <= 0.))
    {
        return Err(Refusal::UnprovedIdentity);
    }
    let major = w.lift(major)?;
    let minor = w.lift(minor)?;
    if minor <= Q::zero() || major <= minor {
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
        for j in 0..9 {
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
            let xm = w.lift(c.control_points[j][0])?;
            let ym = w.lift(c.control_points[j][1])?;
            let radius = w.op(&minor, &xm, |a, b| a * b)?;
            let radius = w.op(&major, &radius, |a, b| a + b)?;
            let local = [
                w.op(&x, &radius, |a, b| a * b)?,
                w.op(&y, &radius, |a, b| a * b)?,
                w.op(&minor, &ym, |a, b| a * b)?,
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
    // On all16 cells actual homogeneous coefficients are one global gauge of
    // X=X_long*(R*W_mer+r*X_mer), Y=Y_long*(R*W_mer+r*X_mer),
    // Z=r*W_long*Y_mer, D=W_long*W_mer. Both reference circle identities and
    // their complete positive-angle traversal are freshly proved, not sampled.
    // R>r>0 implies rho=R+r*cos(v)>0 everywhere. Recover longitude uniquely
    // from (X/rho,Y/rho), and meridian from ((rho-R)/r,Z/r). This proves full
    // regular ring-torus coverage and no self-intersections except identified
    // seams. Exact invertible authored frame preserves these properties.
    // No B-rep incidence, shell orientation, volume or numeric-normal theorem.
    Ok(())
}
