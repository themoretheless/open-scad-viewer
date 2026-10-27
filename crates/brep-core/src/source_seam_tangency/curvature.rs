//! G2 on original private carriers, retaining root and projective correspondence.
use super::Carrier;
use nurbs_core::{
    Error, Result,
    interval_eval::{self, Interval as I},
    surface_g2::Orientation,
    surface_shape_operator::{self, ShapeDifference},
};

pub struct CurvatureLimits {
    pub normal_distance: f64,
    pub curvature_distance: f64,
    pub cells: usize,
    pub curve_spans: usize,
    pub shape_cells: usize,
}
pub struct CurvatureSeam<E> {
    edge: E,
    orientation: Orientation,
    normal_upper: f64,
    curvature_upper: f64,
    normal_tolerance: f64,
    curvature_tolerance: f64,
}
impl<E> CurvatureSeam<E> {
    pub fn edge(&self) -> &E {
        &self.edge
    }
    pub fn orientation(&self) -> Orientation {
        self.orientation
    }
    pub fn normal_upper(&self) -> f64 {
        self.normal_upper
    }
    pub fn curvature_upper(&self) -> f64 {
        self.curvature_upper
    }
    pub fn normal_tolerance(&self) -> f64 {
        self.normal_tolerance
    }
    pub fn curvature_tolerance(&self) -> f64 {
        self.curvature_tolerance
    }
}
pub struct CurvatureReport<E> {
    pub seam: Option<CurvatureSeam<E>>,
    pub cells: usize,
    pub curve_spans: usize,
    pub shape_cells: usize,
    pub uncertain_canonical: Option<[f64; 2]>,
    pub reason: &'static str,
}
struct Cell {
    range: [f64; 2],
    bounds: Option<[ShapeDifference; 2]>,
}
fn evaluate<E: Carrier>(
    edge: &E,
    range: [f64; 2],
    limits: &CurvatureLimits,
    out: &mut CurvatureReport<E>,
) -> Result<Option<Cell>> {
    out.uncertain_canonical = Some(range);
    if out.cells == limits.cells {
        return Ok(None);
    }
    out.cells += 1;
    let mut query = [[[0.; 2]; 2]; 2];
    for side in 0..2 {
        let fragment = edge.fragment(side);
        let curve = fragment.curve();
        let t = edge.inverse_parameter(side, I::new(range[0], range[1])?)?;
        let count = (curve.degree..curve.control_points.len())
            .filter(|&i| {
                curve.knots[i] < curve.knots[i + 1]
                    && curve.knots[i] <= t.hi
                    && curve.knots[i + 1] >= t.lo
            })
            .count();
        if count > limits.curve_spans - out.curve_spans {
            return Ok(None);
        }
        out.curve_spans += count;
        let uv = interval_eval::evaluate_interval(curve, t)?;
        let s = fragment.surface();
        let domain = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        for axis in 0..2 {
            // Original fragment admission proves membership; clip only the
            // outward enclosure, never discard any original root or knot side.
            let q = uv[axis].intersect(domain[axis][0], domain[axis][1])?;
            query[side][axis] = [q.lo, q.hi];
        }
    }
    if out.shape_cells == limits.shape_cells {
        return Ok(None);
    }
    let report = surface_shape_operator::compare_rectangles(
        [edge.fragment(0).surface(), edge.fragment(1).surface()],
        query,
        limits.shape_cells - out.shape_cells,
    )?;
    out.shape_cells += report.cells;
    Ok(Some(Cell {
        range,
        bounds: report.bounds,
    }))
}
/// Whole retained-carrier tolerance G2: regular oriented normals and signed
/// world curvature tensors agree under ONE coherent orientation on every cell.
/// The positional identity is inherited only from sealed, privately admitted
/// carriers. No sampled equality, cropped proxy or caller certificate is used.
/// Unsupported C2 knot sides, singular endpoints and exhausted work never admit.
pub fn qualify_curvature<E: Carrier>(
    edge: &E,
    limits: CurvatureLimits,
) -> Result<CurvatureReport<E>> {
    if !limits.normal_distance.is_finite()
        || !(0. ..1.).contains(&limits.normal_distance)
        || !limits.curvature_distance.is_finite()
        || limits.curvature_distance < 0.
        || [limits.cells, limits.curve_spans, limits.shape_cells]
            .iter()
            .any(|n| !(1..=100000).contains(n))
    {
        return Err(Error::new(
            "BREP_SOURCE_SEAM_CURVATURE",
            "Choose finite shape tolerances and bounded original seam work",
        ));
    }
    let ends = edge.fragment(0).parameter_bounds();
    let lo = ends.iter().map(|r| r[0]).fold(f64::INFINITY, f64::min);
    let hi = ends.iter().map(|r| r[1]).fold(f64::NEG_INFINITY, f64::max);
    let coverage = edge.map_parameter(0, I::new(lo, hi)?)?.intersect(0., 1.)?;
    let mut out = CurvatureReport {
        seam: None,
        cells: 0,
        curve_spans: 0,
        shape_cells: 0,
        uncertain_canonical: None,
        reason: "source-seam-curvature-work-limit",
    };
    let Some(first) = evaluate(edge, [coverage.lo, coverage.hi], &limits, &mut out)? else {
        return Ok(out);
    };
    let mut cells = vec![first];
    let mut witnessed = [[0_f64; 2]; 2];
    loop {
        let mut upper = Some([[0_f64; 2]; 2]);
        for cell in &cells {
            if let Some(bounds) = cell.bounds {
                for sign in 0..2 {
                    witnessed[sign][0] = witnessed[sign][0].max(bounds[sign].normal[0]);
                    witnessed[sign][1] = witnessed[sign][1].max(bounds[sign].tensor[0]);
                    if let Some(u) = &mut upper {
                        u[sign][0] = u[sign][0].max(bounds[sign].normal[1]);
                        u[sign][1] = u[sign][1].max(bounds[sign].tensor[1]);
                    }
                }
            } else {
                upper = None;
            }
        }
        if let Some(u) = upper {
            if let Some(sign) = (0..2).find(|&k| {
                u[k][0] <= limits.normal_distance && u[k][1] <= limits.curvature_distance
            }) {
                out.seam = Some(CurvatureSeam {
                    edge: edge.clone(),
                    orientation: if sign == 0 {
                        Orientation::Same
                    } else {
                        Orientation::Opposite
                    },
                    normal_upper: u[sign][0],
                    curvature_upper: u[sign][1],
                    normal_tolerance: limits.normal_distance,
                    curvature_tolerance: limits.curvature_distance,
                });
                out.reason = "source-seam-curvature-qualified";
                out.uncertain_canonical = None;
                return Ok(out);
            }
        }
        let possible = (0..2)
            .filter(|&k| {
                witnessed[k][0] <= limits.normal_distance
                    && witnessed[k][1] <= limits.curvature_distance
            })
            .collect::<Vec<_>>();
        if possible.is_empty() {
            out.reason = "source-seam-curvature-envelope-separated";
            return Ok(out);
        }
        let sign = *possible
            .iter()
            .min_by(|&&a, &&b| {
                let score = |k: usize| {
                    upper.map_or(f64::INFINITY, |u| {
                        u[k][0] / limits.normal_distance.max(f64::MIN_POSITIVE)
                            + u[k][1] / limits.curvature_distance.max(f64::MIN_POSITIVE)
                    })
                };
                score(a).total_cmp(&score(b))
            })
            .unwrap();
        let index = cells
            .iter()
            .position(|c| {
                c.bounds.is_none_or(|b| {
                    b[sign].normal[1] > limits.normal_distance
                        || b[sign].tensor[1] > limits.curvature_distance
                })
            })
            .unwrap();
        let range = cells[index].range;
        out.uncertain_canonical = Some(range);
        let mid = range[0] * 0.5 + range[1] * 0.5;
        if !(range[0] < mid && mid < range[1]) {
            out.reason = "source-seam-curvature-resolution-limit";
            return Ok(out);
        }
        let Some(left) = evaluate(edge, [range[0], mid], &limits, &mut out)? else {
            return Ok(out);
        };
        let Some(right) = evaluate(edge, [mid, range[1]], &limits, &mut out)? else {
            return Ok(out);
        };
        cells[index] = left;
        cells.push(right);
    }
}

#[cfg(test)]
mod tests;
