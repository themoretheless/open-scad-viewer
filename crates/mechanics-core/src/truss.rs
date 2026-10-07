//! Small-displacement, axial-only 3D bars with explicit zero-displacement supports.
//! No inferred supports, moments, bending, buckling, or strength recommendations.
use crate::buckling::{MAX_MODES, solve_modes};
use crate::diagnostics::{
    MIN_NORMALIZED_PIVOT, SingularityDiagnosis, diagnose_singular, summarize,
};
use crate::{Error, MassModel, Result};
use nalgebra::{Cholesky, DMatrix, DVector, Dyn};
use std::collections::BTreeSet;

pub const MAX_NODES: usize = 125;
pub const MAX_MEMBERS: usize = 400;
/// Global DOF names per node, used in singularity diagnostics.
pub const DOF_NAMES: [&str; 3] = ["x", "y", "z"];
const MAX_RELATIVE_RESIDUAL: f64 = 1e-9;

#[derive(Clone, Debug)]
pub struct Member {
    pub nodes: [usize; 2],
    pub young_mpa: f64,
    pub area_mm2: f64,
}

#[derive(Clone, Debug)]
pub struct Model {
    pub nodes_mm: Vec<[f64; 3]>,
    pub members: Vec<Member>,
    /// One XYZ mask per node; true means zero displacement, not a spring.
    pub restrained: Vec<[bool; 3]>,
    pub forces_n: Vec<[f64; 3]>,
}

#[derive(Clone, Debug)]
pub struct Response {
    pub displacements_mm: Vec<[f64; 3]>,
    /// Nonzero only at restrained DOFs, with sign in the global XYZ frame.
    pub reactions_n: Vec<[f64; 3]>,
    /// Positive means tension. Member order matches the input.
    pub axial_forces_n: Vec<f64>,
    pub axial_stresses_mpa: Vec<f64>,
    pub max_deflection_mm: f64,
    pub max_relative_residual: f64,
    pub free_dofs: usize,
}

fn invalid(message: &str) -> Error {
    Error::new("TRUSS_INVALID_INPUT", message)
}
fn numeric() -> Error {
    Error::new(
        "TRUSS_NUMERIC_RANGE",
        "Truss calculation exceeds finite numeric range",
    )
}
fn singular_diagnosed(stiffness: &DMatrix<f64>, free: &[usize]) -> Error {
    let diagnosis = diagnose_singular(stiffness, free, &DOF_NAMES);
    Error::new(
        "TRUSS_SINGULAR",
        format!(
            "Truss has an unrestrained or numerically singular mode: {}",
            summarize(&diagnosis)
        ),
    )
}

struct Bar {
    nodes: [usize; 2],
    direction: [f64; 3],
    stiffness: f64,
    area: f64,
    length: f64,
}

fn validate(model: &Model) -> Result<Vec<Bar>> {
    let n = model.nodes_mm.len();
    if n == 0 || n > MAX_NODES || model.members.is_empty() || model.members.len() > MAX_MEMBERS {
        return Err(invalid("Truss requires 1-125 nodes and 1-400 members"));
    }
    if model.restrained.len() != n || model.forces_n.len() != n {
        return Err(invalid(
            "Each node requires an explicit support mask and load vector",
        ));
    }
    if model
        .nodes_mm
        .iter()
        .chain(&model.forces_n)
        .flatten()
        .any(|v| !v.is_finite())
    {
        return Err(invalid("Node coordinates and loads must be finite"));
    }
    let mut seen = BTreeSet::new();
    let mut bars = Vec::with_capacity(model.members.len());
    for member in &model.members {
        let [a, b] = member.nodes;
        if a >= n || b >= n || a == b || !seen.insert([a.min(b), a.max(b)]) {
            return Err(invalid(
                "Members require distinct valid nodes and unique unordered edges",
            ));
        }
        if !member.young_mpa.is_finite()
            || member.young_mpa <= 0.
            || !member.area_mm2.is_finite()
            || member.area_mm2 <= 0.
        {
            return Err(invalid(
                "Member modulus and area must be finite and positive",
            ));
        }
        let delta: [f64; 3] = std::array::from_fn(|k| model.nodes_mm[b][k] - model.nodes_mm[a][k]);
        let length = delta[0].hypot(delta[1]).hypot(delta[2]);
        if length == 0. {
            return Err(invalid("Zero-length members are not supported"));
        }
        let stiffness = member.young_mpa * member.area_mm2 / length;
        if !length.is_finite() || !stiffness.is_finite() || stiffness <= 0. {
            return Err(numeric());
        }
        bars.push(Bar {
            nodes: [a, b],
            direction: delta.map(|v| v / length),
            stiffness,
            area: member.area_mm2,
            length,
        });
    }
    Ok(bars)
}

/// Global stiffness of validated bars; shared by `solve` and `diagnose`.
fn assemble_stiffness(bars: &[Bar], ndof: usize) -> DMatrix<f64> {
    let mut stiffness = DMatrix::<f64>::zeros(ndof, ndof);
    for bar in bars {
        let [a, b] = bar.nodes;
        for i in 0..3 {
            for j in 0..3 {
                let value = bar.stiffness * bar.direction[i] * bar.direction[j];
                stiffness[(a * 3 + i, a * 3 + j)] += value;
                stiffness[(b * 3 + i, b * 3 + j)] += value;
                stiffness[(a * 3 + i, b * 3 + j)] -= value;
                stiffness[(b * 3 + i, a * 3 + j)] -= value;
            }
        }
    }
    stiffness
}

