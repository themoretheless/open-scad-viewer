//! Restore wire definitions through the same native qualification gates.
use super::recipe_identity::same_recipe;
use super::*;

pub fn with_restored_transition_boundary<'p, 'b, T, F>(
    proposal: &'p Proposal<'b>,
    definition: value_codec::Value,
    limits: TransitionBoundaryReplayLimits,
    inspect: F,
) -> Result<TransitionBoundaryReplayReport<T>>
where
    F: for<'s> FnOnce(&TransitionBoundary<'s, 'p, 'b>) -> Result<T>,
{
    let invalid = || {
        Error::new(
            "BREP_PROFILE_TRANSITION_RESTORE",
            "Require an owned transition boundary recipe and bounded replay work",
        )
    };
    if definition["version"].as_u64() != Some(1)
        || definition["kind"].as_str() != Some("owned-profile-transition-boundary")
        || limits.controls > 1000000
        || limits.side_cells > 100000
        || limits.side_spans > 100000
    {
        return Err(invalid());
    }
    let recipes = definition["sides"].as_array().ok_or_else(invalid)?;
    if recipes.len() != 2 {
        return Err(invalid());
    }
    let pending = |reason| TransitionBoundaryReplayReport {
        result: None,
        reason,
    };
    let first = restore_root_ended_seam(proposal, recipes[0].clone(), limits.seam)?;
    let Some(a) = first.seam else {
        return Ok(pending(first.reason));
    };
    let second = restore_root_ended_seam(proposal, recipes[1].clone(), limits.seam)?;
    let Some(b) = second.seam else {
        return Ok(pending(second.reason));
    };
    if a.face != b.face || a.axis != b.axis || a.parameters != b.parameters || a.high == b.high {
        return Err(invalid());
    }
    let profile = proposal.profile(a.face).ok_or_else(invalid)?;
    let width = a.parameters[1] - a.parameters[0];
    let bridge = nurbs_core::normal_alignment::ProfileSectionBridge::new(
        profile.section(a.axis, a.parameters[0])?,
        profile.section(a.axis, a.parameters[1])?,
        [width; 2],
    )?;
    let (bridge, _) = bridge.constrain_original_rails(limits.controls)?;
    let Some(bridge) = bridge else {
        return Ok(pending("profile-transition-replay-rail-recipe-unproven"));
    };
    let first = a.qualify_transition_side(&bridge, limits.side_cells, limits.side_spans)?;
    let Some(first) = first.side else {
        return Ok(pending(first.reason));
    };
    let second = b.qualify_transition_side(&bridge, limits.side_cells, limits.side_spans)?;
    let Some(second) = second.side else {
        return Ok(pending(second.reason));
    };
    let boundary = propose_transition_boundary([&first, &second])?;
    Ok(TransitionBoundaryReplayReport {
        result: Some(inspect(&boundary)?),
        reason: "profile-transition-boundary-replayed",
    })
}

pub fn restore_incident_contours<'p, 'b>(
    proposal: &'p Proposal<'b>,
    definition: value_codec::Value,
    seam_limits: RootSeamReplayLimits,
    max_fragments: usize,
    max_exact_work: u64,
    max_driver_cells: usize,
) -> Result<RootedContourReplayReport<'p, 'b>> {
    if max_fragments > 100000 || max_exact_work > 100000000 || max_driver_cells > 100000 {
        return Err(Error::new(
            "BREP_PROFILE_CONTOUR_RESTORE",
            "Bound incident contour and shared carrier replay work",
        ));
    }
    let rooted = restore_root_ended_seam(proposal, definition, seam_limits)?;
    let mut out = RootedContourReplayReport {
        assembly: None,
        fragments: 0,
        exact_work: 0,
        driver_cells: 0,
        reason: rooted.reason,
    };
    let Some(seam) = rooted.seam else {
        return Ok(out);
    };
    let contours = seam.propose_incident_contours(max_fragments)?;
    out.fragments = contours.fragments;
    out.reason = contours.reason;
    let Some(contours) = contours.proposal else {
        return Ok(out);
    };
    let carrier = seam.qualify_incident_edge(max_exact_work, max_driver_cells)?;
    out.exact_work = carrier.work_used;
    out.driver_cells = carrier.driver_cells;
    out.reason = carrier.reason;
    let Some(edge) = carrier.edge else {
        return Ok(out);
    };
    out.assembly = Some(RootedContourReplay {
        seam,
        contours,
        edge,
    });
    out.reason = "profile-incident-contours-and-carrier-replayed";
    Ok(out)
}

