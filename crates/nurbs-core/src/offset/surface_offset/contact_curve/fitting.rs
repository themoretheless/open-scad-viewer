//! Polynomial world-contact proposals; only complete original-root proofs admit.
use super::{FitLimits, Limits, Report};
use crate::{
    Result, bernstein_interpolation as interpolation, check,
    curve::Curve,
    distance_bounds::Interval as I,
    offset_contact_pcurve as uv,
    surface::Surface,
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
#[allow(clippy::too_many_arguments)]
pub fn fit(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    axis: usize,
    drive: [f64; 2],
    free: [f64; 2],
    second: [[f64; 2]; 2],
    tolerance_mm: f64,
    limits: FitLimits,
) -> Result<FitReport> {
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
    let mut degrees = vec![1usize];
    while *degrees.last().unwrap() < limits.max_degree {
        degrees.push((2 * degrees.last().unwrap()).min(limits.max_degree));
    }
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
        let sites: Vec<_> = (0..n)
            .map(|i| {
                if i == 0 {
                    drive[0]
                } else if i == degree {
                    drive[1]
                } else {
                    let t = (1. - (std::f64::consts::PI * i as f64 / degree as f64).cos()) * 0.5;
                    drive[0] * (1. - t) + drive[1] * t
                }
            })
            .collect();
        check(
            sites.windows(2).all(|r| r[0] < r[1]),
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
                surfaces,
                distances,
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
        let mut points = vec![vec![0.; 3]; n];
        for k in 0..3 {
            let nodal: Vec<_> = samples.iter().map(|p| I::point(p[k])).collect();
            let c = interpolation::reconstruct(&nodal, &basis)?;
            out.coefficient_work += n * n;
            for (i, c) in c.into_iter().enumerate() {
                points[i][k] = c.lo * 0.5 + c.hi * 0.5;
            }
        }
        let curve = Curve {
            degree,
            knots: std::iter::repeat_n(drive[0], n)
                .chain(std::iter::repeat_n(drive[1], n))
                .collect(),
            control_points: points,
            weights: vec![1.; n],
            periodic: false,
        };
        let cells = ((limits.proof_cells - out.proof_cells) / (degrees.len() - attempt))
            .max(1)
            .min((limits.root_queries - out.root_queries) / per_root);
        if cells == 0 {
            out.stop = FitStop::QueryLimit;
            break;
        }
        let report = super::certify(
            &curve,
            surfaces,
            distances,
            axis,
            drive,
            free,
            second,
            tolerance_mm,
            Limits {
                cells,
                spans: limits.spans,
                section_refinements: limits.root_refinements,
                band_refinements: limits.root_refinements,
            },
        )?;
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
