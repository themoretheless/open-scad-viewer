//! Common complete original-surface ray census with caller-owned trim proofs.
use super::*;
pub(crate) fn classify_surface_ray(
    surfaces: &[&crate::Surface],
    point: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    max_cells: usize,
    max_domain_cells: usize,
    mut classify: impl FnMut(usize, [[f64; 2]; 2], usize) -> Result<(Location, usize)>,
) -> Result<Report> {
    if surfaces.is_empty()
        || !point.iter().chain(&direction).all(|x| x.is_finite())
        || direction.iter().all(|x| *x == 0.)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || !(1..=1000000).contains(&max_cells)
        || !(1..=8000000).contains(&max_domain_cells)
    {
        return Err(Error::new(
            "BREP_INVALID_INPUT",
            "Invalid original-surface ray census inputs",
        ));
    }
    let mut report = Report {
        parity: None,
        crossings: Vec::new(),
        unresolved: Vec::new(),
        cells: 0,
        domain_cells: 0,
    };
    for (face, s) in surfaces.iter().enumerate() {
        let uv = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        if ray_surface::parameter_bounds(s, uv, point, direction)?[1] < 0. {
            continue;
        }
        let remaining = (max_cells - report.cells).min(100000);
        let spans = |knots: &[f64], degree: usize, count: usize| {
            (degree..count).filter(|&k| knots[k] < knots[k + 1]).count()
        };
        let initial = spans(&s.knots_u, s.degree_u, s.control_points.len()).saturating_mul(spans(
            &s.knots_v,
            s.degree_v,
            s.control_points[0].len(),
        ));
        if remaining < initial {
            report.unresolved.push(Unresolved {
                face,
                uv,
                reason: "work-limit",
            });
            continue;
        }
        let roots = ray_surface::intersections(s, point, direction, tolerance_uv, remaining)?;
        report.cells += roots.cells;
        // An unresolved line cell can only be discarded if its whole image is
        // behind the origin or its complete UV region is outside the trim.
        for uv in roots.unresolved {
            if ray_surface::parameter_bounds(s, uv, point, direction)?[1] < 0. {
                continue;
            }
            let remaining = max_domain_cells - report.domain_cells;
            if remaining == 0 {
                report.unresolved.push(Unresolved {
                    face,
                    uv,
                    reason: "domain-work-limit",
                });
                continue;
            }
            let (location, cells) = classify(face, uv, remaining.min(4096))?;
            if cells > remaining {
                return Err(Error::new(
                    "BREP_INVALID_INPUT",
                    "Trim callback exceeded its work allowance",
                ));
            }
            report.domain_cells += cells;
            if location != Location::Outside {
                report.unresolved.push(Unresolved {
                    face,
                    uv,
                    reason: "root-not-isolated",
                })
            }
        }
        for root in roots.roots {
            if root.parameter[1] < 0. {
                continue;
            }
            let remaining = max_domain_cells - report.domain_cells;
            if remaining == 0 {
                report.unresolved.push(Unresolved {
                    face,
                    uv: root.uv,
                    reason: "domain-work-limit",
                });
                continue;
            }
            let (location, cells) = classify(face, root.uv, remaining.min(4096))?;
            if cells > remaining {
                return Err(Error::new(
                    "BREP_INVALID_INPUT",
                    "Trim callback exceeded its work allowance",
                ));
            }
            report.domain_cells += cells;
            if location == Location::Outside {
                continue;
            }
            let reason = if location != Location::Inside {
                Some("trim-boundary")
            } else if root.parameter[0] <= 0. {
                Some("origin-band")
            } else {
                None
            };
            if let Some(reason) = reason {
                report.unresolved.push(Unresolved {
                    face,
                    uv: root.uv,
                    reason,
                })
            } else {
                report.crossings.push(Crossing {
                    face,
                    uv: root.uv,
                    parameter: root.parameter,
                })
            }
        }
    }
    report
        .crossings
        .sort_by(|a, b| a.parameter[0].total_cmp(&b.parameter[0]));
    for pair in report.crossings.windows(2) {
        if pair[0].parameter[1] >= pair[1].parameter[0] {
            report.unresolved.push(Unresolved {
                face: pair[1].face,
                uv: pair[1].uv,
                reason: "overlapping-root-intervals",
            })
        }
    }
    if report.unresolved.is_empty() {
        report.parity = Some(report.crossings.len() % 2 == 1)
    }
    Ok(report)
}
