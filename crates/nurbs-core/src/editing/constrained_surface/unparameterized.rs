//! Unknown guide UVs: bounded inverse/fitting proposals followed by complete
//! original-geometry network admission. No nearest/unique inverse claim.
use super::{Constraint, boundary, error, network, rational};
use crate::{Result, curve::Curve, surface::Surface};
/// Inferred physical fitting inherits the registered surfaces owner.
pub mod loading;
mod projection;
pub(super) fn proposal_pcurve(c: &Curve, s: &Surface, o: Options) -> Result<Curve> {
    // Construction only. The adaptive owner must still perform complete
    // ORIGINAL network admission after any new basis/UV proposal.
    Ok(projection::propose(c, s, 4, o.inversion_iterations, true)?.0)
}
#[derive(Clone, Copy)]
pub enum Continuity<'a> {
    Position,
    Tangent(&'a Surface),
    Curvature(&'a Surface),
}
#[derive(Clone, Copy)]
pub struct Guide<'a> {
    pub spatial: &'a Curve,
    pub continuity: Continuity<'a>,
}
#[derive(Clone, Copy)]
pub enum Method {
    Fixed(network::CurvatureOptions),
    PositiveWeights(rational::Options),
}
#[derive(Clone, Copy)]
pub struct Options {
    pub method: Method,
    pub max_rounds: usize,
    pub inversion_iterations: usize,
    pub max_segments: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            method: Method::Fixed(network::CurvatureOptions::default()),
            max_rounds: 6,
            inversion_iterations: 24,
            max_segments: 64,
        }
    }
}
enum Owner {
    Fixed(network::Qualified),
    Positive(rational::Qualified),
    LoadedPositive(rational::loading::Qualified),
}
/// Immutable final surface, inferred pcurves AND original world/support owners.
/// Construction inversion/least-squares residuals never grant admission.
pub struct Qualified {
    owner: Owner,
    original: Surface,
    rounds: usize,
    inversions: usize,
    work: (usize, u64),
    loading_work: u64,
    physical_reference: Option<Surface>,
}
impl Qualified {
    pub fn network(&self) -> &network::Qualified {
        match &self.owner {
            Owner::Fixed(q) => q,
            Owner::Positive(q) => q.network(),
            Owner::LoadedPositive(q) => q.network(),
        }
    }
    pub fn surface(&self) -> &Surface {
        self.network().surface()
    }
    pub fn original(&self) -> &Surface {
        &self.original
    }
    pub fn rounds(&self) -> usize {
        self.rounds
    }
    pub fn inversions(&self) -> usize {
        self.inversions
    }
    pub fn work(&self) -> (usize, u64) {
        self.work
    }
    pub fn loading_work(&self) -> u64 {
        self.loading_work
    }
    pub fn physical_reference(&self) -> Option<&Surface> {
        self.physical_reference.as_ref()
    }
    pub fn positive_loading(&self) -> Option<&rational::loading::Qualified> {
        match &self.owner {
            Owner::LoadedPositive(q) => Some(q),
            _ => None,
        }
    }
    pub fn rational(&self) -> Option<&rational::Qualified> {
        match &self.owner {
            Owner::Positive(q) => Some(q),
            Owner::LoadedPositive(q) => Some(q.fitting()),
            _ => None,
        }
    }
    pub fn reverify(&self, max_cells: usize, max_exact: u64) -> Result<network::Verification> {
        if let Some(reference) = &self.physical_reference {
            let owner = self
                .network()
                .loading()
                .ok_or_else(|| error("Inferred physical owner missing"))?;
            let anchor_matches = if let Some(q) = self.positive_loading() {
                q.original() == &self.original
            } else {
                owner.anchor() == &self.original
            };
            if owner.source() != reference || !anchor_matches {
                return Err(error("Inferred physical rest/trial ownership changed"));
            }
        }
        if let Some(q) = self.positive_loading() {
            q.reverify(max_cells, max_exact)
        } else {
            self.network().reverify(max_cells, max_exact)
        }
    }
}
fn settings(method: Method) -> network::CurvatureOptions {
    match method {
        Method::Fixed(o) => o,
        Method::PositiveWeights(o) => o.network,
    }
}
pub(super) fn validate(
    s: &Surface,
    guides: &[Guide<'_>],
    hard: &[Constraint],
    o: Options,
) -> Result<()> {
    s.validate()?;
    if guides.is_empty()
        || guides.len() > 16
        || hard.len() > 128
        || s.periodic_u
        || s.periodic_v
        || s.control_points[0][0].len() != 3
        || s.weights.iter().flatten().any(|w| *w <= 0.)
        || s.control_points.len() * s.control_points[0].len() > 256
        || !(1..=8).contains(&o.max_rounds)
        || !(1..=64).contains(&o.inversion_iterations)
        || !(1..=128).contains(&o.max_segments)
    {
        return Err(error(
            "Unknown UV solving needs bounded positive nonperiodic 3D originals",
        ));
    }
    // Validate the common native proof budget before expensive inverse proposals.
    let options = settings(o.method);
    network::validate_guides(s, &[], &[], options.network)?;
    for g in guides {
        g.spatial.validate()?;
        if g.spatial.periodic
            || g.spatial.degree > 8
            || g.spatial.control_points.len() > 128
            || g.spatial.control_points[0].len() != 3
            || g.spatial.weights.iter().any(|w| *w <= 0.)
        {
            return Err(error(
                "Unknown UV guides require bounded positive original 3D curves",
            ));
        }
        if let Continuity::Tangent(s) | Continuity::Curvature(s) = g.continuity {
            s.validate()?;
            if s.periodic_u
                || s.periodic_v
                || s.control_points[0][0].len() != 3
                || s.weights.iter().flatten().any(|w| *w <= 0.)
                || s.control_points.len() * s.control_points[0].len() > 4096
            {
                return Err(error(
                    "Unknown support UVs require bounded positive original 3D charts",
                ));
            }
        }
    }
    Ok(())
}
/// Infer candidate AND original-support pcurves instead of requiring authored
/// UVs. Alternates bounded numerical inverse proposals and surface fitting.
/// The authored surface basis remains fixed; optional positive weights use
/// the homogeneous solver. Every returned surface needs full original G0/G1/G2
/// proof and actual hard constraints, not closest-point convergence or samples.
/// Local branch discovery is incomplete: no uniqueness/global inverse claim.
pub fn solve(
    seed: &Surface,
    guides: &[Guide<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    options: Options,
) -> Result<Qualified> {
    run(seed, guides, hard, locks, options, None)
}
fn run(
    seed: &Surface,
    guides: &[Guide<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    options: Options,
    physical: Option<&loading::Settings<'_>>,
) -> Result<Qualified> {
    validate(seed, guides, hard, options)?;
    let limits = settings(options.method);
    let mut current = seed.clone();
    let (mut cells, mut exact, mut inversions) = (0, 0, 0);
    let mut reason = String::new();
    let mut loading_work = 0;
    for round in 0..options.max_rounds {
        let segments = (4usize << round).min(options.max_segments);
        let mut pc = Vec::new();
        let mut support_pc = Vec::new();
        for g in guides {
            let (p, work) = projection::propose(
                g.spatial,
                &current,
                segments,
                options.inversion_iterations,
                round % 2 == 0,
            )?;
            inversions += work;
            pc.push(p);
            support_pc.push(match g.continuity {
                Continuity::Position => None,
                Continuity::Tangent(s) | Continuity::Curvature(s) => {
                    let (p, work) = projection::propose(
                        g.spatial,
                        s,
                        segments,
                        options.inversion_iterations,
                        round % 2 == 0,
                    )?;
                    inversions += work;
                    Some(p)
                }
            });
        }
        // A common authored chart can yield two inverses differing only by
        // numerical iteration noise. Reuse the support clock as a CANDIDATE
        // rather than feeding incompatible near-identical clocks to the joint
        // solve. No chart equivalence or lift proof is borrowed: the actual
        // candidate AND original support still need fresh complete admission.
        for (i, g) in guides.iter().enumerate() {
            if let Continuity::Tangent(s) | Continuity::Curvature(s) = g.continuity {
                if seed.degree_u == s.degree_u
                    && seed.degree_v == s.degree_v
                    && seed.knots_u == s.knots_u
                    && seed.knots_v == s.knots_v
                {
                    let p = support_pc[i].as_ref().unwrap();
                    if pc[i].degree == p.degree
                        && pc[i].knots == p.knots
                        && pc[i].weights == p.weights
                        && pc[i]
                            .control_points
                            .iter()
                            .zip(&p.control_points)
                            .all(|(a, b)| {
                                a.iter()
                                    .zip(b)
                                    .all(|(a, b)| (a - b).abs() <= 1e-8 * (1. + a.abs() + b.abs()))
                            })
                    {
                        pc[i] = p.clone();
                    }
                }
            }
        }
        let mut curves = Vec::new();
        let mut curvature = Vec::new();
        for (i, g) in guides.iter().enumerate() {
            let support = match g.continuity {
                Continuity::Position => None,
                Continuity::Tangent(s) | Continuity::Curvature(s) => Some(network::SupportTrace {
                    surface: s,
                    parameters: support_pc[i].as_ref().unwrap(),
                    reverse: false,
                }),
            };
            if matches!(g.continuity, Continuity::Curvature(_)) {
                curvature.push(network::CurvatureConstraint {
                    spatial: g.spatial,
                    parameters: &pc[i],
                    reverse: false,
                    support: support.unwrap(),
                });
            } else {
                curves.push(network::CurveConstraint {
                    spatial: g.spatial,
                    parameters: &pc[i],
                    reverse: false,
                    tangent: support,
                });
            }
        }
        let mut o = limits;
        let remaining = options.max_rounds - round;
        // Early smooth inverses need a substantial original G2 proof budget.
        // Retain half for future UV/basis proposals; unused successful work is
        // reported exactly, failed calls are conservatively charged in full.
        let reservation_divisor = if remaining > 1 { 2 } else { 1 };
        o.network.max_cells = limits.network.max_cells.saturating_sub(cells) / reservation_divisor;
        o.network.max_exact_work =
            limits.network.max_exact_work.saturating_sub(exact) / reservation_divisor as u64;
        let assembly_reserve = physical
            .map(|p| {
                p.options
                    .max_total_assembly_terms
                    .saturating_sub(loading_work)
                    / reservation_divisor as u64
            })
            .unwrap_or(0);
        if physical.is_some() && assembly_reserve == 0 {
            return Err(error(
                "Inferred shared physical assembly allowance exhausted",
            ));
        }
        let result = if let Some(p) = physical {
            let mut settings = p.options;
            settings.curves = o;
            settings.max_total_assembly_terms = assembly_reserve;
            if let Some(positive) = p.positive {
                let mut w = positive.clone();
                w.geometry.network = o;
                w.max_total_assembly_terms = assembly_reserve;
                rational::loading::solve_with_reference(
                    seed,
                    p.reference,
                    network::ConstraintSet {
                        guides: &curves,
                        curvatures: &curvature,
                        hard,
                        locks,
                    },
                    p.model,
                    w,
                )
                .map(Owner::LoadedPositive)
            } else {
                network::loading::solve_with_reference(
                    seed,
                    p.reference,
                    network::ConstraintSet {
                        guides: &curves,
                        curvatures: &curvature,
                        hard,
                        locks,
                    },
                    p.model,
                    settings,
                )
                .map(Owner::Fixed)
            }
        } else {
            match options.method {
                Method::Fixed(_) => {
                    let q = if !curvature.is_empty() {
                        network::solve_with_curvature(seed, &curves, &curvature, hard, locks, o)
                    } else if let Some(locks) = locks {
                        network::solve_preserving_g2(seed, &curves, hard, locks, o.network)
                    } else {
                        network::solve(seed, &curves, hard, o.network)
                    };
                    q.map(Owner::Fixed)
                }
                Method::PositiveWeights(mut positive) => {
                    positive.network = o;
                    rational::solve(seed, &curves, &curvature, hard, locks, positive)
                        .map(Owner::Positive)
                }
            }
        };
        match result {
            Ok(owner) => {
                loading_work += match &owner {
                    Owner::Fixed(q) => q.loading_work(),
                    Owner::LoadedPositive(q) => q.loading_work(),
                    Owner::Positive(_) => 0,
                };
                if let Some(p) = physical {
                    let q = match &owner {
                        Owner::Fixed(q) => q,
                        Owner::LoadedPositive(q) => q.network(),
                        _ => return Err(error("Unexpected inferred physical weight owner")),
                    };
                    let energy = q
                        .loading()
                        .ok_or_else(|| error("Inferred physical result lost equilibrium"))?;
                    let anchor_matches = match &owner {
                        Owner::LoadedPositive(q) => q.original() == seed,
                        _ => energy.anchor() == seed,
                    };
                    if energy.source() != p.reference
                        || !anchor_matches
                        || loading_work > p.options.max_total_assembly_terms
                    {
                        return Err(error("Inferred physical source/budget binding changed"));
                    }
                }
                let work = match &owner {
                    Owner::Fixed(q) => q.work(),
                    Owner::Positive(q) => q.work(),
                    Owner::LoadedPositive(q) => q.work(),
                };
                return Ok(Qualified {
                    owner,
                    original: seed.clone(),
                    rounds: round + 1,
                    inversions,
                    work: (cells + work.0, exact + work.1),
                    loading_work,
                    physical_reference: physical.map(|p| p.reference.clone()),
                });
            }
            Err(e) => {
                loading_work += assembly_reserve;
                cells += o.network.max_cells;
                exact += o.network.max_exact_work;
                reason.push_str(&format!("round {}: {}; ", round + 1, e));
                // Only a new inverse seed. This sampled fit grants no authority
                // and does not replace the ORIGINAL final anchor or constraints.
                if let Ok(s) =
                    proposal_surface(seed, guides, &pc, hard, locks, limits.network.fitting)
                {
                    current = s;
                }
            }
        }
    }
    Err(error(format!(
        "Unknown UV construction/proof allowance exhausted: {reason}"
    )))
}
fn proposal_surface(
    seed: &Surface,
    guides: &[Guide<'_>],
    pc: &[Curve],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    fitting: super::Options,
) -> Result<Surface> {
    let mut rows = hard.to_vec();
    for (g, p) in guides.iter().zip(pc) {
        let mut sites = network::initial_sites(network::CurveConstraint {
            spatial: g.spatial,
            parameters: p,
            reverse: false,
            tangent: None,
        })?;
        sites.extend([0.25, 0.75]);
        sites.sort_by(f64::total_cmp);
        sites.dedup();
        for t in sites {
            let d = p.domain();
            let uv = p.evaluate(d[0] + t * (d[1] - d[0]))?.point;
            let d = g.spatial.domain();
            let point = g.spatial.evaluate(d[0] + t * (d[1] - d[0]))?.point;
            rows.push(Constraint::Point {
                uv: [uv[0], uv[1]],
                point: [point[0], point[1], point[2]],
            });
        }
    }
    if let Some(lock) = locks {
        lock.reference.validate()?;
        if seed.degree_u != lock.reference.degree_u
            || seed.degree_v != lock.reference.degree_v
            || seed.knots_u != lock.reference.knots_u
            || seed.knots_v != lock.reference.knots_v
        {
            return Err(error(
                "Unknown UV boundary proposal needs matching original bases",
            ));
        }
        for [i, j] in boundary::control_rings(seed, lock.boundaries)?
            .into_iter()
            .map(|(i, j)| [i, j])
        {
            let p = &lock.reference.control_points[i][j];
            if p.len() != 3 {
                return Err(error("Original boundary must be 3D"));
            }
            rows.push(Constraint::ControlPoint {
                index: [i, j],
                point: [p[0], p[1], p[2]],
            });
        }
    }
    if rows.len() > 384 {
        return Err(error(
            "Unknown UV construction constraint allowance exhausted",
        ));
    }
    Ok(super::solve(seed, &rows, fitting)?.surface)
}
