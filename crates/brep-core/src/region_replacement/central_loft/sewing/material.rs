//! Full positive-rational control hulls, not a grid of sampled points.
use super::*;
pub struct MaterialReport {
    pub boundary_inside_stock: bool,
    pub support_violations: Vec<(usize, usize, usize, usize)>,
    pub trim_violations: Vec<(usize, usize, usize)>,
    pub stock_bounds: [[f64; 2]; 3],
}
pub(super) fn inspect(
    m: &Model,
    stock: [[f64; 2]; 3],
    trims_valid: bool,
) -> Result<MaterialReport> {
    let mut out = MaterialReport {
        boundary_inside_stock: false,
        support_violations: Vec::new(),
        trim_violations: Vec::new(),
        stock_bounds: stock,
    };
    for (face, f) in m.faces.iter().enumerate() {
        f.surface.validate()?;
        let s = &f.surface;
        for (u, row) in s.control_points.iter().enumerate() {
            for (v, p) in row.iter().enumerate() {
                for k in 0..3 {
                    if p[k] < stock[k][0] || p[k] > stock[k][1] {
                        out.support_violations.push((face, u, v, k));
                    }
                }
            }
        }
        let domain = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        for wire in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for (coedge, c) in m.loops[wire].coedges.iter().enumerate() {
                c.pcurve.validate()?;
                for (i, p) in c.pcurve.control_points.iter().enumerate() {
                    if (0..2).any(|k| p[k] < domain[k][0] || p[k] > domain[k][1]) {
                        out.trim_violations.push((face, coedge, i));
                    }
                }
            }
        }
    }
    out.boundary_inside_stock =
        trims_valid && out.support_violations.is_empty() && out.trim_violations.is_empty();
    Ok(out)
}
