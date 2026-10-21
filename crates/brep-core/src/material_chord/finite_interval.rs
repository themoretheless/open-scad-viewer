//! Complete finite line-interval boundary coverage plus conditional material parity.
//! One chord is not a minimum wall-thickness certificate.
use crate::{Error, Model, Result, face_domain::FaceDomain};
use nurbs_core::{ray_surface, trim_domain::Location};
#[derive(Debug)]
pub struct Crossing {
    pub face: usize,
    pub uv: [[f64; 2]; 2],
    pub parameter: [f64; 2],
}
#[derive(Debug)]
pub struct Unresolved {
    pub face: usize,
    pub uv: [[f64; 2]; 2],
    pub reason: &'static str,
}
#[derive(Debug)]
pub struct IntervalReport {
    /// Length of this specified interior interval, not total wall thickness.
    pub length_mm: [f64; 2],
    pub boundary_free: bool,
    pub coverage_complete: bool,
    pub crossings: Vec<Crossing>,
    pub unresolved: Vec<Unresolved>,
    pub cells: usize,
    pub domain_cells: usize,
}
/// The exact represented line is origin+direction*t. Its CLOSED finite interval
/// must contain zero, so parity can anchor at the original authored origin,
/// without substituting a rounded computed midpoint. Endpoint root bands refuse.
pub fn inspect_interval(
    model: &Model,
    origin: [f64; 3],
    direction: [f64; 3],
    interval: [f64; 2],
    tolerance_uv: f64,
    max_cells: usize,
    max_domains: usize,
) -> Result<IntervalReport> {
    model.validate()?;
    if model.faces.is_empty()
        || !origin
            .iter()
            .chain(direction.iter())
            .chain(interval.iter())
            .all(|x| x.is_finite())
        || direction.iter().all(|x| *x == 0.)
        || !(interval[0] < 0. && interval[1] > 0.)
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || max_cells > 100000
        || max_domains > 1000000
    {
        return Err(Error::new(
            "BREP_CHORD_INPUT",
            "Need an authored finite line interval containing its origin and bounded work",
        ));
    }
    let domains = (0..model.faces.len())
        .map(|f| FaceDomain::new(model, f, tolerance_uv))
        .collect::<Result<Vec<_>>>()?;
    use nurbs_core::interval_eval::Interval as I;
    let mut squared = I::point(0.);
    for d in direction {
        squared = squared.add(I::point(d).mul(I::point(d))?)?;
    }
    let norm = I::new(
        squared.lo.max(0.).sqrt().next_down().max(0.),
        squared.hi.sqrt().next_up(),
    )?;
    let length = norm.mul(I::point(interval[1]).sub(I::point(interval[0]))?)?;
    let mut out = IntervalReport {
        length_mm: [length.lo, length.hi],
        boundary_free: false,
        coverage_complete: false,
        crossings: vec![],
        unresolved: vec![],
        cells: 0,
        domain_cells: 0,
    };
    for (face, f) in model.faces.iter().enumerate() {
        let s = &f.surface;
        let uv = [
            [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
            [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
        ];
        let outside = |t: [f64; 2]| t[1] < interval[0] || t[0] > interval[1];
        if outside(ray_surface::parameter_bounds(s, uv, origin, direction)?) {
            continue;
        }
        let spans = |knots: &[f64], degree: usize, n: usize| {
            (degree..n).filter(|&i| knots[i] < knots[i + 1]).count()
        };
        let initial = spans(&s.knots_u, s.degree_u, s.control_points.len())
            * spans(&s.knots_v, s.degree_v, s.control_points[0].len());
        if max_cells - out.cells < initial {
            out.unresolved.push(Unresolved {
                face,
                uv,
                reason: "work-limit",
            });
            continue;
        }
        let roots =
            ray_surface::intersections(s, origin, direction, tolerance_uv, max_cells - out.cells)?;
        out.cells += roots.cells;
        for uv in roots.unresolved {
            if outside(ray_surface::parameter_bounds(s, uv, origin, direction)?) {
                continue;
            }
            if out.domain_cells == max_domains {
                out.unresolved.push(Unresolved {
                    face,
                    uv,
                    reason: "domain-work-limit",
                });
                continue;
            }
            let trim = domains[face].classify(uv, (max_domains - out.domain_cells).min(4096))?;
            out.domain_cells += trim.cells;
            if trim.location != Location::Outside {
                out.unresolved.push(Unresolved {
                    face,
                    uv,
                    reason: "root-not-isolated",
                });
            }
        }
        for root in roots.roots {
            if outside(root.parameter) {
                continue;
            }
            if out.domain_cells == max_domains {
                out.unresolved.push(Unresolved {
                    face,
                    uv: root.uv,
                    reason: "domain-work-limit",
                });
                continue;
            }
            let trim =
                domains[face].classify(root.uv, (max_domains - out.domain_cells).min(4096))?;
            out.domain_cells += trim.cells;
            if trim.location == Location::Outside {
                continue;
            }
            if trim.location != Location::Inside {
                out.unresolved.push(Unresolved {
                    face,
                    uv: root.uv,
                    reason: "trim-boundary",
                });
            } else if root.parameter[0] <= interval[0] || root.parameter[1] >= interval[1] {
                out.unresolved.push(Unresolved {
                    face,
                    uv: root.uv,
                    reason: "interval-endpoint-band",
                });
            } else {
                out.crossings.push(Crossing {
                    face,
                    uv: root.uv,
                    parameter: root.parameter,
                });
            }
        }
    }
    out.coverage_complete = out.unresolved.is_empty();
    out.boundary_free = out.crossings.is_empty() && out.coverage_complete;
    Ok(out)
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    InsideMaterialInterval,
    BoundaryCrossing,
    OutsideMaterial,
    Unresolved,
}
pub struct Report {
    pub decision: Decision,
    pub interval: IntervalReport,
    pub parity: Option<crate::ray_parity::PointReport>,
    pub boundary: Option<crate::boundary_embedding::Report>,
    pub trim_regions: Option<crate::face_domain::TrimAudit>,
    pub closure: Option<crate::watertight::QualifiedReport>,
    pub volume_preconditions: Option<bool>,
    pub volume_refusal: Option<String>,
    /// True only for this specified closed interval, with fresh solid and
    /// boundary preconditions. It is not a surface-family thickness proof.
    pub material_interval_qualified: bool,
    pub cells: usize,
    pub domain_cells: usize,
}
/// Fresh topology/orientation/containment audit, complete trim regions, exact
/// saved-use closure, boundary absence, complete line coverage and point parity.
/// No caller-provided inside flag or old certificate
/// authorizes material qualification. Budgets are shared between numeric stages.
pub fn inspect(
    model: &Model,
    origin: [f64; 3],
    direction: [f64; 3],
    interval: [f64; 2],
    parity_directions: &[[f64; 3]],
    tolerance_uv: f64,
    max_cells: usize,
    max_domains: usize,
    max_exact: u64,
) -> Result<Report> {
    if max_cells < 4
        || max_cells > 100000
        || max_domains < 4
        || max_domains > 1000000
        || max_exact > 1000000
    {
        return Err(Error::new(
            "BREP_CHORD_LIMITS",
            "Material chord needs bounded shared work",
        ));
    }
    if parity_directions.is_empty()
        || parity_directions.len() > 16
        || parity_directions
            .iter()
            .any(|d| d.iter().any(|x| !x.is_finite()) || d.iter().all(|x| *x == 0.))
    {
        return Err(Error::new(
            "BREP_CHORD_INPUT",
            "Need bounded finite nonzero parity directions",
        ));
    }
    let chord = inspect_interval(
        model,
        origin,
        direction,
        interval,
        tolerance_uv,
        max_cells / 4,
        max_domains / 4,
    )?;
    let mut cells = chord.cells;
    let mut domains = chord.domain_cells;
    if !chord.boundary_free {
        let decision = if chord.crossings.is_empty() {
            Decision::Unresolved
        } else {
            Decision::BoundaryCrossing
        };
        return Ok(Report {
            decision,
            interval: chord,
            parity: None,
            boundary: None,
            trim_regions: None,
            closure: None,
            volume_preconditions: None,
            volume_refusal: None,
            material_interval_qualified: false,
            cells,
            domain_cells: domains,
        });
    }
    let audit = crate::solid_audit::audit_solid(model);
    let volume_preconditions = audit.as_ref().is_ok_and(|c| c.ok);
    let volume_refusal = audit.err().map(|e| format!("{}: {}", e.code, e.message));
    if !volume_preconditions || max_cells - cells < 4 || max_domains == domains || max_exact == 0 {
        return Ok(Report {
            decision: Decision::Unresolved,
            interval: chord,
            parity: None,
            boundary: None,
            trim_regions: None,
            closure: None,
            volume_preconditions: Some(volume_preconditions),
            volume_refusal,
            material_interval_qualified: false,
            cells,
            domain_cells: domains,
        });
    }
    let stage = ((max_cells - cells) / 8).max(1);
    let trims = crate::face_domain::audit_trim_regions(
        model,
        tolerance_uv,
        10000,
        stage,
        stage.min(max_domains - domains),
    )?;
    cells += trims.cells + trims.domain_cells;
    domains += trims.domain_cells;
    if !trims.all_valid {
        return Ok(Report {
            decision: Decision::Unresolved,
            interval: chord,
            parity: None,
            boundary: None,
            trim_regions: Some(trims),
            closure: None,
            volume_preconditions: Some(volume_preconditions),
            volume_refusal,
            material_interval_qualified: false,
            cells,
            domain_cells: domains,
        });
    }
    let closure = crate::watertight::inspect_with_exact(
        model,
        ((max_cells - cells) / 8).max(1),
        (max_exact / 4).max(1),
    )?;
    cells += closure.closure.agreement.cells;
    let closed = closure.closure.status == crate::watertight::Closure::WithinTolerance
        && closure.exact.all_equal
        && closure.exact.all_joins_exact;
    if !closed
        || max_cells - cells < 16
        || max_domains - domains < 4
        || closure.exact.work == max_exact
    {
        return Ok(Report {
            decision: Decision::Unresolved,
            interval: chord,
            parity: None,
            boundary: None,
            trim_regions: Some(trims),
            closure: Some(closure),
            volume_preconditions: Some(volume_preconditions),
            volume_refusal,
            material_interval_qualified: false,
            cells,
            domain_cells: domains,
        });
    }
    let remaining = max_cells - cells;
    let trim_budget = (remaining / 16).max(1);
    let spans = 1024.min(trim_budget);
    let linear = remaining / 16;
    // Contact traversal owns most of this shared ledger on split/holed bodies.
    // Reserving half for the final anchor stranded unused work after a complete
    // boundary-free chord. Keep a bounded eighth for parity and give the rest
    // to the boundary proof; all stages still charge the original total limit.
    let parity_reserve = (remaining / 8).max(1);
    let contact_budget = remaining - 2 * trim_budget - spans - linear - parity_reserve;
    let boundary = crate::boundary_embedding::inspect_with_linear(
        model,
        tolerance_uv,
        crate::boundary_embedding::Limits {
            exact_work: max_exact - closure.exact.work,
            trim_pairs: 10000,
            trim_cells: trim_budget,
            trim_domain_cells: trim_budget.min((max_domains - domains) / 4).max(1),
            spans,
            contacts: crate::face_contacts::Limits {
                pairs: 10000,
                cells: contact_budget,
                domain_cells: (max_domains - domains) / 2,
                cells_per_pair: 1000,
                domain_cells_per_pair: 10000,
            },
        },
        linear,
    )?;
    cells += boundary.trim.cells
        + boundary.trim.domain_cells
        + boundary.intersections.faces.spans
        + boundary.intersections.faces.linear_cells
        + boundary.intersections.pairs.cells;
    domains += boundary.trim.domain_cells + boundary.intersections.pairs.domain_cells;
    let parity = if chord.boundary_free && cells < max_cells && domains < max_domains {
        Some(crate::ray_parity::classify_point(
            model,
            origin,
            parity_directions,
            tolerance_uv,
            max_cells - cells,
            max_domains - domains,
        )?)
    } else {
        None
    };
    if let Some(p) = &parity {
        cells += p.cells;
        domains += p.domain_cells;
    }
    let material_interval_qualified = volume_preconditions
        && boundary.proven
        && chord.boundary_free
        && parity.as_ref().is_some_and(|p| p.parity == Some(true));
    let decision = if material_interval_qualified {
        Decision::InsideMaterialInterval
    } else if !chord.crossings.is_empty() {
        Decision::BoundaryCrossing
    } else if volume_preconditions
        && boundary.proven
        && chord.boundary_free
        && parity.as_ref().is_some_and(|p| p.parity == Some(false))
    {
        Decision::OutsideMaterial
    } else {
        Decision::Unresolved
    };
    Ok(Report {
        decision,
        interval: chord,
        parity,
        boundary: Some(boundary),
        trim_regions: Some(trims),
        closure: Some(closure),
        volume_preconditions: Some(volume_preconditions),
        volume_refusal,
        material_interval_qualified,
        cells,
        domain_cells: domains,
    })
}
