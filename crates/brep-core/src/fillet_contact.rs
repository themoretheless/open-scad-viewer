//! Automatic fillet contact proposals admitted only by whole-interval lifts.
//! Spine sampling proposes geometry; it never certifies contact or radius.
use nurbs_core::{
    Result,
    curve::Curve,
    natural_loft,
    offset::fillet_surfaces::{self, FilletOptions, FilletReport, RadiusLaw},
    surface::{Axis, Surface},
    surface_g2_pcurves::{self, Positions},
};
#[derive(Clone)]
pub struct Contact {
    /// One owned 3D definition, extracted from the actual authored skin.
    pub curve: Curve,
    pub support_pcurve: Curve,
    pub fillet_pcurve: Curve,
    pub positions: Positions,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    VerifiedContacts,
    UnresolvedContacts,
    BuildFailed,
    ClosedSpineUnsupported,
    SmoothSkinUnresolved,
}
pub struct Report {
    pub fillet: FilletReport,
    /// Available only when both complete original-support lifts qualify.
    pub contacts: Option<[Contact; 2]>,
    /// Fresh attempts in support order, retained even when a lift is refused.
    /// An absent entry means that side was not visited within the shared budget.
    pub contact_attempts: [Option<Contact>; 2],
    pub status: Status,
    pub cells: usize,
    pub exact_work: u64,
    /// Whole-surface bound for optional C2 storage preparation.
    pub skin_edit_error_upper: Option<f64>,
    pub skin_edit_work: usize,
}
fn uv_proposal(report: &FilletReport, side: usize) -> Result<Curve> {
    if let Some(pcurves) = &report.contact_pcurves {
        return Ok(pcurves[side].clone());
    }
    let sections = report
        .spine
        .iter()
        .map(|s| {
            let uv = if side == 0 { s.uv_a } else { s.uv_b };
            Curve::from_polyline(vec![vec![uv[0], uv[1], 0.], vec![uv[0], uv[1], 1.]])
        })
        .collect::<Result<Vec<_>>>()?;
    let params = report.spine.iter().map(|s| s.t).collect::<Vec<_>>();
    // Same natural interpolation and normalized station correspondence as the
    // skin. UV station interpolation is a proposal, not an approximation bound.
    let chart = natural_loft::interpolate(&sections, &params)?;
    let mut curve = chart.iso(Axis::U, 0.)?;
    for p in &mut curve.control_points {
        p.truncate(2);
    }
    curve.validate()?;
    Ok(curve)
}
/// Build general surface-pair variable-radius proposals and qualify each actual
/// contact against BOTH its original support and its authored fillet skin.
/// The shared tolerance bounds their combined residual, not independent full
/// tolerances per side. No supplied report or sampled deviation grants admission.
/// Status covers these two contact curves only: it does not prove radius law,
/// G1/G2, branch coverage, trimmed-face membership, ends, corners or a solid.
pub fn build(
    supports: [&Surface; 2],
    law: RadiusLaw,
    options: &FilletOptions,
    position_tolerance: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    if !position_tolerance.is_finite()
        || position_tolerance < 0.
        || max_cells > 100000
        || max_exact > 1000000
    {
        return Err(nurbs_core::Error::new(
            "BREP_FILLET_CONTACT_LIMITS",
            "Contact qualification needs finite tolerance and bounded shared work",
        ));
    }
    let fillet = fillet_surfaces::variable_radius_fillet(supports[0], supports[1], law, options)?;
    qualify(supports, fillet, position_tolerance, max_cells, max_exact)
}
/// Propose quintic G2 sections and qualify their contacts. Section construction
/// alone does not prove G2 of the skinned surface; audit the actual seam next.
pub fn build_g2(
    supports: [&Surface; 2],
    radius: f64,
    options: &FilletOptions,
    position_tolerance: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    build_variable_g2(
        supports,
        RadiusLaw::Constant(radius),
        options,
        position_tolerance,
        max_cells,
        max_exact,
    )
}
/// Variable nominal-radius quintic section proposals with fresh C2 skin
/// preparation and complete contact lifts. VerifiedContacts concerns positions
/// only; actual whole-seam G2 and geometric radius laws need independent proofs.
pub fn build_variable_g2(
    supports: [&Surface; 2],
    law: RadiusLaw,
    options: &FilletOptions,
    position_tolerance: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    if !position_tolerance.is_finite()
        || position_tolerance < 0.
        || max_cells > 100000
        || max_exact > 1000000
    {
        return Err(nurbs_core::Error::new(
            "BREP_FILLET_CONTACT_LIMITS",
            "Contact qualification needs finite tolerance and bounded shared work",
        ));
    }
    let mut fillet = fillet_surfaces::variable_g2_fillet(supports[0], supports[1], law, options)?;
    let prepared = if fillet.surface.is_some() && fillet.failure.is_none() && !fillet.closed_spine {
        prepare_c2_skin(&mut fillet, position_tolerance)?
    } else {
        (Some(0.), 0)
    };
    let mut out = qualify(supports, fillet, position_tolerance, max_cells, max_exact)?;
    out.skin_edit_error_upper = prepared.0;
    out.skin_edit_work = prepared.1;
    if prepared.0.is_none() {
        out.contacts = None;
        out.status = Status::SmoothSkinUnresolved;
    }
    Ok(out)
}
fn qualify(
    supports: [&Surface; 2],
    fillet: FilletReport,
    position_tolerance: f64,
    max_cells: usize,
    max_exact: u64,
) -> Result<Report> {
    let mut out = Report {
        fillet,
        contacts: None,
        contact_attempts: [None, None],
        status: Status::BuildFailed,
        cells: 0,
        exact_work: 0,
        skin_edit_error_upper: None,
        skin_edit_work: 0,
    };
    if out.fillet.surface.is_none() || out.fillet.failure.is_some() {
        return Ok(out);
    }
    if out.fillet.closed_spine {
        out.status = Status::ClosedSpineUnsupported;
        return Ok(out);
    }
    out.status = Status::UnresolvedContacts;
    if max_cells == 0 {
        return Ok(out);
    }
    let skin = out.fillet.surface.as_ref().unwrap();
    let u = [
        skin.knots_u[skin.degree_u],
        skin.knots_u[skin.control_points.len()],
    ];
    let v = [
        skin.knots_v[skin.degree_v],
        skin.knots_v[skin.control_points[0].len()],
    ];
    let mut contacts = Vec::with_capacity(2);
    for side in 0..2 {
        if out.cells == max_cells {
            return Ok(out);
        }
        let curve = skin.iso(Axis::U, u[side])?;
        let support_pcurve = uv_proposal(&out.fillet, side)?;
        let fillet_pcurve = Curve::from_polyline(vec![vec![u[side], v[0]], vec![u[side], v[1]]])?;
        let positions = surface_g2_pcurves::verify_positions(
            &curve,
            [&support_pcurve, &fillet_pcurve],
            [supports[side], skin],
            [false, false],
            position_tolerance,
            max_cells - out.cells,
            max_exact - out.exact_work,
        )?;
        out.cells += positions.cells;
        out.exact_work += positions.exact_work;
        let contact = Contact {
            curve,
            support_pcurve,
            fillet_pcurve,
            positions,
        };
        out.contact_attempts[side] = Some(contact.clone());
        if !contact.positions.proven {
            return Ok(out);
        }
        contacts.push(contact);
    }
    let mut iter = contacts.into_iter();
    out.contacts = Some([iter.next().unwrap(), iter.next().unwrap()]);
    out.status = Status::VerifiedContacts;
    Ok(out)
}

