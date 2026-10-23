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

mod nonlinear;
use nonlinear::*;
pub use nonlinear::{NonlinearOptions, NonlinearStep, TrussNonlinearResponse, solve_nonlinear};


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
#[path = "tests/truss.rs"]
mod tests;