/// Solve only an admitted, numerically stable pin-jointed model. Errors do not
/// carry successful-looking displacement, stiffness, stress, or margin values.
pub fn solve(model: &Model) -> Result<Response> {
    let bars = validate(model)?;
    let ndof = model.nodes_mm.len() * 3;
    let stiffness = assemble_stiffness(&bars, ndof);
    let mut result = solve_stiffness(model, &stiffness)?;
    let mut axial_forces_n = Vec::with_capacity(bars.len());
    let mut axial_stresses_mpa = Vec::with_capacity(bars.len());
    for bar in &bars {
        let [a, b] = bar.nodes;
        let extension = (0..3)
            .map(|k| {
                (result.displacements_mm[b][k] - result.displacements_mm[a][k]) * bar.direction[k]
            })
            .sum::<f64>();
        let force = bar.stiffness * extension;
        let stress = force / bar.area;
        if !force.is_finite() || !stress.is_finite() {
            return Err(numeric());
        }
        axial_forces_n.push(force);
        axial_stresses_mpa.push(stress);
    }
    result.axial_forces_n = axial_forces_n;
    result.axial_stresses_mpa = axial_stresses_mpa;
    Ok(result)
}

/// Explain why a model would be refused as singular, or confirm it is stable.
/// Load-independent: the restraint mask and the member layout decide; forces
/// are merely shape-checked.
pub fn diagnose(model: &Model) -> Result<SingularityDiagnosis> {
    let bars = validate(model)?;
    let ndof = model.nodes_mm.len() * 3;
    let stiffness = assemble_stiffness(&bars, ndof);
    if stiffness.iter().any(|v| !v.is_finite()) {
        return Err(numeric());
    }
    let free: Vec<usize> = (0..ndof)
        .filter(|&i| !model.restrained[i / 3][i % 3])
        .collect();
    Ok(diagnose_singular(&stiffness, &free, &DOF_NAMES))
}

/// One buckling mode of the reference load state, nodal form.
#[derive(Clone, Debug)]
pub struct TrussBucklingMode {
    /// Signed factor on the model loads (buckling load = λ·reference);
    /// negative means the truss buckles under the reversed load.
    pub load_factor: f64,
    /// Mode shape over all DOFs, max |component| = 1; restrained DOFs are zero.
    pub displacements: Vec<[f64; 3]>,
    pub relative_residual: f64,
}

/// Eigenvalue buckling of the truss under its own load vector.
#[derive(Clone, Debug)]
pub struct TrussBucklingResponse {
    /// Ascending |load_factor|; the first entry is the critical mode.
    pub modes: Vec<TrussBucklingMode>,
    /// Member axial forces of the reference state (tension positive) that the
    /// geometric stiffness is built from.
    pub axial_forces_n: Vec<f64>,
    pub free_dofs: usize,
}

/// Linear buckling of a pin-jointed truss under its own load vector as the
/// reference state. The geometric stiffness of a bar is transverse only
/// (N/L·(I − ddᵀ)); bars carry no bending, so lattice shear flexibility is
/// part of the answer. Not a certified stability calculation.
pub fn buckling(model: &Model, modes: usize) -> Result<TrussBucklingResponse> {
    if modes == 0 || modes > MAX_MODES {
        return Err(invalid("Buckling returns between 1 and 8 modes"));
    }
    let bars = validate(model)?;
    let ndof = model.nodes_mm.len() * 3;
    let stiffness = assemble_stiffness(&bars, ndof);
    let reference = solve(model)?;
    let factored = factor_reduced(&stiffness, &model.restrained)?;
    let mut geometric = DMatrix::<f64>::zeros(ndof, ndof);
    for (bar, &axial_n) in bars.iter().zip(&reference.axial_forces_n) {
        let c = axial_n / bar.length;
        let [a, b] = bar.nodes;
        for i in 0..3 {
            for j in 0..3 {
                // B = I − d·dᵀ: transverse geometric stiffness of a bar.
                let value = c * (f64::from(i == j) - bar.direction[i] * bar.direction[j]);
                geometric[(a * 3 + i, a * 3 + j)] += value;
                geometric[(b * 3 + i, b * 3 + j)] += value;
                geometric[(a * 3 + i, b * 3 + j)] -= value;
                geometric[(b * 3 + i, a * 3 + j)] -= value;
            }
        }
    }
    if geometric.iter().any(|v| !v.is_finite()) {
        return Err(numeric());
    }
    let solved = match &factored.factor {
        Some(factor) => solve_modes(
            factor,
            &factored.scales,
            &factored.free,
            &stiffness,
            &geometric,
            modes,
        )?,
        None => Vec::new(),
    };
    let n = model.nodes_mm.len();
    let modes_out = solved
        .into_iter()
        .map(|mode| TrussBucklingMode {
            load_factor: mode.load_factor,
            displacements: (0..n)
                .map(|i| std::array::from_fn(|k| mode.shape[i * 3 + k]))
                .collect(),
            relative_residual: mode.relative_residual,
        })
        .collect();
    Ok(TrussBucklingResponse {
        modes: modes_out,
        axial_forces_n: reference.axial_forces_n,
        free_dofs: factored.free.len(),
    })
}

