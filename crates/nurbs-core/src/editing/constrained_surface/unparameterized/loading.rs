//! Inferred original curve/support UVs plus reference-shell loads. Inverses are
//! proposals; complete original geometry and final fixed-UV equilibrium admit.
use super::*;
use crate::constrained_surface::{network::loading as loaded, variational};
#[derive(Clone, Copy)]
pub struct Options {
    pub network: loaded::Options,
    pub max_rounds: usize,
    pub inversion_iterations: usize,
    pub max_segments: usize,
}
impl Default for Options {
    fn default() -> Self {
        let o = super::Options::default();
        Self {
            network: loaded::Options::default(),
            max_rounds: o.max_rounds,
            inversion_iterations: o.inversion_iterations,
            max_segments: o.max_segments,
        }
    }
}
pub(super) struct Settings<'a> {
    pub reference: &'a Surface,
    pub model: &'a variational::Model,
    pub options: loaded::Options,
    pub positive: Option<&'a rational::loading::Options>,
}
pub fn solve(
    seed: &Surface,
    guides: &[Guide<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    model: &variational::Model,
    options: Options,
) -> Result<Qualified> {
    solve_with_reference(seed, seed, guides, hard, locks, model, options)
}
/// Trial basis and ORIGINAL physical rest may differ on the same authored UV domain.
pub fn solve_with_reference(
    seed: &Surface,
    reference: &Surface,
    guides: &[Guide<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    model: &variational::Model,
    options: Options,
) -> Result<Qualified> {
    let p = options.network;
    if !(1..=500000000).contains(&p.max_total_assembly_terms) {
        return Err(error(
            "Inferred physical solve needs bounded shared assembly work",
        ));
    }
    variational::validate_reference(
        seed,
        reference,
        hard,
        model,
        variational::Options {
            fitting: p.curves.network.fitting,
            quadrature_order: p.quadrature_order,
            subdivisions: p.subdivisions,
            max_sites: p.max_quadrature_sites,
            max_assembly_terms: p.max_assembly_terms_per_call,
            stationarity_tolerance: p.stationarity_tolerance,
        },
    )?;
    super::run(
        seed,
        guides,
        hard,
        locks,
        super::Options {
            method: Method::Fixed(p.curves),
            max_rounds: options.max_rounds,
            inversion_iterations: options.inversion_iterations,
            max_segments: options.max_segments,
        },
        Some(&Settings {
            reference,
            model,
            options: p,
            positive: None,
        }),
    )
}

#[derive(Clone)]
pub struct PositiveOptions {
    pub weights: rational::loading::Options,
    pub max_rounds: usize,
    pub inversion_iterations: usize,
    pub max_segments: usize,
}
impl Default for PositiveOptions {
    fn default() -> Self {
        let o = Options::default();
        Self {
            weights: rational::loading::Options::default(),
            max_rounds: o.max_rounds,
            inversion_iterations: o.inversion_iterations,
            max_segments: o.max_segments,
        }
    }
}
pub fn solve_positive_weights(
    seed: &Surface,
    guides: &[Guide<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    model: &variational::Model,
    options: PositiveOptions,
) -> Result<Qualified> {
    solve_positive_weights_with_reference(seed, seed, guides, hard, locks, model, options)
}
pub fn solve_positive_weights_with_reference(
    seed: &Surface,
    reference: &Surface,
    guides: &[Guide<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    model: &variational::Model,
    options: PositiveOptions,
) -> Result<Qualified> {
    let p = options.weights.loaded(
        options.weights.geometry.network,
        options.weights.max_total_assembly_terms,
    );
    if !(1..=500000000).contains(&p.max_total_assembly_terms) {
        return Err(error(
            "Positive inferred fitting needs bounded shared physical work",
        ));
    }
    super::run(
        seed,
        guides,
        hard,
        locks,
        super::Options {
            method: Method::PositiveWeights(options.weights.geometry),
            max_rounds: options.max_rounds,
            inversion_iterations: options.inversion_iterations,
            max_segments: options.max_segments,
        },
        Some(&Settings {
            reference,
            model,
            options: p,
            positive: Some(&options.weights),
        }),
    )
}
