//! Exact binary64 knot insertion. A rounded or out-of-range operation refuses
//! the identity proof; floating decomposition alone never supplies identity.
use crate::curve::Curve;
fn dyadic(x: f64) -> Option<(i128, i32)> {
    if !x.is_finite() {
        return None;
    }
    let bits = x.to_bits();
    let raw = ((bits >> 52) & 2047) as i32;
    let mut m = (bits & ((1u64 << 52) - 1)) as i128;
    let mut e = if raw == 0 {
        -1074
    } else {
        m += 1i128 << 52;
        raw - 1023 - 52
    };
    if bits >> 63 != 0 {
        m = -m;
    }
    if m == 0 {
        return Some((0, 0));
    }
    while m % 2 == 0 {
        m /= 2;
        e += 1;
    }
    Some((m, e))
}
fn normalized(mut m: i128, mut e: i32) -> (i128, i32) {
    if m == 0 {
        return (0, 0);
    }
    while m % 2 == 0 {
        m /= 2;
        e += 1;
    }
    (m, e)
}
pub fn exact_add(a: f64, b: f64) -> Option<f64> {
    let (am, ae) = dyadic(a)?;
    let (bm, be) = dyadic(b)?;
    if am == 0 {
        return Some(b);
    }
    if bm == 0 {
        return Some(a);
    }
    let e = ae.min(be);
    let shift = |m: i128, s: i32| {
        if s <= 126 {
            m.checked_mul(1i128 << s)
        } else {
            None
        }
    };
    let exact = normalized(shift(am, ae - e)?.checked_add(shift(bm, be - e)?)?, e);
    let result = a + b;
    (dyadic(result)? == exact).then_some(result)
}
pub fn exact_mul(a: f64, b: f64) -> Option<f64> {
    let (am, ae) = dyadic(a)?;
    let (bm, be) = dyadic(b)?;
    let result = a * b;
    (dyadic(result)? == normalized(am.checked_mul(bm)?, ae + be)).then_some(result)
}
#[doc(hidden)]
pub fn exact_div(a: f64, b: f64) -> Option<f64> {
    if b == 0. {
        return None;
    }
    let result = a / b;
    let (rm, re) = dyadic(result)?;
    let (bm, be) = dyadic(b)?;
    (normalized(rm.checked_mul(bm)?, re + be) == dyadic(a)?).then_some(result)
}
/// An exact invertible change of units for a topology-only contour proof.
/// Every retained coordinate must equal the original coordinate times scale;
/// rounded multiplication is rejected rather than treated as correspondence.
pub fn scaled_curve(c: &Curve, scale: f64) -> Option<Curve> {
    if !scale.is_finite() || scale <= 0. {
        return None;
    }
    let mut out = c.clone();
    out.control_points = c
        .control_points
        .iter()
        .map(|p| {
            p.iter()
                .map(|x| exact_mul(*x, scale))
                .collect::<Option<Vec<_>>>()
        })
        .collect::<Option<Vec<_>>>()?;
    out.validate().ok()?;
    Some(out)
}
/// Exact active seam identity from matching rational basis stencils. This
/// does not materialize rounded Bernstein poles, and proves no other topology.
pub fn translated_active_seam_closed(c: &Curve) -> bool {
    if c.validate().is_err() || c.degree == 0 {
        return false;
    }
    let p = c.degree;
    let n = c.control_points.len();
    let [a, b] = c.domain();
    // Strict neighboring knots make the endpoint zero-basis convention
    // identical on the two one-sided active spans.
    if !(c.knots[p - 1] < a && a < c.knots[p + 1] && c.knots[n - 1] < b && b < c.knots[n + 1]) {
        return false;
    }
    let Some(period) = exact_add(b, -a) else {
        return false;
    };
    let shift = n - p;
    (0..=2 * p).all(|j| exact_add(c.knots[j], period) == Some(c.knots[j + shift]))
        && (0..p).all(|j| {
            c.control_points[j] == c.control_points[j + shift]
                && c.weights[j] == c.weights[j + shift]
        })
}
pub fn inspect(c: &Curve) -> Option<Vec<Curve>> {
    inspect_with_cuts(c, &[])
}
/// Exact refinement including requested active-domain cuts; no rounded
/// construction is admitted as an identity proof.
pub fn inspect_with_cuts(c: &Curve, cuts: &[f64]) -> Option<Vec<Curve>> {
    c.validate().ok()?;
    let p = c.degree;
    let n = c.control_points.len();
    let [a, b] = c.domain();
    if p == 0 || cuts.len()>256 || cuts.iter().any(|t| !t.is_finite() || *t<=a || *t>=b) {
        return None;
    }
    if c.periodic {
        if !cuts.is_empty() { return None; }
        let pieces = exact_blossom_segments(c)?;
        // Periodic input validation permits rounded exterior knot translations.
        // Certify the actual active-domain seam, not the periodic flag alone.
        if pieces.first()?.control_points.first()? != pieces.last()?.control_points.last()? {
            return None;
        }
        return Some(pieces);
    }
    if c.knots[..=p].iter().any(|k| *k != a) || c.knots[n..].iter().any(|k| *k != b) {
        if !cuts.is_empty() { return None; }
        return exact_blossom_segments(c);
    }
    // Already authored Bezier spans can be copied exactly for arbitrary
    // binary64 Euclidean controls and weights, without multiplying them.
    let mut authored = c.knots.iter().copied().filter(|t| a<=*t && *t<=b).collect::<Vec<_>>();
    authored.dedup();
    if cuts.iter().all(|t| authored.contains(t)) {
        let direct = authored.windows(2).map(|r| copy_at_full_knots(c,r[0],r[1])).collect::<Option<Vec<_>>>();
        if direct.is_some() { return direct; }
    }
    let mut knots = c.knots.clone();
    let mut controls = c
        .control_points
        .iter()
        .zip(&c.weights)
        .map(|(point, w)| {
            let mut h = point
                .iter()
                .map(|v| exact_mul(*v, *w))
                .collect::<Option<Vec<_>>>()?;
            h.push(*w);
            Some(h)
        })
        .collect::<Option<Vec<_>>>()?;
    let mut breaks = c
        .knots
        .iter()
        .copied()
        .filter(|k| *k > a && *k < b)
        .collect::<Vec<_>>();
    breaks.extend_from_slice(cuts);
    breaks.sort_by(f64::total_cmp);
    breaks.dedup();
    for u in breaks {
        while knots.iter().filter(|k| **k == u).count() < p {
            if controls.len() >= 4096 {
                return None;
            }
            let s = knots.iter().filter(|k| **k == u).count();
            let k = knots.iter().rposition(|v| *v <= u)?;
            let n = controls.len() - 1;
            let mut next = vec![Vec::new(); n + 2];
            next[..=k - p].clone_from_slice(&controls[..=k - p]);
            next[k - s + 1..n + 2].clone_from_slice(&controls[k - s..n + 1]);
            for i in k - p + 1..=k - s {
                let denominator = exact_add(knots[i + p], -knots[i])?;
                if denominator <= 0. {
                    return None;
                }
                let alpha = exact_div(exact_add(u, -knots[i])?, denominator)?;
                let beta = exact_add(1., -alpha)?;
                next[i] = controls[i]
                    .iter()
                    .zip(&controls[i - 1])
                    .map(|(x, y)| exact_add(exact_mul(alpha, *x)?, exact_mul(beta, *y)?))
                    .collect::<Option<Vec<_>>>()?;
            }
            knots.insert(k + 1, u);
            controls = next;
        }
    }
    let mut pieces = Vec::new();
    for i in p..controls.len() {
        if knots[i] >= knots[i + 1] {
            continue;
        }
        let homogeneous = &controls[i - p..=i];
        let weights = homogeneous
            .iter()
            .map(|h| h[h.len() - 1])
            .collect::<Vec<_>>();
        let points = homogeneous
            .iter()
            .map(|h| {
                let w = h[h.len() - 1];
                h[..h.len() - 1]
                    .iter()
                    .map(|v| exact_div(*v, w))
                    .collect::<Option<Vec<_>>>()
            })
            .collect::<Option<Vec<_>>>()?;
        pieces.push(Curve {
            degree: p,
            knots: std::iter::repeat_n(knots[i], p + 1)
                .chain(std::iter::repeat_n(knots[i + 1], p + 1))
                .collect(),
            control_points: points,
            weights,
            periodic: false,
        });
    }
    Some(pieces)
}
/// Endpoint clamping need not be representable as a global knot insertion.
/// Compute each active span's Bernstein controls by exact blossom evaluation.
/// Every operation must agree with its checked dyadic arithmetic identity.
fn exact_blossom_segments(c: &Curve) -> Option<Vec<Curve>> {
    let p = c.degree;
    let spans = crate::span_domains::curve_span_domains(c);
    if spans.len().checked_mul(p + 1)? > 4096 {
        return None;
    }
    let mut pieces = Vec::with_capacity(spans.len());
    for (span, [a, b]) in spans {
        let initial = c.control_points[span - p..=span]
            .iter()
            .zip(&c.weights[span - p..=span])
            .map(|(point, weight)| {
                let mut h = point
                    .iter()
                    .map(|v| exact_mul(*v, *weight))
                    .collect::<Option<Vec<_>>>()?;
                h.push(*weight);
                Some(h)
            })
            .collect::<Option<Vec<_>>>()?;
        let mut points = Vec::with_capacity(p + 1);
        let mut weights = Vec::with_capacity(p + 1);
        for k in 0..=p {
            let mut q = initial.clone();
            for r in 1..=p {
                let parameter = if r - 1 < p - k { a } else { b };
                for j in (r..=p).rev() {
                    let i = span - p + j;
                    let denominator = exact_add(c.knots[i + p - r + 1], -c.knots[i])?;
                    if denominator <= 0. {
                        return None;
                    }
                    let alpha = exact_div(exact_add(parameter, -c.knots[i])?, denominator)?;
                    if !(0. ..=1.).contains(&alpha) {
                        return None;
                    }
                    let beta = exact_add(1., -alpha)?;
                    q[j] = q[j]
                        .iter()
                        .zip(&q[j - 1])
                        .map(|(x, y)| exact_add(exact_mul(alpha, *x)?, exact_mul(beta, *y)?))
                        .collect::<Option<Vec<_>>>()?;
                }
            }
            let weight = *q[p].last()?;
            points.push(
                q[p][..q[p].len() - 1]
                    .iter()
                    .map(|v| exact_div(*v, weight))
                    .collect::<Option<Vec<_>>>()?,
            );
            weights.push(weight);
        }
        let piece = Curve {
            degree: p,
            knots: std::iter::repeat_n(a, p + 1)
                .chain(std::iter::repeat_n(b, p + 1))
                .collect(),
            control_points: points,
            weights,
            periodic: false,
        };
        piece.validate().ok()?;
        pieces.push(piece);
    }
    Some(pieces)
}
#[cfg(test)]
mod tests {
    #[test]
    fn translated_seam_identity_covers_degrees_and_rational_weights() {
        for degree in 1..=6 {
            let n = 2 * degree + 2;
            let shift = n - degree;
            let mut c = Curve {
                degree,
                knots: (0..n + degree + 1).map(|k| k as f64).collect(),
                control_points: (0..n)
                    .map(|j| vec![j as f64 / 7., (j * j) as f64 / 11., 0.])
                    .collect(),
                weights: (0..n).map(|j| (j + 1) as f64).collect(),
                periodic: false,
            };
            for j in 0..degree {
                c.control_points[j + shift] = c.control_points[j].clone();
                c.weights[j + shift] = c.weights[j];
            }
            assert!(translated_active_seam_closed(&c));
            let [a, b] = c.domain();
            let left = c.evaluate(a).unwrap().point;
            let right = c.evaluate(b).unwrap().point;
            // Numerical evaluations are only independent falsification checks.
            assert!(left.iter().zip(right).all(|(x, y)| (x - y).abs() < 1e-12));
            let mut bad = c.clone();
            bad.control_points[shift][0] = f64::from_bits(0f64.to_bits() + 1);
            assert!(!translated_active_seam_closed(&bad));
            let mut bad = c.clone();
            bad.weights[shift] = 2.;
            assert!(!translated_active_seam_closed(&bad));
            let mut bad = c.clone();
            bad.knots[1] = 1f64.next_up();
            assert!(!translated_active_seam_closed(&bad));
        }
    }
    use super::*;
}

