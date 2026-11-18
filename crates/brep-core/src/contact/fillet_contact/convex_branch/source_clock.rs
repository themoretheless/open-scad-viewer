//! Exact inverse brackets for a freshly proved strict rational coordinate clock.
//! Parameters stay rational until the outer report encloses them in f64.
use super::{Q, point};
use num_traits::{FromPrimitive, Zero};
use nurbs_core::curve::Curve;

pub(super) fn straight(c: &Curve) -> Option<[super::Point; 2]> {
    c.validate().ok()?;
    let n = c.control_points.len();
    let p = c.degree;
    if !(1..=32).contains(&p) || n > 4096 || c.periodic
        || c.control_points.iter().any(|p| p.len() != 2)
        || c.weights.iter().any(|w| *w <= 0.)
        || c.knots[..=p].iter().any(|k| *k != c.knots[p])
        || c.knots[n..].iter().any(|k| *k != c.knots[n]) {
        return None;
    }
    let a = point(&c.control_points[0]);
    let b = point(&c.control_points[n - 1]);
    if a == b { return None; }
    let axis = usize::from(a[0] == b[0]);
    let width = &b[axis] - &a[axis];
    for p in &c.control_points {
        let v = point(p);
        let t = (&v[axis] - &a[axis]) / &width;
        if t < Q::zero() || t > Q::from_integer(1.into())
            || super::cross(&a, &b, &v) != Q::zero() { return None; }
    }
    Some([a, b])
}

/// Caller must have proved strict monotonicity on all original spans. Returns
/// normalized pcurve parameters, without midpoint substitution. A hard budget
/// includes exact de Boor work and original-knot span search at every step.
pub(super) fn inverse(
    c: &Curve, axis: usize, target: &Q, reversed: bool,
    work: &mut usize, maximum: usize,
) -> Option<[Q; 2]> {
    let n = c.control_points.len();
    let p = c.degree;
    let start = Q::from_f64(c.control_points[0][axis])?;
    let end = Q::from_f64(c.control_points[n - 1][axis])?;
    let zero = Q::zero();
    let one = Q::from_integer(1.into());
    let flip = |r: [Q; 2]| if reversed { [&one - &r[1], &one - &r[0]] } else { r };
    if target == &start { return Some(flip([zero.clone(), zero])); }
    if target == &end { return Some(flip([one.clone(), one.clone()])); }
    if target < std::cmp::min(&start, &end) || target > std::cmp::max(&start, &end) {
        return None;
    }
    let preparation = c.knots.len().checked_add(3 * n)?;
    if preparation > maximum.saturating_sub(*work) { return None; }
    *work += preparation;
    let knots = c.knots.iter().map(|k| Q::from_f64(*k)).collect::<Option<Vec<_>>>()?;
    let controls = c.control_points.iter().zip(&c.weights).map(|(cp, w)| {
        let w = Q::from_f64(*w)?;
        Some([Q::from_f64(cp[axis])? * &w, w])
    }).collect::<Option<Vec<_>>>()?;
    let origin = &knots[p];
    let width = &knots[n] - origin;
    let mut range = [zero, one.clone()];
    for _ in 0..48 {
        let cost = n.checked_add(12 * (p + 1) * (p + 1))?;
        if cost > maximum.saturating_sub(*work) { return None; }
        *work += cost;
        let mid = (&range[0] + &range[1]) / Q::from_integer(2.into());
        let t = origin + &width * &mid;
        let span = (p..n).find(|&i| knots[i] <= t && t < knots[i + 1])?;
        let mut d = controls[span - p..=span].to_vec();
        for r in 1..=p {
            for j in (r..=p).rev() {
                let i = span - p + j;
                let denominator = &knots[i + p + 1 - r] - &knots[i];
                if denominator.is_zero() { return None; }
                let alpha = (&t - &knots[i]) / denominator;
                d[j] = std::array::from_fn(|k| (&one - &alpha) * &d[j - 1][k] + &alpha * &d[j][k]);
            }
        }
        if d[p][1] <= Q::zero() { return None; }
        let value = &d[p][0] / &d[p][1];
        if &value == target { return Some(flip([mid.clone(), mid])); }
        if (value < *target) == (end > start) { range[0] = mid; }
        else { range[1] = mid; }
    }
    Some(flip(range))
}
