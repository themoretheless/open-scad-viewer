//! Whole original curve networks on fixed-basis positive-weight NURBS.
//! Collocation proposes controls; only independent whole-curve lift/G1 checks
//! admit the actual saved surface. No interior embedding or solid is certified.
use super::{Constraint, Options as FitOptions, Report, boundary, error};
use crate::{Result, curve::Curve, surface::Surface, surface_join::Boundary};
mod curvature;
pub(super) fn curvature_equations(
    current: &Surface,
    uv: [f64; 2],
    support: &Surface,
    support_uv: [f64; 2],
    out: &mut Vec<Constraint>,
) -> Result<()> {
    curvature::equations(current, uv, support, support_uv, out)
}
/// Physical shell objectives share this registered surface-network owner.
pub mod loading;
mod proof;
mod sites;
pub(super) fn validate_guides(
    seed: &Surface,
    guides: &[CurveConstraint<'_>],
    curvatures: &[CurvatureConstraint<'_>],
    options: Options,
) -> Result<()> {
    proof::budget(options)?;
    for g in guides {
        proof::validate(seed, &OwnedGuide::new(*g))?;
    }
    for g in curvatures {
        proof::validate(seed, &OwnedGuide::new_curvature(*g))?;
    }
    Ok(())
}
pub(super) fn initial_sites(guide: CurveConstraint<'_>) -> Result<Vec<f64>> {
    sites::initial(&OwnedGuide::new(guide))
}
pub use curvature::{CurvatureConstraint, CurvatureOptions, solve_with_curvature};
#[derive(Clone, Copy)]
pub struct SupportTrace<'a> {
    pub surface: &'a Surface,
    pub parameters: &'a Curve,
    /// Original spatial curve orientation relative to this support pcurve.
    pub reverse: bool,
}
#[derive(Clone, Copy)]
pub struct CurveConstraint<'a> {
    pub spatial: &'a Curve,
    pub parameters: &'a Curve,
    /// Original spatial curve orientation relative to the candidate pcurve.
    pub reverse: bool,
    pub tangent: Option<SupportTrace<'a>>,
}
#[derive(Clone, Copy)]
pub struct Options {
    pub fitting: FitOptions,
    pub normal_chord_tolerance: f64,
    pub max_iterations: usize,
    pub max_sites_per_curve: usize,
    pub max_total_sites: usize,
    pub max_cells: usize,
    pub max_exact_work: u64,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            fitting: FitOptions::default(),
            normal_chord_tolerance: 1e-5,
            max_iterations: 6,
            max_sites_per_curve: 65,
            max_total_sites: 96,
            max_cells: 100000,
            max_exact_work: 1000000,
        }
    }
}
#[derive(Clone, Copy)]
pub struct BoundaryLocks<'a> {
    pub reference: &'a Surface,
    pub boundaries: &'a [Boundary],
    pub tolerances: [f64; 3],
}
struct OwnedSupport {
    surface: Surface,
    parameters: Curve,
    reverse: bool,
}
struct OwnedGuide {
    spatial: Curve,
    parameters: Curve,
    reverse: bool,
    tangent: Option<OwnedSupport>,
    curvature: bool,
}
impl OwnedGuide {
    fn new(g: CurveConstraint<'_>) -> Self {
        Self {
            spatial: g.spatial.clone(),
            parameters: g.parameters.clone(),
            reverse: g.reverse,
            curvature: false,
            tangent: g.tangent.map(|s| OwnedSupport {
                surface: s.surface.clone(),
                parameters: s.parameters.clone(),
                reverse: s.reverse,
            }),
        }
    }
    fn new_curvature(g: CurvatureConstraint<'_>) -> Self {
        let mut owned = Self::new(CurveConstraint {
            spatial: g.spatial,
            parameters: g.parameters,
            reverse: g.reverse,
            tangent: Some(g.support),
        });
        owned.curvature = true;
        owned
    }
    fn borrowed(&self) -> CurveConstraint<'_> {
        CurveConstraint {
            spatial: &self.spatial,
            parameters: &self.parameters,
            reverse: self.reverse,
            tangent: self.tangent.as_ref().map(|s| SupportTrace {
                surface: &s.surface,
                parameters: &s.parameters,
                reverse: s.reverse,
            }),
        }
    }
}
pub enum Evidence {
    Position(crate::curve_surface_agreement::Report),
    TangentPlane(crate::surface_g1_pcurves::Report),
    Curvature(crate::surface_g2_pcurves::Report),
}
impl Evidence {
    pub fn admitted(&self) -> bool {
        match self {
            Self::Curvature(r) => {
                r.positions.proven
                    && r.geometric.regularity_proven
                    && r.geometric.decision == crate::surface_join::Decision::WithinTolerance
            }
            Self::Position(r) => {
                r.status == crate::curve_surface_agreement::Status::WithinTolerance
            }
            Self::TangentPlane(r) => {
                r.positions.proven
                    && r.regularity_proven
                    && r.decision == crate::surface_join::Decision::WithinTolerance
            }
        }
    }
    fn witness(&self, reverse: bool) -> Option<f64> {
        match self {
            Self::Curvature(r) => r.positions.sides[0]
                .interval
                .as_ref()
                .and_then(|p| p.witness),
            Self::Position(r) => r.witness,
            Self::TangentPlane(r) => r
                .violation_parameter
                .map(|t| if reverse { 1. - t } else { t })
                .or_else(|| {
                    r.positions.sides[0]
                        .interval
                        .as_ref()
                        .and_then(|p| p.witness)
                }),
        }
    }
}
pub struct Verification {
    pub diagnostics: Vec<String>,
    pub evidence: Vec<Evidence>,
    pub boundary_seams: Vec<(Boundary, crate::surface_g2::Report)>,
    pub admitted: bool,
    pub cells: usize,
    pub exact_work: u64,
    pub loading: Option<super::variational::Diagnostics>,
}
/// Immutable solved geometry and original guide snapshots. Proof is confined
/// to the requested curve network and optional boundary two-jets; it does not
/// certify regularity/injectivity throughout the surface interior.
pub struct Qualified {
    solution: Report,
    guides: Vec<OwnedGuide>,
    evidence: Verification,
    boundary: Option<boundary::Qualified>,
    boundary_tolerances: Option<[f64; 3]>,
    constraints: Vec<Constraint>,
    sites: Vec<Vec<f64>>,
    iterations: usize,
    cells: usize,
    exact_work: u64,
    options: Options,
    curvature_tolerance: Option<f64>,
    loading: Option<super::variational::Qualified>,
    loading_work: u64,
}
impl Qualified {
    pub fn surface(&self) -> &Surface {
        &self.solution.surface
    }
    pub fn solution(&self) -> &Report {
        &self.solution
    }
    pub fn loading(&self) -> Option<&super::variational::Qualified> {
        self.loading.as_ref()
    }
    pub fn loading_work(&self) -> u64 {
        self.loading_work
    }
    pub fn original_guides(&self) -> Vec<CurveConstraint<'_>> {
        self.guides.iter().map(OwnedGuide::borrowed).collect()
    }
    pub fn original_curvature_guides(&self) -> Vec<CurvatureConstraint<'_>> {
        self.guides
            .iter()
            .filter(|g| g.curvature)
            .map(|g| CurvatureConstraint {
                spatial: &g.spatial,
                parameters: &g.parameters,
                reverse: g.reverse,
                support: g
                    .borrowed()
                    .tangent
                    .expect("G2 owner retains its original support"),
            })
            .collect()
    }
    pub fn evidence(&self) -> &Verification {
        &self.evidence
    }
    pub fn boundary_evidence(&self) -> Option<&boundary::Qualified> {
        self.boundary.as_ref()
    }
    pub fn constraints(&self) -> &[Constraint] {
        &self.constraints
    }
    pub fn sites(&self) -> &[Vec<f64>] {
        &self.sites
    }
    pub fn iterations(&self) -> usize {
        self.iterations
    }
    pub fn work(&self) -> (usize, u64) {
        (self.cells, self.exact_work)
    }
    /// Recheck every original guide and every requested boundary two-jet on
    /// the actual saved surface, within one shared verification allowance.
    pub fn reverify(&self, max_cells: usize, max_exact_work: u64) -> Result<Verification> {
        let mut options = self.options;
        options.max_cells = max_cells;
        options.max_exact_work = max_exact_work;
        proof::budget(options)?;
        let mut remaining = max_cells;
        let mut seams = Vec::new();
        if let (Some(boundary), Some(tolerance)) = (&self.boundary, self.boundary_tolerances) {
            for (i, (side, _)) in boundary.seams().iter().enumerate() {
                let report = crate::surface_g2::inspect(
                    self.surface(),
                    *side,
                    boundary.reference(),
                    *side,
                    false,
                    tolerance[0],
                    tolerance[1],
                    tolerance[2],
                    remaining / (boundary.seams().len() - i + 1),
                )?;
                remaining = remaining.saturating_sub(report.cells);
                seams.push((*side, report));
            }
        }
        let mut result = proof::verify(
            self.surface(),
            &self.guides,
            options,
            self.curvature_tolerance,
            remaining,
            max_exact_work,
        )?;
        result.cells += max_cells - remaining;
        result.admitted &= seams.iter().all(|(_, r)| {
            r.decision == crate::surface_join::Decision::WithinTolerance && r.regularity_proven
        });
        result.boundary_seams = seams;
        if let Some(owner) = &self.loading {
            if owner.surface() != self.surface() {
                return Err(error("Saved loaded network geometry changed"));
            }
            result.loading = Some(loading::reverify(self)?);
        }
        Ok(result)
    }
}
/// Solve whole spatial curve/UV networks, optionally with original-support G1.
/// Input UV clocks are authored, never inferred from nearest projections.
/// Numerical sites and rank reports do not authorize a returned Qualified.
pub fn solve(
    seed: &Surface,
    guides: &[CurveConstraint<'_>],
    constraints: &[Constraint],
    options: Options,
) -> Result<Qualified> {
    solve_impl(seed, guides, &[], constraints, None, options, None, None)
}
/// Combine whole interior curve constraints with preserved complete original
/// boundary 0/1/2-jets. Each actual solve must freshly qualify the locked seams;
/// the original curve network is checked afterward on those exact coefficients.
pub fn solve_preserving_g2(
    seed: &Surface,
    guides: &[CurveConstraint<'_>],
    constraints: &[Constraint],
    locks: BoundaryLocks<'_>,
    options: Options,
) -> Result<Qualified> {
    solve_impl(
        seed,
        guides,
        &[],
        constraints,
        Some(locks),
        options,
        None,
        None,
    )
}
fn solve_impl(
    seed: &Surface,
    guides: &[CurveConstraint<'_>],
    curvatures: &[CurvatureConstraint<'_>],
    constraints: &[Constraint],
    locks: Option<BoundaryLocks<'_>>,
    options: Options,
    curvature_tolerance: Option<f64>,
    loading: Option<&loading::Settings<'_>>,
) -> Result<Qualified> {
    seed.validate()?;
    proof::budget(options)?;
    if guides.len() + curvatures.len() == 0
        || guides.len() + curvatures.len() > 16
        || constraints.len() > 384
    {
        return Err(error(
            "Choose 1..16 original curve guides and bounded hard constraints",
        ));
    }
    let mut owned = guides
        .iter()
        .copied()
        .map(OwnedGuide::new)
        .collect::<Vec<_>>();
    owned.extend(curvatures.iter().copied().map(OwnedGuide::new_curvature));
    if !curvatures.is_empty() && curvature_tolerance.is_none_or(|t| !t.is_finite() || t <= 0.) {
        return Err(error(
            "Choose a finite positive whole G2 curvature tolerance",
        ));
    }
    let mut linearization = seed.clone();
    for guide in &owned {
        proof::validate(seed, guide)?;
    }
    let mut sites = owned
        .iter()
        .map(sites::initial)
        .collect::<Result<Vec<_>>>()?;
    let mut cells = 0;
    let mut exact_work = 0;
    let mut loading_work = 0;
    let mut last_diagnostics = String::new();
    let mut fitting = options.fitting;
    fitting.position_tolerance /= 8.;
    fitting.normal_tolerance = fitting
        .normal_tolerance
        .min(options.normal_chord_tolerance.powi(2) / 32.);
    for iteration in 0..options.max_iterations {
        if sites.iter().any(|s| s.len() > options.max_sites_per_curve)
            || sites.iter().map(Vec::len).sum::<usize>() > options.max_total_sites
        {
            return Err(error(format!(
                "Whole curve network proposal-site allowance exhausted: {last_diagnostics}"
            )));
        }
        let mut all = constraints.to_vec();
        for (g, sites) in owned.iter().zip(&sites) {
            // Establish feasible position/tangent constraints before Newton's
            // curvature equations. A flat seed need not share their tangent
            // metric; linearizing all G2 rows there can be inconsistent even
            // when the exact nonlinear problem is feasible.
            sites::constraints(
                g,
                sites,
                (iteration > 0).then_some(&linearization),
                &mut all,
            )?;
        }
        if all.len() > 384 {
            return Err(error(
                "Whole network exceeds the native hard-constraint allowance",
            ));
        }
        let rounds = options.max_iterations - iteration;
        let available = options.max_cells.saturating_sub(cells);
        if available < 2 * rounds {
            return Err(error("Whole network verification work exhausted"));
        }
        let (mut solution, boundary, mut physical) = if let Some(settings) = loading {
            let (s, b, p) = loading::propose(
                seed,
                &all,
                locks,
                fitting,
                available / (2 * rounds),
                settings,
                &mut loading_work,
            )?;
            (s, b, Some(p))
        } else {
            let boundary = if let Some(lock) = locks {
                let owner = boundary::solve_preserving_g2(
                    seed,
                    lock.reference,
                    lock.boundaries,
                    &all,
                    fitting,
                    lock.tolerances,
                    available / (2 * rounds),
                )?;
                Some(owner)
            } else {
                None
            };
            let solution = if let Some(b) = &boundary {
                b.solution().clone()
            } else {
                super::solve(seed, &all, fitting)?
            };
            (solution, boundary, None)
        };
        if let Some(b) = &boundary {
            cells += b.seams().iter().map(|(_, r)| r.cells).sum::<usize>();
        }
        let mut verified = proof::verify(
            &solution.surface,
            &owned,
            options,
            curvature_tolerance,
            options.max_cells.saturating_sub(cells) / rounds,
            options.max_exact_work.saturating_sub(exact_work) / rounds as u64,
        )?;
        cells += verified.cells;
        exact_work += verified.exact_work;
        if !verified.diagnostics.is_empty() {
            last_diagnostics = verified.diagnostics.join("; ");
        }
        if let Some(b) = &boundary {
            verified.boundary_seams = b.seams().to_vec();
            verified.cells += b.seams().iter().map(|(_, r)| r.cells).sum::<usize>();
        }
        if cells > options.max_cells || exact_work > options.max_exact_work {
            return Err(error("Curve network proof exceeded its shared allowance"));
        }
        let mut relinearize_loading = false;
        if verified.admitted {
            if let (Some(settings), Some(current)) = (loading, physical.as_ref()) {
                let fit = loading::NetworkFit { owned: &owned, sites: &sites, constraints, locks, fitting };
                match loading::admit_current(
                    seed,
                    &solution.surface,
                    fit,
                    settings,
                    &mut loading_work,
                    current.diagnostics().assembly_terms,
                )? {
                    Ok(owner) => {
                        solution = owner.solution().clone();
                        physical = Some(owner);
                    }
                    Err(reason) => {
                        if curvatures.is_empty() {
                            return Err(error(reason));
                        }
                        relinearize_loading = true;
                        last_diagnostics = reason;
                    }
                }
            }
        }
        if verified.admitted && !relinearize_loading {
            verified.loading = physical.as_ref().map(|p| p.diagnostics().clone());
            return Ok(Qualified {
                solution,
                guides: owned,
                evidence: verified,
                boundary,
                boundary_tolerances: locks.map(|l| l.tolerances),
                constraints: constraints.to_vec(),
                sites,
                iterations: iteration + 1,
                cells,
                exact_work,
                options,
                curvature_tolerance,
                loading: physical,
                loading_work,
            });
        }
        let mut changed = relinearize_loading;
        for (i, e) in verified.evidence.iter().enumerate() {
            if !e.admitted() {
                if owned[i].curvature
                    && matches!(e,Evidence::Curvature(r) if r.positions.proven)
                    && curvature::needs_update(
                        &solution.surface,
                        &owned[i],
                        &sites[i],
                        curvature_tolerance.unwrap() / 8.,
                        options.normal_chord_tolerance / 8.,
                    )?
                {
                    changed = true;
                    continue;
                }
                changed |= sites::refine(
                    &mut sites[i],
                    e.witness(owned[i].reverse),
                    options.max_sites_per_curve,
                );
            }
        }
        linearization = solution.surface;
        if !changed {
            return Err(error(format!(
                "No new bounded sites can resolve the whole original curve proof: {}",
                verified.diagnostics.join("; ")
            )));
        }
    }
    Err(error(format!(
        "Whole original curve/G1/G2 network remains unproven after bounded refinement: {last_diagnostics}"
    )))
}
