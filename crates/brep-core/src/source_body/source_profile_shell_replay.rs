//! Scoped reconstruction of exact expression-shell incidence.
//! Embedding, material volume and thickness require subsequent fresh gates.
use super::*;
pub struct ShellReplayLimits {
    pub boundary: TransitionBoundaryReplayLimits,
    pub projection: [[f64; 3]; 2],
    pub cut_spans_per_section: usize,
    pub mapping_cells_per_section: usize,
    pub fragments_per_section: usize,
    pub controls: usize,
    pub join_cells: usize,
    pub join_spans: usize,
    pub exterior_drivers: usize,
    pub collapsed_ends: [usize; 2],
    pub subdivisions: usize,
    pub quotient_cells: usize,
    pub retained_cells: usize,
    pub retained_spans: usize,
    pub transition_cells: usize,
    pub transition_spans: usize,
    pub exact_work_per_rail: u64,
    pub drivers_per_rail: usize,
    pub fragments_per_rail: usize,
    pub outer_exact_work: u64,
    pub outer_drivers: usize,
    pub shell_uses: usize,
}
pub struct ShellReplayReport<T> {
    pub result: Option<T>,
    pub reason: &'static str,
}
/// Keep all reconstructed owners alive while the caller runs further native
/// qualification. Only owned results can leave this scoped reconstruction.
pub fn with_restored_expression_shell<'p, 'b, T, F>(
    proposal: &'p Proposal<'b>,
    definition: value_codec::Value,
    limits: ShellReplayLimits,
    inspect: F,
) -> Result<ShellReplayReport<T>>
where
    F: for<'x, 'o, 'i, 'f, 'a, 's> FnOnce(
        &ExpressionShell<'x, 'o, 'i, 'f, 'a, 's, 'p, 'b>,
    ) -> Result<T>,
{
    if [
        limits.cut_spans_per_section,
        limits.mapping_cells_per_section,
        limits.fragments_per_section,
        limits.controls,
        limits.join_cells,
        limits.join_spans,
        limits.exterior_drivers,
        limits.subdivisions,
        limits.quotient_cells,
        limits.retained_cells,
        limits.retained_spans,
        limits.transition_cells,
        limits.transition_spans,
        limits.drivers_per_rail,
        limits.fragments_per_rail,
        limits.outer_drivers,
        limits.shell_uses,
    ]
    .iter()
    .any(|n| *n > 100000)
        || limits.exact_work_per_rail > 100000000
        || limits.outer_exact_work > 100000000
    {
        return Err(Error::new(
            "BREP_PROFILE_SHELL_RESTORE",
            "Bound every shell reconstruction stage",
        ));
    }
    let replay =
        with_restored_transition_boundary(proposal, definition, limits.boundary, |boundary| {
            let pending = |reason| ShellReplayReport {
                result: None,
                reason,
            };
            let mut sections = Vec::new();
            for high in [false, true] {
                let report = boundary.replay_section_contours(
                    high,
                    limits.cut_spans_per_section,
                    limits.mapping_cells_per_section,
                    limits.fragments_per_section,
                )?;
                let Some(section) = report.section else {
                    return Ok(pending(report.reason));
                };
                sections.push(section);
            }
            let mut exteriors = Vec::new();
            for section in &sections {
                let joined =
                    section.qualify_join(limits.controls, limits.join_cells, limits.join_spans)?;
                if joined.join.is_none() {
                    return Ok(pending(joined.reason));
                }
                let report = section.qualify_exterior(limits.controls, limits.exterior_drivers)?;
                let Some(exterior) = report.region else {
                    return Ok(pending(report.reason));
                };
                exteriors.push(exterior);
            }
            let mut retained = Vec::new();
            for (end, exterior) in exteriors.iter().enumerate() {
                let report = exterior.qualify_face(
                    limits.collapsed_ends[end],
                    limits.subdivisions,
                    limits.quotient_cells,
                    limits.retained_cells,
                    limits.retained_spans,
                )?;
                let Some(face) = report.face else {
                    return Ok(pending(report.reason));
                };
                retained.push(face);
            }
            let report = boundary.qualify_face_from_rails(
                limits.projection,
                limits.controls,
                limits.transition_cells,
                limits.transition_spans,
            )?;
            let Some(face) = report.face else {
                return Ok(pending(report.reason));
            };
            let report = face.qualify_rail_incidence(
                limits.exact_work_per_rail,
                limits.drivers_per_rail,
                limits.fragments_per_rail,
            )?;
            let Some(rails) = report.incidence else {
                return Ok(pending(report.reason));
            };
            let report = rails.qualify_sections([&exteriors[0], &exteriors[1]])?;
            let Some(patch) = report.patch else {
                return Ok(pending(report.reason));
            };
            let report = patch.qualify_outer_edges(
                [&retained[0], &retained[1]],
                limits.outer_exact_work,
                limits.outer_drivers,
            )?;
            let Some(outer) = report.assembly else {
                return Ok(pending(report.reason));
            };
            let report = outer.assemble_shell(limits.shell_uses)?;
            let Some(shell) = report.shell else {
                return Ok(pending(report.reason));
            };
            Ok(ShellReplayReport {
                result: Some(inspect(&shell)?),
                reason: "profile-expression-shell-replayed",
            })
        })?;
    Ok(replay.result.unwrap_or(ShellReplayReport {
        result: None,
        reason: replay.reason,
    }))
}
