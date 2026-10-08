//! Separation of retained original face domains, including root-valued trims.
//! Outside proofs prune products; unresolved domains remain covered geometrically.
use crate::{source_contour_winding, source_volume::Body};
use nurbs_core::{
    Error, Result,
    surface::Surface,
    surface_distance::{enclosure_distance, rectangle_bounds},
    trim_domain::Location,
};
type Product = [[[f64; 2]; 2]; 2];
pub struct Limits {
    pub cells: usize,
    pub spans: usize,
    pub domain_cells: usize,
    pub domain_cells_per_query: usize,
    pub tolerance_uv: f64,
}
pub struct Certificate<'a> {
    body: &'a Body,
    faces: [usize; 2],
    lower_mm: f64,
}
impl<'a> Certificate<'a> {
    pub fn body(&self) -> &'a Body {
        self.body
    }
    pub fn faces(&self) -> [usize; 2] {
        self.faces
    }
    pub fn lower_mm(&self) -> f64 {
        self.lower_mm
    }
}
pub struct Report<'a> {
    pub certificate: Option<Certificate<'a>>,
    pub cells: usize,
    pub spans: usize,
    pub domain_cells: usize,
    pub domain_queries: usize,
    pub endpoint_queries: usize,
    pub outside_products: usize,
    pub unresolved_products: Vec<Product>,
    pub reason: &'static str,
}
fn domain(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn spans(s: &Surface, r: [[f64; 2]; 2]) -> usize {
    let count = |k: &[f64], p: usize, n: usize, d: [f64; 2]| {
        (p..n)
            .filter(|&i| k[i] < k[i + 1] && k[i] <= d[1] && k[i + 1] >= d[0])
            .count()
    };
    count(&s.knots_u, s.degree_u, s.control_points.len(), r[0])
        * count(&s.knots_v, s.degree_v, s.control_points[0].len(), r[1])
}
/// A certificate requires complete coverage of the unchanged domain product.
/// Proximity, boundary membership and exhausted work never create a certificate.
pub fn qualify<'a>(
    body: &'a Body,
    faces: [usize; 2],
    minimum_mm: f64,
    limits: Limits,
) -> Result<Report<'a>> {
    let shell = body.geometry().shell();
    if faces[0] == faces[1]
        || faces.iter().any(|&f| f >= shell.faces().len())
        || !minimum_mm.is_finite()
        || minimum_mm <= 0.
        || [
            limits.cells,
            limits.spans,
            limits.domain_cells,
            limits.domain_cells_per_query,
        ]
        .iter()
        .any(|v| !(1..=100000).contains(v))
        || !limits.tolerance_uv.is_finite()
        || limits.tolerance_uv <= 0.
    {
        return Err(Error::new(
            "BREP_SOURCE_RETAINED_GAP",
            "Choose disjoint owned faces, a positive gap and bounded domain work",
        ));
    }
    let regions = shell.regions().ok_or_else(|| {
        Error::new(
            "BREP_SOURCE_RETAINED_GAP",
            "Original retained regions are required",
        )
    })?;
    let surfaces = faces.map(|f| shell.faces()[f][0].edges()[0].surface());
    let natural = surfaces.map(domain);
    let mut queue = vec![natural];
    let mut out = Report {
        certificate: None,
        cells: 0,
        spans: 0,
        domain_cells: 0,
        domain_queries: 0,
        endpoint_queries: 0,
        outside_products: 0,
        unresolved_products: Vec::new(),
        reason: "source-retained-gap-work-limit",
    };
    while let Some(uv) = queue.pop() {
        if out.cells == limits.cells {
            out.unresolved_products.push(uv);
            out.unresolved_products.extend(queue);
            return Ok(out);
        }
        out.cells += 1;
        let mut outside = false;
        for side in 0..2 {
            if out.domain_cells == limits.domain_cells {
                break;
            }
            let classification = source_contour_winding::classify(
                regions[faces[side]].loops(),
                uv[side],
                limits.tolerance_uv,
                (limits.domain_cells - out.domain_cells).min(limits.domain_cells_per_query),
            )?;
            out.domain_cells += classification.cells;
            out.domain_queries += 1;
            out.endpoint_queries += classification.endpoint_queries;
            if classification.location == Location::Outside {
                outside = true;
                break;
            }
        }
        if outside {
            out.outside_products += 1;
            continue;
        }
        let work = spans(surfaces[0], uv[0]) + spans(surfaces[1], uv[1]);
        if work > limits.spans - out.spans {
            out.unresolved_products.push(uv);
            out.unresolved_products.extend(queue);
            return Ok(out);
        }
        out.spans += work;
        let boxes = [
            rectangle_bounds(surfaces[0], uv[0])?,
            rectangle_bounds(surfaces[1], uv[1])?,
        ];
        if enclosure_distance(&boxes[0], &boxes[1])?.0 >= minimum_mm {
            continue;
        }
        let chosen = (0..2)
            .flat_map(|side| (0..2).map(move |axis| (side, axis)))
            .filter(|&(side, axis)| {
                let r = uv[side][axis];
                let t = r[0] * 0.5 + r[1] * 0.5;
                r[0] < t && t < r[1]
            })
            .max_by(|&(s, a), &(t, b)| {
                let width = |(s, a): (usize, usize)| {
                    (uv[s][a][1] - uv[s][a][0]) / (natural[s][a][1] - natural[s][a][0])
                };
                width((s, a)).total_cmp(&width((t, b)))
            });
        let Some((side, axis)) = chosen else {
            out.unresolved_products.push(uv);
            continue;
        };
        let t = uv[side][axis][0] * 0.5 + uv[side][axis][1] * 0.5;
        let mut left = uv;
        let mut right = uv;
        left[side][axis][1] = t;
        right[side][axis][0] = t;
        queue.push(right);
        queue.push(left);
    }
    if out.unresolved_products.is_empty() {
        out.certificate = Some(Certificate {
            body,
            faces,
            lower_mm: minimum_mm,
        });
        out.reason = "source-retained-gap-qualified";
    } else {
        out.reason = "source-retained-gap-resolution-limit";
    }
    Ok(out)
}
