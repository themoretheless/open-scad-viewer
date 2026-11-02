//! Actual saved charts, trim regions and complete boundary lifts. Numerical
//! mass establishes an orientation diagnostic, never a global volume proof.
use super::*;
mod periodic_charts;
mod pole_charts;
pub(crate) fn limits(o: Limits) -> Result<()> {
    if !(1..=100000).contains(&o.max_cells)
        || !(1..=100000).contains(&o.max_pairs)
        || !(1..=1000000).contains(&o.max_domain_cells)
        || !(100..=2000000).contains(&o.mass_evaluations)
    {
        return Err(error("Choose bounded original feature proof/analysis work"));
    }
    Ok(())
}
pub(crate) fn verify(m: &Model, o: Limits) -> Result<Verification> {
    verify_impl(m, o, false, false)
}
/// Edge refinement admits proved periodic FACE enclosures and independent
/// original whole-chart monotonicity certificates for ordinary charts.
pub(crate) fn verify_edge_refinement(m: &Model, o: Limits) -> Result<Verification> {
    verify_impl(m, o, true, true)
}
/// Additional full-original-chart monotonicity for constructors that retain
/// general saved carriers. Regularity and all boundary/solid gates still apply.
pub(crate) fn verify_with_linear_charts(m: &Model, o: Limits) -> Result<Verification> {
    verify_impl(m, o, false, true)
}
fn verify_impl(
    m: &Model,
    o: Limits,
    edge_refinement: bool,
    linear_charts: bool,
) -> Result<Verification> {
    limits(o)?;
    m.validate()?;
    if m.is_empty() {
        return Ok(Verification {
            boundary_complete: true,
            trim_regions_valid: true,
            charts_injective: true,
            cells: 0,
            pairs: 0,
            domain_cells: 0,
            numerical_volume_mm3: 0.,
            audit: crate::solid_audit::audit_solid(m)?,
        });
    }
    let mut remaining = o.max_cells;
    let mut chart_pairs = 0;
    for (face, f) in m.faces.iter().enumerate() {
        if remaining == 0 {
            return Err(error("Feature chart proof work exhausted"));
        }
        if edge_refinement && (f.surface.periodic_u || f.surface.periodic_v) {
            let regular =
                periodic_charts::verify(m, face, &mut remaining, &mut chart_pairs, o.max_pairs)?;
            if !regular {
                if remaining == 0 {
                    return Err(error("Feature regularity proof work exhausted"));
                }
                let r = nurbs_core::surface_regularity::inspect(&f.surface, remaining)?;
                remaining = remaining.saturating_sub(r.cells);
                if !r.spanwise_regular {
                    return Err(error("Saved periodic face regularity remains unproven"));
                }
            }
            continue;
        }
        if edge_refinement && pole_charts::verify(m, face, &mut remaining)? {
            continue;
        }
        let r = nurbs_core::surface_regularity::inspect(&f.surface, remaining)?;
        remaining = remaining.saturating_sub(r.cells);
        if !r.spanwise_regular {
            return Err(error("Saved feature chart regularity remains unproven"));
        }
        if remaining == 0 {
            return Err(error("Feature injectivity proof work exhausted"));
        }
        let r = nurbs_core::surface_injectivity::certify(&f.surface, remaining)?;
        remaining = remaining.saturating_sub(r.spans);
        if !r.proven {
            if !linear_charts || remaining == 0 {
                return Err(error("Saved feature chart injectivity remains unproven"));
            }
            let linear =
                nurbs_core::surface_linear_monotonicity::inspect_candidate(&f.surface, remaining)?;
            remaining = remaining.saturating_sub(linear.cells);
            if !linear.certified {
                return Err(error(
                    "Saved feature whole-chart monotonicity remains unproven",
                ));
            }
        }
    }
    if remaining == 0 {
        return Err(error("Feature boundary proof work exhausted"));
    }
    let boundary = crate::boundary_agreement::verify(m, remaining)?;
    remaining = remaining.saturating_sub(boundary.cells);
    if !boundary.complete {
        return Err(error(
            "Saved feature edge/pcurve agreement remains unproven",
        ));
    }
    // Authored trim correspondence may cover only a subrange. The face region
    // uses complete stored pcurves, so partial metadata cannot stand in for a
    // whole-boundary lift certificate.
    for face in &m.faces {
        for wire in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
            for c in &m.loops[wire].coedges {
                if c.trim().is_some() {
                    if remaining == 0 {
                        return Err(error("Complete trimmed-use proof work exhausted"));
                    }
                    let full = nurbs_core::curve_surface_agreement::verify(
                        &m.edges[c.edge].curve,
                        &c.pcurve,
                        &face.surface,
                        c.reversed,
                        m.tolerance_mm,
                        remaining,
                    )?;
                    remaining = remaining.saturating_sub(full.cells);
                    if full.status != nurbs_core::curve_surface_agreement::Status::WithinTolerance {
                        return Err(error(
                            "Complete original boundary is not proved outside its trim correspondence",
                        ));
                    }
                }
            }
        }
    }
    if remaining == 0 {
        return Err(error("Feature trim proof work exhausted"));
    }
    let audit_trim = if edge_refinement {
        crate::face_domain::audit_edge_trim_regions
    } else {
        crate::face_domain::audit_trim_regions
    };
    if chart_pairs >= o.max_pairs {
        return Err(error("Feature chart pair proof work exhausted"));
    }
    let trim = audit_trim(
        m,
        1e-8,
        o.max_pairs - chart_pairs,
        remaining,
        o.max_domain_cells,
    )?;
    remaining = remaining.saturating_sub(trim.cells);
    if !trim.all_valid {
        return Err(error(
            "Saved feature trim winding/simplicity remains unproven",
        ));
    }
    let audit = crate::solid_audit::audit_solid(m)?;
    let mass = crate::analysis::mass_properties(m, 1e-6, o.mass_evaluations)?;
    if mass.signed_volume_mm3 <= 0. {
        return Err(error(
            "Feature role has no resolved positive oriented volume",
        ));
    }
    if !m.persistent_naming_complete() {
        return Err(error("Feature naming provenance incomplete"));
    }
    Ok(Verification {
        boundary_complete: true,
        trim_regions_valid: true,
        charts_injective: true,
        cells: o.max_cells - remaining,
        pairs: chart_pairs + trim.pairs,
        domain_cells: trim.domain_cells,
        numerical_volume_mm3: mass.signed_volume_mm3,
        audit,
    })
}
