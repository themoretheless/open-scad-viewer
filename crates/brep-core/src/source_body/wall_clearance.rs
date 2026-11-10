//! Global minimum clearance to a selected opposing face set, with original
//! trims, admitted witnesses and explicit unvisited coverage. No normal-ray,
//! material-chord or whole-solid thickness certificate is implied.
use crate::{Error, Model, Result, face_domain::FaceDomain};
use nurbs_core::trimmed_surface_distance::{self, TrimmedDistance};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    MeetsRequiredClearance,
    BelowRequiredClearance,
    Unresolved,
}
pub struct Pair {
    pub target_face: usize,
    pub distance: TrimmedDistance,
}
pub struct Report {
    pub source_face: usize,
    pub target_faces: Vec<usize>,
    pub pairs: Vec<Pair>,
    pub pending_faces: Vec<usize>,
    pub lower_bound_mm: f64,
    pub upper_bound_mm: Option<f64>,
    pub witness_pair: Option<usize>,
    pub required_mm: f64,
    pub decision: Decision,
    pub coverage_complete: bool,
    pub converged: bool,
    pub cells: usize,
    pub domain_cells: usize,
}
pub struct AutomaticReport {
    pub selection: crate::wall_opposition::Report,
    /// Clearance to the conservative angular candidate set. If selection is
    /// unresolved, a close candidate is not yet a confirmed opposing wall.
    pub candidate_clearance: Option<Report>,
    pub cells: usize,
    pub domain_cells: usize,
}
/// Automatic angular discovery followed by the original trimmed-face distance
/// proof. Both stages share work. The selected shell/angle scope does not
/// establish material chords, cavity pairing or whole-solid wall thickness.
pub fn inspect_shell(
    model: &Model,
    shell: usize,
    source_face: usize,
    cutoff: f64,
    required_mm: f64,
    tolerance_mm: f64,
    tolerance_uv: f64,
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<AutomaticReport> {
    if !required_mm.is_finite()
        || required_mm <= 0.
        || !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
        || max_cells > 100000
        || max_domain_cells > 1000000
    {
        return Err(Error::new(
            "BREP_WALL_CLEARANCE_INPUT",
            "Automatic clearance needs positive finite requirements and bounded shared work",
        ));
    }
    let selection_budget = if max_cells == 0 {
        0
    } else {
        (max_cells / 4).max(1)
    };
    let selection = crate::wall_opposition::inspect(
        model,
        shell,
        source_face,
        cutoff,
        tolerance_uv,
        selection_budget,
    )?;
    let mut cells = selection.cells;
    let mut domains = 0;
    let candidate_clearance =
        if selection.candidate_faces.is_empty() || cells == max_cells || max_domain_cells == 0 {
            None
        } else {
            let r = inspect(
                model,
                source_face,
                &selection.candidate_faces,
                required_mm,
                tolerance_mm,
                tolerance_uv,
                max_cells - cells,
                max_domain_cells,
            )?;
            cells += r.cells;
            domains += r.domain_cells;
            Some(r)
        };
    Ok(AutomaticReport {
        selection,
        candidate_clearance,
        cells,
        domain_cells: domains,
    })
}
/// Every selected face contributes to the global lower bound. Unvisited
/// targets contribute zero, never disappear. An upper bound always points to
/// an admitted original-face witness in `pairs[witness_pair]`.
pub fn inspect(
    model: &Model,
    source_face: usize,
    targets: &[usize],
    required_mm: f64,
    tolerance_mm: f64,
    tolerance_uv: f64,
    max_cells: usize,
    max_domain_cells: usize,
) -> Result<Report> {
    model.validate()?;
    let unique = targets
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if source_face >= model.faces.len()
        || targets.is_empty()
        || targets.len() > 1024
        || unique.len() != targets.len()
        || targets
            .iter()
            .any(|&f| f >= model.faces.len() || f == source_face)
        || !required_mm.is_finite()
        || required_mm <= 0.
        || !tolerance_mm.is_finite()
        || tolerance_mm <= 0.
        || !tolerance_uv.is_finite()
        || tolerance_uv <= 0.
        || max_cells > 100000
        || max_domain_cells > 1000000
    {
        return Err(Error::new(
            "BREP_WALL_CLEARANCE_INPUT",
            "Choose distinct source/opposing faces, positive finite tolerances and bounded shared work",
        ));
    }
    let source = FaceDomain::new(model, source_face, tolerance_uv)?;
    let mut report = Report {
        source_face,
        target_faces: targets.to_vec(),
        pairs: Vec::new(),
        pending_faces: Vec::new(),
        lower_bound_mm: f64::INFINITY,
        upper_bound_mm: None,
        witness_pair: None,
        required_mm,
        decision: Decision::Unresolved,
        coverage_complete: false,
        converged: false,
        cells: 0,
        domain_cells: 0,
    };
    for (index, &face) in targets.iter().enumerate() {
        if report.cells == max_cells || report.domain_cells == max_domain_cells {
            report.pending_faces.extend_from_slice(&targets[index..]);
            break;
        }
        let remaining = targets.len() - index;
        let cells = ((max_cells - report.cells) / remaining).max(1);
        let domains = ((max_domain_cells - report.domain_cells) / remaining).max(1);
        let target = FaceDomain::new(model, face, tolerance_uv)?;
        let distance = trimmed_surface_distance::distance(
            &model.faces[source_face].surface,
            &source.region,
            &model.faces[face].surface,
            &target.region,
            tolerance_mm,
            cells,
            domains,
        )?;
        report.cells += distance.cells;
        report.domain_cells += distance.domain_cells;
        report.lower_bound_mm = report.lower_bound_mm.min(distance.lower_bound_mm);
        if let Some(upper) = distance.upper_bound_mm {
            if report.upper_bound_mm.is_none_or(|old| upper < old) {
                report.upper_bound_mm = Some(upper);
                report.witness_pair = Some(report.pairs.len());
            }
        }
        report.pairs.push(Pair {
            target_face: face,
            distance,
        });
    }
    report.coverage_complete =
        report.pending_faces.is_empty() && report.pairs.len() == targets.len();
    if !report.coverage_complete {
        report.lower_bound_mm = 0.;
    }
    if let Some(upper) = report.upper_bound_mm {
        report.converged = report.coverage_complete
            && nurbs_core::interval_eval::Interval::point(upper)
                .sub(nurbs_core::interval_eval::Interval::point(
                    report.lower_bound_mm,
                ))?
                .hi
                <= tolerance_mm;
    }
    report.decision = if report
        .upper_bound_mm
        .is_some_and(|upper| upper < required_mm)
    {
        Decision::BelowRequiredClearance
    } else if report.coverage_complete
        && report.lower_bound_mm.is_finite()
        && report.upper_bound_mm.is_some()
        && report.lower_bound_mm >= required_mm
    {
        Decision::MeetsRequiredClearance
    } else {
        Decision::Unresolved
    };
    Ok(report)
}
