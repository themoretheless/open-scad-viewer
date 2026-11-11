use super::*;

/// How the offset edge is driven: the fixed axis, its drive interval, the two offset distances and
/// the driver cell limit.
#[derive(Clone, Copy, Debug)]
pub struct EdgeDrive {
    pub axis: usize,
    pub interval: [f64; 2],
    pub distances: [f64; 2],
    pub driver_cells: usize,
}
/// UV tolerance and limits of the offset path pcurve stage.
#[derive(Clone, Copy)]
pub struct PcurveStage {
    pub tolerance_uv: f64,
    pub limits: nurbs_core::offset_path_pcurves::Limits,
}
/// Lift and contact tolerances (mm) and limits of the world lift stage.
#[derive(Clone, Copy)]
pub struct LiftStage {
    pub tolerance_mm: f64,
    pub contact_tolerance_mm: f64,
    pub limits: nurbs_core::curve_surface_lift::Limits,
}
/// Fresh source UV generation and 3D lifts with pointwise millimeter contact
/// bounds. A whole-chart point-map derivative bound propagates UV error.
/// Positional qualification does not admit exact coedges, normals or a solid.
pub fn propose_world_contacts(
    prepared: &PreparedEdge,
    drive: EdgeDrive,
    pcurve: PcurveStage,
    lift: LiftStage,
) -> Result<WorldContactReport> {
    let EdgeDrive { axis, interval: drive, distances, driver_cells } = drive;
    let PcurveStage { tolerance_uv, limits: pcurve_limits } = pcurve;
    let LiftStage { tolerance_mm: lift_tolerance_mm, contact_tolerance_mm, limits: lift_limits } = lift;
    if !lift_tolerance_mm.is_finite()
        || lift_tolerance_mm <= 0.
        || !contact_tolerance_mm.is_finite()
        || contact_tolerance_mm <= 0.
        || !(1..=100000).contains(&lift_limits.proposal_cells)
        || !(1..=100000).contains(&lift_limits.agreement_cells)
        || !(1..=100000).contains(&lift_limits.cells_per_attempt)
    {
        return Err(Error::new(
            "BREP_OFFSET_WORLD_INPUT",
            "Choose finite positive world tolerances and bounded lift work",
        ));
    }
    let pcurves = propose_source_pcurves(
        prepared,
        axis,
        drive,
        distances,
        driver_cells,
        tolerance_uv,
        pcurve_limits,
    )?;
    let mut out = WorldContactReport {
        pcurves,
        lifts: vec![],
        source_scale_upper: [None; 2],
        contact_error_upper_mm: [None; 2],
        world_contacts_proven: false,
        reason: "source-contact-pcurves-unqualified",
    };
    let Some(pc) = &out.pcurves.pcurves else {
        return Ok(out);
    };
    if !pc.source_pcurves_proven {
        return Ok(out);
    }
    for side in 0..2 {
        let surface = &prepared.supports[side].face.face().surface;
        let mut original = vec![];
        let mut uv_error = 0f64;
        for piece in &pc.pieces {
            let attempt = &pc.attempts[piece.attempt.unwrap()];
            original.push(attempt.curves.as_ref().unwrap()[side].clone());
            for cell in &attempt.correspondence[side].cells {
                let Some(error) = cell.error_upper_uv else {
                    return Err(Error::new(
                        "BREP_OFFSET_WORLD_EVIDENCE",
                        "A qualified contact pcurve has no complete error bound",
                    ));
                };
                if !cell.admitted || !error.is_finite() || error < 0. {
                    return Err(Error::new(
                        "BREP_OFFSET_WORLD_EVIDENCE",
                        "A qualified contact pcurve has inconsistent error evidence",
                    ));
                }
                uv_error = uv_error.max(error);
            }
        }
        let lift = nurbs_core::curve_surface_lift::propose(
            surface,
            &original,
            lift_tolerance_mm,
            lift_limits,
        )?;
        out.source_scale_upper[side] = nurbs_core::curve_surface_lift::lipschitz_upper(
            surface,
            pcurve_limits.trims.path.spans,
        )?;
        if lift.source_lift_proven {
            if let Some(scale) = out.source_scale_upper[side] {
                let upper = ((uv_error * scale).next_up() + lift_tolerance_mm).next_up();
                if upper.is_finite() {
                    out.contact_error_upper_mm[side] = Some(upper);
                }
            }
        }
        out.lifts.push(lift);
    }
    out.world_contacts_proven = out.lifts.iter().all(|r| r.source_lift_proven)
        && out
            .contact_error_upper_mm
            .iter()
            .all(|r| r.is_some_and(|x| x <= contact_tolerance_mm));
    out.reason = if out.world_contacts_proven {
        "source-world-contact-tolerance-proven"
    } else if out.lifts.iter().any(|r| !r.source_lift_proven) {
        "source-world-lift-unqualified"
    } else if out.source_scale_upper.iter().any(|r| r.is_none()) {
        "source-point-map-scale-unresolved"
    } else {
        "source-world-contact-bound-exceeds-tolerance"
    };
    Ok(out)
}
