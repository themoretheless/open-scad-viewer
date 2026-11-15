//! Polynomial world-contact proposals; only complete original-root proofs admit.
use super::{FitLimits, Limits, Report};
use crate::offset::polynomial_fit;
use crate::{
    Result, bernstein_interpolation as interpolation, check,
    curve::Curve,
    offset_contact_pcurve as uv,
    surface_offset::{self, ContactBand},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FitStop {
    Certified,
    NoContact,
    BandUnresolved,
    QueryLimit,
    CoefficientLimit,
    ProofLimit,
    DegreeLimit,
    SectionUnresolved,
}
pub struct FitAttempt {
    pub degree: usize,
    pub report: Report,
}
pub struct FitReport {
    pub curve: Option<Curve>,
    pub band: ContactBand,
    pub attempts: Vec<FitAttempt>,
    pub root_queries: usize,
    pub coefficient_work: usize,
    pub proof_cells: usize,
    pub stop: FitStop,
}
/// Certified section-root boxes define only an interpolating world proposal.
/// Actual rounded poles need fresh COMPLETE world correspondence. No UV curve,
/// shared-edge receipt or positive certificate is accepted from the caller.
pub fn fit(
    faces: crate::offset::contact_pcurve::OffsetFaces<'_>,
    axis: usize,
    drive: [f64; 2],
    free: [f64; 2],
    second: [[f64; 2]; 2],
    tolerance_mm: f64,
    limits: FitLimits,
) -> Result<FitReport> {
    let crate::offset::contact_pcurve::OffsetFaces {
        surfaces,
        distances,
    } = faces;
    check(
        tolerance_mm.is_finite()
            && tolerance_mm > 0.
            && (1..=16).contains(&limits.max_degree)
            && (1..=10000000).contains(&limits.root_queries)
            && (1..=10000000).contains(&limits.coefficient_work)
            && (1..=100000).contains(&limits.proof_cells)
            && (1..=100000).contains(&limits.spans)
            && limits.root_refinements <= 16,
        "Choose bounded world-contact fitting and proof work",
    )?;
    check(
        distances.iter().all(|d| d.is_finite() && *d != 0.)
            && distances[0].abs() == distances[1].abs(),
        "World fitting requires equal nonzero signed offset magnitudes",
    )?;
    let band = surface_offset::certify_contact_band(
        surfaces,
        distances,
        axis,
        drive,
        free,
        second,
        limits.spans,
    )?;
    let mut out = FitReport {
        curve: None,
        band,
        attempts: vec![],
        root_queries: 0,
        coefficient_work: 0,
        proof_cells: 0,
        stop: FitStop::DegreeLimit,
    };
    match &out.band {
        ContactBand::Excluded => {
            out.stop = FitStop::NoContact;
            return Ok(out);
        }
        ContactBand::Unresolved => {
            out.stop = FitStop::BandUnresolved;
            return Ok(out);
        }
        ContactBand::ContinuousBranch(_) => {}
    }
    let degrees = polynomial_fit::degree_ladder(limits.max_degree);
    for (attempt, &degree) in degrees.iter().enumerate() {
        let n = degree + 1;
        let per_root = limits.root_refinements + 1;
        if out.proof_cells == limits.proof_cells {
            out.stop = FitStop::ProofLimit;
            break;
        }
        if n * per_root > limits.root_queries - out.root_queries {
            out.stop = FitStop::QueryLimit;
            break;
        }
        let required = n * degree * (degree + 3) / 2 + 3 * n * n;
        if required > limits.coefficient_work - out.coefficient_work {
            out.stop = FitStop::CoefficientLimit;
            break;
        }
        let sites = polynomial_fit::chebyshev_sites(
            drive,
            degree,
            "World fit nodes reached original clock resolution",
        )?;
        let (Some(basis), work) = interpolation::basis_on(
            &sites,
            drive,
            limits.coefficient_work - out.coefficient_work,
        )?
        else {
            unreachable!("prechecked coefficient work")
        };
        out.coefficient_work += work;
        let mut samples = Vec::with_capacity(n);
        for &t in &sites {
            let (root, queries) = uv::section(
                crate::offset::contact_pcurve::OffsetFaces {
                    surfaces,
                    distances,
                },
                axis,
                t,
                free,
                second,
                limits.spans,
                limits.root_refinements,
            )?;
            out.root_queries += queries;
            let Some(root) = root else {
                out.stop = FitStop::SectionUnresolved;
                return Ok(out);
            };
            samples.push(root.point.map(|p| p[0] * 0.5 + p[1] * 0.5));
        }
        let points =
            polynomial_fit::interpolate_poles(&samples, &basis, &mut out.coefficient_work)?;
        let curve = polynomial_fit::bezier(drive, points);
        let cells = ((limits.proof_cells - out.proof_cells) / (degrees.len() - attempt))
            .max(1)
            .min((limits.root_queries - out.root_queries) / per_root);
        if cells == 0 {
            out.stop = FitStop::QueryLimit;
            break;
        }
        let report = super::certify(&curve, crate::offset::contact_pcurve::OffsetFaces { surfaces, distances }, super::ContactDomain { axis, drive, free, second }, tolerance_mm, Limits {
                cells,
                spans: limits.spans,
                section_refinements: limits.root_refinements,
                band_refinements: limits.root_refinements,
            })?;
        out.proof_cells += report.visited;
        out.root_queries += report.section_queries;
        let accepted = report.world_correspondence_proven;
        out.attempts.push(FitAttempt { degree, report });
        if accepted {
            out.curve = Some(curve);
            out.stop = FitStop::Certified;
            break;
        }
    }
    Ok(out)
}