/// One vibration mode, nodal form.
#[derive(Clone, Debug)]
pub struct TrussModalMode {
    pub frequency_hz: f64,
    /// Angular frequency ω = 2πf.
    pub omega_rad_s: f64,
    /// Mode shape over all DOFs, max |component| = 1; restrained DOFs are zero.
    pub displacements: Vec<[f64; 3]>,
    /// ‖Kφ − ω²Mφ‖∞ / (‖Kφ‖∞ + ‖ω²Mφ‖∞) on the free DOFs.
    pub relative_residual: f64,
}

/// Natural frequencies and mode shapes of the truss.
#[derive(Clone, Debug)]
pub struct TrussModalResponse {
    /// Ascending frequency.
    pub modes: Vec<TrussModalMode>,
    /// Total bar mass in tonnes (1 t·mm/s² = 1 N with mm units).
    pub total_mass_t: f64,
    pub free_dofs: usize,
}

/// Small-displacement modal analysis: Kφ = ω²Mφ with bar mass from
/// `densities_t_mm3` (t/mm³; steel ≈ 7.85e-9; zero means a massless bar).
/// Lumped mass halves each bar onto its ends; consistent uses the isotropic
/// ρAL/6·[[2I,I],[I,2I]] block. Massless free DOFs have no finite frequency and
/// do not appear among the modes. Not a certified dynamic calculation.
pub fn modal(
    model: &Model,
    densities_t_mm3: &[f64],
    mass_model: MassModel,
    modes: usize,
) -> Result<TrussModalResponse> {
    if modes == 0 || modes > MAX_MODES {
        return Err(invalid("Modal analysis returns between 1 and 8 modes"));
    }
    if densities_t_mm3.len() != model.members.len() {
        return Err(invalid("Each member requires a density (0 for massless)"));
    }
    if densities_t_mm3.iter().any(|d| !d.is_finite() || *d < 0.) {
        return Err(invalid("Densities must be finite and nonnegative"));
    }
    let bars = validate(model)?;
    let ndof = model.nodes_mm.len() * 3;
    let stiffness = assemble_stiffness(&bars, ndof);
    let factored = factor_reduced(&stiffness, &model.restrained)?;
    let mut mass = DMatrix::<f64>::zeros(ndof, ndof);
    for (bar, &rho) in bars.iter().zip(densities_t_mm3) {
        let total = rho * bar.area * bar.length;
        let [a, b] = bar.nodes;
        match mass_model {
            MassModel::Lumped => {
                for node in [a, b] {
                    for axis in 0..3 {
                        mass[(node * 3 + axis, node * 3 + axis)] += total / 2.;
                    }
                }
            }
            MassModel::Consistent => {
                for i in 0..3 {
                    for j in 0..3 {
                        let block = total / 6. * f64::from(i == j);
                        mass[(a * 3 + i, a * 3 + j)] += 2. * block;
                        mass[(b * 3 + i, b * 3 + j)] += 2. * block;
                        mass[(a * 3 + i, b * 3 + j)] += block;
                        mass[(b * 3 + i, a * 3 + j)] += block;
                    }
                }
            }
        }
    }
    if mass.iter().any(|v| !v.is_finite()) {
        return Err(numeric());
    }
    // Kφ = ω²Mφ is the shared pencil with geometric := −M; load_factor = ω².
    let negative_mass = -&mass;
    let solved = match &factored.factor {
        Some(factor) => solve_modes(
            factor,
            &factored.scales,
            &factored.free,
            &stiffness,
            &negative_mass,
            modes,
        )?,
        None => Vec::new(),
    };
    let total_mass_t = bars
        .iter()
        .zip(densities_t_mm3)
        .map(|(bar, rho)| rho * bar.area * bar.length)
        .sum();
    let n = model.nodes_mm.len();
    let modes_out = solved
        .into_iter()
        .map(|mode| {
            let omega = mode.load_factor.sqrt();
            TrussModalMode {
                frequency_hz: omega / (2. * std::f64::consts::PI),
                omega_rad_s: omega,
                displacements: (0..n)
                    .map(|i| std::array::from_fn(|k| mode.shape[i * 3 + k]))
                    .collect(),
                relative_residual: mode.relative_residual,
            }
        })
        .collect();
    Ok(TrussModalResponse {
        modes: modes_out,
        total_mass_t,
        free_dofs: factored.free.len(),
    })
}

/// Maximum requested load increments in one nonlinear solve.
pub const MAX_LOAD_STEPS: usize = 200;
/// Maximum Newton iterations per load increment.
pub const MAX_NR_ITERATIONS: usize = 200;

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

/// Equilibrated Cholesky factorization of the free-DOF block, shared by the
/// linear solve and the buckling eigenproblem.
pub(crate) struct Factored {
    pub(crate) free: Vec<usize>,
    pub(crate) scales: Vec<f64>,
    pub(crate) factor: Option<Cholesky<f64, Dyn>>,
}

