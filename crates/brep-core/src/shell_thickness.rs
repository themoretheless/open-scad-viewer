//! Closed rectangular patch skins with explicitly sampled thickness assurance.
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
    pub continuous_thickness_error_upper: Option<f64>,
    pub rounding_inclusive_thickness_certified: bool,
    pub regularity: Vec<surface_regularity::Report>,
    pub regularity_cells: usize,
    pub agreement: Option<crate::boundary_agreement::Report>,
    pub solid_geometry_certified: bool,
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
    source.validate()?;
    let clamped = |k: &[f64], p: usize, n: usize| {
        n == p + 1 && k[..n].iter().all(|&x| x == k[p]) && k[n..].iter().all(|&x| x == k[n])
    };
    if !(1..=16).contains(&source.degree_u)
        || !(1..=16).contains(&source.degree_v)
        || source.periodic_u
        || source.periodic_v
        || !clamped(
            &source.knots_u,
            source.degree_u,
            source.control_points.len(),
        )
        || !clamped(
            &source.knots_v,
            source.degree_v,
            source.control_points[0].len(),
        )
        || source
            .control_points
            .iter()
            .flatten()
            .any(|p| p.len() != 3 || p.iter().any(|x| x.abs() > 1e6))
        || !signed_thickness.is_finite()
        || signed_thickness == 0.
        || signed_thickness.abs() > 1e6
        || !tolerance.is_finite()
        || !(1e-10..=1e-2).contains(&tolerance)
        || !(1..=1000000).contains(&limits.regularity_cells)
        || !(1..=1000000).contains(&limits.agreement_cells)
        || !(1..=1000000).contains(&limits.exact_work)
    {
        return Err(Error::new(
            "BREP_THICKNESS_INPUT",
            "Expected one bounded full rational Bezier chart, nonzero signed thickness, tolerance 1e-10..1e-2 and positive bounded work",
        ));
    }
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
    };
    // Two exactly constant coordinates constrain the entire positive rational
    // image to a line. Its two-dimensional normal is identically zero; no
    // subdivision budget can repair this source-rank deficiency.
    let fixed_axes = (0..3)
        .filter(|&k| {
            source
                .control_points
                .iter()
                .flatten()
                .all(|p| p[k] == source.control_points[0][0][k])
        })
        .count();
    if fixed_axes >= 2 {
        out.stop = Stop::SourceDegenerate;
        return Ok(out);
    }
    let source_reg = surface_regularity::inspect(source, limits.regularity_cells.min(100000))?;
    out.regularity_cells += source_reg.cells;
    let valid = source_reg.spanwise_regular;
    out.regularity.push(source_reg);
    if !valid {
        return Ok(out);
    }
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
        return Ok(out);
    }
    if offset.max_deviation > tolerance {
        out.stop = Stop::SampledToleranceNotMet;
        return Ok(out);
    }
    if offset
        .surface
        .control_points
        .iter()
        .flatten()
        .any(|p| p.iter().any(|x| !x.is_finite() || x.abs() > 1e6))
    {
        return Err(Error::new(
            "BREP_THICKNESS_INPUT",
            "Offset coordinates exceed bound",
        ));
    }
    let mut patches = Vec::with_capacity(6);
    patches.push(source.clone());
    patches.push(offset.surface);
    for i in 0..4 {
        patches.push(side(
            &crate::rounded_patch_network::boundary(source, i).0,
            &crate::rounded_patch_network::boundary(&patches[1], i).0,
        ));
    }
    for patch in patches.iter().skip(1) {
        if out.regularity_cells == limits.regularity_cells {
            return Ok(out);
        }
        let r = surface_regularity::inspect(
            patch,
            (limits.regularity_cells - out.regularity_cells).min(100000),
        )?;
        out.regularity_cells += r.cells;
        let valid = r.spanwise_regular;
        out.regularity.push(r);
        if !valid {
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
    out.stop = Stop::ConstructedSampledThickness;
    Ok(out)
}
