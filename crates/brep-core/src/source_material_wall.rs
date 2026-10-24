//! Whole selected-face-union bounds for minimum normal material chord length.
//! Scope is explicit groups; this does not claim coverage of every body wall.
use crate::{source_face_gap, source_material_chord, source_volume::Body};
use nurbs_core::{Error, Result};
pub struct Limits {
    pub gap: source_face_gap::Limits,
    pub chord: source_material_chord::Limits,
}
pub struct Certificate<'a,E = crate::source_shared_edge::SharedEdge> {
    gap: source_face_gap::Certificate<'a,E>,
    chord: source_material_chord::Certificate<'a,E>,
    interval_mm: [f64; 2],
}
impl<'a,E:crate::source_seam_tangency::Carrier> Certificate<'a,E> {
    pub fn body(&self) -> &'a Body<E> {
        self.gap.body()
    }
    pub fn faces(&self) -> [&[usize]; 2] {
        self.gap.faces()
    }
    pub fn chord(&self) -> &source_material_chord::Certificate<'a,E> {
        &self.chord
    }
    pub fn interval_mm(&self) -> [f64; 2] {
        self.interval_mm
    }
}
pub struct Report<'a,E = crate::source_shared_edge::SharedEdge> {
    pub certificate: Option<Certificate<'a,E>>,
    pub converged: bool,
    pub reason: &'static str,
    pub clearance: source_face_gap::Report<'a,E>,
    pub candidate: source_material_chord::Report<'a,E>,
}
/// Fresh full pair coverage supplies a lower bound. A normal material chord
/// connecting the selected unions supplies an upper witness. Positive evidence
/// for both gates is mandatory, including matching immutable Body identities.
pub fn qualify<'a,E:crate::source_seam_tangency::Carrier>(
    body: &'a Body<E>,
    groups: [&[usize]; 2],
    minimum_mm: f64,
    tolerance_mm: f64,
    origin: [f64; 3],
    direction: [f64; 3],
    tolerance_uv: f64,
    limits: Limits,
) -> Result<Report<'a,E>> {
    if !tolerance_mm.is_finite() || tolerance_mm <= 0. {
        return Err(Error::new(
            "BREP_SOURCE_WALL_TOLERANCE",
            "Choose a finite positive thickness interval tolerance",
        ));
    }
    let mut clearance = source_face_gap::qualify(body, groups, minimum_mm, limits.gap)?;
    let mut candidate =
        source_material_chord::qualify(body, origin, direction, tolerance_uv, limits.chord)?;
    let mut reason = "source-wall-clearance-unproven";
    let mut certificate = None;
    if let Some(gap) = clearance.certificate.as_ref() {
        reason = "source-wall-material-chord-unproven";
        if let Some(chord) = candidate.certificate.as_ref() {
            let f = chord.faces();
            if !(groups[0].contains(&f[0]) && groups[1].contains(&f[1])
                || groups[0].contains(&f[1]) && groups[1].contains(&f[0]))
            {
                reason = "source-wall-candidate-outside-groups";
            } else {
                let interval_mm = [gap.lower_mm(), chord.length_mm()[1]];
                if !std::ptr::eq(gap.body(), chord.body()) || interval_mm[0] > interval_mm[1] {
                    return Err(Error::new(
                        "BREP_SOURCE_WALL_INCONSISTENT",
                        "Original material bounds or body identity are inconsistent",
                    ));
                }
                certificate = Some(Certificate {
                    gap: clearance.certificate.take().unwrap(),
                    chord: candidate.certificate.take().unwrap(),
                    interval_mm,
                });
                reason = "source-wall-thickness-bounds-qualified";
            }
        }
    }
    let converged = certificate
        .as_ref()
        .is_some_and(|c| (c.interval_mm[1] - c.interval_mm[0]).next_up() <= tolerance_mm);
    Ok(Report {
        certificate,
        converged,
        reason,
        clearance,
        candidate,
    })
}