pub(crate) fn factor_reduced(stiffness: &DMatrix<f64>, restrained: &[[bool; 3]]) -> Result<Factored> {
    let ndof = stiffness.nrows();
    let free: Vec<usize> = (0..ndof)
        .filter(|&i| !restrained[i / 3][i % 3])
        .collect();
    let mut scales = Vec::new();
    let mut factor = None;
    if !free.is_empty() {
        // Diagonal equilibration makes the refusal threshold independent of a
        // common modulus/unit scale. This is not a condition-number estimate.
        scales.reserve(free.len());
        let mut zero_diagonal = false;
        for &i in &free {
            let diagonal = stiffness[(i, i)];
            if diagonal <= 0. {
                zero_diagonal = true;
                break;
            }
            scales.push(diagonal.sqrt());
        }
        if zero_diagonal {
            return Err(singular_diagnosed(stiffness, &free));
        }
        let reduced = DMatrix::from_fn(free.len(), free.len(), |i, j| {
            stiffness[(free[i], free[j])] / scales[i] / scales[j]
        });
        if reduced.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        let Some(chol) = reduced.cholesky() else {
            return Err(singular_diagnosed(stiffness, &free));
        };
        if chol
            .l_dirty()
            .diagonal()
            .iter()
            .any(|&v| !v.is_finite() || v * v <= MIN_NORMALIZED_PIVOT)
        {
            return Err(singular_diagnosed(stiffness, &free));
        }
        factor = Some(chol);
    }
    Ok(Factored {
        free,
        scales,
        factor,
    })
}

