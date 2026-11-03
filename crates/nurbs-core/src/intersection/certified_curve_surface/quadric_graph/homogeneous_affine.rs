//! Exact original Bernstein certificate for Hxy=(O+u*U+v*V)*W.
use super::*;
mod bspline;
/// Elevate the original Hxy and W once in both directions. Multiplication
/// by u (respectively v) selects the lower-index source coefficient with
/// elevation factor I/(p+1) (respectively J/(q+1)). The following census
/// compares every coefficient exactly in that common original basis.
/// Positive original W then proves the projected affine identity everywhere,
/// including the boundary. No factor extraction or rounded new poles occur.
pub(super) fn prove(
    s: &Surface,
    axes: [usize; 2],
    origin: &[Q; 2],
    u: &[Q; 2],
    v: &[Q; 2],
) -> bool {
    if s.control_points.len() != s.degree_u + 1 || s.control_points[0].len() != s.degree_v + 1 {
        return bspline::prove(s, axes, origin, u, v);
    }
    let h: Vec<Vec<[Q; 3]>> = s
        .control_points
        .iter()
        .enumerate()
        .map(|(i, row)| {
            row.iter()
                .enumerate()
                .map(|(j, p)| {
                    let w = q(s.weights[i][j]);
                    [q(p[axes[0]]) * &w, q(p[axes[1]]) * &w, w]
                })
                .collect()
        })
        .collect();
    certificate(&h, origin, u, v)
}
fn certificate(h: &[Vec<[Q; 3]>], origin: &[Q; 2], u: &[Q; 2], v: &[Q; 2]) -> bool {
    let (p, r) = (h.len() - 1, h[0].len() - 1);
    for i in 0..=p + 1 {
        for j in 0..=r + 1 {
            for axis in 0..2 {
                let mut residual = q(0.);
                for du in 0..=1 {
                    for dv in 0..=1 {
                        if i < du || j < dv || i - du > p || j - dv > r {
                            continue;
                        }
                        let a = if du == 1 {
                            q(i as f64) / q((p + 1) as f64)
                        } else {
                            q(1.) - q(i as f64) / q((p + 1) as f64)
                        };
                        let b = if dv == 1 {
                            q(j as f64) / q((r + 1) as f64)
                        } else {
                            q(1.) - q(j as f64) / q((r + 1) as f64)
                        };
                        residual += a
                            * b
                            * (&h[i - du][j - dv][axis]
                                - (&origin[axis]
                                    + &u[axis] * q(du as f64)
                                    + &v[axis] * q(dv as f64))
                                    * &h[i - du][j - dv][2]);
                    }
                }
                if residual != q(0.) {
                    return false;
                }
            }
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn original_coefficient_census_covers_all_degree_pairs_and_detects_exact_drift() {
        for p in 1..=8 {
            for r in 1..=8 {
                let mut s = Surface {
                    degree_u: p,
                    degree_v: r,
                    knots_u: [vec![0.; p + 1], vec![1.; p + 1]].concat(),
                    knots_v: [vec![0.; r + 1], vec![1.; r + 1]].concat(),
                    control_points: (0..=p)
                        .map(|i| (0..=r).map(|j| vec![i as f64, j as f64, 0.]).collect())
                        .collect(),
                    weights: vec![vec![1.; r + 1]; p + 1],
                    periodic_u: false,
                    periodic_v: false,
                };
                let o = [q(0.), q(0.)];
                let u = [q(p as f64), q(0.)];
                let v = [q(0.), q(r as f64)];
                assert!(prove(&s, [0, 1], &o, &u, &v), "degrees {p},{r}");
                s.control_points[p][r][0] = f64::from_bits((p as f64).to_bits() + 1);
                assert!(!prove(&s, [0, 1], &o, &u, &v));
            }
        }
    }
}