/// Sufficient exact coordinate incidence using original homogeneous de Boor.
/// Returns None if any intermediate operation is not exactly representable,
/// never a rounded evaluation interpreted as a mathematical zero.
fn homogeneous_coordinate(c: &Curve, span: usize, t: f64, axis: usize) -> Option<[f64;2]> {
    c.validate().ok()?;
    let p=c.degree;
    if !t.is_finite() || axis>=c.control_points[0].len()
        || span<p || span>=c.control_points.len() || c.knots[span]>=c.knots[span+1]
        || t<c.knots[span] || t>c.knots[span+1] {return None;}
    let mut d=Vec::with_capacity(p+1);
    for i in span-p..=span {
        d.push([exact_mul(c.control_points[i][axis],c.weights[i])?,c.weights[i]]);
    }
    for r in 1..=p {
        for j in (r..=p).rev() {
            let i=span-p+j;
            let numerator=exact_add(t,-c.knots[i])?;
            let denominator=exact_add(c.knots[i+p-r+1],-c.knots[i])?;
            let alpha=exact_div(numerator,denominator)?;
            let beta=exact_add(1.,-alpha)?;
            for k in 0..2 {
                d[j][k]=exact_add(exact_mul(beta,d[j-1][k])?,exact_mul(alpha,d[j][k])?)?;
            }
        }
    }
    if d[p][1]<=0. {return None;}
    Some(d[p])
}

