//! Automatic local transverse offset-root proposals for a pair of original
//! full patches. Certified section roots propose rectangular cap windows;
//! only complete mapped-cap and actual closed-model gates admit the skin.
use super::mapped_network;
pub use super::planar_network::Limits;
use crate::feature_ops::region_pattern::Verification;
use crate::{Error, Model, Result, rounded_patch_network as sewing};
use nurbs_core::{
    offset_contact_predictor as predictor, surface::Surface, surface_contact::Verdict,
};
fn refused(s: &str) -> Error {
    Error::new("BREP_INTERSECTED_OFFSET_PAIR_REFUSED", s)
}
#[derive(Clone, Copy, Debug)]
pub struct Search {
    pub iterations: usize,
    pub spans: usize,
    pub numerical_tolerance_mm: f64,
    pub padding_fraction: f64,
}
impl Default for Search {
    fn default() -> Self {
        Self {
            iterations: 32,
            spans: 128,
            numerical_tolerance_mm: 1e-11,
            padding_fraction: 1e-3,
        }
    }
}
pub struct Thickness {
    mapped: mapped_network::Thickness,
    reversed: [bool; 2],
    distance: f64,
    normal_tolerance: f64,
    normal_cells: usize,
    sewing_tolerance: f64,
    limits: Limits,
    search: Search,
    seam: [usize; 2],
    section: predictor::Report,
    source_cells: usize,
    source_exact: u64,
}
impl Thickness {
    pub fn model(&self) -> &Model {
        self.mapped.model()
    }
    pub fn mapped(&self) -> &mapped_network::Thickness {
        &self.mapped
    }
    /// Directed natural boundary sides on the two literal original charts.
    pub fn source_seam(&self) -> [usize; 2] {
        self.seam
    }
    /// Unique root evidence for one section, not whole-branch completeness.
    pub fn section(&self) -> &predictor::Report {
        &self.section
    }
    /// Shared sewing/chart proof work, including source-seam discovery.
    pub fn sewing_proof_cells(&self) -> usize {
        self.source_cells + self.mapped.evidence().cells
    }
    pub fn exact_work(&self) -> u64 {
        self.source_exact + self.mapped.exact_work()
    }
    /// Repeat source-seam discovery, native numerical/interval search, cap
    /// proposal, endpoint coupling and complete original/closed-model proofs.
    pub fn reverify(&self) -> Result<Verification> {
        let sources: [Surface; 2] = self
            .mapped
            .source_patches()
            .to_vec()
            .try_into()
            .map_err(|_| refused("Saved offset pair has an invalid source count"))?;
        let fresh = thicken(
            &sources,
            self.reversed,
            self.distance,
            self.normal_tolerance,
            self.normal_cells,
            self.sewing_tolerance,
            self.limits,
            self.search,
        )?;
        if fresh.model() != self.model()
            || fresh.seam != self.seam
            || fresh.mapped.source_windows() != self.mapped.source_windows()
            || fresh.mapped.cap_patches() != self.mapped.cap_patches()
            || fresh.section.first_uv != self.section.first_uv
            || fresh.section.second_uv != self.section.second_uv
        {
            return Err(refused(
                "Saved offset pair differs from complete fresh search/reconstruction",
            ));
        }
        Ok(fresh.mapped.evidence().clone())
    }
}
fn domain(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn fixed(side: usize) -> (usize, bool) {
    match side {
        0 => (1, false),
        1 => (0, true),
        2 => (1, true),
        _ => (0, false),
    }
}
/// Discover one complete original shared boundary and a transverse offset
/// intersection from its middle station. No caller-authored root, UV window or
/// offset cap is needed. The native interval section witness is mandatory;
/// its free coordinates merely PROPOSE rectangular windows. Full original-map
/// normal bounds and all actual saved seam/lift/closed-role gates must then
/// prove the resulting construction. Varying intersections that cannot be
/// represented this way refuse; no sample is promoted to a whole-branch proof.
///
/// Source support extension, tangential/nonisolated roots, multiple shared
/// edges, global branch discovery/completeness and nonrectangular cap trimming
/// require different constructors. Numerical search may refuse an existing root.
#[allow(clippy::too_many_arguments)]
pub fn thicken(
    sources: &[Surface; 2],
    reversed: [bool; 2],
    signed_thickness: f64,
    normal_tolerance: f64,
    normal_cells: usize,
    sewing_tolerance: f64,
    limits: Limits,
    search: Search,
) -> Result<Thickness> {
    super::freeform_network::validate(
        sources,
        sources,
        &reversed,
        crate::shell_thickness::planar_network::OffsetSettings {
            signed_thickness,
            normal_tolerance,
            normal_cells,
            sewing_tolerance,
        },
        limits,
    )?;
    if !(1..=32).contains(&search.iterations)
        || !(1..=100000).contains(&search.spans)
        || !search.numerical_tolerance_mm.is_finite()
        || search.numerical_tolerance_mm <= 0.
        || !search.padding_fraction.is_finite()
        || search.padding_fraction <= 0.
        || search.padding_fraction > 0.25
    {
        return Err(refused(
            "Choose bounded positive native offset-root search work",
        ));
    }
    let (seam, source_cells, source_exact) = discover(sources, reversed, sewing_tolerance, limits)?;
    let mut remaining = limits;
    remaining.proof.max_cells = limits
        .proof
        .max_cells
        .checked_sub(source_cells)
        .filter(|n| *n > 0)
        .ok_or_else(|| refused("Source-seam discovery consumed the sewing allowance"))?;
    remaining.exact_work = limits
        .exact_work
        .checked_sub(source_exact)
        .filter(|n| *n > 0)
        .ok_or_else(|| refused("Source-seam discovery consumed the exact allowance"))?;
    let domains = sources.each_ref().map(domain);
    let mut seeds = domains.map(|d| d.map(|r| r[0] * 0.5 + r[1] * 0.5));
    for side in 0..2 {
        let (axis, high) = fixed(seam[side]);
        seeds[side][axis] = domains[side][axis][usize::from(high)];
    }
    let drive_axis = 1 - fixed(seam[0]).0;
    let distances = reversed.map(|r| {
        if r {
            -signed_thickness
        } else {
            signed_thickness
        }
    });
    let section = predictor::propose(
        nurbs_core::offset_contact_pcurve::OffsetFaces {
            surfaces: sources.each_ref(),
            distances,
        },
        seeds,
        drive_axis,
        search.numerical_tolerance_mm,
        search.iterations,
        search.padding_fraction,
        search.spans,
    )?;
    if section.residual_mm > search.numerical_tolerance_mm
        || !matches!(section.certificate, Some(Verdict::Witness(_)))
    {
        return Err(refused("Native transverse offset section remains unproven"));
    }
    let stations = [section.first_uv, section.second_uv];
    let mut windows = domains;
    for side in 0..2 {
        let (axis, high) = fixed(seam[side]);
        let value = stations[side][axis];
        if !value.is_finite() || value <= domains[side][axis][0] || value >= domains[side][axis][1]
        {
            return Err(refused(
                "Offset intersection requires support extension or a consumed chart",
            ));
        }
        windows[side][axis][usize::from(high)] = value;
    }
    let mapped = mapped_network::thicken_from_windows(
        sources,
        &windows,
        &reversed,
        signed_thickness,
        normal_tolerance,
        normal_cells,
        sewing_tolerance,
        remaining,
    )?;
    Ok(Thickness {
        mapped,
        reversed,
        distance: signed_thickness,
        normal_tolerance,
        normal_cells,
        sewing_tolerance,
        limits,
        search,
        seam,
        section,
        source_cells,
        source_exact,
    })
}

// Endpoints only remove impossible candidates. Complete original world/lift
// identity (or its continuous tolerance proof) decides source seam ownership.
pub(super) fn discover(
    sources: &[Surface; 2],
    reversed: [bool; 2],
    tolerance: f64,
    limits: Limits,
) -> Result<([usize; 2], usize, u64)> {
    let mut seams = Vec::new();
    let mut cells = 0;
    let mut exact = 0;
    let reverse = !(reversed[0] ^ reversed[1]);
    for a in 0..4 {
        for b in 0..4 {
            let ca = sewing::boundary(&sources[0], a).0;
            let (cb, pc) = sewing::boundary(&sources[1], b);
            let end = cb.control_points.len() - 1;
            if ca.control_points[0] != cb.control_points[if reverse { end } else { 0 }]
                || *ca.control_points.last().unwrap()
                    != cb.control_points[if reverse { 0 } else { end }]
            {
                continue;
            }
            let cost = 4 * (ca.control_points.len() + cb.control_points.len()) as u64;
            if cost > limits.exact_work - exact {
                return Err(refused("Source boundary correspondence work exhausted"));
            }
            exact += cost;
            let same = if sewing::same_owned_boundary(&ca, &cb, reverse) {
                true
            } else {
                let pc = sewing::prepared_clock(&ca, &cb, &pc, reverse)?;
                let proof = nurbs_core::curve_surface_agreement::verify_exact(
                    &ca,
                    &pc,
                    &sources[1],
                    reverse,
                    limits.exact_work - exact,
                )?;
                let equal = if let Some(proof) = proof {
                    if proof.work_used > limits.exact_work - exact {
                        return Err(refused("Original source identity exceeded allowance"));
                    }
                    exact += proof.work_used;
                    proof.outcome == cad_predicates::BezierIdentity::Equal
                } else {
                    false
                };
                equal
                    || sewing::prepared_lift(
                        &ca,
                        &pc,
                        &sources[1],
                        reverse,
                        tolerance,
                        limits.proof.max_cells,
                        &mut cells,
                    )?
            };
            if same {
                seams.push([a, b]);
            }
        }
    }
    if seams.len() != 1 {
        return Err(refused(
            "Expected exactly one complete proved original shared boundary",
        ));
    }
    Ok((seams[0], cells, exact))
}
