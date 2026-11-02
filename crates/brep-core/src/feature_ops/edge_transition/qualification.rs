//! Geometry-specific admission on actual saved closed transition topology.
use super::*;
pub(super) fn radius(
    mut topology: TopologyTransition,
    patch: RadiusPatch<'_>,
) -> Result<Transition> {
    let model = &topology.cap.replacement.model;
    let mut seams = Vec::new();
    for edges in &topology.rails {
        for &actual in edges {
            let mut uses = Vec::new();
            for (f, face) in model.faces.iter().enumerate() {
                for w in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                    for c in &model.loops[w].coedges {
                        if c.edge == actual {
                            uses.push((f, c));
                        }
                    }
                }
            }
            if uses.len() != 2 {
                return Err(refused(
                    "Transition contact lacks exactly two actual saved uses",
                ));
            }
            let report = surface_g1_pcurves::inspect(
                &model.edges[actual].curve,
                [&uses[0].1.pcurve, &uses[1].1.pcurve],
                [
                    &model.faces[uses[0].0].surface,
                    &model.faces[uses[1].0].surface,
                ],
                [uses[0].1.reversed, uses[1].1.reversed],
                model.tolerance_mm,
                topology.work.limits.normal_tolerance,
                topology.work.allowance(4)?,
                topology.work.exact_remaining() / 4,
            )?;
            topology.work.charge(report.cells, report.work())?;
            if !report.positions.proven
                || !report.regularity_proven
                || report.decision != Decision::WithinTolerance
            {
                return Err(refused(&format!(
                    "Actual saved original-support/transition G1 remains unproven: edge {actual}, faces {:?}, positions {}, regularity {}, decision {:?}, cells {}",
                    uses.iter().map(|(f, _)| *f).collect::<Vec<_>>(),
                    report.positions.proven,
                    report.regularity_proven,
                    report.decision,
                    report.cells
                )));
            }
            seams.push(report);
        }
    }
    let carrier = &model.faces[topology.cap.replacement.face].surface;
    let proof_surface = if matches!(patch.spine_axis, Axis::V) {
        nurbs_core::surface_edit::transpose(carrier)?
    } else {
        carrier.clone()
    };
    let mut radius = nurbs_core::moving_radius::qualify(
        &proof_surface,
        patch.centers,
        patch.radius,
        model.tolerance_mm,
        topology.work.allowance(1)?,
        topology.work.exact_remaining(),
    )?;
    topology.work.charge(radius.cells, radius.work)?;
    let radius_reason = radius.reason;
    let radius_error = radius.error_upper;
    let radius = radius
        .certificate
        .take()
        .ok_or_else(|| refused(&format!("Whole ACTUAL transition radius law remains unproven: {radius_reason}; upper={radius_error:?}")))?;
    let mut envelope = nurbs_core::moving_envelope::qualify(
        radius,
        nurbs_core::moving_envelope::Limits {
            max_sine_squared: (topology.work.limits.normal_tolerance * 0.5)
                .powi(2)
                .min(0.25),
            cells: topology.work.allowance(1)?,
            surface_spans: topology.work.allowance(1)?,
            center_spans: topology.work.allowance(1)?,
            radial_work: topology.work.exact_remaining(),
        },
    )?;
    topology.work.charge(envelope.cells, envelope.radial_work)?;
    let envelope = envelope.envelope.take().ok_or_else(|| {
        refused(&format!(
            "Whole ACTUAL sphere envelope remains unproven: {}; {:?}",
            envelope.reason, envelope.uncertain_uv
        ))
    })?;
    Ok(Transition {
        topology,
        seams,
        envelope,
    })
}
