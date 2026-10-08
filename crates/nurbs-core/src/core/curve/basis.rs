use super::{multiplicity, validate_basis};
use crate::{Result, check, numeric};

#[derive(Debug)]
pub struct Basis {
    pub basis: Vec<f64>,
    pub d1: Vec<f64>,
    pub d2: Vec<f64>,
    pub domain: [f64; 2],
    pub continuity: Option<i32>,
    pub derivative_status: &'static str,
    pub derivative_side: &'static str,
}

pub fn basis(p: usize, k: &[f64], n: usize, u: f64, periodic: bool) -> Result<Basis> {
    let domain = validate_basis(p, k, n)?;
    check(
        u.is_finite() && u >= domain[0] && u <= domain[1],
        "Parameter is outside the active knot domain",
    )?;
    let mut span = p;
    if u == domain[1] {
        span = n - 1;
        while span > 0 && k[span] == u {
            span -= 1;
        }
    } else {
        while span + 1 < k.len() && k[span + 1] <= u {
            span += 1;
        }
    }
    let mut b = vec![0.; k.len() - 1];
    b[span] = 1.;
    let mut d1 = vec![0.; b.len()];
    let mut d2 = d1.clone();
    for order in 1..=p {
        let mut next = vec![0.; k.len() - order - 1];
        let mut first = next.clone();
        let mut second = next.clone();
        for i in 0..next.len() {
            let left = k[i + order] - k[i];
            let right = k[i + order + 1] - k[i + 1];
            if left != 0. {
                next[i] += (u - k[i]) / left * b[i];
                first[i] += order as f64 / left * b[i];
                second[i] += order as f64 / left * d1[i];
            }
            if right != 0. {
                next[i] += (k[i + order + 1] - u) / right * b[i + 1];
                first[i] -= order as f64 / right * b[i + 1];
                second[i] -= order as f64 / right * d1[i + 1];
            }
        }
        b = next;
        d1 = first;
        d2 = second;
    }
    let (continuity, status, side) = continuity_meta(p, k, domain, u, periodic);
    numeric(
        b.iter().chain(&d1).chain(&d2).all(|v| v.is_finite()),
        "Knot scale exhausted finite derivative precision",
    )?;
    Ok(Basis {
        basis: b,
        d1,
        d2,
        domain,
        continuity,
        derivative_status: status,
        derivative_side: side,
    })
}

/// Shared continuity/derivative metadata for `basis`, `LocalBasis::scatter`
/// and `CurveEvaluator` so all evaluation paths report identical status.
pub(super) fn continuity_meta(
    p: usize,
    k: &[f64],
    domain: [f64; 2],
    u: f64,
    periodic: bool,
) -> (Option<i32>, &'static str, &'static str) {
    let endpoint = u == domain[0] || u == domain[1];
    let m = if endpoint && !periodic {
        0
    } else {
        multiplicity(k, u)
    };
    let continuity = if m == 0 {
        None
    } else {
        Some(p as i32 - m as i32)
    };
    let status = if continuity.unwrap_or(2) >= 2 {
        "available"
    } else {
        "insufficient_continuity"
    };
    let side = if u == domain[0] {
        "right"
    } else if u == domain[1] {
        "left"
    } else if status == "insufficient_continuity" {
        "right"
    } else {
        "two_sided"
    };
    (continuity, status, side)
}

/// Binary-search knot-span location per The NURBS Book A2.1. Returns the
/// index `span` with `knots[span] <= u < knots[span + 1]` inside the active
/// domain; `u == knots[n_controls]` (clamped end) returns `n_controls - 1`.
pub fn find_span(degree: usize, knots: &[f64], n_controls: usize, u: f64) -> Result<usize> {
    check((1..=25).contains(&degree), "Degree must be an integer in [1, 25]")?;
    check(
        n_controls > degree && n_controls <= 256,
        "Control point count must be between degree+1 and 256",
    )?;
    check(
        knots.len() == n_controls + degree + 1,
        "Expanded knot count must equal control point count + degree + 1",
    )?;
    let (a, b) = (knots[degree], knots[n_controls]);
    check(a < b, "The active knot domain must have positive length")?;
    check(
        u.is_finite() && u >= a && u <= b,
        "Parameter is outside the active knot domain",
    )?;
    if u == b {
        return Ok(n_controls - 1);
    }
    let (mut low, mut high) = (degree, n_controls + 1);
    let mut mid = (low + high) / 2;
    while u < knots[mid] || u >= knots[mid + 1] {
        if u < knots[mid] {
            high = mid;
        } else {
            low = mid;
        }
        mid = (low + high) / 2;
    }
    Ok(mid)
}

