//! Small-displacement, axial-only 3D bars with explicit zero-displacement supports.
//! No inferred supports, moments, bending, buckling, or strength recommendations.
use crate::{Error, Result};
use nalgebra::{DMatrix, DVector};
use std::collections::BTreeSet;

pub const MAX_NODES: usize = 125;
pub const MAX_MEMBERS: usize = 400;
const MIN_NORMALIZED_PIVOT: f64 = 1e-12;
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
fn singular() -> Error {
    Error::new(
        "TRUSS_SINGULAR",
        "Truss has an unrestrained or numerically singular mode",
    )
}

struct Bar {
    nodes: [usize; 2],
    direction: [f64; 3],
    stiffness: f64,
    area: f64,
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
        });
    }
    Ok(bars)
}

/// Solve only an admitted, numerically stable pin-jointed model. Errors do not
/// carry successful-looking displacement, stiffness, stress, or margin values.
pub fn solve(model: &Model) -> Result<Response> {
    let bars = validate(model)?;
    let ndof = model.nodes_mm.len() * 3;
    let mut stiffness = DMatrix::<f64>::zeros(ndof, ndof);
    for bar in &bars {
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

/// Shared bounded linear solve. Callers validate their model and assembled matrix.
pub(crate) fn solve_stiffness(model: &Model, stiffness: &DMatrix<f64>) -> Result<Response> {
    let ndof = model.nodes_mm.len() * 3;
    if stiffness.iter().any(|v| !v.is_finite()) {
        return Err(numeric());
    }
    let free: Vec<usize> = (0..ndof)
        .filter(|&i| !model.restrained[i / 3][i % 3])
        .collect();
    let mut displacement = DVector::<f64>::zeros(ndof);
    if !free.is_empty() {
        // Diagonal equilibration makes the refusal threshold independent of a
        // common modulus/unit scale. This is not a condition-number estimate.
        let mut scales = Vec::with_capacity(free.len());
        for &i in &free {
            let diagonal = stiffness[(i, i)];
            if diagonal <= 0. {
                return Err(singular());
            }
            scales.push(diagonal.sqrt());
        }
        let reduced = DMatrix::from_fn(free.len(), free.len(), |i, j| {
            stiffness[(free[i], free[j])] / scales[i] / scales[j]
        });
        let rhs = DVector::from_iterator(
            free.len(),
            free.iter()
                .enumerate()
                .map(|(i, &dof)| model.forces_n[dof / 3][dof % 3] / scales[i]),
        );
        if reduced.iter().chain(rhs.iter()).any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        let factor = reduced.cholesky().ok_or_else(singular)?;
        if factor
            .l_dirty()
            .diagonal()
            .iter()
            .any(|&v| !v.is_finite() || v * v <= MIN_NORMALIZED_PIVOT)
        {
            return Err(singular());
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
}
