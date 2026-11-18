//! Tangent planes on complete original carriers with collapsed endpoint limits.
use super::{Carrier, Limits};
use nurbs_core::{
    Error, Result,
    factored_surface_normals::PlaneLimit,
    interval_eval::{self, Interval as I},
};
pub struct PolarSeam<E: Carrier> {
    edge: E,
    bound: [f64; 2],
    tolerance: f64,
    factor_orders: [usize; 2],
}
impl<E: Carrier> PolarSeam<E> {
    pub fn edge(&self) -> &E {
        &self.edge
    }
    pub fn sine_squared_bounds(&self) -> [f64; 2] {
        self.bound
    }
    pub fn tolerance(&self) -> f64 {
        self.tolerance
    }
    pub fn factor_orders(&self) -> [usize; 2] {
        self.factor_orders
    }
}
pub struct PolarReport<E: Carrier> {
    pub seam: Option<PolarSeam<E>>,
    pub cells: usize,
    pub curve_spans: usize,
    pub normal_coefficients: usize,
    pub accepted_cells: usize,
    pub interval_refinements: usize,
    pub uncertain_canonical: Option<[f64; 2]>,
    pub reason: &'static str,
}
/// Plane limits must retain exactly the represented original surfaces of the
/// sealed carrier. Comparison is structural source equality, not a tolerance
/// or a rounded extraction. Positional ownership remains the carrier's proof.
/// These are regular tangent planes off the collapsed side and limit planes
/// on it. No quotient graph or pole G2 is inferred by this factory alone.
pub fn qualify_polar_tangency<E: Carrier>(
    edge: &E,
    planes: [&PlaneLimit<'_>; 2],
    limits: Limits,
    max_normal_coefficients: usize,
) -> Result<PolarReport<E>> {
    super::validate_limits(&limits)?;
    if !(1..=100000000).contains(&max_normal_coefficients)
        || (0..2).any(|side| planes[side].surface() != edge.fragment(side).surface())
    {
        return Err(Error::new(
            "BREP_SOURCE_POLAR_SEAM",
            "Use exact original carrier surfaces and bounded normal coefficients",
        ));
    }
    let ends = edge.fragment(0).parameter_bounds();
    let lo = ends.iter().map(|r| r[0]).fold(f64::INFINITY, f64::min);
    let hi = ends.iter().map(|r| r[1]).fold(f64::NEG_INFINITY, f64::max);
    let coverage = edge.map_parameter(0, I::new(lo, hi)?)?.intersect(0., 1.)?;
    let mut queue = vec![[coverage.lo, coverage.hi]];
    let mut out = PolarReport {
        seam: None,
        cells: 0,
        curve_spans: 0,
        normal_coefficients: 0,
        accepted_cells: 0,
        interval_refinements: 0,
        uncertain_canonical: None,
        reason: "source-polar-seam-work-limit",
    };
    let mut bound = [1_f64, 0_f64];
    while let Some(q) = queue.pop() {
        out.uncertain_canonical = Some(q);
        if out.cells == limits.cells || out.normal_coefficients == max_normal_coefficients {
            return Ok(out);
        }
        out.cells += 1;
        let query = (|| -> Result<Option<[[[f64; 2]; 2]; 2]>> {
            let mut rectangles = [[[0.; 2]; 2]; 2];
            for side in 0..2 {
                let fragment = edge.fragment(side);
                let c = fragment.curve();
                let t = edge.inverse_parameter(side, I::new(q[0], q[1])?)?;
                let count = (c.degree..c.control_points.len())
                    .filter(|&i| {
                        c.knots[i] < c.knots[i + 1] && c.knots[i] <= t.hi && c.knots[i + 1] >= t.lo
                    })
                    .count();
                if count > limits.curve_spans - out.curve_spans {
                    return Ok(None);
                }
                out.curve_spans += count;
                let uv = interval_eval::evaluate_interval(c, t)?;
                let s = fragment.surface();
                let domains = [
                    [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
                    [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
                ];
                for axis in 0..2 {
                    let r = uv[axis].intersect(domains[axis][0], domains[axis][1])?;
                    rectangles[side][axis] = [r.lo, r.hi];
                }
            }
            Ok(Some(rectangles))
        })();
        let rectangles = match query {
            Ok(Some(r)) => Some(r),
            Ok(None) => return Ok(out),
            Err(e) if e.code == "NURBS_NUMERIC_ERROR" => {
                out.interval_refinements += 1;
                None
            }
            Err(e) => return Err(e),
        };
        if let Some(rectangles) = rectangles {
            let r = planes[0].compare_normal_rectangles(
                planes[1],
                rectangles,
                limits.max_sine_squared,
                max_normal_coefficients - out.normal_coefficients,
            )?;
            out.normal_coefficients += r.coefficients;
            if r.coefficients == 0 {
                return Ok(out);
            }
            if r.aligned == Some(true) {
                let b = r.sine_squared_bounds.unwrap();
                bound[0] = bound[0].min(b[0]);
                bound[1] = bound[1].max(b[1]);
                out.accepted_cells += 1;
                continue;
            }
            if r.aligned == Some(false) {
                out.reason = "source-polar-seam-angular-envelope-oblique";
                return Ok(out);
            }
        }
        let mid = q[0] * 0.5 + q[1] * 0.5;
        if !(q[0] < mid && mid < q[1]) {
            out.reason = "source-polar-seam-resolution-limit";
            return Ok(out);
        }
        queue.push([mid, q[1]]);
        queue.push([q[0], mid]);
    }
    out.seam = Some(PolarSeam {
        edge: edge.clone(),
        bound,
        tolerance: limits.max_sine_squared,
        factor_orders: planes.map(|p| p.factor_order()),
    });
    out.uncertain_canonical = None;
    out.reason = "source-polar-seam-closed-interval-tangent-limits-qualified";
    Ok(out)
}