/// Hinted span search (item 251): validates `hint`, then gallops locally from
/// it and finishes with a binary search — O(1) amortized under monotone
/// parameter sequences, O(log n) worst case for arbitrarily stale hints.
/// Accepts any hint value (clamped into the legal span range) and updates it
/// in place to the returned span, so evaluation loops can thread one `usize`.
pub fn find_span_hinted(
    degree: usize,
    knots: &[f64],
    n_controls: usize,
    u: f64,
    hint: &mut usize,
) -> Result<usize> {
    check((1..=25).contains(&degree), "Degree must be an integer in [1, 25]")?;
    check(
        n_controls > degree && n_controls <= 256,
        "Control point count must be between degree+1 and 256",
    )?;
    check(
        knots.len() == n_controls + degree + 1,
        "Expanded knot count must equal control point count + degree + 1",
    )?;
    let (a, b) = (knots[degree], knots[n_controls]);
    check(a < b, "The active knot domain must have positive length")?;
    check(
        u.is_finite() && u >= a && u <= b,
        "Parameter is outside the active knot domain",
    )?;
    if u == b {
        *hint = n_controls - 1;
        return Ok(n_controls - 1);
    }
    let lo_bound = degree;
    let hi_bound = n_controls - 1;
    let span = (*hint).clamp(lo_bound, hi_bound);
    // Establish a bracket `knots[low] <= u < knots[high]` by galloping away
    // from the hint with doubling steps, then bisect inside the bracket.
    let (mut low, mut high);
    if u < knots[span] {
        high = span;
        let mut step = 1usize;
        loop {
            let cand = span.saturating_sub(step).max(lo_bound);
            if u >= knots[cand] || cand == lo_bound {
                low = cand;
                break;
            }
            high = cand;
            step = step.saturating_mul(2);
        }
    } else if u >= knots[span + 1] {
        low = span;
        let mut step = 1usize;
        loop {
            let cand = (low + step).min(n_controls);
            if u < knots[cand] || cand == n_controls {
                high = cand;
                break;
            }
            low = cand;
            step = step.saturating_mul(2);
        }
    } else {
        *hint = span;
        return Ok(span);
    }
    while high - low > 1 {
        let mid = (low + high) / 2;
        if u < knots[mid] {
            high = mid;
        } else {
            low = mid;
        }
    }
    *hint = low;
    Ok(low)
}

/// The `p + 1` nonvanishing basis functions (and derivatives up to order 2)
/// on a single span, computed locally in O(p²) per The NURBS Book A2.2/A2.3.
/// `values[j]`, `d1[j]`, `d2[j]` belong to control index `span - p + j`.
#[derive(Debug)]
pub struct LocalBasis {
    pub span: usize,
    pub values: Vec<f64>,
    pub d1: Vec<f64>,
    pub d2: Vec<f64>,
}
impl LocalBasis {
    /// Scatter the local window into a full-width `Basis` compatible with
    /// the existing `basis()` consumers.
    pub fn scatter(
        &self,
        p: usize,
        k: &[f64],
        n: usize,
        u: f64,
        periodic: bool,
    ) -> Result<Basis> {
        let domain = validate_basis(p, k, n)?;
        check(
            self.values.len() == p + 1 && self.d1.len() == p + 1 && self.d2.len() == p + 1,
            "Local basis window does not match the degree",
        )?;
        check(self.span >= p, "Local basis span is below the degree")?;
        let width = k.len() - p - 1;
        let mut b = vec![0.; width];
        let mut d1 = vec![0.; width];
        let mut d2 = vec![0.; width];
        for j in 0..=p {
            let i = self.span - p + j;
            check(i < width, "Local basis span is outside the knot vector")?;
            b[i] = self.values[j];
            d1[i] = self.d1[j];
            d2[i] = self.d2[j];
        }
        let (continuity, status, side) = continuity_meta(p, k, domain, u, periodic);
        Ok(Basis {
            basis: b,
            d1,
            d2,
            domain,
            continuity,
            derivative_status: status,
            derivative_side: side,
        })
    }
}