pub fn coordinate_incidence(c: &Curve, span: usize, t: f64, axis: usize, value: f64) -> Option<bool> {
    if !value.is_finite() { return None; }
    let [h,w]=homogeneous_coordinate(c,span,t,axis)?;
    Some(crate::exact_products::sum_sign(&[(h,1.),(-value,w)])==std::cmp::Ordering::Equal)
}
/// Exact axis-independent original homogeneous plane residual.
pub fn plane_incidence(c: &Curve, span: usize, t: f64, normal: [f64;3], offset: f64) -> Option<bool> {
    if !offset.is_finite() || !normal.iter().all(|x|x.is_finite()) {return None;}
    let x=homogeneous_coordinate(c,span,t,0)?;
    let y=homogeneous_coordinate(c,span,t,1)?;
    let z=homogeneous_coordinate(c,span,t,2)?;
    Some(crate::exact_products::sum_sign(&[(normal[0],x[0]),(normal[1],y[0]),(normal[2],z[0]),(-offset,x[1])])==std::cmp::Ordering::Equal)
}

#[cfg(test)]
mod incidence_tests {
    use super::*;
    #[test]
    fn homogeneous_incidence_is_exact_and_refuses_rounded_zero() {
        let c=Curve{degree:2,knots:vec![2.,2.,2.,8.,8.,8.],control_points:vec![vec![0.9375,0.25],vec![0.9375,0.5],vec![1.9375,0.75]],weights:vec![1.;3],periodic:false};
        assert_eq!(coordinate_incidence(&c,2,3.5,0,1.),Some(true));
        assert_eq!(coordinate_incidence(&c,2,3.5,0,1f64.next_up()),Some(false));
        assert_ne!(coordinate_incidence(&c,2,3.5f64.next_up(),0,1.),Some(true));
        assert_eq!(coordinate_incidence(&c,2,3.5,2,1.),None);
        assert_eq!(coordinate_incidence(&c,1,3.5,0,1.),None);
        let rational=Curve{degree:2,knots:vec![0.,0.,0.,1.,1.,1.],control_points:vec![vec![0.5,0.25],vec![1.,0.5],vec![1.5,0.75]],weights:vec![1.,2.,1.],periodic:false};
        assert_eq!(coordinate_incidence(&rational,2,0.5,0,1.),Some(true));
    }
}

