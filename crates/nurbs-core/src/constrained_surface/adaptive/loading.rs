//! Fixed-weight basis adaptation with an immutable ORIGINAL physical reference.
//! Geometric basis proposals do not replace physical rest/area/connection data.
use super::*;
use crate::constrained_surface::{network::loading as loaded, variational::Model};
#[derive(Clone, Copy)]
pub struct Options {
    pub network: loaded::Options,
    pub max_refinements: usize,
    pub max_degree: usize,
    pub max_controls: usize,
}
impl Default for Options {
    fn default() -> Self {
        let a = super::Options::default();
        Self {
            network: loaded::Options::default(),
            max_refinements: a.max_refinements,
            max_degree: a.max_degree,
            max_controls: a.max_controls,
        }
    }
}
pub(super) struct Settings<'a> {
    pub model: &'a Model,
    pub options: loaded::Options,
    pub positive: Option<&'a rational::loading::Options>,
}
pub fn solve(
    seed: &Surface,
    guides: &[network::CurveConstraint<'_>],
    curvature: &[network::CurvatureConstraint<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    model: &Model,
    options: Options,
) -> Result<Qualified> {
    if !(1..=500000000).contains(&options.network.max_total_assembly_terms) {
        return Err(error(
            "Adaptive physical assembly needs one bounded total allowance",
        ));
    }
    let settings = Settings {
        model,
        options: options.network,
        positive: None,
    };
    super::run(
        seed,
        Input::Authored(guides, curvature),
        hard,
        locks,
        super::Options {
            method: Method::Fixed(options.network.curves),
            max_refinements: options.max_refinements,
            max_degree: options.max_degree,
            max_controls: options.max_controls,
        },
        Some(&settings),
    )
}

/// Infer original guide/support pcurves while adapting the trial basis. Physical
/// rest remains the immutable original seed across every inverse/basis round.
pub fn solve_unparameterized(
    seed: &Surface,
    guides: &[unparameterized::Guide<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    model: &Model,
    options: Options,
    inversion: Inversion,
) -> Result<Qualified> {
    if !(1..=500000000).contains(&options.network.max_total_assembly_terms) {
        return Err(error(
            "Adaptive inferred physical assembly needs bounded total work",
        ));
    }
    let settings = Settings {
        model,
        options: options.network,
        positive: None,
    };
    super::run(
        seed,
        Input::Inferred(guides, inversion),
        hard,
        locks,
        super::Options {
            method: Method::Fixed(options.network.curves),
            max_refinements: options.max_refinements,
            max_degree: options.max_degree,
            max_controls: options.max_controls,
        },
        Some(&settings),
    )
}

#[derive(Clone)]
pub struct PositiveOptions {
    pub weights: rational::loading::Options,
    pub max_refinements: usize,
    pub max_degree: usize,
    pub max_controls: usize,
}
impl Default for PositiveOptions {
    fn default() -> Self {
        let o = Options::default();
        Self {
            weights: rational::loading::Options::default(),
            max_refinements: o.max_refinements,
            max_degree: o.max_degree,
            max_controls: o.max_controls,
        }
    }
}
pub fn solve_positive_weights(
    seed: &Surface,
    guides: &[network::CurveConstraint<'_>],
    curvature: &[network::CurvatureConstraint<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    model: &Model,
    options: PositiveOptions,
) -> Result<Qualified> {
    positive(
        seed,
        Input::Authored(guides, curvature),
        hard,
        locks,
        model,
        options,
    )
}
pub fn solve_unparameterized_positive_weights(
    seed: &Surface,
    guides: &[unparameterized::Guide<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    model: &Model,
    options: PositiveOptions,
    inversion: Inversion,
) -> Result<Qualified> {
    positive(
        seed,
        Input::Inferred(guides, inversion),
        hard,
        locks,
        model,
        options,
    )
}
fn positive(
    seed: &Surface,
    input: Input<'_>,
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    model: &Model,
    options: PositiveOptions,
) -> Result<Qualified> {
    let p = options.weights.loaded(
        options.weights.geometry.network,
        options.weights.max_total_assembly_terms,
    );
    if !(1..=500000000).contains(&p.max_total_assembly_terms) {
        return Err(error(
            "Positive adaptive fitting needs bounded shared physical work",
        ));
    }
    let settings = Settings {
        model,
        options: p,
        positive: Some(&options.weights),
    };
    super::run(
        seed,
        input,
        hard,
        locks,
        super::Options {
            method: Method::PositiveWeights(options.weights.geometry),
            max_refinements: options.max_refinements,
            max_degree: options.max_degree,
            max_controls: options.max_controls,
        },
        Some(&settings),
    )
}
