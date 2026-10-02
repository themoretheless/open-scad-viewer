//! Continuous finite-line evidence inside a qualified material volume.
//! The segment is P(t)=origin+t*direction, 0<=t<=1, with authored binary64
//! coefficients. Boundary endpoints and unresolved bands are not admitted.
use crate::{Error, Model, Result, face_domain::FaceDomain, ray_parity, volume_validity};
use nurbs_core::{ray_surface, trim_domain::Location};

pub struct SegmentReport {
    pub boundary_free: bool,
    pub contacts: Vec<ray_parity::Crossing>,
    pub unresolved: Vec<ray_parity::Unresolved>,
    pub cells: usize,
    pub domain_cells: usize,
}
#[derive(Clone, Copy)]
pub struct Limits {
    pub volume: volume_validity::Limits,
    pub point_cells: usize,
    pub point_domain_cells: usize,
    pub segment_cells: usize,
    pub segment_domain_cells: usize,
}
pub struct Report {
    pub proven: bool,
    pub reason: &'static str,
    pub validity: volume_validity::Report,
    pub seed: Option<ray_parity::PointReport>,
    pub segment: Option<SegmentReport>,
}
fn valid_line(
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    cells: usize,
    domains: usize,
) -> Result<()> {
    if !origin.iter().chain(&direction).all(|v| v.is_finite())
        || direction.iter().all(|v| *v == 0.)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || !(1..=1000000).contains(&cells)
        || !(1..=8000000).contains(&domains)
    {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Material segment requires finite origin, nonzero direction and bounded positive work and UV tolerance",
        ));
    }
    Ok(())
}
fn disjoint(t: [f64; 2]) -> bool {
    t[1] < 0. || t[0] > 1.
}
/// Complete boundary-image exclusion on the closed finite parameter interval.
/// This alone does not classify material or qualify the model volume.
pub fn inspect_boundary(
    model: &Model,
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<SegmentReport> {
    model.validate()?;
    valid_line(origin, direction, tolerance_uv, max_cells, max_domain_cells)?;
    if model.faces.is_empty() {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Segment boundary audit requires faces",
        ));
    }
    let domains = (0..model.faces.len())
        .map(|i| FaceDomain::new(model, i, tolerance_uv))
        .collect::<Result<Vec<_>>>()?;
    let mut out = SegmentReport {
        boundary_free: false,
        contacts: Vec::new(),
        unresolved: Vec::new(),
        cells: 0,
        domain_cells: 0,
    };
    for (face, f) in model.faces.iter().enumerate() {
        let s = &f.surface;
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
            let domain = domains[face].classify(uv, remaining.min(4096))?;
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
            let domain = domains[face].classify(root.uv, remaining.min(4096))?;
            out.domain_cells += domain.cells;
            if domain.location == Location::Outside {
                continue;
            }
            if domain.location != Location::Inside {
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
/// Proves the complete segment is in the interior material. A qualified closed
/// boundary, an inside seed and absence of any segment/boundary intersection
/// make the segment stay in the same connected complement component.
pub fn inspect(
    model: &Model,
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report> {
    valid_line(
        origin,
        direction,
        tolerance_uv,
        limits.segment_cells,
        limits.segment_domain_cells,
    )?;
    valid_line(
        origin,
        direction,
        tolerance_uv,
        limits.point_cells,
        limits.point_domain_cells,
    )?;
    let validity = volume_validity::inspect(model, tolerance_uv, limits.volume)?;
    let mut out = Report {
        proven: false,
        reason: "volume-unproven",
        validity,
        seed: None,
        segment: None,
    };
    if !out.validity.proven {
        return Ok(out);
    }
    let seed = ray_parity::classify_point(
        model,
        origin,
        &[[1., 0.317, 0.173], [0.239, 1., 0.419], [0.137, 0.283, 1.]],
        tolerance_uv,
        limits.point_cells,
        limits.point_domain_cells,
    )?;
    let parity = seed.parity;
    out.seed = Some(seed);
    if parity != Some(true) {
        out.reason = if parity == Some(false) {
            "seed-outside"
        } else {
            "seed-unresolved"
        };
        return Ok(out);
    }
    let segment = inspect_boundary(
        model,
        origin,
        direction,
        tolerance_uv,
        limits.segment_cells,
        limits.segment_domain_cells,
    )?;
    out.proven = segment.boundary_free;
    out.reason = if out.proven {
        "interior-segment"
    } else if !segment.contacts.is_empty() {
        "boundary-contact"
    } else {
        "segment-unresolved"
    };
    out.segment = Some(segment);
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    fn limits() -> Limits {
        Limits {
            volume: volume_validity::Limits {
                boundary: crate::boundary_embedding::Limits {
                    exact_work: 1000000,
                    trim_pairs: 10000,
                    trim_cells: 100000,
                    trim_domain_cells: 1000000,
                    spans: 4096,
                    contacts: crate::face_contacts::Limits {
                        pairs: 10000,
                        cells: 150000,
                        domain_cells: 1500000,
                        cells_per_pair: 1024,
                        domain_cells_per_pair: 100000,
                    },
                },
                nesting_pairs: 100,
                nesting_cells: 100000,
                nesting_domain_cells: 1000000,
                orientation_cells: 100000,
                orientation_domain_cells: 1000000,
                orientation_spans: 100,
            },
            point_cells: 100000,
            point_domain_cells: 1000000,
            segment_cells: 100000,
            segment_domain_cells: 1000000,
        }
    }
    #[test]
    fn interior_segment_requires_volume_seed_and_complete_boundary_exclusion() {
        let model = crate::cuboid([0.; 3], [10.; 3]).unwrap();
        let before = format!("{model:?}");
        let r = inspect(&model, [2., 5., 5.], [6., 0., 0.], 1e-8, limits()).unwrap();
        assert!(r.proven && r.validity.proven && r.seed.as_ref().unwrap().parity == Some(true));
        assert!(r.segment.unwrap().boundary_free);
        let r = inspect(&model, [2., 5., 5.], [10., 0., 0.], 1e-8, limits()).unwrap();
        assert!(!r.proven && r.reason == "boundary-contact");
        assert_eq!(r.segment.unwrap().contacts.len(), 1);
        let r = inspect(&model, [-2., 5., 5.], [-1., 0., 0.], 1e-8, limits()).unwrap();
        assert!(!r.proven && r.reason == "seed-outside");
        let r = inspect(&model, [2., 5., 5.], [8., 0., 0.], 1e-8, limits()).unwrap();
        assert!(!r.proven, "a boundary endpoint must stay unqualified");
        let r = inspect(
            &model,
            [2., 5., 5.],
            [6., 0., 0.],
            1e-8,
            Limits {
                segment_cells: 1,
                ..limits()
            },
        )
        .unwrap();
        assert!(!r.proven && r.reason == "segment-unresolved");
        assert_eq!(format!("{model:?}"), before);
    }
    #[test]
    fn cavity_crossing_is_not_admitted_even_with_inside_endpoints() {
        let model = crate::operations::boolean(
            &crate::cuboid([0.; 3], [10.; 3]).unwrap(),
            &crate::cuboid([4.; 3], [6.; 3]).unwrap(),
            "difference",
        )
        .unwrap();
        let r = inspect(&model, [2., 5., 5.], [6., 0., 0.], 1e-8, limits()).unwrap();
        assert!(r.validity.proven && r.seed.unwrap().parity == Some(true));
        assert!(!r.proven && r.reason == "boundary-contact");
        assert_eq!(r.segment.unwrap().contacts.len(), 2);
    }
    #[test]
    fn curved_material_and_cylindrical_cavity_require_continuous_evidence() {
        let model =
            crate::circular_blend::partial_annular_quarter(20., 5., 6., 1.25, 1., 1e-7).unwrap();
        let before = format!("{model:?}");
        let r = inspect(&model, [10., 0., 3.], [5., 0., 0.], 1e-7, limits()).unwrap();
        assert!(r.proven, "{}", r.reason);
        let r = inspect(&model, [10., 2., 3.], [-20., 0., 0.], 1e-7, limits()).unwrap();
        assert!(r.validity.proven);
        assert!(
            !r.proven && r.reason == "boundary-contact",
            "reason={} contacts={} unresolved={}",
            r.reason,
            r.segment.as_ref().map_or(0, |s| s.contacts.len()),
            r.segment.as_ref().map_or(0, |s| s.unresolved.len())
        );
        assert_eq!(r.segment.unwrap().contacts.len(), 2);
        let r = inspect(&model, [0., 0., 3.], [1., 0., 0.], 1e-7, limits()).unwrap();
        assert!(!r.proven && r.reason == "seed-outside");
        assert_eq!(format!("{model:?}"), before);
    }
    #[test]
    fn invalid_lines_and_missing_work_are_rejected() {
        let model = crate::cuboid([0.; 3], [10.; 3]).unwrap();
        for (origin, direction, tolerance, cells, domains) in [
            ([f64::NAN, 5., 5.], [1., 0., 0.], 1e-8, 100, 100),
            ([2., 5., 5.], [f64::INFINITY, 0., 0.], 1e-8, 100, 100),
            ([2., 5., 5.], [0.; 3], 1e-8, 100, 100),
            ([2., 5., 5.], [1., 0., 0.], 0., 100, 100),
            ([2., 5., 5.], [1., 0., 0.], 1e-8, 0, 100),
            ([2., 5., 5.], [1., 0., 0.], 1e-8, 100, 0),
        ] {
            assert!(
                inspect_boundary(&model, origin, direction, tolerance, cells, domains).is_err()
            );
        }
        let r = inspect_boundary(&model, [2., 5., 5.], [10., 0., 0.], 1e-8, 10000, 1).unwrap();
        assert!(!r.boundary_free);
        assert!(!r.unresolved.is_empty());
    }
}