/// Local computation of the nonvanishing basis functions and their
/// derivatives up to order `nder` (at most 2, at most `degree`) on `span`,
/// per The NURBS Book A2.3 (0/0 terms are defined as 0).
pub fn basis_funs_ders(
    degree: usize,
    knots: &[f64],
    span: usize,
    u: f64,
    nder: usize,
) -> Result<LocalBasis> {
    let p = degree;
    check((1..=25).contains(&p), "Degree must be an integer in [1, 25]")?;
    check(nder <= 2, "Derivative order must not exceed 2")?;
    // Orders above the degree are identically zero (matching `basis()`).
    let nn = nder.min(p);
    check(
        knots.len() >= 2 * p + 2,
        "Knot vector is too short for the degree",
    )?;
    check(
        span >= p && span + p + 1 <= knots.len(),
        "Span is outside the valid knot range",
    )?;
    check(
        u.is_finite() && u >= knots[span] && u <= knots[span + 1],
        "Parameter is outside the given span",
    )?;
    let mut ndu = vec![vec![0.; p + 1]; p + 1];
    let mut left = vec![0.; p + 1];
    let mut right = vec![0.; p + 1];
    ndu[0][0] = 1.;
    for j in 1..=p {
        left[j] = u - knots[span + 1 - j];
        right[j] = knots[span + j] - u;
        let mut saved = 0.;
        for r in 0..j {
            ndu[j][r] = right[r + 1] + left[j - r];
            let temp = if ndu[j][r] == 0. {
                0.
            } else {
                ndu[r][j - 1] / ndu[j][r]
            };
            ndu[r][j] = saved + right[r + 1] * temp;
            saved = left[j - r] * temp;
        }
        ndu[j][j] = saved;
    }
    let mut ders = vec![vec![0.; p + 1]; nder + 1];
    for j in 0..=p {
        ders[0][j] = ndu[j][p];
    }
    let mut a = vec![vec![0.; p + 1]; 2];
    for r in 0..=p {
        let (mut s1, mut s2) = (0, 1);
        a[0][0] = 1.;
        for kk in 1..=nn {
            let mut d = 0.;
            let rk = r as i64 - kk as i64;
            let pk = p - kk;
            if r >= kk {
                let den = ndu[pk + 1][rk as usize];
                a[s2][0] = if den == 0. { 0. } else { a[s1][0] / den };
                d = a[s2][0] * ndu[rk as usize][pk];
            }
            let j1 = if rk >= -1 { 1 } else { (-rk) as usize };
            let j2 = if (r as i64) - 1 <= pk as i64 {
                kk - 1
            } else {
                p - r
            };
            for j in j1..=j2 {
                let idx = (rk + j as i64) as usize;
                let den = ndu[pk + 1][idx];
                a[s2][j] = if den == 0. {
                    0.
                } else {
                    (a[s1][j] - a[s1][j - 1]) / den
                };
                d += a[s2][j] * ndu[idx][pk];
            }
            if r <= pk {
                let den = ndu[pk + 1][r];
                a[s2][kk] = if den == 0. {
                    0.
                } else {
                    -a[s1][kk - 1] / den
                };
                d += a[s2][kk] * ndu[r][pk];
            }
            ders[kk][r] = d;
            std::mem::swap(&mut s1, &mut s2);
        }
    }
    let mut factor = p as f64;
    for kk in 1..=nn {
        for j in 0..=p {
            ders[kk][j] *= factor;
        }
        factor *= (p - kk) as f64;
    }
    numeric(
        ders.iter().flatten().all(|v| v.is_finite()),
        "Knot scale exhausted finite derivative precision",
    )?;
    let empty = || vec![0.; p + 1];
    Ok(LocalBasis {
        span,
        values: std::mem::take(&mut ders[0]),
        d1: if nder >= 1 {
            std::mem::take(&mut ders[1])
        } else {
            empty()
        },
        d2: if nder >= 2 {
            std::mem::take(&mut ders[2])
        } else {
            empty()
        },
    })
}
