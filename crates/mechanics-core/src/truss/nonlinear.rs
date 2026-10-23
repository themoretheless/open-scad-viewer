use super::*;

/// Tuning of the corotational nonlinear solve.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NonlinearOptions {
    /// Requested equal load increments up to the full reference load (1-200);
    /// increments subdivide adaptively when Newton stalls.
    pub steps: usize,
    /// Convergence: ∞-norm of the free-DOF residual relative to the current
    /// force level (0 < tolerance ≤ 1e-3).
    pub tolerance: f64,
    /// Newton iterations per increment (1-200).
    pub max_iterations: usize,
}

/// One converged load increment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NonlinearStep {
    pub load_factor: f64,
    pub iterations: usize,
    pub relative_residual: f64,
}

/// Large-displacement equilibrium state of a pin-jointed truss.
#[derive(Clone, Debug)]
pub struct TrussNonlinearResponse {
    pub displacements_mm: Vec<[f64; 3]>,
    /// Nonzero only at restrained DOFs, with sign in the global XYZ frame.
    pub reactions_n: Vec<[f64; 3]>,
    /// Positive means tension, computed from the current bar lengths.
    pub axial_forces_n: Vec<f64>,
    pub axial_stresses_mpa: Vec<f64>,
    /// Converged increments in order.
    pub steps: Vec<NonlinearStep>,
    /// Reached fraction of the reference load; 1.0 on full convergence.
    pub load_factor: f64,
    /// False when load control stalled at a limit point (snap-through): the
    /// returned state is the last converged equilibrium on the path.
    pub converged: bool,
}

