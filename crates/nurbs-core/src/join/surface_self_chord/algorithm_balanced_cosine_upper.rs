//! surface self chord algorithm: balanced_cosine_upper.
use super::*;
pub fn balanced_cosine_upper(j: [[I; 2]; 3]) -> Result<f64> {
    // Raw fourth-order Gram arithmetic can overflow on a valid uniformly
    // scaled chart. Keep it unresolved and try the normalized enclosure.
    let mut best = cosine_upper(j).unwrap_or(1.);
    // J D is the same original differential in rescaled parameter coordinates:
    // chord = M D (D^-1 delta), and n.(J D)=0. The intrinsic inequality
    // therefore applies unchanged to any fixed positive diagonal D.
    let scales: [f64; 2] = std::array::from_fn(|a| {
        1. / j
            .iter()
            .map(|row| row[a].lo.abs().max(row[a].hi.abs()))
            .fold(0_f64, f64::max)
    });
    if scales.iter().all(|s| s.is_finite() && *s > 0.) {
        let mut balanced = j;
        let mut finite = true;
        for k in 0..3 {
            for a in 0..2 {
                match j[k][a].mul(I::point(scales[a])) {
                    Ok(x) => balanced[k][a] = x,
                    Err(_) => finite = false,
                }
            }
        }
        if finite {
            if let Ok(bound) = cosine_upper(balanced) {
                best = best.min(bound);
            }
            // A fixed triangular parameter shear is invertible (det=1),
            // so JQ describes the same chords with delta=Q*z, and n.JQ=0.
            // Midpoints choose Q only; every transformed derivative remains
            // outward enclosed. No sampled rank or normal enters the proof.
            for target in 0..2 {
                let base = 1 - target;
                let midpoint = balanced.map(|row| row.map(|v| v.lo * 0.5 + v.hi * 0.5));
                let norm: f64 = midpoint.iter().map(|row| row[base] * row[base]).sum();
                if norm <= f64::MIN_POSITIVE { continue; }
                let coefficient = midpoint.iter().map(|row| row[base] * row[target]).sum::<f64>() / norm;
                if !coefficient.is_finite() { continue; }
                let candidate = (|| -> Result<[[I; 2]; 3]> {
                    let mut sheared = balanced;
                    for k in 0..3 {
                        sheared[k][target] = balanced[k][target]
                            .sub(balanced[k][base].mul(I::point(coefficient))?)?;
                    }
                    for axis in 0..2 {
                        let scale = sheared.iter().map(|row| row[axis].lo.abs().max(row[axis].hi.abs()))
                            .fold(0_f64, f64::max);
                        if scale >= f64::MIN_POSITIVE {
                            for row in &mut sheared { row[axis] = row[axis].div(I::point(scale))?; }
                        }
                    }
                    Ok(sheared)
                })();
                if let Ok(candidate) = candidate {
                    if let Ok(bound) = cosine_upper(candidate) { best = best.min(bound); }
                }
            }
        }
    }
    Ok(best)
}
