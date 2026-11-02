//! Exact crossings and interval overlaps of rational degree-one iso-curves.
use super::*;
use num_traits::{One, Zero};
type Q = BigRational;
fn q(x: f64) -> Q {
    Q::from_f64(x).unwrap()
}

// Collapse the other parameter at its exact active-domain midpoint. Original
// homogeneous tensor coefficients define a rational curve of the original axis degree.
pub(in crate::intersection) fn iso(s: &Surface, axis: usize) -> Option<Vec<[Q; 4]>> {
    iso_at(s, axis, 0.5)
}
pub(in crate::intersection) fn iso_at(
    s: &Surface,
    axis: usize,
    parameter: f64,
) -> Option<Vec<[Q; 4]>> {
    if axis > 1 || !parameter.is_finite() || parameter <= 0. || parameter >= 1. {
        return None;
    }
    let (knots, p, n, count) = if axis == 0 {
        (
            &s.knots_v,
            s.degree_v,
            s.control_points[0].len(),
            s.control_points.len(),
        )
    } else {
        (
            &s.knots_u,
            s.degree_u,
            s.control_points.len(),
            s.control_points[0].len(),
        )
    };
    if p > 16 {
        return None;
    }
    let t = q(knots[p]) + (q(knots[n]) - q(knots[p])) * q(parameter);
    let span = (p..n).find(|&i| q(knots[i]) <= t && t < q(knots[i + 1]))?;
    let start = span - p;
    let mut basis = vec![Q::zero(); p + 2];
    basis[p] = Q::one();
    for d in 1..=p {
        let mut next = vec![Q::zero(); p + 2];
        for local in p - d..=p {
            let i = start + local;
            let left = q(knots[i + d]) - q(knots[i]);
            if !left.is_zero() {
                next[local] += (&t - q(knots[i])) / left * &basis[local];
            }
            let right = q(knots[i + d + 1]) - q(knots[i + 1]);
            if !right.is_zero() {
                next[local] += (q(knots[i + d + 1]) - &t) / right * &basis[local + 1];
            }
        }
        basis = next;
    }
    Some(
        (0..count)
            .map(|i| {
                let mut h = std::array::from_fn(|_| Q::zero());
                for (local, b) in basis.iter().enumerate().take(p + 1) {
                    let (u, v) = if axis == 0 {
                        (i, start + local)
                    } else {
                        (start + local, i)
                    };
                    let w = b * q(s.weights[u][v]);
                    for (c, value) in h.iter_mut().enumerate().take(3) {
                        *value += &w * q(s.control_points[u][v][c]);
                    }
                    h[3] += w;
                }
                h
            })
            .collect(),
    )
}
fn intersection(a: &[Q; 4], b: &[Q; 4], c: &[Q; 4], d: &[Q; 4]) -> Option<[Q; 2]> {
    let point = |h: &[Q; 4]| std::array::from_fn::<_, 3, _>(|i| &h[i] / &h[3]);
    let (a, b, c, d) = (point(a), point(b), point(c), point(d));
    let r: [Q; 3] = std::array::from_fn(|i| &b[i] - &a[i]);
    let s: [Q; 3] = std::array::from_fn(|i| &d[i] - &c[i]);
    let delta: [Q; 3] = std::array::from_fn(|i| &c[i] - &a[i]);
    for [x, y] in [[0, 1], [0, 2], [1, 2]] {
        let det = &r[x] * &s[y] - &r[y] * &s[x];
        if det.is_zero() {
            continue;
        }
        let l = (&delta[x] * &s[y] - &delta[y] * &s[x]) / &det;
        let m = (&delta[x] * &r[y] - &delta[y] * &r[x]) / &det;
        if l <= Q::zero() || l >= Q::one() || m <= Q::zero() || m >= Q::one() {
            return None;
        }
        if (0..3).all(|i| &a[i] + &l * &r[i] == &c[i] + &m * &s[i]) {
            return Some([l, m]);
        }
        return None; // a projected crossing is not a spatial intersection
    }
    // Parallel directions: prove collinearity in all three coordinates before
    // intersecting the scalar intervals. A positive overlap has a midpoint
    // strictly inside BOTH segments; an endpoint-only touch is excluded.
    let x = (0..3).find(|&i| !r[i].is_zero())?;
    if s[x].is_zero()
        || !(0..3).all(|i| &delta[i] * &r[x] == &delta[x] * &r[i] && &s[i] * &r[x] == &s[x] * &r[i])
    {
        return None;
    }
    let lo = a[x]
        .clone()
        .min(b[x].clone())
        .max(c[x].clone().min(d[x].clone()));
    let hi = a[x]
        .clone()
        .max(b[x].clone())
        .min(c[x].clone().max(d[x].clone()));
    if lo >= hi {
        return None;
    }
    let midpoint = (lo + hi) / q(2.);
    Some([(&midpoint - &a[x]) / &r[x], (&midpoint - &c[x]) / &s[x]])
}
fn clock(lambda: &Q, a: &Q, b: &Q) -> Q {
    lambda * a / (b * (Q::one() - lambda) + lambda * a)
}

pub(super) fn find(s: &Surface, max_pairs: usize) -> Option<[[Q; 2]; 2]> {
    let mut visited = 0;
    for axis in 0..2 {
        let (degree, knots, count) = if axis == 0 {
            (s.degree_u, &s.knots_u, s.control_points.len())
        } else {
            (s.degree_v, &s.knots_v, s.control_points[0].len())
        };
        if degree != 1 {
            continue;
        }
        let Some(h) = iso(s, axis) else {
            continue;
        };
        let spans: Vec<_> = (1..count).filter(|&i| knots[i] < knots[i + 1]).collect();
        for (i, &a) in spans.iter().enumerate() {
            // Keep nonadjacent pairs first, then also inspect an adjacent
            // backtracking span. Merely shared endpoints still fail admission.
            for &b in spans.iter().skip(i + 2).chain(spans.get(i + 1)) {
                if visited == max_pairs {
                    return None;
                }
                visited += 1;
                let Some([l, m]) = intersection(&h[a - 1], &h[a], &h[b - 1], &h[b]) else {
                    continue;
                };
                let mut witness = std::array::from_fn(|_| std::array::from_fn(|_| q(0.5)));
                for (j, (span, lambda)) in [(a, l), (b, m)].into_iter().enumerate() {
                    let t = clock(&lambda, &h[span - 1][3], &h[span][3]);
                    let world = q(knots[span]) + t * (q(knots[span + 1]) - q(knots[span]));
                    witness[j][axis] = (world - q(knots[1])) / (q(knots[count]) - q(knots[1]));
                }
                return Some(witness);
            }
        }
    }
    None
}