/// Shared bounded linear solve. Callers validate their model and assembled matrix.
pub(crate) fn solve_stiffness(model: &Model, stiffness: &DMatrix<f64>) -> Result<Response> {
    let ndof = model.nodes_mm.len() * 3;
    if stiffness.iter().any(|v| !v.is_finite()) {
        return Err(numeric());
    }
    let Factored {
        free,
        scales,
        factor,
    } = factor_reduced(stiffness, &model.restrained)?;
    let mut displacement = DVector::<f64>::zeros(ndof);
    if let Some(factor) = &factor {
        let rhs = DVector::from_iterator(
            free.len(),
            free.iter()
                .enumerate()
                .map(|(i, &dof)| model.forces_n[dof / 3][dof % 3] / scales[i]),
        );
        if rhs.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        let solved = factor.solve(&rhs);
        for (i, &dof) in free.iter().enumerate() {
            displacement[dof] = solved[i] / scales[i];
        }
        if displacement.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
    }

    // Normwise backward-error scale also handles nominally zero rows whose
    // tiny transverse displacement is only floating-point roundoff.
    let matrix_norm = (0..ndof)
        .map(|i| (0..ndof).map(|j| stiffness[(i, j)].abs()).sum::<f64>())
        .fold(0., f64::max);
    let displacement_norm = displacement.iter().map(|x| x.abs()).fold(0., f64::max);
    let force_norm = model
        .forces_n
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(0., f64::max);
    let system_scale = matrix_norm * displacement_norm + force_norm;
    if !system_scale.is_finite() {
        return Err(numeric());
    }
    let mut reactions = vec![[0.; 3]; model.nodes_mm.len()];
    let mut max_relative_residual = 0f64;
    for i in 0..ndof {
        let force = model.forces_n[i / 3][i % 3];
        let mut residual = -force;
        let mut scale = force.abs();
        for j in 0..ndof {
            let contribution = stiffness[(i, j)] * displacement[j];
            residual += contribution;
            scale += contribution.abs();
        }
        if !residual.is_finite() || !scale.is_finite() {
            return Err(numeric());
        }
        if model.restrained[i / 3][i % 3] {
            reactions[i / 3][i % 3] = residual;
        } else {
            let relative = if system_scale == 0. {
                0.
            } else {
                residual.abs() / system_scale
            };
            max_relative_residual = max_relative_residual.max(relative);
        }
    }
    if max_relative_residual > MAX_RELATIVE_RESIDUAL {
        return Err(Error::new(
            "TRUSS_RESIDUAL",
            "Truss solution does not satisfy free-DOF equilibrium",
        ));
    }
    let displacements_mm: Vec<[f64; 3]> = (0..model.nodes_mm.len())
        .map(|i| std::array::from_fn(|k| displacement[i * 3 + k]))
        .collect();
    let max_deflection_mm = displacements_mm
        .iter()
        .map(|d| d[0].hypot(d[1]).hypot(d[2]))
        .fold(0., f64::max);
    if !max_deflection_mm.is_finite() {
        return Err(numeric());
    }
    Ok(Response {
        displacements_mm,
        reactions_n: reactions,
        axial_forces_n: Vec::new(),
        axial_stresses_mpa: Vec::new(),
        max_deflection_mm,
        max_relative_residual,
        free_dofs: free.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::{DofDiagnosis, DofIssue};

    fn bar() -> Model {
        Model {
            nodes_mm: vec![[0., 0., 0.], [0., 0., 10.]],
            members: vec![Member {
                nodes: [0, 1],
                young_mpa: 2000.,
                area_mm2: 2.,
            }],
            restrained: vec![[true; 3], [true, true, false]],
            forces_n: vec![[0.; 3], [0., 0., 100.]],
        }
    }
    fn close(a: f64, b: f64) {
        assert!((a - b).abs() <= 1e-10 * b.abs().max(1.), "{a} != {b}");
    }

    #[test]
    fn axial_bar_has_analytical_deflection_stress_and_reaction() {
        let response = solve(&bar()).unwrap();
        close(response.displacements_mm[1][2], 0.25);
        close(response.axial_forces_n[0], 100.);
        close(response.axial_stresses_mpa[0], 50.);
        close(response.reactions_n[0][2], -100.);
        close(response.reactions_n[1][2], 0.);
        assert!(response.max_relative_residual < 1e-12);
        assert_eq!(response.free_dofs, 1);
    }

    #[test]
    fn refuses_the_branch_two_node_lateral_load_regression() {
        let mut model = bar();
        model.restrained[1] = [false; 3];
        model.forces_n[1] = [100., 0., 0.];
        assert_eq!(solve(&model).unwrap_err().code, "TRUSS_SINGULAR");
        model.forces_n[1] = [0.; 3];
        assert_eq!(solve(&model).unwrap_err().code, "TRUSS_SINGULAR");
    }

    #[test]
    fn series_members_share_force_and_preserve_equilibrium() {
        let mut model = bar();
        model.nodes_mm.push([0., 0., 20.]);
        model.members.push(Member {
            nodes: [1, 2],
            young_mpa: 2000.,
            area_mm2: 2.,
        });
        model.restrained.push([true, true, false]);
        model.forces_n[1] = [0.; 3];
        model.forces_n.push([0., 0., 100.]);
        let result = solve(&model).unwrap();
        close(result.displacements_mm[1][2], 0.25);
        close(result.displacements_mm[2][2], 0.5);
        for force in result.axial_forces_n {
            close(force, 100.);
        }
        close(result.reactions_n.iter().map(|r| r[2]).sum::<f64>(), -100.);
    }

    #[test]
    fn restraint_loads_are_reactions_not_member_force_proxies() {
        let mut model = bar();
        model.restrained[1] = [true; 3];
        let result = solve(&model).unwrap();
        assert_eq!(result.free_dofs, 0);
        assert_eq!(result.displacements_mm, vec![[0.; 3]; 2]);
        assert_eq!(result.reactions_n[1], [0., 0., -100.]);
        assert_eq!(result.axial_forces_n, [0.]);
    }

    #[test]
    fn uniform_modulus_and_load_scaling_preserves_displacements() {
        for scale in [1e-100, 1e-20, 1., 1e20, 1e100] {
            let mut model = bar();
            model.members[0].young_mpa *= scale;
            model.forces_n[1][2] *= scale;
            let result = solve(&model).unwrap();
            close(result.displacements_mm[1][2], 0.25);
            close(result.reactions_n[0][2] / scale, -100.);
        }
    }

    #[test]
    fn rejects_invalid_input_before_matrix_allocation() {
        let mut models = Vec::new();
        let mut m = bar();
        m.nodes_mm[1] = m.nodes_mm[0];
        models.push(m);
        let mut m = bar();
        m.nodes_mm[0][0] = f64::NAN;
        models.push(m);
        let mut m = bar();
        m.forces_n[1][2] = f64::INFINITY;
        models.push(m);
        let mut m = bar();
        m.restrained.clear();
        models.push(m);
        let mut m = bar();
        m.members[0].nodes = [0, 2];
        models.push(m);
        let mut m = bar();
        m.members.push(m.members[0].clone());
        models.push(m);
        let mut m = bar();
        m.members[0].young_mpa = 0.;
        models.push(m);
        let mut m = bar();
        m.members[0].area_mm2 = -1.;
        models.push(m);
        let mut m = bar();
        m.nodes_mm = vec![[0.; 3]; 126];
        models.push(m);
        let mut m = bar();
        m.members = vec![m.members[0].clone(); 401];
        models.push(m);
        for model in models {
            assert_eq!(solve(&model).unwrap_err().code, "TRUSS_INVALID_INPUT");
        }
    }

    #[test]
    fn refuses_nearly_collinear_free_modes_and_numeric_overflow() {
        let model = Model {
            nodes_mm: vec![[0., 0., 0.], [1., 1., 0.], [1., 1. + 1e-8, 0.]],
            members: vec![
                Member {
                    nodes: [0, 1],
                    young_mpa: 2000.,
                    area_mm2: 2.,
                },
                Member {
                    nodes: [0, 2],
                    young_mpa: 2000.,
                    area_mm2: 2.,
                },
            ],
            restrained: vec![[false, false, true], [true; 3], [true; 3]],
            forces_n: vec![[100., 0., 0.], [0.; 3], [0.; 3]],
        };
        assert_eq!(solve(&model).unwrap_err().code, "TRUSS_SINGULAR");
        let mut model = bar();
        model.members[0].young_mpa = f64::MAX;
        assert_eq!(solve(&model).unwrap_err().code, "TRUSS_NUMERIC_RANGE");
    }

    #[test]
    fn rotated_translated_tripod_preserves_response_and_vector_reactions() {
        let model = Model {
            nodes_mm: vec![[0.; 3], [10., 0., 0.], [0., 10., 0.], [0., 0., 10.]],
            members: (1..4)
                .map(|i| Member {
                    nodes: [0, i],
                    young_mpa: 2000.,
                    area_mm2: 2.,
                })
                .collect(),
            restrained: vec![[false; 3], [true; 3], [true; 3], [true; 3]],
            forces_n: vec![[100., 200., 300.], [0.; 3], [0.; 3], [0.; 3]],
        };
        let response = solve(&model).unwrap();
        for (actual, expected) in response.displacements_mm[0].iter().zip([0.25, 0.5, 0.75]) {
            close(*actual, expected);
        }
        let rotate = |p: [f64; 3]| {
            let h = 0.5f64.sqrt();
            [(p[0] - p[1]) * h, (p[0] + p[1]) * h, p[2]]
        };
        let mut transformed = model.clone();
        transformed.nodes_mm = model
            .nodes_mm
            .iter()
            .map(|&p| {
                let r = rotate(p);
                std::array::from_fn(|k| r[k] + [100., -40., 17.][k])
            })
            .collect();
        transformed.forces_n = model.forces_n.iter().copied().map(rotate).collect();
        let rotated = solve(&transformed).unwrap();
        for i in 0..4 {
            for k in 0..3 {
                close(
                    rotated.displacements_mm[i][k],
                    rotate(response.displacements_mm[i])[k],
                );
                close(
                    rotated.reactions_n[i][k],
                    rotate(response.reactions_n[i])[k],
                );
            }
        }
        for k in 0..3 {
            close(
                rotated.reactions_n.iter().map(|r| r[k]).sum::<f64>(),
                -transformed.forces_n[0][k],
            );
        }
        // Reversing member endpoints must not reverse the tension convention.
        for member in &mut transformed.members {
            member.nodes.swap(0, 1);
        }
        let reversed = solve(&transformed).unwrap();
        for (a, b) in reversed.axial_forces_n.iter().zip(rotated.axial_forces_n) {
            close(*a, b);
        }
    }

    #[test]
    fn admits_the_node_limit_with_independent_stable_tripods() {
        let mut model = Model {
            nodes_mm: vec![[10., 0., 0.], [0., 10., 0.], [0., 0., 0.]],
            members: Vec::new(),
            restrained: vec![[true; 3]; 3],
            forces_n: vec![[0.; 3]; 3],
        };
        for i in 3..MAX_NODES {
            model.nodes_mm.push([1., 2., 10. + i as f64 / 10.]);
            model.restrained.push([false; 3]);
            model.forces_n.push([1., -2., -3.]);
            for anchor in 0..3 {
                model.members.push(Member {
                    nodes: [anchor, i],
                    young_mpa: 2000.,
                    area_mm2: 2.,
                });
            }
        }
        let result = solve(&model).unwrap();
        assert_eq!(result.free_dofs, 366);
        assert!(result.max_relative_residual < 1e-12);
        for k in 0..3 {
            close(
                result.reactions_n.iter().map(|r| r[k]).sum::<f64>(),
                -model.forces_n.iter().map(|f| f[k]).sum::<f64>(),
            );
        }
    }

    #[test]
    fn diagnose_names_unrestrained_dofs_and_enriches_the_error() {
        // The bar runs along z, so the freed node keeps axial stiffness; x/y
        // are completely unrestrained.
        let mut model = bar();
        model.restrained[1] = [false; 3];
        let error = solve(&model).unwrap_err();
        assert_eq!(error.code, "TRUSS_SINGULAR");
        assert!(error.contains("node 1 x unrestrained"));
        assert!(error.contains("node 1 y unrestrained"));
        let report = diagnose(&model).unwrap();
        assert!(!report.is_stable());
        assert_eq!(report.min_normalized_pivot, None);
        assert_eq!(
            report.issues,
            vec![
                DofDiagnosis {
                    node: 1,
                    dof: 0,
                    dof_name: "x",
                    issue: DofIssue::Unrestrained,
                },
                DofDiagnosis {
                    node: 1,
                    dof: 1,
                    dof_name: "y",
                    issue: DofIssue::Unrestrained,
                },
            ]
        );
    }

    #[test]
    fn diagnose_names_collinear_mechanism_dofs_and_stable_margin() {
        let mechanism = Model {
            nodes_mm: vec![[0., 0., 0.], [1., 1., 0.], [1., 1. + 1e-8, 0.]],
            members: vec![
                Member {
                    nodes: [0, 1],
                    young_mpa: 2000.,
                    area_mm2: 2.,
                },
                Member {
                    nodes: [0, 2],
                    young_mpa: 2000.,
                    area_mm2: 2.,
                },
            ],
            restrained: vec![[false, false, true], [true; 3], [true; 3]],
            forces_n: vec![[100., 0., 0.], [0.; 3], [0.; 3]],
        };
        let error = solve(&mechanism).unwrap_err();
        assert_eq!(error.code, "TRUSS_SINGULAR");
        assert!(error.contains("node 0"));
        assert!(error.contains("in a mechanism"));
        let report = diagnose(&mechanism).unwrap();
        assert!(!report.is_stable());
        assert!(
            report
                .issues
                .iter()
                .all(|d| d.node == 0 && d.issue == DofIssue::Mechanism)
        );
        assert!(report.min_normalized_pivot.unwrap() <= 1e-12);
        let stable = diagnose(&bar()).unwrap();
        assert!(stable.is_stable());
        assert!(stable.min_normalized_pivot.unwrap() > 1e-12);
    }

    /// Planar two-chord lattice cantilever along +X (span 2000, chords 50
    /// apart), fixed at the left pair of nodes, tip compressed in −x.
    fn lattice_column(area: f64, tip_load: f64) -> Model {
        let mut nodes_mm = Vec::new();
        for i in 0..5 {
            nodes_mm.push([i as f64 * 500., 0., 0.]); // bottom chord, ids 0-4
        }
        for i in 0..5 {
            nodes_mm.push([i as f64 * 500., 50., 0.]); // top chord, ids 5-9
        }
        let bar_member = |a: usize, b: usize| Member {
            nodes: [a, b],
            young_mpa: 200_000.,
            area_mm2: area,
        };
        let mut members = Vec::new();
        for i in 0..4 {
            members.push(bar_member(i, i + 1)); // bottom chord
            members.push(bar_member(5 + i, 6 + i)); // top chord
            members.push(bar_member(i, 6 + i)); // diagonal
        }
        for i in 0..5 {
            members.push(bar_member(i, 5 + i)); // verticals
        }
        let mut restrained = vec![[false, false, true]; 10];
        restrained[0] = [true; 3];
        restrained[5] = [true; 3];
        let mut forces_n = vec![[0.; 3]; 10];
        forces_n[4] = [tip_load / 2., 0., 0.];
        forces_n[9] = [tip_load / 2., 0., 0.];
        Model {
            nodes_mm,
            members,
            restrained,
            forces_n,
        }
    }

    #[test]
    fn buckling_lattice_column_near_euler_and_scales_with_area() {
        let reference = buckling(&lattice_column(10., -1000.), 2).unwrap();
        assert!(!reference.modes.is_empty());
        let lambda = reference.modes[0].load_factor;
        // Effective I = A·h²/2 = 12500 mm⁴ → Euler P_cr ≈ π²EI/(4L²) ≈ 1542 N;
        // lattice shear flexibility lowers the value, so admit a wide band.
        let euler = std::f64::consts::PI.powi(2) * 200_000. * 12_500. / (4. * 2000. * 2000.);
        assert!(
            (lambda - euler / 1000.).abs() < 0.35 * euler / 1000.,
            "{lambda} vs {}",
            euler / 1000.
        );
        assert!(lambda > 0.);
        assert!(reference.modes[0].relative_residual < 1e-8);
        // In-plane lateral mode: the peak component is a y displacement.
        let peak_y = reference.modes[0]
            .displacements
            .iter()
            .map(|d| d[1].abs())
            .fold(0., f64::max);
        assert_eq!(peak_y, 1.);
        // Doubling every bar area doubles the critical load exactly (both the
        // elastic and the lattice-shear stiffness scale with A).
        let doubled = buckling(&lattice_column(20., -1000.), 1).unwrap();
        close(doubled.modes[0].load_factor, 2. * lambda);
        // Tension reference: buckling appears only under the reversed load.
        let tension = buckling(&lattice_column(10., 1000.), 1).unwrap();
        assert!(tension.modes[0].load_factor < 0.);
        close(tension.modes[0].load_factor, -lambda);
    }

    #[test]
    fn buckling_validates_mode_count_and_propagates_singularity() {
        for bad in [0, MAX_MODES + 1] {
            assert_eq!(
                buckling(&lattice_column(10., -1000.), bad).unwrap_err().code,
                "TRUSS_INVALID_INPUT"
            );
        }
        let mut free = lattice_column(10., -1000.);
        free.restrained = vec![[false; 3]; 10];
        assert_eq!(
            buckling(&free, 1).unwrap_err().code,
            "TRUSS_SINGULAR"
        );
    }

    /// Fixed-free axial rod along +X from four bars (L = 1000, only x free).
    fn axial_rod() -> Model {
        Model {
            nodes_mm: (0..=4).map(|i| [i as f64 * 250., 0., 0.]).collect(),
            members: (0..4)
                .map(|i| Member {
                    nodes: [i, i + 1],
                    young_mpa: 200_000.,
                    area_mm2: 100.,
                })
                .collect(),
            restrained: [vec![[true; 3]], vec![[false, true, true]; 4]].concat(),
            forces_n: vec![[0.; 3]; 5],
        }
    }

    #[test]
    fn modal_axial_rod_matches_rod_theory_and_scales_with_density() {
        let densities = [8e-9; 4];
        let r = modal(&axial_rod(), &densities, MassModel::Consistent, 2).unwrap();
        assert_eq!(r.modes.len(), 2);
        close(r.total_mass_t, 8e-9 * 100. * 1000.);
        // f₁ = c/(4L), c = √(E/ρ); f₃ = 3f₁ for the fixed-free rod.
        let f1 = (200_000f64 / 8e-9).sqrt() / (4. * 1000.);
        assert!(
            (r.modes[0].frequency_hz - f1).abs() < 0.02 * f1,
            "{} vs {f1}",
            r.modes[0].frequency_hz
        );
        assert!(
            (r.modes[1].frequency_hz - 3. * f1).abs() < 0.08 * 3. * f1,
            "{} vs {}",
            r.modes[1].frequency_hz,
            3. * f1
        );
        // Axial mode: the peak component is an x displacement.
        assert_eq!(
            r.modes[0]
                .displacements
                .iter()
                .map(|d| d[0].abs())
                .fold(0., f64::max),
            1.
        );
        assert!(r.modes[0].relative_residual < 1e-8);
        // Doubling density divides the frequency by √2 exactly.
        let heavy = modal(&axial_rod(), &[16e-9; 4], MassModel::Consistent, 1).unwrap();
        close(r.modes[0].frequency_hz / 2f64.sqrt(), heavy.modes[0].frequency_hz);
        // Lumped lands slightly below; zero density leaves no modes.
        let lumped = modal(&axial_rod(), &densities, MassModel::Lumped, 1).unwrap();
        let f1_lumped = lumped.modes[0].frequency_hz;
        assert!(f1_lumped > 0.9 * f1 && f1_lumped < 1.02 * f1, "{f1_lumped} vs {f1}");
        assert!(
            modal(&axial_rod(), &[0.; 4], MassModel::Lumped, 1)
                .unwrap()
                .modes
                .is_empty()
        );
    }

    #[test]
    fn modal_validates_densities_and_mode_count() {
        for bad in [0, MAX_MODES + 1] {
            assert_eq!(
                modal(&axial_rod(), &[8e-9; 4], MassModel::Lumped, bad)
                    .unwrap_err()
                    .code,
                "TRUSS_INVALID_INPUT"
            );
        }
        for bad_densities in [vec![8e-9; 3], vec![8e-9, -1., 8e-9, 8e-9]] {
            assert_eq!(
                modal(&axial_rod(), &bad_densities, MassModel::Lumped, 1)
                    .unwrap_err()
                    .code,
                "TRUSS_INVALID_INPUT"
            );
        }
    }

    /// von Mises toggle: supports at (±1000, 0), apex at (0, 30) pressed down.
    /// Planar model: the apex is restrained out of plane.
    fn toggle(apex_load: f64) -> Model {
        Model {
            nodes_mm: vec![[-1000., 0., 0.], [1000., 0., 0.], [0., 30., 0.]],
            members: vec![
                Member {
                    nodes: [0, 2],
                    young_mpa: 200_000.,
                    area_mm2: 100.,
                },
                Member {
                    nodes: [1, 2],
                    young_mpa: 200_000.,
                    area_mm2: 100.,
                },
            ],
            restrained: vec![[true; 3], [true; 3], [false, false, true]],
            forces_n: vec![[0.; 3], [0.; 3], [0., -apex_load, 0.]],
        }
    }

    /// Exact toggle path: P(y) = 2·EA·(L₀−L)/L₀·(y/L), y = current apex height.
    fn toggle_load(y: f64) -> f64 {
        let l0 = (1000f64.powi(2) + 30f64.powi(2)).sqrt();
        let l = (1000f64.powi(2) + y * y).sqrt();
        2. * 200_000. * 100. * (l0 - l) / l0 * (y / l)
    }

    const NR: NonlinearOptions = NonlinearOptions {
        steps: 10,
        tolerance: 1e-9,
        max_iterations: 50,
    };

    #[test]
    fn nonlinear_toggle_tracks_the_exact_equilibrium_path() {
        // Ask for the load that belongs to apex height y = 20 (v = 10 mm down).
        let p = toggle_load(20.);
        let r = solve_nonlinear(&toggle(p), &NR).unwrap();
        assert!(r.converged);
        assert_eq!(r.load_factor, 1.);
        close(r.displacements_mm[2][1], -10.);
        assert!(r.displacements_mm[2][0].abs() < 1e-9);
        // Member force and reaction equilibrium at the exact state.
        let l0 = (1000f64.powi(2) + 900.).sqrt();
        let l = (1000f64.powi(2) + 400.).sqrt();
        let n = 200_000. * 100. * (l - l0) / l0;
        close(r.axial_forces_n[0], n);
        close(r.axial_forces_n[1], n);
        close(r.axial_stresses_mpa[0], n / 100.);
        close(r.reactions_n[0][1] + r.reactions_n[1][1], p);
        assert!(r.steps.len() <= 10 + 8);
        for step in &r.steps {
            assert!(step.relative_residual <= 1e-9);
        }
    }

    #[test]
    fn nonlinear_small_load_matches_the_linear_solve() {
        let model = bar();
        let linear = solve(&model).unwrap();
        let r = solve_nonlinear(&model, &NR).unwrap();
        assert!(r.converged);
        close(r.displacements_mm[1][2], linear.displacements_mm[1][2]);
        close(r.axial_forces_n[0], linear.axial_forces_n[0]);
        close(r.reactions_n[0][2], linear.reactions_n[0][2]);
    }

    #[test]
    fn nonlinear_beyond_the_limit_point_reports_snap_through() {
        // The toggle's load-controlled limit is ≈ 207.7 N; 400 N cannot be
        // reached: the solve stalls near λ ≈ 207.7/400 ≈ 0.52.
        let r = solve_nonlinear(&toggle(400.), &NR).unwrap();
        assert!(!r.converged);
        assert!(r.load_factor > 0.35 && r.load_factor < 0.65, "{}", r.load_factor);
        assert!(!r.steps.is_empty());
    }

    #[test]
    fn nonlinear_validates_options_and_keeps_mechanism_errors() {
        let model = toggle(100.);
        for bad in [
            NonlinearOptions {
                steps: 0,
                ..NR
            },
            NonlinearOptions {
                steps: MAX_LOAD_STEPS + 1,
                ..NR
            },
            NonlinearOptions {
                tolerance: 0.,
                ..NR
            },
            NonlinearOptions {
                tolerance: 0.01,
                ..NR
            },
            NonlinearOptions {
                max_iterations: 0,
                ..NR
            },
        ] {
            assert_eq!(
                solve_nonlinear(&model, &bad).unwrap_err().code,
                "TRUSS_INVALID_INPUT"
            );
        }
        // A mechanism from the start keeps the diagnosed singularity error.
        let mut free = toggle(100.);
        free.restrained[2] = [false; 3];
        assert_eq!(
            solve_nonlinear(&free, &NR).unwrap_err().code,
            "TRUSS_SINGULAR"
        );
    }
}
