//! Exact original graph jets with full regular closed-carrier shape coverage.
use super::*;
use nurbs_core::factored_surface_normals::prepare_exact_meridian_jets;
pub struct ExactCurvatureReport<E: Carrier> {
    pub seam: Option<CurvatureSeam<E>>,
    pub exact_work: u64,
    pub cells: usize,
    pub curve_spans: usize,
    pub shape_cells: usize,
    pub uncertain_canonical: Option<[f64; 2]>,
    pub reason: &'static str,
}
/// `axis` is only a projection proposal. Original polynomial identities must
/// prove both gradients and all three Hessian components, with a projected
/// Jacobian polynomial that is not identically zero on either source.
/// Equality on that dense locus extends across isolated projection zeros only
/// because complete native regularity and continuous shape coverage are also
/// freshly proved. One certified anchor selects the SAME orientation for both
/// normals and signed tensors; it supplies no sampled equality credit.
/// Singular endpoints, unsupported original maps and exhausted work refuse.
pub fn qualify_curvature_exact<E: Carrier>(
    edge: &E,
    limits: CurvatureLimits,
    axis: usize,
    max_exact_work: u64,
) -> Result<ExactCurvatureReport<E>> {
    validate_limits(&limits)?;
    if axis > 2 || !(1..=100000000).contains(&max_exact_work) {
        return Err(Error::new(
            "BREP_SOURCE_EXACT_CURVATURE",
            "Choose a world projection and bounded exact work",
        ));
    }
    let mut out = ExactCurvatureReport {
        seam: None,
        exact_work: 0,
        cells: 0,
        curve_spans: 0,
        shape_cells: 0,
        uncertain_canonical: None,
        reason: "exact-regular-meridian-maps-unproven",
    };
    let (Some(a), Some(b)) = (edge.exact_parameter_curve(0), edge.exact_parameter_curve(1)) else {
        return Ok(out);
    };
    let r = prepare_exact_meridian_jets(
        [edge.fragment(0).surface(), edge.fragment(1).surface()],
        [a.0, b.0],
        [a.1, b.1],
        [a.2, b.2],
        axis,
        max_exact_work,
    )?;
    out.exact_work = r.work;
    if r.certificate.is_none() {
        out.reason = r.reason;
        return Ok(out);
    }
    let mut orientation = None;
    let mut refusal = None;
    let report = qualify_using(edge, limits, |range, limits, work| {
        if orientation.is_none() {
            let mid = range[0] * 0.5 + range[1] * 0.5;
            let anchor = match evaluate(edge, [mid, mid], limits, work) {
                Ok(Some(anchor)) => anchor,
                Ok(None) => return Ok(None),
                Err(e) if e.code == "NURBS_NUMERIC_ERROR" => {
                    refusal = Some("exact-regular-orientation-unproven");
                    return Ok(None);
                }
                Err(e) => return Err(e),
            };
            if let Some(bounds) = anchor.bounds {
                orientation = (0..2).find(|&sign| bounds[sign].normal[1] < 1.);
            }
            if orientation.is_none() {
                refusal = Some("exact-regular-orientation-unproven");
                return Ok(None);
            }
        }
        let mut cell = match evaluate(edge, range, limits, work) {
            Ok(Some(cell)) => cell,
            Ok(None) => return Ok(None),
            Err(e) if e.code == "NURBS_NUMERIC_ERROR" => {
                return Ok(Some(Cell {
                    range,
                    bounds: None,
                }));
            }
            Err(e) => return Err(e),
        };
        if let Some(bounds) = &mut cell.bounds {
            bounds[orientation.unwrap()] = ShapeDifference {
                normal: [0.; 2],
                tensor: [0.; 2],
            };
        }
        Ok(Some(cell))
    })?;
    out.cells = report.cells;
    out.curve_spans = report.curve_spans;
    out.shape_cells = report.shape_cells;
    out.uncertain_canonical = report.uncertain_canonical;
    out.reason = refusal.unwrap_or(report.reason);
    out.seam = report.seam.map(|mut seam| {
        seam.exact = true;
        seam
    });
    if out.seam.is_some() {
        out.reason = "exact-original-regular-carrier-g2-qualified";
    }
    Ok(out)
}
