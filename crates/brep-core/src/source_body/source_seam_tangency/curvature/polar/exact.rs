//! Exact meridian graph jets plus complete native regularity/limit coverage.
use super::*;
use nurbs_core::factored_surface_normals::{self as normals, compare_polar_shape_rectangles};
/// Prove exact G2 compatibility, not a relaxed tolerance. Whole-polynomial
/// original jet identity supplies equality. Native interval fields must still
/// cover the ENTIRE retained closed carrier with finite regular/limit shapes.
/// A certified anchor selects only the constant native orientation; it never
/// substitutes for the polynomial identity or whole-interval coverage.
pub fn qualify_polar_curvature_exact<E: Carrier>(
    edge: &E,
    planes: [&PlaneLimit<'_>; 2],
    limits: CurvatureLimits,
    max_products: u64,
    max_coefficients: usize,
    max_exact_work: u64,
) -> Result<PolarCurvatureReport<E>> {
    validate_limits(&limits)?;
    if !(1..=100000000).contains(&max_products)
        || !(1..=100000000).contains(&max_coefficients)
        || !(1..=100000000).contains(&max_exact_work)
        || (0..2).any(|s| planes[s].surface() != edge.fragment(s).surface())
    {
        return Err(Error::new(
            "BREP_SOURCE_EXACT_POLAR_CURVATURE",
            "Use exact original surfaces and bounded work",
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
        reason: "exact-polar-meridian-layout-unproven",
    };
    let (Some(a), Some(b)) = (edge.exact_parameter_curve(0), edge.exact_parameter_curve(1)) else {
        return Ok(out);
    };
    let normals = planes.map(|p| p.normal_bounds());
    let axis = (0..3).find(|&a| normals.iter().all(|n| n[a][0] > 0. || n[a][1] < 0.));
    let Some(axis) = axis else {
        return Ok(out);
    };
    let r = normals::prepare_exact_meridian_jets(
        [edge.fragment(0).surface(), edge.fragment(1).surface()],
        [a.0, b.0],
        [a.1, b.1],
        [a.2, b.2],
        axis,
        max_exact_work,
    )?;
    out.exact_work = r.work;
    let Some(_identity) = r.certificate else {
        out.reason = r.reason;
        return Ok(out);
    };
    let mut fields = Vec::new();
    for p in planes {
        if out.products == max_products {
            out.reason = "exact-polar-shape-product-limit";
            return Ok(out);
        }
        let r = p.prepare_graph_shape(axis, max_products - out.products)?;
        out.products += r.products;
        let Some(f) = r.certificate else {
            out.reason = r.reason;
            return Ok(out);
        };
        fields.push(f);
    }
    let mut orientation = None;
    let mut coefficients = 0;
    let mut refusal = None;
    let r = qualify_using(edge, limits, |range, limits, work| {
        let queries = if orientation.is_none() {
            let mid = range[0] * 0.5 + range[1] * 0.5;
            vec![([mid, mid], true), (range, false)]
        } else {
            vec![(range, false)]
        };
        let mut cell = None;
        for (q, anchor) in queries {
            let query = match rectangles(edge, q, limits, work) {
                Ok(Some(q)) => q,
                Ok(None) => return Ok(None),
                Err(e) if e.code == "NURBS_NUMERIC_ERROR" && !anchor => {
                    return Ok(Some(Cell {
                        range,
                        bounds: None,
                    }));
                }
                Err(e) if e.code == "NURBS_NUMERIC_ERROR" => {
                    refusal = Some("exact-polar-orientation-unproven");
                    return Ok(None);
                }
                Err(e) => return Err(e),
            };
            if coefficients == max_coefficients {
                refusal = Some("exact-polar-shape-coefficient-limit");
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
                refusal = Some(r.reason);
                return Ok(None);
            }
            let Some(mut bounds) = r.bounds else {
                if anchor {
                    refusal = Some("exact-polar-orientation-unproven");
                    return Ok(None);
                }
                return Ok(Some(Cell {
                    range,
                    bounds: None,
                }));
            };
            if anchor {
                orientation = (0..2).find(|&sign| bounds[sign].normal[1] < 1.);
                if orientation.is_none() {
                    refusal = Some("exact-polar-orientation-unproven");
                    return Ok(None);
                }
            } else {
                // Equality on the dense nonzero-projection locus, combined
                // with complete continuous regular/limit coverage, extends
                // through projection zeros. Nonzero normals keep the anchored
                // orientation constant. No distance-box narrowing is needed.
                bounds[orientation.unwrap()] = ShapeDifference {
                    normal: [0.; 2],
                    tensor: [0.; 2],
                };
                cell = Some(Cell {
                    range,
                    bounds: Some(bounds),
                });
            }
        }
        Ok(cell)
    })?;
    out.coefficients = coefficients;
    out.cells = r.cells;
    out.curve_spans = r.curve_spans;
    out.regular_queries = r.shape_cells;
    out.uncertain_canonical = r.uncertain_canonical;
    out.reason = refusal.unwrap_or(r.reason);
    out.seam = r
        .seam
        .map(|inner| PolarCurvatureSeam { inner, exact: true });
    if out.seam.is_some() {
        out.reason = "exact-original-polar-carrier-g2-qualified";
    }
    Ok(out)
}
