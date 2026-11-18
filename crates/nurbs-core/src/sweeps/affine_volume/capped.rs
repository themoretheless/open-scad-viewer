//! Two original normalized carrier maps under one coefficient-owned affine law.
//! Restriction to a material domain and actual boundary binding are independent.
use super::*;
/// F(u,v,t)=T(t)+A(t)((1-t)C0(u,v)+t*C1(u,v)-origin).
/// Both source carriers have independent original degrees, knots and weights.
/// A source-C1 P-matrix proof or a fixed-metric/exhaustive-pair atlas covers
/// the whole normalized UV x [0,1] volume. Every original motion piece and
/// one-sided derivative remains owned. Point Jacobians, coincident traces and
/// caller-supplied reports supply no global credit.
pub fn certify_rational_caps(
    caps: [&Surface; 2],
    origin: [f64; 3],
    pieces: &[RationalPiece],
    projection: [[f64; 3]; 3],
    max_cells: usize,
    max_exact_work: u64,
) -> Result<IdealVolumeReport> {
    certify_motion_impl(MotionSource { carrier: caps[0], origin, pieces: Motion::Rational(pieces), closed: false, projection, uv_window: [[0.,1.];2], second: Some(caps[1]) }, MotionWork { max_cells, max_exact_work }, true, ProofOutputs { coordinates: &mut None, source_shear: &mut None })
}
/// Conditional WORLD orientation of the same two-carrier source. Its global
/// interpretation requires independent successful whole-volume admission.
pub fn jacobian_orientation_rational_caps(
    caps: [&Surface; 2],
    origin: [f64; 3],
    pieces: &[RationalPiece],
    point: [f64; 3],
    max_cells: usize,
) -> Result<OrientationReport> {
    orientation_motion_with_caps(
        caps[0],
        Some(caps[1]),
        origin,
        Motion::Rational(pieces),
        point,
        max_cells,
    )
}
pub(super) fn jets(
    first: &Surface,
    second: Option<&Surface>,
    separated: Option<&separable::Carrier>,
    difference_owner: Option<&crate::sweeps::progressive_sweep::ideal_volume::cap_difference::Difference<'_>>,
    b: Box3,
    out: &mut IdealVolumeReport,
    max: usize,
) -> Result<Option<([[I; 3]; 3], [I; 3])>> {
    // The exact tensor-weight and coordinate-dependence owner refers to
    // THIS original base carrier. Cancelling its common weight factor keeps
    // independent rational end weights and the full C1-C0 time term intact.
    let base = match separated {
        Some(carrier) => carrier.enclose([b[0], b[1]], &mut out.cells, max)?,
        None => charts::cap_jets(first, [b[0], b[1]], &mut out.cells, max)?,
    };
    let Some(a) = base else { return Ok(None); };
    let Some(second) = second.filter(|s| !std::ptr::eq(*s, first)) else {
        return Ok(Some((a, [I::point(0.); 3])));
    };
    let Some(z) = charts::cap_jets(second, [b[0], b[1]], &mut out.cells, max)? else {
        return Ok(None);
    };
    let t = I::new(b[2][0], b[2][1])?;
    let one = I::point(1.).sub(t)?;
    let mut q = a;
    let mut difference = a;
    for j in 0..3 { difference[j]=iv::sub(z[j],a[j])?; }
    // Intersect two independent ORIGINAL enclosures. Subtracting unrelated
    // Cartesian hulls loses the common homogeneous numerator/weight terms.
    // The owner is pointer-bound to these exact carriers and spends this same
    // ledger, including failed restrictions; it never remints a source jet.
    if let Some(owner)=difference_owner {
        if let Some(correlated)=owner.enclose([first,second],a,[b[0],b[1]],&mut out.cells,max)? {
            for j in 0..3 { for k in 0..3 {
                difference[j][k]=difference[j][k].intersect(correlated[j][k].lo,correlated[j][k].hi)?;
            }}
        }
    }
    for j in 0..3 {
        let convex = iv::add(iv::scale(a[j], one)?, iv::scale(z[j], t)?)?;
        let correlated = iv::add(a[j], iv::scale(difference[j], t)?)?;
        for k in 0..3 {
            q[j][k] = convex[k].intersect(correlated[k].lo, correlated[k].hi)?;
        }
    }
    Ok(Some((q, difference[0])))
}
#[cfg(test)]
mod tests;
