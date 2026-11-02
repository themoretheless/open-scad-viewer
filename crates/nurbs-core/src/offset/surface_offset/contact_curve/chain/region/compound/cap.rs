//! Actual carrier caps: original normal displacement, whole-chart injectivity
//! and unchanged world/UV rim agreement are independent admission gates.
use super::Compound;
use crate::{
    Result, check,
    curve_surface_agreement::{self, Status},
    surface::Surface,
    surface_linear_monotonicity as chart,
    surface_offset::{self, CandidateCertificate, CandidateStatus, ProposalLimits, ProposalReport},
};
#[derive(Clone, Copy)]
pub struct Limits {
    /// One existing bounded analytic/basis offset proposal; its sampling flags
    /// NEVER qualify geometry. Disable for only degree-refined bounded fitting.
    pub prefer_basis: bool,
    pub polynomial: ProposalLimits,
    /// Qualification allowances are independent per attempted carrier (at most
    /// two: basis and polynomial), not refunded into one shared allowance.
    pub normal_cells: usize,
    pub chart_cells: usize,
    pub boundary_cells: usize,
    pub projection_hint: Option<[[i8; 3]; 2]>,
    pub max_poles: usize,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            prefer_basis: true,
            polynomial: ProposalLimits::default(),
            normal_cells: 100000,
            chart_cells: 100000,
            boundary_cells: 100000,
            projection_hint: None,
            max_poles: 4096,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Origin {
    Authored,
    BasisProposal,
    PolynomialProposal,
}
pub struct Trial {
    pub origin: Origin,
    pub reason: String,
}
struct Qualified {
    normal: CandidateCertificate,
    chart: chart::Report,
    boundary: Vec<Vec<curve_surface_agreement::Report>>,
    boundary_cells: usize,
}
pub struct Cap {
    region: Compound,
    surface: Surface,
    normal_tolerance: f64,
    boundary_tolerance: f64,
    limits: Limits,
    origin: Origin,
    qualified: Qualified,
    polynomial: Option<ProposalReport>,
    failed_trials: Vec<Trial>,
}
impl Cap {
    pub fn region(&self) -> &Compound {
        &self.region
    }
    pub fn surface(&self) -> &Surface {
        &self.surface
    }
    pub fn origin(&self) -> Origin {
        self.origin
    }
    pub fn tolerance_uv(&self) -> f64 {
        self.region.outer.chain.tolerance_uv
    }
    pub fn boundary_tolerance(&self) -> f64 {
        self.boundary_tolerance
    }
    pub fn normal_tolerance(&self) -> f64 {
        self.normal_tolerance
    }
    pub fn normal_proof(&self) -> &CandidateCertificate {
        &self.qualified.normal
    }
    pub fn chart_proof(&self) -> &chart::Report {
        &self.qualified.chart
    }
    pub fn boundary_proofs(&self) -> &[Vec<curve_surface_agreement::Report>] {
        &self.qualified.boundary
    }
    pub fn boundary_cells(&self) -> usize {
        self.qualified.boundary_cells
    }
    pub fn polynomial_proposal(&self) -> Option<&ProposalReport> {
        self.polynomial.as_ref()
    }
    pub fn failed_trials(&self) -> &[Trial] {
        &self.failed_trials
    }
    pub fn reverify(&self) -> Result<()> {
        self.region.reverify()?;
        if self.origin == Origin::Authored {
            qualify(
                &self.region,
                &self.surface,
                self.normal_tolerance,
                self.boundary_tolerance,
                self.limits,
            )?;
        } else {
            let (surface, origin, _, _, _) = propose(
                &self.region,
                self.normal_tolerance,
                self.boundary_tolerance,
                self.limits,
            )?;
            check(
                surface == self.surface && origin == self.origin,
                "Cap differs from full original automatic reconstruction",
            )?;
        }
        Ok(())
    }
}
fn domain(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn identity(s: &Surface) -> Surface {
    let [u, v] = domain(s);
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![u[0], u[0], u[1], u[1]],
        knots_v: vec![v[0], v[0], v[1], v[1]],
        control_points: u.map(|u| v.map(|v| vec![u, v, 0.]).to_vec()).to_vec(),
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn source(r: &Compound) -> (&Surface, f64) {
    let i = r.outer.side;
    (&r.outer.chain.sources[i], r.outer.chain.distances[i])
}
fn validate(r: &Compound, n: f64, b: f64, l: Limits) -> Result<()> {
    let p = l.polynomial;
    check(
        n.is_finite()
            && n * 0.25 > 0.
            && b.is_finite()
            && b > 0.
            && (1..=1000000).contains(&l.normal_cells)
            && (1..=100000).contains(&l.chart_cells)
            && (1..=100000).contains(&l.boundary_cells)
            && (4..=1000000).contains(&l.max_poles)
            && (1..=16).contains(&p.max_degree)
            && (1..=100000).contains(&p.evaluations)
            && (1..=10000000).contains(&p.coefficient_work)
            && (1..=1000000).contains(&p.proof_cells),
        "Choose finite cap tolerances and explicit per-attempt work limits",
    )?;
    let (s, _) = source(r);
    check(
        s.control_points.len() * s.control_points[0].len() <= l.max_poles,
        "Original cap carrier exceeds the pole allowance",
    )
}
fn qualify(r: &Compound, candidate: &Surface, n: f64, b: f64, l: Limits) -> Result<Qualified> {
    candidate.validate()?;
    let (s, d) = source(r);
    check(
        candidate.control_points.len() * candidate.control_points[0].len() <= l.max_poles
            && domain(candidate) == domain(s),
        "Actual cap must retain the literal original UV domain within pole limits",
    )?;
    let normal = surface_offset::certify_parametric_candidate(
        s,
        candidate,
        &identity(s),
        d,
        n,
        l.normal_cells,
    )?;
    check(
        normal.status == CandidateStatus::Within,
        "Actual cap normal displacement remains unproved",
    )?;
    let chart = chart::inspect_candidate_with_hint(candidate, l.projection_hint, l.chart_cells)?;
    check(
        chart.certified,
        "Actual cap whole-chart injectivity remains unproved",
    )?;
    let mut boundary = vec![];
    let mut cells = 0;
    for (uv, world) in r.uv_domain().loops().iter().zip(r.world_loops()) {
        let mut proofs = vec![];
        for (p, c) in uv.iter().zip(world) {
            check(
                cells < l.boundary_cells,
                "Shared actual cap boundary work exhausted",
            )?;
            let proof = curve_surface_agreement::verify(
                c,
                p,
                candidate,
                false,
                b,
                l.boundary_cells - cells,
            )?;
            cells += proof.cells;
            check(
                proof.status == Status::WithinTolerance,
                "Unchanged world rim does not agree with the ACTUAL cap",
            )?;
            proofs.push(proof);
        }
        boundary.push(proofs);
    }
    Ok(Qualified {
        normal,
        chart,
        boundary,
        boundary_cells: cells,
    })
}
fn propose(
    r: &Compound,
    n: f64,
    b: f64,
    l: Limits,
) -> Result<(
    Surface,
    Origin,
    Qualified,
    Option<ProposalReport>,
    Vec<Trial>,
)> {
    let (s, d) = source(r);
    let mut failed = vec![];
    if l.prefer_basis {
        let trial = surface_offset::offset(s, d, n * 0.25)
            .and_then(|p| qualify(r, &p.surface, n, b, l).map(|q| (p.surface, q)));
        match trial {
            Ok((surface, proof)) => {
                return Ok((surface, Origin::BasisProposal, proof, None, failed));
            }
            Err(e) => failed.push(Trial {
                origin: Origin::BasisProposal,
                reason: e.to_string(),
            }),
        }
    }
    let proposal =
        surface_offset::propose_parametric_candidate(s, &identity(s), d, n * 0.25, l.polynomial)?;
    let surface = proposal
        .cap
        .as_ref()
        .ok_or_else(|| crate::input("Degree-refined actual cap remains unqualified"))?
        .clone();
    let proof = qualify(r, &surface, n, b, l)?;
    Ok((
        surface,
        Origin::PolynomialProposal,
        proof,
        Some(proposal),
        failed,
    ))
}
/// No cap poles, UV remapping, world-rim edits or positive receipts are supplied.
/// An analytic/basis proposal is tried first, then degree-refined fitting. Every
/// accepted ACTUAL surface must pass full original normal displacement, global
/// chart injectivity and complete agreement with every unchanged rim curve.
/// This builds a trimmed carrier, not a closed shell, material volume or sewn solid.
pub fn build(
    region: Compound,
    normal_tolerance: f64,
    boundary_tolerance: f64,
    limits: Limits,
) -> Result<Cap> {
    validate(&region, normal_tolerance, boundary_tolerance, limits)?;
    let (surface, origin, qualified, polynomial, failed_trials) =
        propose(&region, normal_tolerance, boundary_tolerance, limits)?;
    Ok(Cap {
        region,
        surface,
        normal_tolerance,
        boundary_tolerance,
        limits,
        origin,
        qualified,
        polynomial,
        failed_trials,
    })
}
/// An authored surface is a proposal only. A small normal error cannot waive
/// actual rim agreement or chart injectivity. No fallback silently replaces it.
pub fn with_candidate(
    region: Compound,
    candidate: &Surface,
    normal_tolerance: f64,
    boundary_tolerance: f64,
    limits: Limits,
) -> Result<Cap> {
    validate(&region, normal_tolerance, boundary_tolerance, limits)?;
    let qualified = qualify(
        &region,
        candidate,
        normal_tolerance,
        boundary_tolerance,
        limits,
    )?;
    Ok(Cap {
        region,
        surface: candidate.clone(),
        normal_tolerance,
        boundary_tolerance,
        limits,
        origin: Origin::Authored,
        qualified,
        polynomial: None,
        failed_trials: vec![],
    })
}