/// Restrict at existing full-multiplicity boundaries by copying the original
/// B-spline definition. No homogeneous multiplication or division is needed.
pub fn copy_at_full_knots(c: &Curve, a: f64, b: f64) -> Option<Curve> {
    c.validate().ok()?;
    let p = c.degree;
    let n = c.control_points.len();
    let [start, end] = c.domain();
    if c.periodic || !a.is_finite() || !b.is_finite() || a < start || a >= b || b > end {
        return None;
    }
    let left_full = if a == start {
        c.knots[..=p].iter().all(|k| *k == a)
    } else {
        c.knots.iter().filter(|&&k| k == a).count() >= p
    };
    let right_full = if b == end {
        c.knots[n..].iter().all(|k| *k == b)
    } else {
        c.knots.iter().filter(|&&k| k == b).count() >= p
    };
    if !left_full || !right_full { return None; }
    let first = (p..n).find(|&i| c.knots[i] == a && c.knots[i] < c.knots[i+1])?;
    let last = (p..n).find(|&i| c.knots[i+1] == b && c.knots[i] < c.knots[i+1])?;
    let out = Curve {
        degree: p,
        knots: std::iter::repeat_n(a, p+1)
            .chain(c.knots.iter().copied().filter(|k| a < *k && *k < b))
            .chain(std::iter::repeat_n(b, p+1)).collect(),
        control_points: c.control_points[first-p..=last].to_vec(),
        weights: c.weights[first-p..=last].to_vec(),
        periodic: false,
    };
    out.validate().ok()?;
    Some(out)
}
