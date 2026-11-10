//! Complete carrier curvature compatibility with polar endpoint limits.
use super::*;
use nurbs_core::factored_surface_normals::{PlaneLimit, compare_polar_shape_rectangles};
pub struct PolarCurvatureSeam<E: Carrier> {
    inner: CurvatureSeam<E>,
    exact: bool,
}
impl<E: Carrier> PolarCurvatureSeam<E> {
    pub fn is_exact(&self) -> bool {
        self.exact
    }
    pub fn edge(&self) -> &E {
        self.inner.edge()
    }
    pub fn orientation(&self) -> Orientation {
        self.inner.orientation()
    }
    pub fn normal_upper(&self) -> f64 {
        self.inner.normal_upper()
    }
    pub fn curvature_upper(&self) -> f64 {
        self.inner.curvature_upper()
    }
    pub fn normal_tolerance(&self) -> f64 {
        self.inner.normal_tolerance()
    }
    pub fn curvature_tolerance(&self) -> f64 {
        self.inner.curvature_tolerance()
    }
}
pub struct PolarCurvatureReport<E: Carrier> {
    pub seam: Option<PolarCurvatureSeam<E>>,
    pub products: u64,
    pub exact_work: u64,
    pub coefficients: usize,
    pub cells: usize,
    pub curve_spans: usize,
    pub regular_queries: usize,
    pub uncertain_canonical: Option<[f64; 2]>,
    pub reason: &'static str,
}
/// Fresh full-carrier normal AND signed tensor bounds under one coherent
/// orientation. At collapsed ends these are one-sided curvature limits; a
/// complete quotient pole/fan G2 claim requires its separate body graph/jets.
/// Private normal proofs must retain exactly the sealed edge's source data.
/// Preparation and every closed interval query share their explicit budgets.
pub fn qualify_polar_curvature<E: Carrier>(
    edge: &E,
    planes: [&PlaneLimit<'_>; 2],
    limits: CurvatureLimits,
    max_products: u64,
    max_coefficients: usize,
) -> Result<PolarCurvatureReport<E>> {
    validate_limits(&limits)?;
    if !(1..=100000000).contains(&max_products)
        || !(1..=100000000).contains(&max_coefficients)
        || (0..2).any(|s| planes[s].surface() != edge.fragment(s).surface())
    {
        return Err(Error::new(
            "BREP_SOURCE_POLAR_CURVATURE",
            "Use exact original surfaces and bounded polar work",
        ));
    }
    let mut out = PolarCurvatureReport {
        seam: None,
        products: 0,
        exact_work: 0,
        coefficients: 0,
        cells: 0,
        curve_spans: 0,
        regular_queries: 0,
        uncertain_canonical: None,
        reason: "source-polar-curvature-preparation-work-limit",
    };
    let mut fields = Vec::new();
    for p in planes {
        if out.products == max_products {
            return Ok(out);
        }
        // A proposal from the private original leading bounds; the field
        // factory verifies this projection and all cancellation identities.
        let normal = p.normal_bounds();
        let axis = (0..3)
            .filter(|&a| normal[a][0] > 0. || normal[a][1] < 0.)
            .max_by(|&a, &b| {
                normal[a][0]
                    .abs()
                    .min(normal[a][1].abs())
                    .total_cmp(&normal[b][0].abs().min(normal[b][1].abs()))
            })
            .unwrap();
        let r = p.prepare_graph_shape(axis, max_products - out.products)?;
        out.products += r.products;
        let Some(field) = r.certificate else {
            out.reason = r.reason;
            return Ok(out);
        };
        fields.push(field);
    }
    let mut coefficients = 0;
    let mut exhausted_reason = None;
    let report = qualify_using(edge, limits, |range, limits, work| {
        let query = match rectangles(edge, range, limits, work) {
            Ok(Some(q)) => q,
            Ok(None) => return Ok(None),
            Err(e) if e.code == "NURBS_NUMERIC_ERROR" => {
                return Ok(Some(Cell {
                    range,
                    bounds: None,
                }));
            }
            Err(e) => return Err(e),
        };
        if coefficients == max_coefficients {
            exhausted_reason = Some("source-polar-curvature-coefficient-limit");
            return Ok(None);
        }
        let r = compare_polar_shape_rectangles(
            [&fields[0], &fields[1]],
            query,
            max_coefficients - coefficients,
            limits.shape_cells - work.shape_cells,
        )?;
        coefficients += r.coefficients;
        work.shape_cells += r.regular_queries;
        if r.work_exhausted {
            exhausted_reason = Some(r.reason);
            return Ok(None);
        }
        Ok(Some(Cell {
            range,
            bounds: r.bounds,
        }))
    })?;
    out.coefficients = coefficients;
    out.cells = report.cells;
    out.curve_spans = report.curve_spans;
    out.regular_queries = report.shape_cells;
    out.uncertain_canonical = report.uncertain_canonical;
    out.reason = exhausted_reason.unwrap_or(report.reason);
    out.seam = report.seam.map(|inner| PolarCurvatureSeam {
        inner,
        exact: false,
    });
    if out.seam.is_some() {
        out.reason = "original-polar-carrier-curvature-limits-qualified";
    }
    Ok(out)
}
#[cfg(test)]
mod tests;

mod exact;
pub use exact::qualify_polar_curvature_exact;
