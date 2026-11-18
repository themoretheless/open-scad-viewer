//! Bounded positive-weight physical fitting and admitted merit search.
//! Every accepted trial retains complete ORIGINAL geometry and fixed-weight
//! equilibrium. Pattern-search diagnostics are NOT joint-weight KKT proofs.
use super::*;
use crate::constrained_surface::{network::loading as loaded, variational};
use std::collections::BTreeSet;
mod search;
#[derive(Clone)]
pub struct Options {
    pub geometry: super::Options,
    pub quadrature_order: usize,
    pub subdivisions: usize,
    pub max_quadrature_sites: usize,
    pub max_assembly_terms_per_call: u64,
    pub max_total_assembly_terms: u64,
    pub stationarity_tolerance: f64,
    /// Empty selects all except original gauge and original boundary layers.
    pub variable_weights: Vec<[usize; 2]>,
    /// At most 4096 trials; permits a full +/- sweep at 256 controls.
    pub max_weight_trials: usize,
    pub initial_log_step: f64,
    pub min_log_step: f64,
    /// Penalty .5 * weight_anchor * sum(log(w / original_w)^2).
    pub weight_anchor: f64,
}
impl Default for Options {
    fn default() -> Self {
        let p = loaded::Options::default();
        let mut geometry = super::Options::default();
        geometry.network = p.curves;
        Self {
            geometry,
            quadrature_order: p.quadrature_order,
            subdivisions: p.subdivisions,
            max_quadrature_sites: p.max_quadrature_sites,
            max_assembly_terms_per_call: p.max_assembly_terms_per_call,
            max_total_assembly_terms: p.max_total_assembly_terms,
            stationarity_tolerance: p.stationarity_tolerance,
            variable_weights: Vec::new(),
            max_weight_trials: 32,
            initial_log_step: 0.25,
            min_log_step: 0.015625,
            weight_anchor: 0.001,
        }
    }
}
impl Options {
    pub(crate) fn loaded(&self, curves: network::CurvatureOptions, total: u64) -> loaded::Options {
        loaded::Options {
            curves,
            quadrature_order: self.quadrature_order,
            subdivisions: self.subdivisions,
            max_quadrature_sites: self.max_quadrature_sites,
            max_assembly_terms_per_call: self.max_assembly_terms_per_call,
            max_total_assembly_terms: total,
            stationarity_tolerance: self.stationarity_tolerance,
        }
    }
}
pub(super) struct Settings<'a> {
    pub reference: &'a Surface,
    pub model: &'a variational::Model,
    pub options: &'a Options,
    pub variables: &'a BTreeSet<(usize, usize)>,
}
#[derive(Clone, Debug)]
pub struct SearchReport {
    pub attempted: usize,
    pub admitted: usize,
    pub numerically_unresolved_trials: usize,
    pub accepted: usize,
    pub final_log_step: f64,
    pub completed_sweeps: usize,
    pub trial_budget_exhausted: bool,
    pub resource_exhausted: bool,
    pub final_merit: f64,
    pub weight_penalty: f64,
    /// Value count including frozen weights, not an active-set multiplier claim.
    pub final_weights_at_bounds: usize,
    pub joint_weight_stationarity_proven: bool,
}
pub struct Qualified {
    fit: super::Qualified,
    variables: BTreeSet<(usize, usize)>,
    options: Options,
    report: SearchReport,
    trace: Vec<f64>,
}
impl Qualified {
    /// Homogeneous initialization metadata plus the final immutable network.
    pub fn fitting(&self) -> &super::Qualified {
        &self.fit
    }
    pub fn network(&self) -> &network::Qualified {
        self.fit.network()
    }
    pub fn surface(&self) -> &Surface {
        self.fit.surface()
    }
    pub fn original(&self) -> &Surface {
        self.fit.original()
    }
    pub fn report(&self) -> &SearchReport {
        &self.report
    }
    pub fn merit_trace(&self) -> &[f64] {
        &self.trace
    }
    pub fn work(&self) -> (usize, u64) {
        self.fit.work()
    }
    pub fn loading_work(&self) -> u64 {
        self.fit.loading_work()
    }
    fn check_binding(&self) -> Result<()> {
        for (i, row) in self.surface().weights.iter().enumerate() {
            for (j, w) in row.iter().enumerate() {
                if !self.variables.contains(&(i, j))
                    && w.to_bits() != self.original().weights[i][j].to_bits()
                {
                    return Err(error("Unselected positive weight changed"));
                }
            }
        }
        let p = self
            .network()
            .loading()
            .ok_or_else(|| error("Positive physical owner missing"))?;
        if Some(p.source()) != self.fit.physical_reference.as_ref()
            || p.anchor().control_points != self.original().control_points
            || p.anchor().weights != self.surface().weights
            || self.surface().weights[self.fit.gauge[0]][self.fit.gauge[1]].to_bits()
                != self.original().weights[self.fit.gauge[0]][self.fit.gauge[1]].to_bits()
            || self
                .surface()
                .weights
                .iter()
                .flatten()
                .any(|w| *w < self.fit.weight_bounds[0] || *w > self.fit.weight_bounds[1])
        {
            return Err(error(
                "Positive physical source, gauge, anchor or box changed",
            ));
        }
        Ok(())
    }
    pub fn reverify(&self, max_cells: usize, max_exact: u64) -> Result<network::Verification> {
        self.check_binding()?;
        let out = self.fit.reverify(max_cells, max_exact)?;
        let (_, penalty) = merit(self.network(), self.original(), &self.options)?;
        let merit = out
            .loading
            .as_ref()
            .ok_or_else(|| error("Fresh positive physical energy missing"))?
            .total_potential
            + penalty;
        if (merit - self.report.final_merit).abs() > 1e-10 * (1. + merit.abs())
            || (penalty - self.report.weight_penalty).abs() > 1e-12 * (1. + penalty.abs())
        {
            return Err(error("Saved original physical weight merit changed"));
        }
        Ok(out)
    }
}
fn variables(
    seed: &Surface,
    locks: Option<network::BoundaryLocks<'_>>,
    o: &Options,
) -> Result<BTreeSet<(usize, usize)>> {
    let (nu, nv) = (seed.control_points.len(), seed.control_points[0].len());
    let mut fixed = BTreeSet::new();
    fixed.insert((o.geometry.gauge[0], o.geometry.gauge[1]));
    if let Some(b) = locks {
        fixed.extend(boundary::control_rings(seed, b.boundaries)?);
    }
    let all = if o.variable_weights.is_empty() {
        (0..nu)
            .flat_map(|i| (0..nv).map(move |j| (i, j)))
            .filter(|i| !fixed.contains(i))
            .collect()
    } else {
        let set = o
            .variable_weights
            .iter()
            .map(|p| (p[0], p[1]))
            .collect::<BTreeSet<_>>();
        if set.len() != o.variable_weights.len()
            || set
                .iter()
                .any(|&(i, j)| i >= nu || j >= nv || fixed.contains(&(i, j)))
        {
            return Err(error("Choose distinct unpinned positive-weight indices"));
        }
        set
    };
    Ok(all)
}
fn merit(q: &network::Qualified, seed: &Surface, o: &Options) -> Result<(f64, f64)> {
    let energy = q
        .loading()
        .ok_or_else(|| error("Positive physical equilibrium owner missing"))?
        .diagnostics()
        .total_potential;
    let penalty = 0.5
        * o.weight_anchor
        * q.surface()
            .weights
            .iter()
            .flatten()
            .zip(seed.weights.iter().flatten())
            .map(|(a, b)| (a.ln() - b.ln()).powi(2))
            .sum::<f64>();
    if !energy.is_finite() || !penalty.is_finite() {
        return Err(error("Positive-weight physical merit overflow"));
    }
    Ok((energy + penalty, penalty))
}
fn from_network(
    seed: &Surface,
    reference: &Surface,
    q: network::Qualified,
    o: &Options,
) -> super::Qualified {
    let work = q.work();
    let loading_work = q.loading_work();
    super::Qualified {
        network: q,
        original: seed.clone(),
        weight_bounds: o.geometry.weight_bounds,
        gauge: o.geometry.gauge,
        iterations: 0,
        active_bounds: 0,
        active_iterations: 0,
        homogeneous_rank: 0,
        work,
        loading_work,
        physical_reference: Some(reference.clone()),
    }
}
pub fn solve(
    seed: &Surface,
    guides: &[network::CurveConstraint<'_>],
    curvatures: &[network::CurvatureConstraint<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    model: &variational::Model,
    options: Options,
) -> Result<Qualified> {
    solve_with_reference(
        seed,
        seed,
        network::ConstraintSet {
            guides,
            curvatures,
            hard,
            locks,
        },
        model,
        options,
    )
}
pub fn solve_with_reference(
    seed: &Surface,
    reference: &Surface,
    set: network::ConstraintSet<'_>,
    model: &variational::Model,
    options: Options,
) -> Result<Qualified> {
    let network::ConstraintSet {
        guides,
        curvatures,
        hard,
        locks,
    } = set;
    super::validate(seed, guides, curvatures, hard, options.geometry)?;
    if options.max_weight_trials > 4096
        || !(1..=500000000).contains(&options.max_total_assembly_terms)
        || !options.initial_log_step.is_finite()
        || options.initial_log_step <= 0.
        || options.initial_log_step > 2.
        || !options.min_log_step.is_finite()
        || options.min_log_step <= 0.
        || options.min_log_step > options.initial_log_step
        || !options.weight_anchor.is_finite()
        || options.weight_anchor < 0.
    {
        return Err(error(
            "Choose bounded positive physical merit search settings",
        ));
    }
    variational::validate_reference(
        seed,
        reference,
        hard,
        model,
        variational::Options {
            fitting: options.geometry.network.network.fitting,
            quadrature_order: options.quadrature_order,
            subdivisions: options.subdivisions,
            max_sites: options.max_quadrature_sites,
            max_assembly_terms: options.max_assembly_terms_per_call,
            stationarity_tolerance: options.stationarity_tolerance,
        },
    )?;
    let variables = variables(seed, locks, &options)?;
    let divisor = if options.max_weight_trials > 0 { 2 } else { 1 };
    let mut base = options.geometry;
    base.network.network.max_cells /= divisor;
    base.network.network.max_exact_work /= divisor as u64;
    let mut assembly = options.max_total_assembly_terms / divisor as u64;
    let mut fees = (0, 0, 0);
    let inside = seed
        .weights
        .iter()
        .flatten()
        .all(|w| *w >= base.weight_bounds[0] && *w <= base.weight_bounds[1]);
    let first = if inside {
        let mut p = base.network;
        p.network.max_cells /= 2;
        p.network.max_exact_work /= 2;
        let reserve = assembly / 2;
        match network::loading::solve_with_reference(
            seed,
            reference,
            network::ConstraintSet {
                guides,
                curvatures,
                hard,
                locks,
            },
            model,
            options.loaded(p, reserve),
        ) {
            Ok(q) => Some(from_network(seed, reference, q, &options)),
            Err(_) => {
                fees = (p.network.max_cells, p.network.max_exact_work, reserve);
                base.network.network.max_cells -= fees.0;
                base.network.network.max_exact_work -= fees.1;
                assembly -= fees.2;
                None
            }
        }
    } else {
        None
    };
    let mut fit = if let Some(q) = first {
        q
    } else {
        let mut limits = options.clone();
        limits.max_total_assembly_terms = assembly;
        super::run(
            seed,
            guides,
            curvatures,
            hard,
            locks,
            base,
            Some(&Settings {
                reference,
                model,
                options: &limits,
                variables: &variables,
            }),
        )?
    };
    fit.work.0 += fees.0;
    fit.work.1 += fees.1;
    fit.loading_work += fees.2;
    let (initial, _) = merit(&fit.network, seed, &options)?;
    let mut q = Qualified {
        fit,
        variables,
        options,
        report: SearchReport {
            attempted: 0,
            admitted: 0,
            numerically_unresolved_trials: 0,
            accepted: 0,
            final_log_step: 0.,
            completed_sweeps: 0,
            trial_budget_exhausted: false,
            resource_exhausted: false,
            final_merit: initial,
            weight_penalty: 0.,
            final_weights_at_bounds: 0,
            joint_weight_stationarity_proven: false,
        },
        trace: vec![initial],
    };
    search::run(&mut q, reference, guides, curvatures, hard, locks, model)?;
    // Every retained network already owns complete geometry and actual energy
    // admission. Ownership checks add no uncharged second proof/assembly call.
    q.check_binding()?;
    Ok(q)
}
