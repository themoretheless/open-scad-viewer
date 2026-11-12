//! Degree-refined UV proposals, qualified against the same original root tube.
use crate::{
    Result, bernstein_interpolation as interpolation, check,
    curve::Curve,
    distance_bounds::Interval as I,
    surface::Surface,
    surface_offset::{self, ContactBand},
};

#[derive(Clone, Copy, Debug)]
pub struct FitLimits {
    pub max_degree: usize,
    /// Shared section queries: sampling/refinement AND final curve proofs.
    pub root_queries: usize,
    pub coefficient_work: usize,
    pub proof_cells: usize,
    pub spans: usize,
    pub root_refinements: usize,
}
impl Default for FitLimits {
    fn default() -> Self {
        Self {
            max_degree: 16,
            root_queries: 1000000,
            coefficient_work: 100000,
            proof_cells: 100000,
            spans: 128,
            root_refinements: 8,
        }
    }
}
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
    /// Includes partial attempts; both sides must pass before curves publish.
    pub reports: Vec<super::Report>,
}
pub struct FitReport {
    pub curves: Option<[Curve; 2]>,
    pub band: ContactBand,
    pub attempts: Vec<FitAttempt>,
    pub root_queries: usize,
    pub coefficient_work: usize,
    pub proof_cells: usize,
    pub stop: FitStop,
}
fn nodes(domain: [f64; 2], degree: usize) -> Result<Vec<f64>> {
    let result: Vec<_> = (0..=degree)
        .map(|i| {
            if i == 0 {
                domain[0]
            } else if i == degree {
                domain[1]
            } else {
                let t = (1. - (std::f64::consts::PI * i as f64 / degree as f64).cos()) * 0.5;
                domain[0] * (1. - t) + domain[1] * t
            }
        })
        .collect();
    check(
        result.windows(2).all(|r| r[0] < r[1]),
        "UV fit nodes reached original parameter resolution",
    )?;
    Ok(result)
}
/// Fit both original UV traces of ONE freshly certified continuous contact
/// tube, without caller-authored curves or point seeds. Certified station-root
/// enclosures define polynomial proposals only. Every rounded curve needs fresh
/// full-driving-interval correspondence (implicit tangents and root anchors)
/// against the ORIGINAL tube; endpoint or world proximity never supplies it.
///
/// This is UV correspondence within one admitted band, not trimmed material,
/// world curve/lift agreement, region injectivity, BRep cutting or global branch
/// completeness. Work/degree exhaustion never publishes one successful side.
#[allow(clippy::too_many_arguments)]
pub fn fit_branch(
    surfaces: [&Surface; 2],
    distances: [f64; 2],
    axis: usize,
    drive: [f64; 2],
    free: [f64; 2],
    second: [[f64; 2]; 2],
    tolerance_uv: f64,
    limits: FitLimits,
) -> Result<FitReport> {
    check(
        tolerance_uv.is_finite()
            && tolerance_uv > 0.
            && (1..=16).contains(&limits.max_degree)
            && (1..=10000000).contains(&limits.root_queries)
            && (1..=10000000).contains(&limits.coefficient_work)
            && (1..=100000).contains(&limits.proof_cells)
            && (1..=100000).contains(&limits.spans)
            && limits.root_refinements <= 16,
        "Choose finite UV tolerance and bounded fit/root/proof work",
    )?;
    check(
        distances.iter().all(|d| d.is_finite() && *d != 0.)
            && distances[0].abs() == distances[1].abs(),
        "UV fitting requires equal nonzero signed offset magnitudes",
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
        curves: None,
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
        if limits.proof_cells - out.proof_cells < 2 {
            out.stop = FitStop::ProofLimit;
            break;
        }
        if n * per_root > limits.root_queries - out.root_queries {
            out.stop = FitStop::QueryLimit;
            break;
        }
        let required = n * degree * (degree + 3) / 2 + 4 * n * n;
        if required > limits.coefficient_work - out.coefficient_work {
            out.stop = FitStop::CoefficientLimit;
            break;
        }
        let sites = nodes(drive, degree)?;
        let (Some(basis), work) = interpolation::basis_on(
            &sites,
            drive,
            limits.coefficient_work - out.coefficient_work,
        )?
        else {
            unreachable!("prechecked fit basis work")
        };
        out.coefficient_work += work;
        let mut samples = [
            Vec::<[f64; 2]>::with_capacity(n),
            Vec::<[f64; 2]>::with_capacity(n),
        ];
        for &t in &sites {
            let (root, queries) = super::section(
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
            let middle = |r: [f64; 2]| r[0] * 0.5 + r[1] * 0.5;
            samples[0].push(root.first_uv.map(middle));
            samples[1].push(root.second_uv.map(middle));
        }
        let mut points = [vec![vec![0.; 2]; n], vec![vec![0.; 2]; n]];
        for side in 0..2 {
            for k in 0..2 {
                let nodal: Vec<_> = samples[side].iter().map(|p| I::point(p[k])).collect();
                let c = interpolation::reconstruct(&nodal, &basis)?;
                out.coefficient_work += n * n;
                for (i, c) in c.into_iter().enumerate() {
                    points[side][i][k] = c.lo * 0.5 + c.hi * 0.5;
                }
            }
        }
        let curves = points.map(|control_points| Curve {
            degree,
            knots: std::iter::repeat_n(drive[0], n)
                .chain(std::iter::repeat_n(drive[1], n))
                .collect(),
            control_points,
            weights: vec![1.; n],
            periodic: false,
        });
        let mut reports = vec![];
        for side in 0..2 {
            let available_cells = ((limits.proof_cells - out.proof_cells)
                / (2 * (degrees.len() - attempt) - side))
                .max(1);
            let available_queries = (limits.root_queries - out.root_queries) / per_root;
            let cells = available_cells.min(available_queries);
            if cells == 0 {
                out.stop = if available_queries == 0 {
                    FitStop::QueryLimit
                } else {
                    FitStop::ProofLimit
                };
                out.attempts.push(FitAttempt { degree, reports });
                return Ok(out);
            }
            let proof = super::certify_refined(
                &curves[side],
                side,
                super::OffsetFaces {
                    surfaces,
                    distances,
                },
                super::ContactParameters { axis: axis, drive: drive, free: free, second: second, spans: limits.spans },
                tolerance_uv,
                super::PcurveWork { max_cells: cells, refinements: limits.root_refinements, band_refinements: limits.root_refinements },
            )?;
            out.proof_cells += proof.visited;
            out.root_queries += proof.section_queries;
            reports.push(proof);
        }
        let accepted = reports.iter().all(|r| r.correspondence_proven);
        out.attempts.push(FitAttempt { degree, reports });
        if accepted {
            out.curves = Some(curves);
            out.stop = FitStop::Certified;
            break;
        }
    }
    Ok(out)
}
