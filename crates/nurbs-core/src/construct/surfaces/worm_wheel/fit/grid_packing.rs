//! Grid packing: preserves the original arithmetic and proof budgets.
use super::*;

pub(super) fn build(source: &Source, u: &[f64], beta: &[f64], op: &Operator) -> Result<Vec<Surface>> {
    let mut surfaces = Vec::with_capacity((u.len() - 1) * (beta.len() - 1));
    let is_physical_y = source.physical_y();
    for a in u.windows(2) {
        let mut ku = vec![a[0]; 12];
        ku[6..].fill(a[1]);
        for b in beta.windows(2) {
            let domain = ChartDomain {
                u: [a[0], a[1]],
                beta: [b[0], b[1]],
            };
            let net = ideal_net(source, domain, op)?;
            let mut kb = vec![b[0]; 12];
            kb[6..].fill(b[1]);
            let mut s = Surface {
                degree_u: 5,
                degree_v: 5,
                knots_u: ku.clone(),
                knots_v: kb,
                periodic_u: false,
                periodic_v: false,
                control_points: net
                    .into_iter()
                    .map(|row| row.into_iter().map(|p| p.map(mid).to_vec()).collect())
                    .collect(),
                weights: vec![vec![1.; 6]; 6],
            };
            if is_physical_y {
                let values = physical_y_controls(domain.beta)?;
                for row in &mut s.control_points {
                    for (p, y) in row.iter_mut().zip(values) {
                        p[1] = y;
                    }
                }
            }
            s.validate()?;
            surfaces.push(s);
        }
    }
    Ok(surfaces)
}

pub(super) fn pack(surfaces: &[Surface], u: &[f64], beta: &[f64]) -> Result<Vec<Surface>> {
    let nu = u.len() - 1;
    let nb = beta.len() - 1;
    let mut out = Vec::with_capacity(nu.div_ceil(6) * nb.div_ceil(6));
    for first in (0..nu).step_by(6) {
        for start in (0..nb).step_by(6) {
            let last = (first + 6).min(nu);
            let end = (start + 6).min(nb);
            let mut ku = Vec::with_capacity(12 + 5 * (last - first - 1));
            ku.extend(std::iter::repeat_n(u[first], 6));
            for &x in &u[first + 1..last] {
                ku.extend(std::iter::repeat_n(x, 5));
            }
            ku.extend(std::iter::repeat_n(u[last], 6));
            let mut kb = Vec::with_capacity(12 + 5 * (end - start - 1));
            kb.extend(std::iter::repeat_n(beta[start], 6));
            for &x in &beta[start + 1..end] {
                kb.extend(std::iter::repeat_n(x, 5));
            }
            kb.extend(std::iter::repeat_n(beta[end], 6));
            let rows = 5 * (last - first) + 1;
            let cols = 5 * (end - start) + 1;
            let mut points = Vec::with_capacity(rows);
            for r in 0..rows {
                let (i, ri) = if r == rows - 1 {
                    (last - 1, 5)
                } else {
                    (first + r / 5, r % 5)
                };
                let mut row = Vec::with_capacity(cols);
                for c in 0..cols {
                    let (j, cj) = if c == cols - 1 {
                        (end - 1, 5)
                    } else {
                        (start + c / 5, c % 5)
                    };
                    row.push(surfaces[i * nb + j].control_points[ri][cj].clone());
                }
                points.push(row);
            }
            let s = Surface {
                degree_u: 5,
                degree_v: 5,
                knots_u: ku,
                knots_v: kb,
                control_points: points,
                weights: vec![vec![1.; cols]; rows],
                periodic_u: false,
                periodic_v: false,
            };
            s.validate()?;
            out.push(s);
        }
    }
    Ok(out)
}

/// Search-free original definition replay. Packing introduces no approximation:
/// full degree-five internal multiplicity retains every original Bernstein cell.
pub fn verify_packed_grid(source: &QualifiedEnvelopeGrid, packed: &[Surface]) -> Result<()> {
    source.reverify()?;
    check(
        same_packed_definitions(
            &pack(&source.grid.surfaces, &source.grid.u, &source.grid.beta)?,
            packed,
        ),
        "Packed wheel surfaces do not retain the qualified original cell definitions",
    )
}

pub(super) fn same_scalar_bits(a: &[f64], b: &[f64]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits())
}

pub(super) fn same_row_bits(a: &[Vec<f64>], b: &[Vec<f64>]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same_scalar_bits(x, y))
}

pub(super) fn same_packed_definitions(a: &[Surface], b: &[Surface]) -> bool {
    a.len() == b.len()
        && a.iter().zip(b).all(|(x, y)| {
            x.degree_u == y.degree_u
                && x.degree_v == y.degree_v
                && x.periodic_u == y.periodic_u
                && x.periodic_v == y.periodic_v
                && same_scalar_bits(&x.knots_u, &y.knots_u)
                && same_scalar_bits(&x.knots_v, &y.knots_v)
                && same_row_bits(&x.weights, &y.weights)
                && x.control_points.len() == y.control_points.len()
                && x.control_points
                    .iter()
                    .zip(&y.control_points)
                    .all(|(p, q)| same_row_bits(p, q))
        })
}
