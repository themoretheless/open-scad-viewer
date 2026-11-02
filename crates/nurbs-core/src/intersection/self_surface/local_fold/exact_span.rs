//! Original-span Cox-de Boor polynomials over exact binary64 rationals.
use super::*;
use num_traits::Zero;
type Q = BigRational;
fn q(x: f64) -> Q {
    Q::from_f64(x).unwrap()
}

fn add_linear(out: &mut [Q], input: &[Q], a: Q, b: Q) {
    for (i, c) in input.iter().enumerate() {
        out[i] += c * &a;
        out[i + 1] += c * &b;
    }
}
fn reflected(poly: &[Q]) -> bool {
    let mut reverse = vec![Q::zero(); poly.len()];
    for (k, coefficient) in poly.iter().enumerate() {
        let mut choose = 1u64;
        for (j, dst) in reverse.iter_mut().enumerate().take(k + 1) {
            let term = coefficient * Q::from_integer(choose.into());
            if j % 2 == 0 {
                *dst += term;
            } else {
                *dst -= term;
            }
            if j < k {
                choose = choose * (k - j) as u64 / (j + 1) as u64;
            }
        }
    }
    reverse == poly
}

pub(super) fn reflected_span(s: &Surface, axis: usize, span: usize) -> bool {
    let (knots, degree, other) = if axis == 0 {
        (&s.knots_u, s.degree_u, s.control_points[0].len())
    } else {
        (&s.knots_v, s.degree_v, s.control_points.len())
    };
    // Bound exact polynomial work independently of the source pole count.
    // Higher degrees retain the direct original-Bernstein proof route.
    if degree > 16 {
        return false;
    }
    let start = span - degree;
    let a = q(knots[span]);
    let width = q(knots[span + 1]) - &a;
    let mut basis = vec![vec![Q::zero()]; degree + 2];
    basis[degree][0] = Q::from_integer(1.into());
    for d in 1..=degree {
        let mut next = vec![vec![Q::zero(); d + 1]; degree + 2];
        for local in degree - d..=degree {
            let i = start + local;
            let left = q(knots[i + d]) - q(knots[i]);
            if !left.is_zero() {
                add_linear(
                    &mut next[local],
                    &basis[local],
                    (&a - q(knots[i])) / &left,
                    &width / &left,
                );
            }
            let right = q(knots[i + d + 1]) - q(knots[i + 1]);
            if !right.is_zero() {
                add_linear(
                    &mut next[local],
                    &basis[local + 1],
                    (q(knots[i + d + 1]) - &a) / &right,
                    -&width / &right,
                );
            }
        }
        basis = next;
    }
    for j in 0..other {
        for coordinate in 0..4 {
            let mut polynomial = vec![Q::zero(); degree + 1];
            for local in 0..=degree {
                let (u, v) = if axis == 0 {
                    (start + local, j)
                } else {
                    (j, start + local)
                };
                let mut coefficient = q(s.weights[u][v]);
                if coordinate < 3 {
                    coefficient *= q(s.control_points[u][v][coordinate]);
                }
                for (dst, value) in polynomial.iter_mut().zip(&basis[local]) {
                    *dst += value * &coefficient;
                }
            }
            if !reflected(&polynomial) {
                return false;
            }
        }
    }
    true
}