// Same Bernstein-product coefficient accounting as positional_upper, accumulated
// over all knot removals and the final ORIGINAL-skin comparison.
fn deviation_work(a: &Surface, b: &Surface) -> usize {
    let spans = |x: &[f64], y: &[f64]| {
        let mut knots = x.iter().chain(y).copied().collect::<Vec<_>>();
        knots.sort_by(f64::total_cmp);
        knots.dedup();
        knots.len() - 1
    };
    spans(&a.knots_u, &b.knots_u)
        .saturating_mul(spans(&a.knots_v, &b.knots_v))
        .saturating_mul((a.degree_u + 1) * (a.degree_v + 1))
        .saturating_mul((b.degree_u + 1) * (b.degree_v + 1))
}
fn prepare_c2_skin(fillet: &mut FilletReport, tolerance: f64) -> Result<(Option<f64>, usize)> {
    let mut work = 0usize;
    let original = fillet.surface.as_ref().unwrap();
    let mut candidate = original.clone();
    let degree = candidate.degree_v;
    if degree < 2 || tolerance <= 0. {
        return Ok((None, work));
    }
    let domain = [
        candidate.knots_v[degree],
        candidate.knots_v[candidate.control_points[0].len()],
    ];
    let mut interior = candidate
        .knots_v
        .iter()
        .copied()
        .filter(|&k| domain[0] < k && k < domain[1])
        .collect::<Vec<_>>();
    interior.sort_by(f64::total_cmp);
    interior.dedup();
    let removals = interior
        .iter()
        .map(|&k| {
            candidate
                .knots_v
                .iter()
                .filter(|&&t| t == k)
                .count()
                .saturating_sub(degree - 2)
        })
        .sum::<usize>();
    if removals == 0 {
        return Ok((Some(0.), work));
    }
    let per_edit = (tolerance * 0.25 / removals as f64).next_down();
    if per_edit <= 0. {
        return Ok((None, work));
    }
    for knot in interior {
        while candidate.knots_v.iter().filter(|&&t| t == knot).count() > degree - 2 {
            let cost = deviation_work(&candidate, &candidate);
            if work.saturating_add(cost) > 2000000 {
                return Ok((None, work));
            }
            work += cost;
            let edit = nurbs_core::foundation::approximate_edits::remove_surface_knot_report(
                &candidate,
                Axis::V,
                knot,
                per_edit,
                None,
            )?;
            if !edit.certificate.accepted {
                return Ok((None, work));
            }
            candidate = edit.surface;
        }
    }
    let cost = deviation_work(original, &candidate);
    if work.saturating_add(cost) > 2000000 {
        return Ok((None, work));
    }
    work += cost;
    let error = nurbs_core::continuity::deviation::positional_upper(original, &candidate)?;
    if error > tolerance * 0.25 {
        return Ok((None, work));
    }
    fillet.surface = Some(candidate);
    fillet.skin_deviation_estimate = (fillet.skin_deviation_estimate + error).next_up();
    fillet.warnings.push(format!(
        "C2 storage refit: whole-skin positional change <= {error:e}; spine stations remain proposals and radius needs fresh qualification"
    ));
    Ok((Some(error), work))
}

/// Build a coordinate-station radius proposal and freshly inspect the actual
/// contact lifts. Radius itself still requires a whole-skin saved-face proof.
pub fn build_linear_at_stations(
    supports:[&Surface;2],coordinate:usize,stations:[f64;2],radii:[f64;2],options:&FilletOptions,
    position_tolerance:f64,max_cells:usize,max_exact:u64,
)->Result<(Report,[f64;4])>{
    if !position_tolerance.is_finite() || position_tolerance<0. || max_cells>100000 || max_exact>1000000 {
        return Err(nurbs_core::Error::new("BREP_FILLET_CONTACT_LIMITS","Station contact qualification needs bounded finite work"));
    }
    let proposal=fillet_surfaces::linear_at_stations(supports[0],supports[1],coordinate,stations,radii,options)?;
    if !proposal.converged{return Err(nurbs_core::Error::new("BREP_STATION_RADIUS_UNPROVEN","Coordinate-station radius proposal did not converge"));}
    let field=proposal.radius_field;
    Ok((qualify(supports,proposal.fillet,position_tolerance,max_cells,max_exact)?,field))
}
