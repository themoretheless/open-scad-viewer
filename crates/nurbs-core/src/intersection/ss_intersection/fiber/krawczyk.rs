//! Shared outward fixed-preconditioner Krawczyk map. Source continuity and
//! original value/Jacobian enclosures are established by each source adapter.
use super::*;
pub(crate) struct Map {
    pub image: [I; 3],
    pub magnitudes: [[f64; 3]; 3],
    pub scales: [f64; 3],
    pub q: f64,
}
pub(crate) fn map(
    domain: [[f64; 2]; 4],
    unknown: [usize; 3],
    centers: [f64; 3],
    mut scales: [f64; 3],
    r: [[f64; 3]; 3],
    j: [[I; 3]; 3],
    f: [I; 3],
) -> Result<Map> {
    let mut image = [I::point(0.); 3];
    let mut magnitudes = [[0.; 3]; 3];
    let mut q = 0f64;
    for i in 0..3 {
        let mut base = I::point(centers[i]);
        for k in 0..3 {
            base = base.sub(I::point(r[i][k]).mul(f[k])?)?;
        }
        let mut row = I::point(0.);
        for c in 0..3 {
            let mut v = I::point(if i == c { 1. } else { 0. });
            for k in 0..3 {
                v = v.sub(I::point(r[i][k]).mul(j[k][c])?)?;
            }
            let magnitude = v.lo.abs().max(v.hi.abs());
            magnitudes[i][c] = magnitude;
            row =
                row.add(I::point(magnitude).mul(I::point(scales[c]).div(I::point(scales[i]))?)?)?;
            let delta =
                I::new(domain[unknown[c]][0], domain[unknown[c]][1])?.sub(I::point(centers[c]))?;
            base = base.add(v.mul(delta)?)?;
        }
        q = q.max(row.hi);
        image[i] = base;
    }
    if q >= 1. {
        let (new_scales, new_q) = rebalance_norm(magnitudes, scales, q)?;
        scales = new_scales;
        q = new_q;
    }

    Ok(Map {
        image,
        magnitudes,
        scales,
        q,
    })
}