/// Geometrically nonlinear corotational truss solve: large rotations and
/// displacements with small axial strain, N = EA·(L−L₀)/L₀ in the current
/// configuration. The reference load scales by a load proportionality factor;
/// Newton iterations on the tangent stiffness (material EA/L₀·nnᵀ plus
/// geometric N/L·(I−nnᵀ) per bar) advance in increments that halve when a
/// step fails, so the equilibrium path is traced up to a load-controlled
/// limit point. Beyond it (`converged: false`) the path continues only under
/// displacement or arc-length control, which is out of scope. Linear elastic
/// material; no buckling, no strength check.
pub fn solve_nonlinear(
    model: &Model,
    options: &NonlinearOptions,
) -> Result<TrussNonlinearResponse> {
    if options.steps == 0 || options.steps > MAX_LOAD_STEPS {
        return Err(invalid("Nonlinear solves admit 1-200 load steps"));
    }
    if !options.tolerance.is_finite()
        || options.tolerance <= 0.
        || options.tolerance > 1e-3
    {
        return Err(invalid("Tolerance must be finite, positive, and ≤ 1e-3"));
    }
    if options.max_iterations == 0 || options.max_iterations > MAX_NR_ITERATIONS {
        return Err(invalid("Nonlinear solves admit 1-200 Newton iterations per step"));
    }
    let bars = validate(model)?;
    let n = model.nodes_mm.len();
    let ndof = n * 3;
    let external = DVector::from_iterator(
        ndof,
        (0..ndof).map(|d| model.forces_n[d / 3][d % 3]),
    );
    let free: Vec<usize> = (0..ndof)
        .filter(|&i| !model.restrained[i / 3][i % 3])
        .collect();
    // Corotational bar state at a trial displacement: internal nodal forces
    // and the tangent stiffness; None when a bar collapses to zero length.
    let state = |u: &DVector<f64>| -> Option<(DVector<f64>, DMatrix<f64>, Vec<f64>)> {
        let mut f_int = DVector::<f64>::zeros(ndof);
        let mut tangent = DMatrix::<f64>::zeros(ndof, ndof);
        let mut forces = Vec::with_capacity(bars.len());
        for bar in &bars {
            let [a, b] = bar.nodes;
            let ea = bar.stiffness * bar.length;
            let d: [f64; 3] =
                std::array::from_fn(|k| model.nodes_mm[b][k] + u[b * 3 + k]
                    - model.nodes_mm[a][k]
                    - u[a * 3 + k]);
            let length = d[0].hypot(d[1]).hypot(d[2]);
            if !length.is_finite() || length <= 1e-12 {
                return None;
            }
            let dir = d.map(|v| v / length);
            let force = ea * (length - bar.length) / bar.length;
            forces.push(force);
            for i in 0..3 {
                f_int[a * 3 + i] -= force * dir[i];
                f_int[b * 3 + i] += force * dir[i];
                for j in 0..3 {
                    // Tangent 3×3: EA/L₀·nnᵀ + N/L·(I − nnᵀ).
                    let block = bar.stiffness * dir[i] * dir[j]
                        + force / length * (f64::from(i == j) - dir[i] * dir[j]);
                    tangent[(a * 3 + i, a * 3 + j)] += block;
                    tangent[(b * 3 + i, b * 3 + j)] += block;
                    tangent[(a * 3 + i, b * 3 + j)] -= block;
                    tangent[(b * 3 + i, a * 3 + j)] -= block;
                }
            }
        }
        Some((f_int, tangent, forces))
    };
    let mut displacement = DVector::<f64>::zeros(ndof);
    let mut lambda = 0.;
    let h0 = 1. / options.steps as f64;
    let mut h = h0;
    let mut halvings = 0;
    let mut steps_out = Vec::new();
    let mut converged = true;
    let mut first_step_error: Option<Error> = None;
    // Global work budget: creeping along a limit point with ever-smaller
    // converged increments must still terminate.
    let mut total_iterations = 0usize;
    const MAX_TOTAL_ITERATIONS: usize = 20_000;
    const MAX_ACCEPTED_STEPS: usize = 4096;
    'path: while lambda < 1. {
        h = h.min(1. - lambda);
        let lambda_trial = lambda + h;
        let scale = (lambda_trial * &external)
            .iter()
            .map(|v| v.abs())
            .fold(1e-30, f64::max);
        let mut trial = displacement.clone();
        let mut accepted = false;
        let mut iterations = 0;
        let mut rel_residual = f64::INFINITY;
        while iterations < options.max_iterations {
            let Some((f_int, tangent, _)) = state(&trial) else {
                break;
            };
            let residual = lambda_trial * &external - f_int;
            rel_residual = free
                .iter()
                .map(|&d| residual[d].abs())
                .fold(0., f64::max)
                / scale;
            if !rel_residual.is_finite() {
                break;
            }
            if rel_residual <= options.tolerance {
                accepted = true;
                break;
            }
            iterations += 1;
            // Newton correction on the free-DOF tangent; any factorization
            // failure (limit point, mechanism) fails the step, not the solve.
            let factored = match factor_reduced(&tangent, &model.restrained) {
                Ok(f) => f,
                Err(e) => {
                    if steps_out.is_empty() && lambda == 0. {
                        first_step_error.get_or_insert(e);
                    }
                    break;
                }
            };
            let Some(factor) = &factored.factor else {
                // No free DOFs: nothing to correct; residual decides.
                break;
            };
            let rhs = DVector::from_iterator(
                factored.free.len(),
                factored
                    .free
                    .iter()
                    .enumerate()
                    .map(|(i, &d)| residual[d] / factored.scales[i]),
            );
            let solved = factor.solve(&rhs);
            if solved.iter().any(|v| !v.is_finite()) {
                break;
            }
            for (i, &d) in factored.free.iter().enumerate() {
                trial[d] += solved[i] / factored.scales[i];
            }
            if trial.iter().any(|v| !v.is_finite()) {
                break;
            }
        }
        total_iterations += iterations;
        if accepted {
            displacement = trial;
            lambda = lambda_trial;
            halvings = 0;
            steps_out.push(NonlinearStep {
                load_factor: lambda,
                iterations,
                relative_residual: rel_residual,
            });
            h = (h * 2.).min(h0);
        } else {
            halvings += 1;
            if halvings > 8 {
                if steps_out.is_empty() {
                    // Not a limit point: the model never carried load.
                    return Err(first_step_error.unwrap_or_else(numeric));
                }
                // Load control stalls: limit point reached.
                converged = false;
                break 'path;
            }
            h /= 2.;
        }
        if total_iterations > MAX_TOTAL_ITERATIONS || steps_out.len() >= MAX_ACCEPTED_STEPS {
            if steps_out.is_empty() {
                return Err(first_step_error.unwrap_or_else(numeric));
            }
            converged = false;
            break 'path;
        }
    }
    let Some((f_int, _, forces)) = state(&displacement) else {
        return Err(numeric());
    };
    if !displacement.iter().all(|v| v.is_finite()) {
        return Err(numeric());
    }
    let mut reactions_n = vec![[0.; 3]; n];
    for i in 0..n {
        for k in 0..3 {
            if model.restrained[i][k] {
                reactions_n[i][k] = f_int[i * 3 + k] - lambda * model.forces_n[i][k];
            }
        }
    }
    let axial_stresses_mpa = forces
        .iter()
        .zip(&bars)
        .map(|(f, bar)| f / bar.area)
        .collect::<Vec<_>>();
    if axial_stresses_mpa.iter().any(|v| !v.is_finite()) {
        return Err(numeric());
    }
    Ok(TrussNonlinearResponse {
        displacements_mm: (0..n)
            .map(|i| std::array::from_fn(|k| displacement[i * 3 + k]))
            .collect(),
        reactions_n,
        axial_forces_n: forces,
        axial_stresses_mpa,
        steps: steps_out,
        load_factor: lambda,
        converged,
    })
}
