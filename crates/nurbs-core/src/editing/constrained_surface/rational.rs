//! Joint homogeneous coefficient/positive-weight proposals, with complete
//! original G0/G1/G2 admission delegated to the immutable network owner.
use super::{Constraint, JetEquation, boundary, error, network};
use crate::{Result, curve::Curve, surface::Surface};
mod fit;
/// Positive weights under ORIGINAL physical shell energy; surfaces owner.
pub mod loading;
mod rows;
#[derive(Clone, Copy)]
pub struct Options {
    pub network: network::CurvatureOptions,
    pub weight_bounds: [f64; 2],
    /// Original weight kept exactly to remove the global projective gauge.
    pub gauge: [usize; 2],
    pub max_iterations: usize,
    pub max_active_iterations: usize,
}
impl Default for Options {
    fn default() -> Self {
        Self {
            network: network::CurvatureOptions::default(),
            weight_bounds: [0.05, 20.],
            gauge: [0, 0],
            max_iterations: 5,
            max_active_iterations: 64,
        }
    }
}
/// Whole-curve qualified final geometry; homogeneous samples/KKT diagnostics
/// are construction evidence only, never replacements for original proofs.
pub struct Qualified {
    network: network::Qualified,
    original: Surface,
    weight_bounds: [f64; 2],
    gauge: [usize; 2],
    iterations: usize,
    active_bounds: usize,
    active_iterations: usize,
    homogeneous_rank: usize,
    work: (usize, u64),
    loading_work: u64,
    physical_reference: Option<Surface>,
}
impl Qualified {
    pub fn network(&self) -> &network::Qualified {
        &self.network
    }
    pub fn surface(&self) -> &Surface {
        self.network.surface()
    }
    pub fn original(&self) -> &Surface {
        &self.original
    }
    pub fn weight_bounds(&self) -> [f64; 2] {
        self.weight_bounds
    }
    pub fn gauge(&self) -> [usize; 2] {
        self.gauge
    }
    pub fn iterations(&self) -> usize {
        self.iterations
    }
    pub fn active_weight_bounds(&self) -> usize {
        self.active_bounds
    }
    pub fn active_iterations(&self) -> usize {
        self.active_iterations
    }
    pub fn homogeneous_rank(&self) -> usize {
        self.homogeneous_rank
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
    pub fn reverify(&self, max_cells: usize, max_exact: u64) -> Result<network::Verification> {
        if let Some(reference) = &self.physical_reference {
            let p = self
                .network
                .loading()
                .ok_or_else(|| error("Positive physical owner missing"))?;
            if p.source() != reference
                || p.anchor().control_points != self.original.control_points
                || self.surface().weights[self.gauge[0]][self.gauge[1]].to_bits()
                    != self.original.weights[self.gauge[0]][self.gauge[1]].to_bits()
                || self
                    .surface()
                    .weights
                    .iter()
                    .flatten()
                    .any(|w| *w < self.weight_bounds[0] || *w > self.weight_bounds[1])
            {
                return Err(error("Positive physical source, gauge or box changed"));
            }
        }
        self.network.reverify(max_cells, max_exact)
    }
}
/// Optimize Cartesian homogeneous coordinates AND positive weights on the
/// authored knot basis. Every candidate still needs complete original G0/G1/G2
/// network admission, optional original boundary two-jets and actual hard jets.
/// Unknown UVs/bases and global embedding remain separate. Limits bound both
/// weight working sets and all nested network proof attempts.
pub fn solve(
    seed: &Surface,
    guides: &[network::CurveConstraint<'_>],
    curvatures: &[network::CurvatureConstraint<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    options: Options,
) -> Result<Qualified> {
    run(seed, guides, curvatures, hard, locks, options, None)
}
fn validate(
    seed: &Surface,
    guides: &[network::CurveConstraint<'_>],
    curvatures: &[network::CurvatureConstraint<'_>],
    hard: &[Constraint],
    options: Options,
) -> Result<()> {
    seed.validate()?;
    let (nu, nv) = (seed.control_points.len(), seed.control_points[0].len());
    let n = nu * nv;
    if n > 256
        || guides.len() + curvatures.len() == 0
        || guides.len() + curvatures.len() > 16
        || seed.periodic_u
        || seed.periodic_v
        || seed.control_points[0][0].len() != 3
        || seed.weights.iter().flatten().any(|w| *w <= 0.)
        || !options.weight_bounds.iter().all(|w| w.is_finite())
        || options.weight_bounds[0] <= 0.
        || options.weight_bounds[0] > options.weight_bounds[1]
        || options.gauge[0] >= nu
        || options.gauge[1] >= nv
        || !(1..=8).contains(&options.max_iterations)
        || !(1..=512).contains(&options.max_active_iterations)
        || hard.len() > 384
    {
        return Err(error(
            "Choose a bounded nonperiodic positive-weight homogeneous problem",
        ));
    }
    network::validate_guides(seed, guides, curvatures, options.network.network)?;
    if !options.network.curvature_tolerance.is_finite()
        || options.network.curvature_tolerance <= 0.
        || [
            options.network.network.fitting.position_tolerance,
            options.network.network.fitting.derivative_tolerance,
            options.network.network.fitting.normal_tolerance,
        ]
        .iter()
        .any(|x| !x.is_finite() || *x <= 0.)
    {
        return Err(error("Finite positive final geometry tolerances required"));
    }
    Ok(())
}
fn run(
    seed: &Surface,
    guides: &[network::CurveConstraint<'_>],
    curvatures: &[network::CurvatureConstraint<'_>],
    hard: &[Constraint],
    locks: Option<network::BoundaryLocks<'_>>,
    options: Options,
    physical: Option<&loading::Settings<'_>>,
) -> Result<Qualified> {
    validate(seed, guides, curvatures, hard, options)?;
    let (nu, nv) = (seed.control_points.len(), seed.control_points[0].len());
    let n = nu * nv;
    let origin = [
        seed.control_points[0][0][0],
        seed.control_points[0][0][1],
        seed.control_points[0][0][2],
    ];
    let gauge = options.gauge[0] * nv + options.gauge[1];
    let mut fixed = vec![(
        3 * n + gauge,
        seed.weights[options.gauge[0]][options.gauge[1]],
    )];
    let mut boundary_pins = Vec::new();
    if let Some(lock) = locks {
        lock.reference.validate()?;
        if lock.reference.control_points[0][0].len() != 3
            || lock.reference.periodic_u
            || lock.reference.periodic_v
            || seed.degree_u != lock.reference.degree_u
            || seed.degree_v != lock.reference.degree_v
            || seed.knots_u != lock.reference.knots_u
            || seed.knots_v != lock.reference.knots_v
        {
            return Err(error(
                "Homogeneous boundary locks need the original clamped basis",
            ));
        }
        for (i, j) in boundary::control_rings(seed, lock.boundaries)? {
            let index = i * nv + j;
            let w = lock.reference.weights[i][j];
            if seed.weights[i][j] != w {
                return Err(error(
                    "Original boundary weight layers must match before optimization",
                ));
            }
            fixed.push((3 * n + index, w));
            for k in 0..3 {
                fixed.push((
                    k * n + index,
                    (lock.reference.control_points[i][j][k] - origin[k]) * w,
                ));
            }
            boundary_pins.push((i, j));
        }
    }
    if let Some(p) = physical {
        for i in 0..nu {
            for j in 0..nv {
                if !p.variables.contains(&(i, j)) {
                    fixed.push((3 * n + i * nv + j, seed.weights[i][j]));
                }
            }
        }
    }
    if fixed.iter().any(|(i, w)| {
        *i >= 3 * n && (*w < options.weight_bounds[0] || *w > options.weight_bounds[1])
    }) {
        return Err(error(
            "Fixed original boundary/gauge weight lies outside the requested box",
        ));
    }
    let mut sites = guides
        .iter()
        .copied()
        .chain(curvatures.iter().map(|g| network::CurveConstraint {
            spatial: g.spatial,
            parameters: g.parameters,
            reverse: g.reverse,
            tangent: Some(g.support),
        }))
        .map(network::initial_sites)
        .collect::<Result<Vec<_>>>()?;
    let mut current = seed.clone();
    let mut cells = 0;
    let mut exact = 0;
    let mut active_iterations = 0;
    let mut loading_work = 0;
    let limits = options.network.network;
    let mut reason = String::new();
    for iteration in 0..options.max_iterations {
        if sites.iter().any(|s| s.len() > limits.max_sites_per_curve)
            || sites.iter().map(Vec::len).sum::<usize>() > limits.max_total_sites
        {
            return Err(error("Homogeneous proposal-site allowance exhausted"));
        }
        let rows = rows::build(
            &current,
            origin,
            guides,
            curvatures,
            &sites,
            hard,
            iteration > 0,
        )?;
        let proposed = match fit::propose(seed, &current, origin, rows, &fixed, options) {
            Ok(proposed) => proposed,
            Err(_) if iteration > 0 => {
                // A nonlinear jet linearization can conflict with exact
                // position rows even when the actual problem is feasible.
                // Retry only the construction stage without those Newton rows;
                // the final network still enforces EVERY original hard jet/G2.
                active_iterations += options.max_active_iterations;
                let rows = rows::build(&current, origin, guides, curvatures, &sites, hard, false)?;
                fit::propose(seed, &current, origin, rows, &fixed, options)?
            }
            Err(e) => return Err(e),
        };
        current = proposed.surface;
        active_iterations += proposed.active_iterations;
        if let Some(lock) = locks {
            for &(i, j) in &boundary_pins {
                current.control_points[i][j] = lock.reference.control_points[i][j].clone();
                current.weights[i][j] = lock.reference.weights[i][j];
            }
        }
        let rounds = options.max_iterations - iteration;
        let mut verify_options = options.network;
        verify_options.network.max_cells = limits.max_cells.saturating_sub(cells) / rounds;
        verify_options.network.max_exact_work =
            limits.max_exact_work.saturating_sub(exact) / rounds as u64;
        let assembly_reserve = physical
            .map(|p| {
                p.options
                    .max_total_assembly_terms
                    .saturating_sub(loading_work)
                    / rounds as u64
            })
            .unwrap_or(0);
        if physical.is_some() && assembly_reserve == 0 {
            return Err(error(
                "Positive-weight physical assembly allowance exhausted",
            ));
        }
        let candidate = if let Some(p) = physical {
            let mut anchor = seed.clone();
            anchor.weights = current.weights.clone();
            network::loading::solve_with_reference(
                &anchor,
                p.reference,
                guides,
                curvatures,
                hard,
                locks,
                p.model,
                p.options.loaded(verify_options, assembly_reserve),
            )
        } else if curvatures.is_empty() {
            if let Some(lock) = locks {
                network::solve_preserving_g2(&current, guides, hard, lock, verify_options.network)
            } else {
                network::solve(&current, guides, hard, verify_options.network)
            }
        } else {
            network::solve_with_curvature(&current, guides, curvatures, hard, locks, verify_options)
        };
        match candidate {
            Ok(network) => {
                loading_work += network.loading_work();
                cells += network.work().0;
                exact += network.work().1;
                if network
                    .surface()
                    .weights
                    .iter()
                    .flatten()
                    .any(|w| *w < options.weight_bounds[0] || *w > options.weight_bounds[1])
                {
                    return Err(error(
                        "Final original-network owner changed the positive weight box",
                    ));
                }
                return Ok(Qualified {
                    network,
                    original: seed.clone(),
                    weight_bounds: options.weight_bounds,
                    gauge: options.gauge,
                    iterations: iteration + 1,
                    active_bounds: proposed.active_bounds,
                    active_iterations,
                    homogeneous_rank: proposed.equation_rank,
                    work: (cells, exact),
                    loading_work,
                    physical_reference: physical.map(|p| p.reference.clone()),
                });
            }
            Err(error) => {
                loading_work += assembly_reserve;
                // Failed lower-level calls expose no used-work report. Charge
                // the entire reservation before another candidate is proposed.
                cells += verify_options.network.max_cells;
                exact += verify_options.network.max_exact_work;
                reason = format!("{error:?}");
            }
        }
        for s in &mut sites {
            let mid = s
                .windows(2)
                .map(|p| p[0] / 2. + p[1] / 2.)
                .collect::<Vec<_>>();
            s.extend(mid);
            s.sort_by(f64::total_cmp);
        }
    }
    Err(error(format!(
        "No positive-weight candidate passed complete original network admission: {reason}"
    )))
}
