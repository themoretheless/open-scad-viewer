//! Bounded surface basis adaptation. Refined anchors/reference representations
//! are proposals; whole original guides AND original boundary seams must pass.
use super::{Constraint, error, network, rational, unparameterized};
use crate::{
    Result,
    curve::Curve,
    surface::{Axis, Surface},
    surface_join::Boundary,
};
mod basis;
/// Physical fixed-weight fitting remains owned by this registered surface parent.
pub mod loading;
pub use basis::{Direction, Step};
pub use unparameterized::Method;
#[derive(Clone, Copy)]
pub struct Options {
    pub method: Method,
    pub max_refinements: usize,
    pub max_degree: usize,
    pub max_controls: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            method: Method::Fixed(network::CurvatureOptions::default()),
            max_refinements: 8,
            max_degree: 8,
            max_controls: 256,
        }
    }
}
#[derive(Clone, Copy)]
pub struct Inversion {
    pub max_rounds: usize,
    pub iterations: usize,
    pub max_segments: usize,
}
impl Default for Inversion {
    fn default() -> Self {
        Self {
            max_rounds: 4,
            iterations: 24,
            max_segments: 64,
        }
    }
}
enum Owner {
    Network(network::Qualified),
    Rational(rational::Qualified),
    LoadedPositive(rational::loading::Qualified),
    Inferred(unparameterized::Qualified),
}
impl Owner {
    fn network(&self) -> &network::Qualified {
        match self {
            Self::Network(q) => q,
            Self::Rational(q) => q.network(),
            Self::LoadedPositive(q) => q.network(),
            Self::Inferred(q) => q.network(),
        }
    }
    fn loading_work(&self) -> u64 {
        match self {
            Self::Network(q) => q.loading_work(),
            Self::Inferred(q) => q.loading_work(),
            Self::Rational(_) => 0,
            Self::LoadedPositive(q) => q.loading_work(),
        }
    }
    fn positive_loading(&self) -> Option<&rational::loading::Qualified> {
        match self {
            Self::LoadedPositive(q) => Some(q),
            Self::Inferred(q) => q.positive_loading(),
            _ => None,
        }
    }
    fn work(&self) -> (usize, u64) {
        match self {
            Self::Network(q) => q.work(),
            Self::Rational(q) => q.work(),
            Self::LoadedPositive(q) => q.work(),
            Self::Inferred(q) => q.work(),
        }
    }
}
/// Immutable accepted geometry and ORIGINAL seed/boundary owners. Basis edits
/// retain parameter domains, never confer shape-preservation proof by themselves.
pub struct Qualified {
    owner: Owner,
    original: Surface,
    anchor: Surface,
    steps: Vec<Step>,
    original_boundary: Option<OriginalBoundary>,
    seams: Vec<(Boundary, crate::surface_g2_pcurves::Report)>,
    work: (usize, u64),
    loading_work: u64,
}
struct OriginalBoundary {
    reference: Surface,
    sides: Vec<Boundary>,
    tolerances: [f64; 3],
}
pub struct Verification {
    pub network: network::Verification,
    pub original_seams: Vec<(Boundary, crate::surface_g2_pcurves::Report)>,
    pub admitted: bool,
    pub cells: usize,
    pub exact_work: u64,
}
impl Qualified {
    pub fn surface(&self) -> &Surface {
        self.owner.network().surface()
    }
    pub fn network(&self) -> &network::Qualified {
        self.owner.network()
    }
    pub fn positive_loading(&self) -> Option<&rational::loading::Qualified> {
        self.owner.positive_loading()
    }
    pub fn original(&self) -> &Surface {
        &self.original
    }
    /// Refined representation used ONLY as the final fitting anchor.
    pub fn anchor(&self) -> &Surface {
        &self.anchor
    }
    pub fn refinements(&self) -> &[Step] {
        &self.steps
    }
    pub fn original_seams(&self) -> &[(Boundary, crate::surface_g2_pcurves::Report)] {
        &self.seams
    }
    pub fn original_boundary_reference(&self) -> Option<&Surface> {
        self.original_boundary.as_ref().map(|o| &o.reference)
    }
    pub fn work(&self) -> (usize, u64) {
        self.work
    }
    pub fn loading_work(&self) -> u64 {
        self.loading_work
    }
    pub fn reverify(&self, max_cells: usize, max_exact: u64) -> Result<Verification> {
        if !(1..=100000).contains(&max_cells) || max_exact > 1000000 {
            return Err(error("Bounded adaptive replay allowance required"));
        }
        if let Some(p) = self.network().loading() {
            let anchor_matches = if let Some(q) = self.positive_loading() {
                q.original() == &self.anchor
            } else {
                p.anchor() == &self.anchor
            };
            if p.source() != &self.original || !anchor_matches {
                return Err(error(
                    "Adaptive physical original/reference ownership changed",
                ));
            }
        }
        let seam_budget = if self.original_boundary.is_some() {
            max_cells / 2
        } else {
            0
        };
        let seams = verify_boundaries(
            self.surface(),
            self.original_boundary.as_ref(),
            seam_budget,
            max_exact / 3,
        )?;
        let cells = seams.iter().map(|(_, r)| r.geometric.cells).sum::<usize>();
        let exact = seams
            .iter()
            .map(|(_, r)| r.positions.exact_work)
            .sum::<u64>();
        let network = if let Some(q) = self.positive_loading() {
            q.reverify(
                max_cells.saturating_sub(cells),
                max_exact.saturating_sub(exact),
            )?
        } else {
            self.network().reverify(
                max_cells.saturating_sub(cells),
                max_exact.saturating_sub(exact),
            )?
        };
        let admitted = network.admitted && seams_admitted(&seams);
        Ok(Verification {
            cells: cells + network.cells,
            exact_work: exact + network.exact_work,
            network,
            original_seams: seams,
            admitted,
        })
    }
}
fn seams_admitted(seams: &[(Boundary, crate::surface_g2_pcurves::Report)]) -> bool {
    seams.iter().all(|(_, r)| {
        r.positions.proven
            && r.geometric.regularity_proven
            && r.geometric.decision == crate::surface_join::Decision::WithinTolerance
    })
}
fn verify_boundaries(
    s: &Surface,
    original: Option<&OriginalBoundary>,
    budget: usize,
    max_exact: u64,
) -> Result<Vec<(Boundary, crate::surface_g2_pcurves::Report)>> {
    let mut out = Vec::new();
    let mut remaining = budget;
    let mut exact = max_exact;
    if let Some(o) = original {
        for (i, &side) in o.sides.iter().enumerate() {
            let reference = &o.reference;
            let u = [
                reference.knots_u[reference.degree_u],
                reference.knots_u[reference.control_points.len()],
            ];
            let v = [
                reference.knots_v[reference.degree_v],
                reference.knots_v[reference.control_points[0].len()],
            ];
            let (axis, value, ends) = match side {
                Boundary::UMin => (Axis::U, u[0], [[u[0], v[0]], [u[0], v[1]]]),
                Boundary::UMax => (Axis::U, u[1], [[u[1], v[0]], [u[1], v[1]]]),
                Boundary::VMin => (Axis::V, v[0], [[u[0], v[0]], [u[1], v[0]]]),
                Boundary::VMax => (Axis::V, v[1], [[u[0], v[1]], [u[1], v[1]]]),
            };
            // Extraction only proposes a common spatial carrier. The native
            // inspector independently verifies its ENTIRE lift on BOTH the actual
            // saved patch AND the immutable ORIGINAL reference, with half the
            // position tolerance per side and an outward triangle bound. Rounded
            // extraction/refinement never grants an identity receipt.
            let curve = reference.iso(axis, value)?;
            let pc = Curve::from_polyline(ends.into_iter().map(|p| p.to_vec()).collect())?;
            let r = crate::surface_g2_pcurves::inspect(
                crate::surface_g2_pcurves::PcurveSeam { curve: &curve, pcurves: [&pc, &pc], surfaces: [s, reference], reversed: [false, false] },
                crate::surface_g2_pcurves::G2PcurveTolerance { position: o.tolerances[0], normal: o.tolerances[1], curvature: o.tolerances[2] },
                remaining / (o.sides.len() - i),
                exact / (o.sides.len() - i) as u64,
            )?;
            remaining = remaining.saturating_sub(r.geometric.cells);
            exact = exact.saturating_sub(r.positions.exact_work);
            out.push((side, r));
        }
    }
    Ok(out)
}
enum Input<'a> {
    Authored(
        &'a [network::CurveConstraint<'a>],
        &'a [network::CurvatureConstraint<'a>],
    ),
    Inferred(&'a [unparameterized::Guide<'a>], Inversion),
}
fn settings(method: Method) -> network::CurvatureOptions {
    match method {
        Method::Fixed(o) => o,
        Method::PositiveWeights(o) => o.network,
    }
}
/// Automatically elevate degrees/insert local knots when the authored surface
/// basis cannot satisfy complete original guides. UV clocks stay authored.
pub fn solve(
    seed: &Surface,
    guides: &[network::CurveConstraint<'_>],
    curvature: &[network::CurvatureConstraint<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    options: Options,
) -> Result<Qualified> {
    run(
        seed,
        Input::Authored(guides, curvature),
        hard,
        locks,
        options,
        None,
    )
}
/// Infer candidate/support UV curves as well as adapting the surface basis.
/// Local inverses/basis choices remain incomplete bounded construction search.
pub fn solve_unparameterized(
    seed: &Surface,
    guides: &[unparameterized::Guide<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    options: Options,
    inversion: Inversion,
) -> Result<Qualified> {
    run(
        seed,
        Input::Inferred(guides, inversion),
        hard,
        locks,
        options,
        None,
    )
}
fn run(
    seed: &Surface,
    input: Input<'_>,
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    options: Options,
    physical: Option<&loading::Settings<'_>>,
) -> Result<Qualified> {
    seed.validate()?;
    if seed.periodic_u
        || seed.periodic_v
        || seed.control_points[0][0].len() != 3
        || seed.weights.iter().flatten().any(|w| *w <= 0.)
        || !(1..=256).contains(&options.max_controls)
        || !(1..=8).contains(&options.max_degree)
        || options.max_refinements > 16
        || seed.degree_u > options.max_degree
        || seed.degree_v > options.max_degree
        || seed.control_points.len() * seed.control_points[0].len() > options.max_controls
        || hard.len() > 384
    {
        return Err(error(
            "Bounded nonperiodic positive adaptive surface problem required",
        ));
    }
    let limits = settings(options.method);
    network::validate_guides(seed, &[], &[], limits.network)?;
    match &input {
        Input::Authored(g, c) => {
            if g.len() + c.len() == 0 || g.len() + c.len() > 16 {
                return Err(error("Choose 1..16 original adaptive curve guides"));
            }
            network::validate_guides(seed, g, c, limits.network)?;
        }
        Input::Inferred(g, i) => unparameterized::validate(
            seed,
            g,
            hard,
            unparameterized::Options {
                method: options.method,
                max_rounds: i.max_rounds,
                inversion_iterations: i.iterations,
                max_segments: i.max_segments,
            },
        )?,
    }
    let original_boundary = locks.map(|l| OriginalBoundary {
        reference: l.reference.clone(),
        sides: l.boundaries.to_vec(),
        tolerances: l.tolerances,
    });
    if let Some(l) = locks {
        l.reference.validate()?;
        basis::validate_boundary_seed(seed, l.boundaries)?;
        if l.reference.periodic_u
            || l.reference.periodic_v
            || l.reference.control_points[0][0].len() != 3
            || l.reference.weights.iter().flatten().any(|w| *w <= 0.)
            || l.tolerances.iter().any(|v| !v.is_finite() || *v < 0.)
            || l.tolerances[1] > 2.
        {
            return Err(error(
                "Original adaptive boundary references require positive nonperiodic 3D charts and finite tolerances",
            ));
        }
        if seed.degree_u != l.reference.degree_u
            || seed.degree_v != l.reference.degree_v
            || seed.knots_u != l.reference.knots_u
            || seed.knots_v != l.reference.knots_v
        {
            return Err(error(
                "Adaptive original boundary construction starts on the authored common basis",
            ));
        }
    }
    let mut anchor = seed.clone();
    let mut reference = locks.map(|l| l.reference.clone());
    let mut steps = Vec::new();
    let (mut cells, mut exact) = (0, 0);
    let mut reason = String::new();
    let mut loading_work = 0;
    for attempt in 0..=options.max_refinements {
        let remaining = options.max_refinements + 1 - attempt;
        let divisor = if remaining > 1 { 2 } else { 1 };
        let total = limits.network.max_cells.saturating_sub(cells) / divisor;
        let seam_reserve = if locks.is_some() { total / 3 } else { 0 };
        let mut o = limits;
        o.network.max_cells = total - seam_reserve;
        let total_exact = limits.network.max_exact_work.saturating_sub(exact) / divisor as u64;
        let seam_exact = if locks.is_some() { total_exact / 3 } else { 0 };
        o.network.max_exact_work = total_exact - seam_exact;
        let construction_locks = locks.map(|l| network::BoundaryLocks {
            reference: reference.as_ref().unwrap(),
            boundaries: l.boundaries,
            tolerances: l.tolerances,
        });
        let assembly_reserve = physical
            .map(|p| {
                p.options
                    .max_total_assembly_terms
                    .saturating_sub(loading_work)
                    / divisor as u64
            })
            .unwrap_or(0);
        if physical.is_some() && assembly_reserve == 0 {
            return Err(error("Adaptive shared physical assembly work exhausted"));
        }
        let result = if let Some(p) = physical {
            let mut settings = p.options;
            settings.curves = o;
            settings.max_total_assembly_terms = assembly_reserve;
            if let Some(positive) = p.positive {
                let mut w = positive.clone();
                w.geometry.network = o;
                w.max_total_assembly_terms = assembly_reserve;
                match &input {
                    Input::Authored(g, c) => rational::loading::solve_with_reference(
                        &anchor,
                        seed,
                        g,
                        c,
                        hard,
                        construction_locks,
                        p.model,
                        w,
                    )
                    .map(Owner::LoadedPositive),
                    Input::Inferred(g, i) => {
                        unparameterized::loading::solve_positive_weights_with_reference(
                            &anchor,
                            seed,
                            g,
                            hard,
                            construction_locks,
                            p.model,
                            unparameterized::loading::PositiveOptions {
                                weights: w,
                                max_rounds: i.max_rounds,
                                inversion_iterations: i.iterations,
                                max_segments: i.max_segments,
                            },
                        )
                        .map(Owner::Inferred)
                    }
                }
            } else {
                match &input {
                    Input::Authored(g, c) => network::loading::solve_with_reference(
                        &anchor,
                        seed,
                        g,
                        c,
                        hard,
                        construction_locks,
                        p.model,
                        settings,
                    )
                    .map(Owner::Network),
                    Input::Inferred(g, i) => unparameterized::loading::solve_with_reference(
                        &anchor,
                        seed,
                        g,
                        hard,
                        construction_locks,
                        p.model,
                        unparameterized::loading::Options {
                            network: settings,
                            max_rounds: i.max_rounds,
                            inversion_iterations: i.iterations,
                            max_segments: i.max_segments,
                        },
                    )
                    .map(Owner::Inferred),
                }
            }
        } else {
            call(&anchor, &input, hard, construction_locks, options.method, o)
        };
        match result {
            Ok(owner) => {
                loading_work += owner.loading_work();
                let used = owner.work();
                cells += used.0;
                exact += used.1;
                let seams = verify_boundaries(
                    owner.network().surface(),
                    original_boundary.as_ref(),
                    seam_reserve,
                    seam_exact,
                )?;
                cells += seams.iter().map(|(_, r)| r.geometric.cells).sum::<usize>();
                exact += seams
                    .iter()
                    .map(|(_, r)| r.positions.exact_work)
                    .sum::<u64>();
                if seams_admitted(&seams) {
                    if let Some(p) = physical {
                        let equilibrium = owner.network().loading().ok_or_else(|| {
                            error("Adaptive loaded result lost its physical owner")
                        })?;
                        let anchor_matches = if let Some(q) = owner.positive_loading() {
                            q.original() == &anchor
                        } else {
                            equilibrium.anchor() == &anchor
                        };
                        if equilibrium.source() != seed
                            || !anchor_matches
                            || loading_work > p.options.max_total_assembly_terms
                        {
                            return Err(error(
                                "Adaptive result changed original physical ownership or exceeded shared assembly work",
                            ));
                        }
                    }
                    if cells > limits.network.max_cells || exact > limits.network.max_exact_work {
                        return Err(error(
                            "Actual adaptive proof work exceeded the shared original allowance",
                        ));
                    }
                    return Ok(Qualified {
                        owner,
                        original: seed.clone(),
                        anchor,
                        steps,
                        original_boundary,
                        seams,
                        work: (cells, exact),
                        loading_work,
                    });
                }
                reason =
                    "Actual refined surface remains unproven on ORIGINAL boundary seams".into();
            }
            Err(e) => {
                loading_work += assembly_reserve;
                cells += o.network.max_cells;
                exact += o.network.max_exact_work;
                reason = e.to_string();
            }
        }
        if attempt == options.max_refinements {
            break;
        }
        if hard
            .iter()
            .any(|c| matches!(c, Constraint::ControlPoint { .. }))
        {
            return Err(error(
                "Adaptive refusal: original control indices cannot be silently reinterpreted after basis edits; use geometric constraints",
            ));
        }
        if matches!(options.method,Method::PositiveWeights(o) if o.gauge!=[0,0]) {
            return Err(error(
                "Adaptive positive weights currently preserve the original corner gauge [0,0] only",
            ));
        }
        if physical
            .and_then(|p| p.positive)
            .is_some_and(|p| !p.variable_weights.is_empty())
        {
            return Err(error(
                "Adaptive positive weight indices cannot be silently reinterpreted after basis edits; use an automatic unpinned mask",
            ));
        }
        let traces = traces(&input, &anchor, options.method)?;
        let step = basis::select(&anchor, &traces, hard, locks, options, steps.len())?;
        let next = basis::apply(&anchor, step)?;
        if next.control_points.len() * next.control_points[0].len() > options.max_controls {
            return Err(error("Adaptive surface control allowance exhausted"));
        }
        if let Some(r) = &mut reference {
            *r = basis::apply(r, step)?;
        }
        anchor = next;
        steps.push(step);
    }
    Err(error(format!(
        "Adaptive basis/proof allowance exhausted: {reason}"
    )))
}
fn call(
    anchor: &Surface,
    input: &Input<'_>,
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    method: Method,
    o: network::CurvatureOptions,
) -> Result<Owner> {
    match input {
        Input::Authored(g, c) => match method {
            Method::Fixed(_) => {
                if !c.is_empty() {
                    network::solve_with_curvature(anchor, g, c, hard, locks, o)
                } else if let Some(l) = locks {
                    network::solve_preserving_g2(anchor, g, hard, l, o.network)
                } else {
                    network::solve(anchor, g, hard, o.network)
                }
            }
            .map(Owner::Network),
            Method::PositiveWeights(mut p) => {
                p.network = o;
                rational::solve(anchor, g, c, hard, locks, p).map(Owner::Rational)
            }
        },
        Input::Inferred(g, i) => {
            let mut method = method;
            match &mut method {
                Method::Fixed(p) => *p = o,
                Method::PositiveWeights(p) => p.network = o,
            };
            unparameterized::solve(
                anchor,
                g,
                hard,
                locks,
                unparameterized::Options {
                    method,
                    max_rounds: i.max_rounds,
                    inversion_iterations: i.iterations,
                    max_segments: i.max_segments,
                },
            )
            .map(Owner::Inferred)
        }
    }
}
struct Trace {
    spatial: Curve,
    parameters: Curve,
    reverse: bool,
    curvature: bool,
}
fn traces(input: &Input<'_>, s: &Surface, method: Method) -> Result<Vec<Trace>> {
    let mut out = Vec::new();
    match input {
        Input::Authored(g, c) => {
            for g in *g {
                out.push(Trace {
                    spatial: g.spatial.clone(),
                    parameters: g.parameters.clone(),
                    reverse: g.reverse,
                    curvature: false,
                });
            }
            for g in *c {
                out.push(Trace {
                    spatial: g.spatial.clone(),
                    parameters: g.parameters.clone(),
                    reverse: g.reverse,
                    curvature: true,
                });
            }
        }
        Input::Inferred(g, i) => {
            for g in *g {
                let p = unparameterized::proposal_pcurve(
                    g.spatial,
                    s,
                    unparameterized::Options {
                        method,
                        max_rounds: i.max_rounds,
                        inversion_iterations: i.iterations,
                        max_segments: i.max_segments,
                    },
                )?;
                out.push(Trace {
                    spatial: g.spatial.clone(),
                    parameters: p,
                    reverse: false,
                    curvature: matches!(g.continuity, unparameterized::Continuity::Curvature(_)),
                });
            }
        }
    }
    Ok(out)
}
