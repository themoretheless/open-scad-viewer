//! Same/or opposite oriented comparisons of freshly owned polar shape fields.
use super::*;
use crate::surface_shape_operator::{self as shapes, ShapeDifference};
pub struct PolarShapePairReport {
    pub bounds: Option<[ShapeDifference; 2]>,
    pub coefficients: usize,
    pub regular_queries: usize,
    pub work_exhausted: bool,
    pub reason: &'static str,
}
fn strictly_away(shape: &PolarGraphShape<'_>, query: [[f64; 2]; 2]) -> bool {
    let (axis, end) = axes(shape.boundary);
    let s = shape.surface;
    let domains = [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ];
    if end {
        query[axis][1] < domains[axis][1]
    } else {
        query[axis][0] > domains[axis][0]
    }
}
/// Bounds all original rectangle pairs with one shared coefficient allowance.
/// First try the canceled graph field; if its projection is unresolved, a
/// fresh original regular-jet query may cover a rectangle strictly away from
/// the collapsed side. The source is privately single Bezier, so each fallback
/// is bounded by ONE knot cell, reserved even on numeric interval refusal.
/// Source orientation applies to both normal and tensor for either path.
pub fn compare_polar_shape_rectangles(
    fields: [&PolarGraphShape<'_>; 2],
    query: [[[f64; 2]; 2]; 2],
    max_coefficients: usize,
    max_regular_queries: usize,
) -> Result<PolarShapePairReport> {
    check(
        (1..=100000000).contains(&max_coefficients) && max_regular_queries <= 100000,
        "Choose bounded shared polar coefficients and regular queries",
    )?;
    let mut out = PolarShapePairReport {
        bounds: None,
        coefficients: 0,
        regular_queries: 0,
        work_exhausted: false,
        reason: "polar-shape-pair-unresolved",
    };
    let mut reports = Vec::new();
    for side in 0..2 {
        if out.coefficients == max_coefficients {
            out.work_exhausted = true;
            out.reason = "polar-shape-pair-coefficient-limit";
            return Ok(out);
        }
        let r = fields[side].rectangle_bounds(query[side], max_coefficients - out.coefficients)?;
        out.coefficients += r.coefficients;
        if r.coefficients == 0 {
            out.work_exhausted = true;
            out.reason = "polar-shape-pair-coefficient-limit";
            return Ok(out);
        }
        if let Some(b) = r.bounds {
            reports.push(shapes::Report {
                status: crate::surface_differential::Status::Available,
                normal: Some(b.normal),
                tensor: Some(b.tensor),
            });
        } else if strictly_away(fields[side], query[side]) {
            if out.regular_queries == max_regular_queries {
                out.work_exhausted = true;
                out.reason = "polar-shape-pair-regular-query-limit";
                return Ok(out);
            }
            out.regular_queries += 1;
            match shapes::rectangle_bounds(fields[side].surface, query[side], 1) {
                Ok(r) => {
                    assert!(r.cells <= 1);
                    reports.push(r.shape);
                }
                Err(e) if e.code == crate::NUMERIC_ERROR => return Ok(out),
                Err(e) => return Err(e),
            }
        } else {
            return Ok(out);
        }
    }
    out.bounds = shapes::compare_reports(reports.remove(0), reports.remove(0))?;
    if out.bounds.is_some() {
        out.reason = "original-polar-shape-pair-bounds-qualified";
    }
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_zero_at_far_end_needs_paid_original_regular_queries() {
        let s = Surface {
            degree_u: 2,
            degree_v: 1,
            knots_u: vec![0., 0., 0., 1., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: (0..3)
                .map(|i| {
                    (0..2)
                        .map(|j| vec![[0., 0.5, 0.5][i], (i * j) as f64 / 2., f64::from(i == 2)])
                        .collect()
                })
                .collect(),
            weights: vec![vec![1.; 2]; 3],
            periodic_u: false,
            periodic_v: false,
        };
        let p = prepare_plane_limit(&s, Boundary::UMin, 100000000)
            .unwrap()
            .certificate
            .unwrap();
        let f = p
            .prepare_graph_shape(2, 100000000)
            .unwrap()
            .certificate
            .unwrap();
        let query = [[[0.5, 1.], [0., 0.]]; 2];
        let r = compare_polar_shape_rectangles([&f, &f], query, 1000000, 2).unwrap();
        assert!(r.bounds.is_some());
        assert_eq!(r.regular_queries, 2);
        let low = compare_polar_shape_rectangles([&f, &f], query, 1000000, 1).unwrap();
        assert!(low.bounds.is_none() && low.work_exhausted);
        assert_eq!(low.regular_queries, 1);
        let low = compare_polar_shape_rectangles([&f, &f], query, r.coefficients - 1, 2).unwrap();
        assert!(low.bounds.is_none() && low.work_exhausted);
        let whole = compare_polar_shape_rectangles([&f, &f], [[[0., 1.], [0., 0.]]; 2], 1000000, 2)
            .unwrap();
        assert!(whole.bounds.is_none());
        assert_eq!(whole.regular_queries, 0);
    }
}