pub struct SearchReport<'a,E = crate::source_shared_edge::SharedEdge> {
    pub certificate: Option<Certificate<'a,E>>,
    pub converged: bool,
    pub reason: &'static str,
    pub clearance: source_face_gap::Report<'a,E>,
    pub search: crate::source_wall_search::Report<'a,E>,
}
/// Automatic upper witness search plus complete lower coverage of the supplied
/// unions. Exhausting a finite search does not itself prove a global minimum;
/// only the full source pair bound supplies the lower certificate.
pub fn search_and_qualify<'a,E:crate::source_seam_tangency::Carrier>(
    body: &'a Body<E>,
    groups: [&[usize]; 2],
    minimum_mm: f64,
    tolerance_mm: f64,
    tolerance_uv: f64,
    grid: usize,
    max_attempts: usize,
    limits: Limits,
) -> Result<SearchReport<'a,E>> {
    if !tolerance_mm.is_finite() || tolerance_mm <= 0. {
        return Err(Error::new(
            "BREP_SOURCE_WALL_TOLERANCE",
            "Choose a finite positive thickness interval tolerance",
        ));
    }
    let mut clearance = source_face_gap::qualify(body, groups, minimum_mm, limits.gap)?;
    let mut search = crate::source_wall_search::search(
        body,
        groups,
        grid,
        max_attempts,
        tolerance_uv,
        limits.chord,
    )?;
    let mut certificate = None;
    let mut reason = "source-wall-clearance-unproven";
    if let Some(gap) = clearance.certificate.as_ref() {
        reason = "source-wall-search-witness-unproven";
        if let Some(chord) = search.best.as_ref() {
            let f = chord.faces();
            if !std::ptr::eq(gap.body(), chord.body())
                || !(groups[0].contains(&f[0]) && groups[1].contains(&f[1])
                    || groups[0].contains(&f[1]) && groups[1].contains(&f[0]))
            {
                return Err(Error::new(
                    "BREP_SOURCE_WALL_INCONSISTENT",
                    "Search witness must belong to the same original body and opposing groups",
                ));
            }
            let interval_mm = [gap.lower_mm(), chord.length_mm()[1]];
            if interval_mm[0] > interval_mm[1] {
                return Err(Error::new(
                    "BREP_SOURCE_WALL_INCONSISTENT",
                    "Original material bounds are inconsistent",
                ));
            }
            certificate = Some(Certificate {
                gap: clearance.certificate.take().unwrap(),
                chord: search.best.take().unwrap(),
                interval_mm,
            });
            reason = "source-wall-searched-thickness-bounds-qualified";
        }
    }
    let converged = certificate
        .as_ref()
        .is_some_and(|c| (c.interval_mm[1] - c.interval_mm[0]).next_up() <= tolerance_mm);
    Ok(SearchReport {
        certificate,
        converged,
        reason,
        clearance,
        search,
    })
}

pub struct RefinedReport<'a,E = crate::source_shared_edge::SharedEdge> {
    pub result: SearchReport<'a,E>,
    pub refinement: Option<source_face_gap::Report<'a,E>>,
    pub cells: usize,
    pub spans: usize,
}
/// Tighten the lower certificate toward a searched material upper witness.
/// Initial and refinement full-pair visits share one gap budget. Refusal keeps
/// the original valid interval and exposes the failed refinement separately.
pub fn search_and_refine<'a,E:crate::source_seam_tangency::Carrier>(
    body: &'a Body<E>,
    groups: [&[usize]; 2],
    minimum_mm: f64,
    tolerance_mm: f64,
    tolerance_uv: f64,
    grid: usize,
    max_attempts: usize,
    limits: Limits,
) -> Result<RefinedReport<'a,E>> {
    let budget = [limits.gap.cells, limits.gap.spans];
    let result = search_and_qualify(
        body,
        groups,
        minimum_mm,
        tolerance_mm,
        tolerance_uv,
        grid,
        max_attempts,
        limits,
    )?;
    let mut out = RefinedReport {
        cells: result.clearance.cells,
        spans: result.clearance.spans,
        result,
        refinement: None,
    };
    if out.result.converged {
        return Ok(out);
    }
    let Some(proof) = out.result.certificate.as_ref() else {
        return Ok(out);
    };
    let remaining = [budget[0] - out.cells, budget[1] - out.spans];
    if remaining.contains(&0) {
        return Ok(out);
    }
    // This rounded threshold proposes work only. Original interval bounds alone
    // admit the lower certificate, with outward width checked again afterward.
    let target = proof.interval_mm[1] - 0.75 * tolerance_mm;
    if !target.is_finite() || target <= proof.interval_mm[0] {
        return Ok(out);
    }
    let mut refined = source_face_gap::qualify(
        body,
        groups,
        target,
        source_face_gap::Limits {
            cells: remaining[0],
            spans: remaining[1],
        },
    )?;
    out.cells += refined.cells;
    out.spans += refined.spans;
    if let Some(gap) = refined.certificate.take() {
        let proof = out.result.certificate.as_mut().unwrap();
        if !std::ptr::eq(gap.body(), proof.body()) || gap.lower_mm() > proof.interval_mm[1] {
            return Err(Error::new(
                "BREP_SOURCE_WALL_INCONSISTENT",
                "Refined original material bounds are inconsistent",
            ));
        }
        proof.interval_mm[0] = gap.lower_mm();
        proof.gap = gap;
        out.result.converged =
            (proof.interval_mm[1] - proof.interval_mm[0]).next_up() <= tolerance_mm;
    }
    out.refinement = Some(refined);
    Ok(out)
}
