//! Injectivity by recovering one UV parameter from an affine world coordinate,
//! then recovering the other from a strictly ordered rational Bernstein fiber.
use super::*;
use num_rational::BigRational;
use num_traits::Zero;

fn charge(work: &mut usize, max: usize) -> Option<()> {
    if *work >= max {
        return None;
    }
    *work += 1;
    Some(())
}

/// Rounded products only prioritize a trial. No certificate depends on them.
pub(crate) fn likely_coupled(s: &Surface) -> bool {
    let w = &s.weights;
    w.iter().skip(1).any(|row| {
        row.iter()
            .zip(&w[0])
            .skip(1)
            .any(|(&x, &y)| x * w[0][0] != row[0] * y)
    })
}

pub(crate) fn certify(s: &Surface, work: &mut usize, max: usize) -> Option<[usize; 2]> {
    let degree = [s.degree_u, s.degree_v];
    if s.periodic_u
        || s.periodic_v
        || degree.iter().any(|d| !(1..=32).contains(d))
        || s.control_points.len() != degree[0] + 1
        || s.control_points[0].len() != degree[1] + 1
    {
        return None;
    }
    for (knots, d) in [(&s.knots_u, degree[0]), (&s.knots_v, degree[1])] {
        for (i, knot) in knots.iter().enumerate() {
            charge(work, max)?;
            if *knot != knots[if i <= d { d } else { d + 1 }] {
                return None;
            }
        }
    }
    for axis in 0..2 {
        let m = degree[axis];
        let n = degree[1 - axis];
        let indices = |i, j| if axis == 0 { (i, j) } else { (j, i) };
        let point = |i, j, k: usize| {
            let (u, v): (usize, usize) = indices(i, j);
            s.control_points[u][v][k]
        };
        let weight = |i, j| {
            let (u, v): (usize, usize) = indices(i, j);
            s.weights[u][v]
        };
        for fixed in 0..3 {
            charge(work, max)?;
            let a = point(0, 0, fixed);
            let b = point(m, 0, fixed);
            if a == b {
                continue;
            }
            let qa = BigRational::from_float(a)?;
            let qb = BigRational::from_float(b)?;
            let mut affine = true;
            for j in 0..=n {
                charge(work, max)?;
                if point(0, j, fixed) != a || point(m, j, fixed) != b {
                    affine = false;
                    break;
                }
                // Degree-(m+1) Bernstein coefficients of N - (a+(b-a)t)W.
                // Each work unit checks one coefficient with a fixed number
                // of exact operations on original binary64 values.
                for i in 1..=m {
                    charge(work, max)?;
                    let current = BigRational::from_float(point(i, j, fixed))?;
                    let previous = BigRational::from_float(point(i - 1, j, fixed))?;
                    let left = (current - &qa) * BigRational::from_float(weight(i, j))?;
                    let right = (previous - &qb) * BigRational::from_float(weight(i - 1, j))?;
                    let residual = left * BigRational::from_integer((m + 1 - i).into())
                        + right * BigRational::from_integer(i.into());
                    if !residual.is_zero() {
                        affine = false;
                        break;
                    }
                }
                if !affine {
                    break;
                }
            }
            if !affine {
                continue;
            }
            for fiber in (0..3).filter(|&k| k != fixed) {
                charge(work, max)?;
                let increasing = point(0, 0, fiber) < point(0, n, fiber);
                let orient = |x: f64| if increasing { x } else { -x };
                let mut ordered = true;
                for j in 0..n {
                    let mut upper = f64::NEG_INFINITY;
                    let mut lower = f64::INFINITY;
                    for i in 0..=m {
                        charge(work, max)?;
                        upper = upper.max(orient(point(i, j, fiber)));
                        lower = lower.min(orient(point(i, j + 1, fiber)));
                    }
                    if upper >= lower {
                        ordered = false;
                        break;
                    }
                }
                // At each fixed parameter, collapse that axis. Each resulting
                // Cartesian pole lies in the corresponding positive-weight
                // hull above. Ordered poles imply strict rational monotonicity
                // by the pair formula for the derivative numerator. At the
                // endpoints the first/last pole difference gives a nonzero
                // one-sided jet (all weights are positive). The recovered
                // affine coordinate has nonzero derivative on the other axis,
                // so this same triangular minor also proves full rank.
                if ordered {
                    return Some(if axis == 0 {
                        [fixed, fiber]
                    } else {
                        [fiber, fixed]
                    });
                }
            }
        }
    }
    None
}