pub fn restore_root_ended_seam<'p, 'b>(
    proposal: &'p Proposal<'b>,
    definition: value_codec::Value,
    limits: RootSeamReplayLimits,
) -> Result<RootEndedSeamReport<'p, 'b>> {
    let invalid = || {
        Error::new(
            "BREP_PROFILE_SEAM_RESTORE",
            "Require an owned root-ended seam recipe and bounded replay work",
        )
    };
    if definition["version"].as_u64() != Some(1)
        || definition["kind"].as_str() != Some("owned-root-ended-profile-seam")
        || !same_recipe(&definition["proposal"], &proposal.definition()?)
        || limits.cut_spans > 100000
        || limits.mapping_cells > 100000
        || limits.seam_spans > 100000
        || !(1..=100000).contains(&limits.angular_cells)
        || !limits.max_sine_squared.is_finite()
        || !(0. ..1.).contains(&limits.max_sine_squared)
    {
        return Err(invalid());
    }
    let ends = definition["ends"].as_array().ok_or_else(invalid)?;
    if ends.len() != 2 {
        return Err(invalid());
    }
    let pending = |reason| RootEndedSeamReport {
        seam: None,
        carrier_spans: 0,
        angular: None,
        reason,
    };
    let first = restore_cut_endpoint(proposal, ends[0].clone(), limits.cut_spans)?;
    let Some(a) = first.endpoint else {
        return Ok(pending(first.reason));
    };
    let second = restore_cut_endpoint(
        proposal,
        ends[1].clone(),
        limits.cut_spans.saturating_sub(first.spans),
    )?;
    let Some(b) = second.endpoint else {
        return Ok(pending(second.reason));
    };
    if limits.mapping_cells == 0 {
        return Ok(pending("profile-root-seam-mapping-work-limit"));
    }
    let first = a.split_source_use(limits.mapping_cells)?;
    let Some(pa) = first.partition else {
        return Ok(pending(first.reason));
    };
    let remaining_mapping = limits.mapping_cells.saturating_sub(first.mapping_cells);
    if remaining_mapping == 0 {
        return Ok(pending("profile-root-seam-mapping-work-limit"));
    }
    let second = b.split_source_use(remaining_mapping)?;
    let Some(pb) = second.partition else {
        return Ok(pending(second.reason));
    };
    proposal.qualify_root_ended_seam(
        [&pa, &pb],
        limits.max_sine_squared,
        limits.angular_cells,
        limits.seam_spans,
    )
}

pub fn restore_cut_endpoint<'p, 'b>(
    proposal: &'p Proposal<'b>,
    definition: value_codec::Value,
    max_spans: usize,
) -> Result<CutEndpointReport<'p, 'b>> {
    use value_codec::Deserialize;
    let invalid = || {
        Error::new(
            "BREP_PROFILE_CUT_RESTORE",
            "Require an owned profile section-end recipe",
        )
    };
    if definition["version"].as_u64() != Some(1)
        || definition["kind"].as_str() != Some("owned-profile-section-end")
    {
        return Err(invalid());
    }
    proposal.cut_endpoint(
        usize::from_value(definition["face"].clone()).map_err(|_| invalid())?,
        usize::from_value(definition["axis"].clone()).map_err(|_| invalid())?,
        f64::from_value(definition["parameter"].clone()).map_err(|_| invalid())?,
        bool::from_value(definition["highFreeEnd"].clone()).map_err(|_| invalid())?,
        usize::from_value(definition["edge"].clone()).map_err(|_| invalid())?,
        max_spans,
    )
}

pub fn restore_section_bridge<'p, 'b>(
    proposal: &'p Proposal<'b>,
    definition: value_codec::Value,
) -> Result<nurbs_core::normal_alignment::ProfileSectionBridge<'p, 'b, 'p, 'b>> {
    use value_codec::Deserialize;
    let invalid = || {
        Error::new(
            "BREP_PROFILE_BRIDGE_RESTORE",
            "Require an owned two-section profile bridge recipe",
        )
    };
    if definition["version"].as_u64() != Some(1)
        || definition["kind"].as_str() != Some("exact-profile-section-bridge")
        || !same_recipe(&definition["proposal"], &proposal.definition()?)
    {
        return Err(invalid());
    }
    let rows = definition["sections"].as_array().ok_or_else(invalid)?;
    if rows.len() != 2 {
        return Err(invalid());
    }
    let section = |row: &value_codec::Value| {
        let face = usize::from_value(row["face"].clone()).map_err(|_| invalid())?;
        let axis = usize::from_value(row["axis"].clone()).map_err(|_| invalid())?;
        let parameter = f64::from_value(row["parameter"].clone()).map_err(|_| invalid())?;
        proposal
            .profile(face)
            .ok_or_else(invalid)?
            .section(axis, parameter)
    };
    let scales = <[f64; 2]>::from_value(definition["scales"].clone()).map_err(|_| invalid())?;
    nurbs_core::normal_alignment::ProfileSectionBridge::new(
        section(&rows[0])?,
        section(&rows[1])?,
        scales,
    )
}

pub fn restore<'a>(
    body: &'a Body,
    definition: value_codec::Value,
    limits: Limits,
) -> Result<Report<'a>> {
    use value_codec::Deserialize;
    let invalid = || {
        Error::new(
            "BREP_PROFILE_BODY_RESTORE",
            "Require an owned original body and a valid exact profile recipe",
        )
    };
    if definition["version"].as_u64() != Some(1)
        || definition["kind"].as_str() != Some("boundary-preserving-weight-profiles")
        || !same_recipe(&definition["originalBody"], &body.definition()?)
    {
        return Err(invalid());
    }
    let tolerance =
        f64::from_value(definition["positionTolerance"].clone()).map_err(|_| invalid())?;
    if !tolerance.is_finite()
        || tolerance < 0.
        || tolerance.to_bits() != limits.position_tolerance.to_bits()
    {
        return Err(invalid());
    }
    let rows = definition["profiles"].as_array().ok_or_else(invalid)?;
    let face_count = body.geometry().shell().faces().len();
    if rows.is_empty() || rows.len() > face_count {
        return Err(invalid());
    }
    let specs = rows
        .iter()
        .map(|row| {
            Ok(FaceSpec {
                face: usize::from_value(row["face"].clone()).map_err(|_| invalid())?,
                anchor: <[usize; 2]>::from_value(row["anchor"].clone()).map_err(|_| invalid())?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    prepare(body, &specs, limits)
}
