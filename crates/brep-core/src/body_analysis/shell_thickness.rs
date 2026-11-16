//! Closed patch-network skins with independent offset and sewing contracts.
pub mod freeform_network;
pub mod intersected_pair;
pub mod mapped_network;
pub mod miter_network;
mod network;
pub mod planar_network;
pub mod quadrilateral_network;
pub mod parametric_network;
pub mod contact_pair;
use crate::{Model, Result};
use nurbs_core::{Error, curve::Curve, surface::Surface, surface_offset, surface_regularity};
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub regularity_cells: usize,
    pub agreement_cells: usize,
    pub exact_work: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            regularity_cells: 100000,
            agreement_cells: 100000,
            exact_work: 1000000,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Stop {
    ConstructedSampledThickness,
    ConstructedCertifiedNormalDisplacement,
    NormalOffsetUnresolved,
    NormalOffsetExceedsTolerance,
    RegularityUnresolved,
    SourceDegenerate,
    OffsetDegeneracy,
    SampledToleranceNotMet,
    AgreementUnresolved,
}
pub struct Report {
    pub model: Option<Model>,
    pub stop: Stop,
    pub max_sampled_thickness_deviation: Option<f64>,
    pub offset_fallback_reason: Option<String>,
    /// Whole-domain pointwise normal-displacement error, not a closest-distance bound.
    pub continuous_thickness_error_upper: Option<f64>,
    /// Certifies the normal-displacement error above, including saved-pole rounding.
    /// Does not certify side-wall thickness, offset injectivity, or a valid solid.
    pub rounding_inclusive_thickness_certified: bool,
    pub regularity: Vec<surface_regularity::Report>,
    pub regularity_cells: usize,
    pub agreement: Option<crate::boundary_agreement::Report>,
    pub solid_geometry_certified: bool,
    pub normal_offset_certificate: Option<surface_offset::CandidateCertificate>,
}
fn side(a: &Curve, b: &Curve) -> Surface {
    Surface {
        degree_u: a.degree,
        degree_v: 1,
        knots_u: a.knots.clone(),
        knots_v: vec![0., 0., 1., 1.],
        control_points: a
            .control_points
            .iter()
            .zip(&b.control_points)
            .map(|(a, b)| vec![a.clone(), b.clone()])
            .collect(),
        weights: a.weights.iter().map(|&w| vec![w, w]).collect(),
        periodic_u: false,
        periodic_v: false,
    }
}
/// Thicken one original full rectangular rational Bezier patch along its
/// normal orientation (negative signed thickness uses the opposite side).
/// Retains the original source cap byte-for-byte, creates a same-basis offset
/// cap and four exactly shared ruled sides. Every saved face needs interval
/// regularity, exact owned boundary sewing and continuous edge/lift agreement.
/// Normal-offset deviation is explicitly SAMPLED; global constant thickness,
/// rounding-inclusive ideal error and embedded-solid validity are not certified.
/// Source degrees 1..16, finite coordinates <=1e6, |thickness|<=1e6 and positive
/// tolerance. Original source/trims remain immutable; trimmed, periodic,
/// multispan and pole-degenerate inputs need a different topology constructor.
pub fn thicken_patch(
    source: &Surface,
    signed_thickness: f64,
    tolerance: f64,
    limits: Limits,
) -> Result<Report> {
    thicken_impl(source, signed_thickness, tolerance, limits, None)
}

/// Close a full Bezier patch with an authored same-basis cap. Admission uses
/// a continuous pointwise normal-displacement bound, independently of the
/// sewing tolerance. This does not certify closest thickness or global solid
/// embedding. Source/cap rank and every saved boundary lift are checked anew.
pub fn thicken_patch_with_cap(
    source: &Surface,
    cap: &Surface,
    signed_thickness: f64,
    normal_error_tolerance: f64,
    normal_cells: usize,
    sewing_tolerance: f64,
    limits: Limits,
) -> Result<Report> {
    thicken_impl(
        source,
        signed_thickness,
        sewing_tolerance,
        limits,
        Some((cap, normal_error_tolerance, normal_cells)),
    )
}

