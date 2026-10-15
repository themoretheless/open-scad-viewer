//! Source path/chord comparison in normalized traversal units. Translation is
//! cancelled before bounding the smaller relative pose; no sampled derivative
//! or rounded trimmed curve enters either interpolation inequality.
use super::*;
use crate::numerics::interval_vec3::{norm as length, sub as subtract};
fn decode(v: [[f64; 2]; 3]) -> Result<[I; 3]> {
    Ok([
        I::new(v[0][0], v[0][1])?,
        I::new(v[1][0], v[1][1])?,
        I::new(v[2][0], v[2][1])?,
    ])
}
pub(super) fn continuous(path: &Curve) -> bool {
    let [a, b] = path.domain();
    let mut i = 0;
    while i < path.knots.len() {
        let k = path.knots[i];
        let mut end = i + 1;
        while end < path.knots.len() && path.knots[end] == k {
            end += 1;
        }
        if k > a && k < b && end - i > path.degree {
            return false;
        }
        i = end;
    }
    true
}
pub(super) fn upper(
    path: &Curve,
    jets: &vector_certificate::Report,
    range: [f64; 2],
    points: [[f64; 3]; 2],
    endpoints: [[[f64; 2]; 3]; 2],
    continuous: bool,
) -> Result<Option<f64>> {
    let mut rounding = 0_f64;
    for i in 0..2 {
        rounding =
            rounding.max(length(subtract(decode(endpoints[i])?, points[i].map(I::point))?)?.hi);
    }
    let [a, b] = path.domain();
    let rate = I::point(b).sub(I::point(a))?;
    let h = I::point(range[1]).sub(I::point(range[0]))?;
    if h.lo <= 0. {
        return Ok(None);
    }
    let interpolation = if jets.single_span {
        let Some(second) = jets.second else {
            return Ok(None);
        };
        // Authored-parameter second derivatives map by (b-a)^2. h²/8
        // applies on the smooth open span and its enclosed one-sided limits.
        length(decode(second)?)?
            .mul(rate)?
            .mul(rate)?
            .mul(h)?
            .mul(h)?
            .div(I::point(8.))?
    } else {
        if !continuous {
            return Ok(None);
        }
        let Some(first) = jets.first else {
            return Ok(None);
        };
        let mut defect = [I::point(0.); 3];
        for k in 0..3 {
            let slope = I::point(points[1][k]).sub(I::point(points[0][k]))?.div(h)?;
            defect[k] = I::new(first[k][0], first[k][1])?.mul(rate)?.sub(slope)?;
        }
        // A continuous piecewise C1 path integrates its derivative defect
        // from either endpoint; the nearer endpoint is at distance <=h/2.
        length(defect)?.mul(h)?.div(I::point(2.))?
    };
    Ok(Some(I::point(rounding).add(interpolation)?.hi))
}
