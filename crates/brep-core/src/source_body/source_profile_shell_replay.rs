//! Scoped reconstruction of exact expression-shell incidence.
//! Embedding, material volume and thickness require subsequent fresh gates.
use super::*;
pub struct ShellReplayLimits {
    pub boundary: TransitionBoundaryReplayLimits,
    pub projection: [[f64; 3]; 2],
    pub controls: usize,
    pub section: ShellSectionLimits,
    pub join: ShellJoinLimits,
    pub quotient: ShellQuotientLimits,
    pub patches: ShellPatchLimits,
    pub rail: ShellRailLimits,
    pub outer: ShellOuterLimits,
    pub shell_uses: usize,
}
/// Work allowed for each profile section.
#[derive(Clone, Copy, Debug)]
pub struct ShellSectionLimits {
    pub cut_spans: usize,
    pub mapping_cells: usize,
    pub fragments: usize,
}
/// Work allowed for the profile joins.
#[derive(Clone, Copy, Debug)]
pub struct ShellJoinLimits {
    pub cells: usize,
    pub spans: usize,
    pub exterior_drivers: usize,
}
/// Collapsed ends and the quotient subdivision around them.
#[derive(Clone, Copy, Debug)]
pub struct ShellQuotientLimits {
    pub collapsed_ends: [usize; 2],
    pub subdivisions: usize,
    pub cells: usize,
}
/// Cells and spans of the retained and transition support patches.
#[derive(Clone, Copy, Debug)]
pub struct ShellPatchLimits {
    pub retained_cells: usize,
    pub retained_spans: usize,
    pub transition_cells: usize,
    pub transition_spans: usize,
}
/// Work allowed for each rail.
#[derive(Clone, Copy, Debug)]
pub struct ShellRailLimits {
    pub exact_work: u64,
    pub drivers: usize,
    pub fragments: usize,
}
/// Work allowed for the outer boundary.
#[derive(Clone, Copy, Debug)]
pub struct ShellOuterLimits {
    pub exact_work: u64,
    pub drivers: usize,
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
        limits.section.cut_spans,
        limits.section.mapping_cells,
        limits.section.fragments,
        limits.controls,
        limits.join.cells,
        limits.join.spans,
        limits.join.exterior_drivers,
        limits.quotient.subdivisions,
        limits.quotient.cells,
        limits.patches.retained_cells,
        limits.patches.retained_spans,
        limits.patches.transition_cells,
        limits.patches.transition_spans,
        limits.rail.drivers,
        limits.rail.fragments,
        limits.outer.drivers,
        limits.shell_uses,
    ]
    .iter()
    .any(|n| *n > 100000)
        || limits.rail.exact_work > 100000000
        || limits.outer.exact_work > 100000000
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
                    limits.section.cut_spans,
                    limits.section.mapping_cells,
                    limits.section.fragments,
                )?;
                let Some(section) = report.section else {
                    return Ok(pending(report.reason));
                };
                sections.push(section);
            }
            let mut exteriors = Vec::new();
            for section in &sections {
                let joined =
                    section.qualify_join(limits.controls, limits.join.cells, limits.join.spans)?;
                if joined.join.is_none() {
                    return Ok(pending(joined.reason));
                }
                let report = section.qualify_exterior(limits.controls, limits.join.exterior_drivers)?;
                let Some(exterior) = report.region else {
                    return Ok(pending(report.reason));
                };
                exteriors.push(exterior);
            }
            let mut retained = Vec::new();
            for (end, exterior) in exteriors.iter().enumerate() {
                let report = exterior.qualify_face(
                    limits.quotient.collapsed_ends[end],
                    limits.quotient.subdivisions,
                    limits.quotient.cells,
                    limits.patches.retained_cells,
                    limits.patches.retained_spans,
                )?;
                let Some(face) = report.face else {
                    return Ok(pending(report.reason));
                };
                retained.push(face);
            }
            let report = boundary.qualify_face_from_rails(
                limits.projection,
                limits.controls,
                limits.patches.transition_cells,
                limits.patches.transition_spans,
            )?;
            let Some(face) = report.face else {
                return Ok(pending(report.reason));
            };
            let report = face.qualify_rail_incidence(
                limits.rail.exact_work,
                limits.rail.drivers,
                limits.rail.fragments,
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
                limits.outer.exact_work,
                limits.outer.drivers,
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