const MAX_DEGREE: usize = 16;
const MAX_COORDINATE: f64 = 1e6;
const MIN_TOLERANCE: f64 = 1e-10;
const MAX_TOLERANCE: f64 = 1e-2;
const MAX_LIMIT: usize = 1000000;
const MAX_INSPECTION_CELLS: usize = 100000;
const INPUT_CODE: &str = "BREP_THICKNESS_INPUT";

fn thicken_impl(
    source: &Surface,
    signed_thickness: f64,
    tolerance: f64,
    limits: Limits,
    cap: Option<(&Surface, f64, usize)>,
) -> Result<Report> {
    source.validate()?;
    admit_input(source, signed_thickness, tolerance, limits)?;
    let mut out = Report {
        model: None,
        stop: Stop::RegularityUnresolved,
        max_sampled_thickness_deviation: None,
        offset_fallback_reason: None,
        continuous_thickness_error_upper: None,
        rounding_inclusive_thickness_certified: false,
        regularity: Vec::with_capacity(6),
        regularity_cells: 0,
        agreement: None,
        solid_geometry_certified: false,
        normal_offset_certificate: None,
    };
    if source_rank_deficient(source) {
        out.stop = Stop::SourceDegenerate;
        return Ok(out);
    }
    if !inspect_regular(&mut out, source, limits)? {
        return Ok(out);
    }
    let offset_surface = match cap {
        Some(cap) => certified_cap(&mut out, source, signed_thickness, cap)?,
        None => sampled_offset(&mut out, source, signed_thickness, tolerance)?,
    };
    let Some(offset_surface) = offset_surface else {
        return Ok(out);
    };
    if offset_surface
        .control_points
        .iter()
        .flatten()
        .any(|p| p.iter().any(|x| !x.is_finite() || x.abs() > MAX_COORDINATE))
    {
        return Err(Error::new(INPUT_CODE, "Offset coordinates exceed bound"));
    }
    let patches = closed_patches(source, offset_surface);
    for patch in patches.iter().skip(1) {
        if out.regularity_cells == limits.regularity_cells
            || !inspect_regular(&mut out, patch, limits)?
        {
            return Ok(out);
        }
    }
    let reverse = signed_thickness < 0.;
    let orientations = vec![!reverse, reverse, reverse, reverse, reverse, reverse];
    let assembled = crate::rounded_patch_network::assemble(
        &patches,
        &orientations,
        tolerance,
        limits.exact_work,
        &[],
    )?;
    let agreement = crate::boundary_agreement::verify(&assembled.model, limits.agreement_cells)?;
    let complete = agreement.complete;
    out.agreement = Some(agreement);
    if !complete {
        out.stop = Stop::AgreementUnresolved;
        return Ok(out);
    }
    out.model = Some(assembled.model);
    out.stop = if cap.is_some() {
        Stop::ConstructedCertifiedNormalDisplacement
    } else {
        Stop::ConstructedSampledThickness
    };
    Ok(out)
}

fn admit_input(
    source: &Surface,
    signed_thickness: f64,
    tolerance: f64,
    limits: Limits,
) -> Result<()> {
    let clamped = |k: &[f64], p: usize, n: usize| {
        n == p + 1 && k[..n].iter().all(|&x| x == k[p]) && k[n..].iter().all(|&x| x == k[n])
    };
    let columns = source.control_points.first().map_or(0, Vec::len);
    let admitted = (1..=MAX_DEGREE).contains(&source.degree_u)
        && (1..=MAX_DEGREE).contains(&source.degree_v)
        && !source.periodic_u
        && !source.periodic_v
        && clamped(
            &source.knots_u,
            source.degree_u,
            source.control_points.len(),
        )
        && clamped(&source.knots_v, source.degree_v, columns)
        && !source
            .control_points
            .iter()
            .flatten()
            .any(|p| p.len() != 3 || p.iter().any(|x| x.abs() > MAX_COORDINATE))
        && signed_thickness.is_finite()
        && signed_thickness != 0.
        && signed_thickness.abs() <= MAX_COORDINATE
        && tolerance.is_finite()
        && (MIN_TOLERANCE..=MAX_TOLERANCE).contains(&tolerance)
        && (1..=MAX_LIMIT).contains(&limits.regularity_cells)
        && (1..=MAX_LIMIT).contains(&limits.agreement_cells)
        && (1..=MAX_LIMIT as u64).contains(&limits.exact_work);
    if admitted {
        Ok(())
    } else {
        Err(Error::new(
            INPUT_CODE,
            "Expected one bounded full rational Bezier chart, nonzero signed thickness, tolerance 1e-10..1e-2 and positive bounded work",
        ))
    }
}

