//! Robust orientation signs for binary64 points: a floating-point filter
//! with an exact expansion-arithmetic fallback (non-overlapping expansions
//! built from error-free sums and FMA products), so the returned sign is the
//! sign of the exact real determinant of the given inputs.

use std::cmp::Ordering;

pub(crate) type P = [f64; 3];

fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bv = s - a;
    let av = s - bv;
    (s, (a - av) + (b - bv))
}

fn two_product(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    (p, a.mul_add(b, -p))
}

/// Sum of two non-overlapping expansions (increasing magnitude), zero
/// components removed.
fn expansion_sum(e: &[f64], f: &[f64]) -> Vec<f64> {
    let mut out = e.to_vec();
    for &b in f {
        let mut q = b;
        let mut next = Vec::with_capacity(out.len() + 1);
        for &a in &out {
            let (s, err) = two_sum(q, a);
            if err != 0.0 {
                next.push(err);
            }
            q = s;
        }
        if q != 0.0 {
            next.push(q);
        }
        out = next;
    }
    out
}

fn scale_expansion(e: &[f64], b: f64) -> Vec<f64> {
    let mut acc: Vec<f64> = Vec::new();
    for &a in e {
        let (p, err) = two_product(a, b);
        acc = expansion_sum(&acc, &[err, p]);
    }
    acc
}

fn product(e: &[f64], f: &[f64]) -> Vec<f64> {
    f.iter().fold(Vec::new(), |acc, &b| {
        expansion_sum(&acc, &scale_expansion(e, b))
    })
}

fn negate(e: &[f64]) -> Vec<f64> {
    e.iter().map(|v| -v).collect()
}

fn sign_of(e: &[f64]) -> Ordering {
    // Components grow in magnitude; the last nonzero one carries the sign.
    e.iter()
        .rev()
        .find(|v| **v != 0.0)
        .map_or(Ordering::Equal, |v| v.total_cmp(&0.0))
}

fn diff(a: f64, b: f64) -> Vec<f64> {
    let (s, err) = two_sum(a, -b);
    [err, s].into_iter().filter(|v| *v != 0.0).collect()
}

/// Exact `a*d - b*c`.
fn det2(a: &[f64], b: &[f64], c: &[f64], d: &[f64]) -> Vec<f64> {
    expansion_sum(&product(a, d), &negate(&product(b, c)))
}

/// Sign of det[b-a, c-a, d-a]: positive when `d` lies on the side of plane
/// `abc` that the right-hand normal (b-a)x(c-a) points to.
pub(crate) fn orient3d(a: P, b: P, c: P, d: P) -> Ordering {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let w = [d[0] - a[0], d[1] - a[1], d[2] - a[2]];
    let det = u[0] * (v[1] * w[2] - v[2] * w[1])
        + u[1] * (v[2] * w[0] - v[0] * w[2])
        + u[2] * (v[0] * w[1] - v[1] * w[0]);
    let perm = u[0].abs() * ((v[1] * w[2]).abs() + (v[2] * w[1]).abs())
        + u[1].abs() * ((v[2] * w[0]).abs() + (v[0] * w[2]).abs())
        + u[2].abs() * ((v[0] * w[1]).abs() + (v[1] * w[0]).abs());
    // Generous relative bound; it also covers the three rounded differences.
    if det.is_finite() && det.abs() > perm * 1e-14 {
        return det.total_cmp(&0.0);
    }
    let u: [Vec<f64>; 3] = std::array::from_fn(|k| diff(b[k], a[k]));
    let v: [Vec<f64>; 3] = std::array::from_fn(|k| diff(c[k], a[k]));
    let w: [Vec<f64>; 3] = std::array::from_fn(|k| diff(d[k], a[k]));
    let m0 = product(&u[0], &det2(&v[1], &v[2], &w[1], &w[2]));
    let m1 = product(&u[1], &det2(&v[2], &v[0], &w[2], &w[0]));
    let m2 = product(&u[2], &det2(&v[0], &v[1], &w[0], &w[1]));
    sign_of(&expansion_sum(&expansion_sum(&m0, &m1), &m2))
}

/// Sign of the 2D orientation of `(a, b, c)` in the coordinate plane that
/// drops `axis` (axes `axis+1`, `axis+2` in cyclic order).
pub(crate) fn orient2d(a: P, b: P, c: P, axis: usize) -> Ordering {
    let (x, y) = ((axis + 1) % 3, (axis + 2) % 3);
    let l = (b[x] - a[x]) * (c[y] - a[y]);
    let r = (b[y] - a[y]) * (c[x] - a[x]);
    let det = l - r;
    if det.is_finite() && det.abs() > (l.abs() + r.abs()) * 1e-14 {
        return det.total_cmp(&0.0);
    }
    sign_of(&det2(
        &diff(b[x], a[x]),
        &diff(b[y], a[y]),
        &diff(c[x], a[x]),
        &diff(c[y], a[y]),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orient3d_matches_simple_signs() {
        let o = [0.0; 3];
        let (x, y) = ([1., 0., 0.], [0., 1., 0.]);
        assert_eq!(orient3d(o, x, y, [0., 0., 1.]), Ordering::Greater);
        assert_eq!(orient3d(o, x, y, [0., 0., -1.]), Ordering::Less);
        assert_eq!(orient3d(o, x, y, [3., 7., 0.]), Ordering::Equal);
    }

    #[test]
    fn orient3d_is_exact_one_ulp_off_a_tilted_plane() {
        // Every point has z == x, so they lie exactly on the plane z = x even
        // though no coordinate is a short binary fraction.
        let (a, b, c) = ([0.1, 0.0, 0.1], [0.7, 1.0, 0.7], [0.3, 0.9, 0.3]);
        assert_eq!(orient3d(a, b, c, [0.2, 0.5, 0.2]), Ordering::Equal);
        let up = orient3d(a, b, c, [0.2, 0.5, f64::from_bits(0.2f64.to_bits() + 1)]);
        let down = orient3d(a, b, c, [0.2, 0.5, f64::from_bits(0.2f64.to_bits() - 1)]);
        assert_ne!(up, Ordering::Equal);
        assert_eq!(up, down.reverse());
    }

    #[test]
    fn orient2d_detects_exact_collinearity() {
        // x == y for all three points, so the exact determinant is zero.
        let (a, b, c) = ([0.1, 0.1, 9.0], [0.2, 0.2, 9.0], [0.3, 0.3, 9.0]);
        assert_eq!(orient2d(a, b, c, 2), Ordering::Equal);
        let up = orient2d(a, b, [0.3, f64::from_bits(0.3f64.to_bits() + 1), 9.0], 2);
        assert_eq!(up, Ordering::Greater);
        assert_eq!(
            orient2d([0.; 3], [1., 0., 0.], [0., 1., 0.], 2),
            Ordering::Greater
        );
    }
}
