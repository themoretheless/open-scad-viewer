//! Finite segment intersection audit on immutable original source regions.
//! No mesh or Model conversion is used. Boundary exclusion alone does not
//! establish that the segment is inside material.
use crate::{
    material_segment::{SegmentReport, valid_line},
    ray_parity, source_contour_winding,
    source_volume::Body,
};
use nurbs_core::{Error, Result, ray_surface, trim_domain::Location};
fn disjoint(t: [f64; 2]) -> bool {
    t[1] < 0. || t[0] > 1.
}
/// Audit every original trimmed face, retaining uncertainty and endpoint bands.
pub fn inspect_boundary(
    body: &Body,
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<SegmentReport> {
    valid_line(origin, direction, tolerance_uv, max_cells, max_domain_cells)?;
    let regions = body.geometry().shell().regions().ok_or_else(|| {
        Error::new(
            "BREP_SOURCE_SEGMENT_REGIONS",
            "Original source regions are required",
        )
    })?;
    let mut out = SegmentReport {
        boundary_free: false,
        contacts: Vec::new(),
        unresolved: Vec::new(),
        cells: 0,
        domain_cells: 0,
    };
    for (face, region) in regions.iter().enumerate() {
        let s = region.loops()[0][0].surface();
        let uv = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        if disjoint(ray_surface::parameter_bounds(s, uv, origin, direction)?) {
            continue;
        }
        let remaining = (max_cells - out.cells).min(100000);
        let spans = |k: &[f64], d: usize, n: usize| (d..n).filter(|&i| k[i] < k[i + 1]).count();
        let initial = spans(&s.knots_u, s.degree_u, s.control_points.len())
            * spans(&s.knots_v, s.degree_v, s.control_points[0].len());
        if remaining < initial {
            out.unresolved.push(ray_parity::Unresolved {
                face,
                uv,
                reason: "work-limit",
            });
            continue;
        }
        let roots = ray_surface::intersections(s, origin, direction, tolerance_uv, remaining)?;
        out.cells += roots.cells;
        for uv in roots.unresolved {
            if disjoint(ray_surface::parameter_bounds(s, uv, origin, direction)?) {
                continue;
            }
            let remaining = max_domain_cells - out.domain_cells;
            if remaining == 0 {
                out.unresolved.push(ray_parity::Unresolved {
                    face,
                    uv,
                    reason: "domain-work-limit",
                });
                continue;
            }
            let domain = source_contour_winding::classify(
                region.loops(),
                uv,
                tolerance_uv,
                remaining.min(100000),
            )?;
            out.domain_cells += domain.cells;
            if domain.location != Location::Outside {
                out.unresolved.push(ray_parity::Unresolved {
                    face,
                    uv,
                    reason: "root-not-isolated",
                });
            }
        }
        for root in roots.roots {
            if disjoint(root.parameter) {
                continue;
            }
            let remaining = max_domain_cells - out.domain_cells;
            if remaining == 0 {
                out.unresolved.push(ray_parity::Unresolved {
                    face,
                    uv: root.uv,
                    reason: "domain-work-limit",
                });
                continue;
            }
            let domain = source_contour_winding::classify(
                region.loops(),
                root.uv,
                tolerance_uv,
                remaining.min(100000),
            )?;
            out.domain_cells += domain.cells;
            if domain.location == Location::Outside {
                continue;
            }
            if domain.location != Location::Inside || domain.winding != Some(region.chart_winding())
            {
                out.unresolved.push(ray_parity::Unresolved {
                    face,
                    uv: root.uv,
                    reason: "trim-boundary",
                });
            } else if root.parameter[0] <= 0. || root.parameter[1] >= 1. {
                out.unresolved.push(ray_parity::Unresolved {
                    face,
                    uv: root.uv,
                    reason: "endpoint-band",
                });
            } else {
                out.contacts.push(ray_parity::Crossing {
                    face,
                    uv: root.uv,
                    parameter: root.parameter,
                });
            }
        }
    }
    out.boundary_free = out.contacts.is_empty() && out.unresolved.is_empty();
    Ok(out)
}