/// Two exactly constant coordinates constrain the entire positive rational
/// image to a line. Its two-dimensional normal is identically zero; no
/// subdivision budget can repair this source-rank deficiency.
fn source_rank_deficient(source: &Surface) -> bool {
    let Some(first) = source.control_points.iter().flatten().next() else {
        return false;
    };
    let fixed_axes = (0..3)
        .filter(|&k| {
            source
                .control_points
                .iter()
                .flatten()
                .all(|p| p[k] == first[k])
        })
        .count();
    fixed_axes >= 2
}

/// Inspect one patch within the remaining shared budget; records the report.
fn inspect_regular(out: &mut Report, patch: &Surface, limits: Limits) -> Result<bool> {
    let r = surface_regularity::inspect(
        patch,
        (limits.regularity_cells - out.regularity_cells).min(MAX_INSPECTION_CELLS),
    )?;
    out.regularity_cells += r.cells;
    let valid = r.spanwise_regular;
    out.regularity.push(r);
    Ok(valid)
}

fn certified_cap(
    out: &mut Report,
    source: &Surface,
    signed_thickness: f64,
    (candidate, error_tolerance, cells): (&Surface, f64, usize),
) -> Result<Option<Surface>> {
    let proof = surface_offset::certify_candidate(
        source,
        candidate,
        signed_thickness,
        error_tolerance,
        cells,
    )?;
    let status = proof.status;
    out.continuous_thickness_error_upper = proof.error_upper;
    out.rounding_inclusive_thickness_certified = status == surface_offset::CandidateStatus::Within;
    out.normal_offset_certificate = Some(proof);
    if status != surface_offset::CandidateStatus::Within {
        out.stop = if status == surface_offset::CandidateStatus::Exceeds {
            Stop::NormalOffsetExceedsTolerance
        } else {
            Stop::NormalOffsetUnresolved
        };
        return Ok(None);
    }
    Ok(Some(candidate.clone()))
}

fn sampled_offset(
    out: &mut Report,
    source: &Surface,
    signed_thickness: f64,
    tolerance: f64,
) -> Result<Option<Surface>> {
    let offset = match surface_offset::offset(source, signed_thickness, tolerance) {
        Ok(candidate)
            if candidate.surface.degree_u == source.degree_u
                && candidate.surface.degree_v == source.degree_v
                && candidate.surface.knots_u == source.knots_u
                && candidate.surface.knots_v == source.knots_v
                && candidate.surface.weights == source.weights =>
        {
            candidate
        }
        candidate => {
            out.offset_fallback_reason = Some(match candidate {
                Ok(_) => "Adaptive offset changed the original basis; retaining an exactly owned same-basis candidate".into(),
                Err(error) => format!("Adaptive offset unavailable: {}", error.message),
            });
            surface_offset::offset_preserving_basis(source, signed_thickness)?
        }
    };
    out.max_sampled_thickness_deviation = Some(offset.max_deviation);
    if !offset.degenerate_regions.is_empty() || offset.exceeds_fold_free_bound {
        out.stop = Stop::OffsetDegeneracy;
        return Ok(None);
    }
    if offset.max_deviation > tolerance {
        out.stop = Stop::SampledToleranceNotMet;
        return Ok(None);
    }
    Ok(Some(offset.surface))
}

/// Source cap, offset cap and the four exactly shared ruled sides.
fn closed_patches(source: &Surface, offset_surface: Surface) -> Vec<Surface> {
    let mut patches = Vec::with_capacity(6);
    patches.push(source.clone());
    patches.push(offset_surface);
    for i in 0..4 {
        patches.push(side(
            &crate::rounded_patch_network::boundary(source, i).0,
            &crate::rounded_patch_network::boundary(&patches[1], i).0,
        ));
    }
    patches
}
