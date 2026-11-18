//! Owned progressive body: preserves the original arithmetic and proof budgets.
use super::*;
#[path = "owned_progressive_body/stages.rs"]
mod stages;

/// Exact-work allowance of retained caps when no annular limits are given.
const DEFAULT_CAP_EXACT_WORK: u64 = 1_000_000;
/// Product allowance of the retained section partition.
const PARTITION_MAX_PRODUCTS: usize = 1_000_000;

pub(super) fn progressive_profile_body_internal(
    laws: ProgressiveLaws<'_>,
    options: nurbs_core::progressive_sweep::Options,
    policy: ProgressiveBodyPolicy,
    annular: Option<crate::retained_annular_cap::reference_filling::Limits>,
    automatic_caps: bool,
) -> Result<ProgressiveBodyEvidence> {
    let rmf_policy = policy.rmf_policy;
    stages::validate_request(&laws, &options, rmf_policy, annular)?;
    let profiles = laws.loops.iter().flatten().cloned().collect::<Vec<_>>();
    let spans = stages::section_spans(&profiles)?;
    let options = stages::face_bounded_options(&laws, options, annular, automatic_caps, spans)?;
    let approximation = stages::approximate(&profiles, &laws, options, rmf_policy)?;
    let report = stages::accepted_level(&approximation, rmf_policy, &options)?;
    let mut sections = stages::transported_sections(&profiles, &laws, options, report.sections)?;
    let (profile_correction_error_upper, cap_correction_error_upper, correction_error_upper) =
        stages::apply_corrections(&mut sections, policy, report.closed_path, &laws, &options)?;
    let mut model = if report.closed_path {
        periodic_section_loft(&sections)?
    } else {
        rational_section_loft(&sections)?
    };
    let (use_annular, cap_selection_work) = stages::annular_cap_selection(
        &model,
        annular,
        automatic_caps,
        report.closed_path,
        [laws.loops.len(), spans],
    )?;
    let (retained_sections, body_decomposition_error_upper, body_decomposition_products) =
        retained_section_partition(&sections, PARTITION_MAX_PRODUCTS)?;
    let sizes = laws.loops.iter().map(Vec::len).collect::<Vec<_>>();
    let (retained_caps, cap_projection) = if report.closed_path || use_annular {
        (None, None)
    } else {
        let max_exact_work =
            annular.map_or(DEFAULT_CAP_EXACT_WORK, |l| l.exact_work) - cap_selection_work;
        let caps = stages::retained_caps(&model, &retained_sections, max_exact_work)?;
        let source = stages::sweep_source(&profiles, &laws, options, rmf_policy)?;
        let projection = stages::cap_projection(&source, &sizes, &model)?;
        (Some(caps), Some(projection))
    };
    let mut filled_cap_error_upper = cap_projection.as_ref().and_then(|projection| {
        stages::filled_cap_error(
            projection,
            retained_caps.as_ref().is_some_and(|r| r.exact),
            report,
            correction_error_upper,
            body_decomposition_error_upper,
        )
    });
    let retained_walls = stages::retained_walls(&model, &retained_sections, report.closed_path)?;
    let wall_error = stages::wall_error(
        report,
        body_decomposition_error_upper,
        retained_walls.certified,
        correction_error_upper,
    );
    let annular_caps = match annular.filter(|_| use_annular) {
        Some(limits) => {
            let source = stages::sweep_source(&profiles, &laws, options, rmf_policy)?;
            let assembled = annular_caps::assemble(
                &source,
                &model,
                &sizes,
                options.max_deviation,
                stages::CAP_PROJECTION_TOLERANCE,
                limits,
                cap_selection_work,
            )?;
            filled_cap_error_upper = assembled.evidence.filling().filled_cap_error_upper;
            model = assembled.model;
            Some(assembled.evidence)
        }
        None => None,
    };
    let boundary_error_upper = nurbs_core::sweeps::filled_cap_error::boundary(
        wall_error,
        filled_cap_error_upper,
        report.closed_path,
    );
    Ok(ProgressiveBodyEvidence {
        model,
        retained_caps,
        cap_projection,
        filled_cap_error_upper,
        cap_correction_error_upper,
        profile_correction_error_upper,
        retained_walls,
        boundary_error_upper,
        boundary_error_within_budget: boundary_error_upper
            .map(|upper| upper <= options.max_deviation),
        body_decomposition_error_upper,
        body_decomposition_products,
        station_sections: sections,
        station_boundary: nurbs_core::sweeps::filled_cap_error::compose_boundary(
            wall_error,
            filled_cap_error_upper,
            report.closed_path,
            Some(options.max_deviation),
        ),
        station_closed: report.closed_path,
        station_budget: options.max_deviation,
        annular_caps,
        cap_selection_work,
        approximation,
    })
}
