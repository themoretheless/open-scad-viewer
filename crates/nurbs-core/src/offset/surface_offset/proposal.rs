//! Degree-refined cap proposals; only fresh complete offset bounds admit poles.
use super::{CandidateCertificate, CandidateStatus, mapped, parametric};
use crate::{
    Result, bernstein_interpolation as interpolation, check, distance_bounds::Interval as I,
    surface::Surface,
};

#[derive(Clone, Copy, Debug)]
pub struct ProposalLimits {
    pub max_degree: usize,
    /// Number of target station evaluations (map followed by original offset).
    pub evaluations: usize,
    pub coefficient_work: usize,
    /// Shared residual-proof cell allowance across every degree attempt.
    pub proof_cells: usize,
}
impl Default for ProposalLimits {
    fn default() -> Self {
        Self {
            max_degree: 16,
            evaluations: 4096,
            coefficient_work: 100000,
            proof_cells: 100000,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProposalStop {
    Certified,
    DegreeLimit,
    EvaluationLimit,
    CoefficientLimit,
    ProofLimit,
}
pub struct ProposalAttempt {
    pub degree: usize,
    pub certificate: CandidateCertificate,
}
pub struct ProposalReport {
    /// Present only after this literal rounded net passed a complete proof.
    pub cap: Option<Surface>,
    pub attempts: Vec<ProposalAttempt>,
    pub evaluations: usize,
    pub coefficient_work: usize,
    pub proof_cells: usize,
    pub stop: ProposalStop,
}
fn nodes(domain: [f64; 2], degree: usize) -> Result<Vec<f64>> {
    let out: Vec<_> = (0..=degree)
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
        out.windows(2).all(|p| p[0] < p[1]),
        "Cap proposal nodes reached parameter resolution",
    )?;
    Ok(out)
}
/// Original map and offsets are sampled only to define a polynomial proposal.
/// No assumption that the original expression is polynomial is made. Interval
/// nodal reconstruction encloses the interpolant's coefficients; midpoint poles
/// are then independently checked against the original function over the WHOLE
/// candidate. Both evaluation and control rounding are covered by that check.
pub fn propose_parametric_candidate(
    source: &Surface,
    uv_map: &Surface,
    distance: f64,
    tolerance: f64,
    limits: ProposalLimits,
) -> Result<ProposalReport> {
    check(
        distance.is_finite() && tolerance.is_finite() && tolerance > 0.,
        "Invalid cap proposal distance/tolerance",
    )?;
    check(
        (1..=16).contains(&limits.max_degree)
            && (1..=100000).contains(&limits.evaluations)
            && (1..=10000000).contains(&limits.coefficient_work)
            && (1..=1000000).contains(&limits.proof_cells),
        "Choose bounded positive cap proposal work",
    )?;
    parametric::validate_map(source, uv_map)?;
    let domain = mapped::domain(uv_map);
    let mut degrees = vec![1usize];
    while *degrees.last().unwrap() < limits.max_degree {
        degrees.push((2 * degrees.last().unwrap()).min(limits.max_degree));
    }
    let mut report = ProposalReport {
        cap: None,
        attempts: vec![],
        evaluations: 0,
        coefficient_work: 0,
        proof_cells: 0,
        stop: ProposalStop::DegreeLimit,
    };
    for (attempt, &degree) in degrees.iter().enumerate() {
        let n = degree + 1;
        if n * n > limits.evaluations - report.evaluations {
            report.stop = ProposalStop::EvaluationLimit;
            break;
        }
        // These are the owners' exact bounded work formulas. Check the whole
        // attempt before creating bases, station grids or coefficient arrays.
        let required = n * degree * (degree + 3) + 3 * n * n * (2 * n);
        if required > limits.coefficient_work - report.coefficient_work {
            report.stop = ProposalStop::CoefficientLimit;
            break;
        }
        if report.proof_cells == limits.proof_cells {
            report.stop = ProposalStop::ProofLimit;
            break;
        }
        let u = nodes(domain[0], degree)?;
        let v = nodes(domain[1], degree)?;
        let (Some(bu), wu) = interpolation::basis_on(
            &u,
            domain[0],
            limits.coefficient_work - report.coefficient_work,
        )?
        else {
            unreachable!("prechecked basis work")
        };
        report.coefficient_work += wu;
        let (Some(bv), wv) = interpolation::basis_on(
            &v,
            domain[1],
            limits.coefficient_work - report.coefficient_work,
        )?
        else {
            unreachable!("prechecked basis work")
        };
        report.coefficient_work += wv;
        let mut samples = vec![vec![[0.; 3]; n]; n];
        for (i, &u) in u.iter().enumerate() {
            for (j, &v) in v.iter().enumerate() {
                report.evaluations += 1;
                let uv = uv_map.evaluate(u, v)?.point;
                samples[i][j] = super::evaluate(source, [uv[0], uv[1]], distance)?.point;
            }
        }
        let mut controls = vec![vec![vec![0.; 3]; n]; n];
        for k in 0..3 {
            let nodal: Vec<Vec<I>> = samples
                .iter()
                .map(|row| row.iter().map(|p| I::point(p[k])).collect())
                .collect();
            let (Some(coefficients), work) = interpolation::reconstruct_tensor(
                &nodal,
                [&bu, &bv],
                limits.coefficient_work - report.coefficient_work,
            )?
            else {
                unreachable!("prechecked tensor work")
            };
            report.coefficient_work += work;
            for i in 0..n {
                for j in 0..n {
                    let c = coefficients[i][j];
                    controls[i][j][k] = c.lo * 0.5 + c.hi * 0.5;
                }
            }
        }
        let knots = |r: [f64; 2]| {
            std::iter::repeat_n(r[0], n)
                .chain(std::iter::repeat_n(r[1], n))
                .collect()
        };
        let cap = Surface {
            degree_u: degree,
            degree_v: degree,
            knots_u: knots(domain[0]),
            knots_v: knots(domain[1]),
            control_points: controls,
            weights: vec![vec![1.; n]; n],
            periodic_u: false,
            periodic_v: false,
        };
        // Reserve proof work for later degrees; an unresolved coarse attempt
        // cannot consume every future proposal's allowance.
        let remaining = limits.proof_cells - report.proof_cells;
        let cells = (remaining / (degrees.len() - attempt)).max(1);
        let certificate =
            super::certify_parametric_candidate(source, &cap, uv_map, distance, tolerance, cells)?;
        report.proof_cells += certificate.cells;
        let accepted = certificate.status == CandidateStatus::Within;
        report.attempts.push(ProposalAttempt {
            degree,
            certificate,
        });
        if accepted {
            report.cap = Some(cap);
            report.stop = ProposalStop::Certified;
            break;
        }
    }
    Ok(report)
}
