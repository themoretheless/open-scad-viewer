//! Ground-structure topology optimization of pin-jointed trusses (SIMP).
//!
//! Design variables are per-member densities ρ ∈ [ρ_min, 1]; the member
//! stiffness is ρ^p·E·A_ref/L and the material volume is linear in ρ. The
//! objective is total compliance Σ w_c·F_cᵀu_c over weighted load cases,
//! minimized under Σ ρ_e·A_ref,e·L_e ≤ volume_fraction · Σ A_ref,e·L_e.
//! Classic optimality-criteria update with damping 1/2, move limits, and a
//! bisected Lagrange multiplier for the volume constraint; the start point
//! is the uniform feasible density ρ = volume_fraction.
//!
//! The ground structure must be kinematically stable at full density —
//! every loaded free DOF needs a connected bar. DOFs with no stiffness and
//! no load (out-of-plane components of planar models, isolated nodes) are
//! condensed out, as in the linear truss solver's restraint rules.
//!
//! Reported axial forces come from a FINAL LINEAR re-analysis with physical
//! areas ρ·A_ref (penalty removed), so they compose with `truss_screen`.
//! This is a compliance layout estimate, not a certified strength design:
//! buckling, joint cost and manufacturability are out of scope.
use crate::truss::{MAX_MEMBERS, MAX_NODES, Model};
use crate::{Error, Result};
use nalgebra::{Cholesky, DMatrix, DVector};
use std::collections::BTreeSet;

/// Minimum density: keeps the ground structure assembled while members vanish.
pub const MIN_DENSITY: f64 = 1e-4;
/// Load-case budget per optimization request.
pub const MAX_LOAD_CASES: usize = 8;

fn invalid(message: &str) -> Error {
    Error::new("TRUSS_TOPOLOGY_INVALID_INPUT", message)
}
fn numeric() -> Error {
    Error::new(
        "TRUSS_TOPOLOGY_NUMERIC_RANGE",
        "Truss topology optimization exceeds finite numeric range",
    )
}
fn singular() -> Error {
    Error::new(
        "TRUSS_TOPOLOGY_SINGULAR",
        "Ground structure is kinematically unstable: a loaded free DOF has no stiffness",
    )
}

/// One weighted load case: nodal forces (global XYZ, per node) and a
/// positive weight in the total compliance objective.
#[derive(Clone, Debug)]
pub struct LoadCase {
    pub forces_n: Vec<[f64; 3]>,
    pub weight: f64,
}

/// SIMP controls. Defaults: penalty 3, 200 iterations, move limit 0.2,
/// convergence tolerance 1e-3 on the maximum density change.
#[derive(Clone, Copy, Debug)]
pub struct TopologyOptions {
    /// Target material fraction of the full ground-structure volume, (0, 1].
    pub volume_fraction: f64,
    /// Stiffness penalty exponent p ≥ 1.
    pub penalty: f64,
    pub max_iterations: usize,
    /// Per-iteration density move limit, (0, 1].
    pub move_limit: f64,
    /// Convergence when max |Δρ| falls below this, (0, 1).
    pub tolerance: f64,
}

impl Default for TopologyOptions {
    fn default() -> Self {
        Self {
            volume_fraction: 0.5,
            penalty: 3.,
            max_iterations: 200,
            move_limit: 0.2,
            tolerance: 1e-3,
        }
    }
}

/// Optimized layout and its final linear response.
#[derive(Clone, Debug)]
pub struct TopologyReport {
    /// Final densities in input member order, each in [MIN_DENSITY, 1].
    pub densities: Vec<f64>,
    /// Physical areas ρ·A_ref, matching the reported axial forces.
    pub areas_mm2: Vec<f64>,
    pub volume_mm3: f64,
    pub target_volume_mm3: f64,
    /// Per-case compliance of the final penalized model, N·mm.
    pub compliances_nmm: Vec<f64>,
    /// Weighted total compliance at the uniform feasible start point.
    pub initial_compliance_nmm: f64,
    /// Weighted total compliance of the final penalized model.
    pub compliance_nmm: f64,
    pub iterations: usize,
    /// False means the iteration budget was exhausted, not invalid input.
    pub converged: bool,
    pub max_density_change: f64,
    /// Members with final density ≥ 0.5, ascending input order.
    pub active_members: Vec<usize>,
    /// Per-case axial forces (tension positive) of the FINAL LINEAR model
    /// with areas ρ·A_ref, in input member order.
    pub axial_forces_n: Vec<Vec<f64>>,
}

struct Bar {
    nodes: [usize; 2],
    direction: [f64; 3],
    /// E·A_ref/L, the unit-density axial stiffness.
    k0: f64,
    /// A_ref·L, the unit-density material volume.
    volume: f64,
    area_ref: f64,
    young: f64,
    length: f64,
}

fn validate(model: &Model, cases: &[LoadCase], options: &TopologyOptions) -> Result<Vec<Bar>> {
    let n = model.nodes_mm.len();
    if n == 0 || n > MAX_NODES || model.members.is_empty() || model.members.len() > MAX_MEMBERS {
        return Err(invalid("Ground structure requires 1-125 nodes and 1-400 members"));
    }
    if model.restrained.len() != n {
        return Err(invalid("Provide one restraint mask per node"));
    }
    if cases.is_empty() || cases.len() > MAX_LOAD_CASES {
        return Err(invalid("Provide 1-8 weighted load cases"));
    }
    for case in cases {
        if case.forces_n.len() != n
            || !case.weight.is_finite()
            || case.weight <= 0.
            || case
                .forces_n
                .iter()
                .flatten()
                .any(|v| !v.is_finite())
        {
            return Err(invalid(
                "Load cases need per-node finite forces and a positive finite weight",
            ));
        }
    }
    if !options.volume_fraction.is_finite()
        || options.volume_fraction <= 0.
        || options.volume_fraction > 1.
        || !options.penalty.is_finite()
        || options.penalty < 1.
        || options.penalty > 10.
        || options.max_iterations == 0
        || options.max_iterations > 1000
        || !options.move_limit.is_finite()
        || options.move_limit <= 0.
        || options.move_limit > 1.
        || !options.tolerance.is_finite()
        || options.tolerance <= 0.
        || options.tolerance >= 1.
    {
        return Err(invalid(
            "Options: volume fraction (0,1], penalty 1-10, 1-1000 iterations, move limit (0,1], tolerance (0,1)",
        ));
    }
    let mut bars = Vec::with_capacity(model.members.len());
    let mut seen = BTreeSet::new();
    for member in &model.members {
        let [i, j] = member.nodes;
        if i >= n || j >= n || i == j || !seen.insert((i.min(j), i.max(j))) {
            return Err(invalid(
                "Members need valid, distinct, non-duplicated node pairs",
            ));
        }
        if !member.young_mpa.is_finite()
            || member.young_mpa <= 0.
            || !member.area_mm2.is_finite()
            || member.area_mm2 <= 0.
        {
            return Err(invalid("Members need positive finite E and reference area"));
        }
        let delta = [
            model.nodes_mm[j][0] - model.nodes_mm[i][0],
            model.nodes_mm[j][1] - model.nodes_mm[i][1],
            model.nodes_mm[j][2] - model.nodes_mm[i][2],
        ];
        if delta.iter().any(|v| !v.is_finite()) {
            return Err(invalid("Node coordinates must be finite"));
        }
        let length = delta.iter().map(|v| v * v).sum::<f64>().sqrt();
        if !length.is_finite() || length <= 0. {
            return Err(invalid("Members need positive finite length"));
        }
        let k0 = member.young_mpa * member.area_mm2 / length;
        let volume = member.area_mm2 * length;
        if !k0.is_finite() || !volume.is_finite() {
            return Err(numeric());
        }
        bars.push(Bar {
            nodes: [i, j],
            direction: delta.map(|v| v / length),
            k0,
            volume,
            area_ref: member.area_mm2,
            young: member.young_mpa,
            length,
        });
    }
    Ok(bars)
}

/// One assembled-and-solved state at a given stiffness exponent.
struct Analysis {
    /// Per-case nodal displacements (full 3n vector, condensed DOFs zero).
    displacements: Vec<DVector<f64>>,
    compliances: Vec<f64>,
}

fn analyze(
    bars: &[Bar],
    densities: &[f64],
    model: &Model,
    cases: &[LoadCase],
    exponent: f64,
) -> Result<Analysis> {
    let n = model.nodes_mm.len();
    let dofs = 3 * n;
    let mut k = DMatrix::<f64>::zeros(dofs, dofs);
    for (bar, &rho) in bars.iter().zip(densities) {
        let stiffness = bar.k0 * rho.powf(exponent);
        let [i, j] = bar.nodes;
        for a in 0..3 {
            for b in 0..3 {
                let term = stiffness * bar.direction[a] * bar.direction[b];
                k[(3 * i + a, 3 * i + b)] += term;
                k[(3 * j + a, 3 * j + b)] += term;
                k[(3 * i + a, 3 * j + b)] -= term;
                k[(3 * j + a, 3 * i + b)] -= term;
            }
        }
    }
    // Condense zero-stiffness DOFs; restrained DOFs stay in the matrix with
    // a unit diagonal and zero RHS, which is exact for zero-displacement
    // supports and keeps the indexing trivial.
    let mut rhs: Vec<DVector<f64>> = cases
        .iter()
        .map(|case| DVector::from_iterator(dofs, case.forces_n.iter().flatten().copied()))
        .collect();
    for node in 0..n {
        for a in 0..3 {
            let dof = 3 * node + a;
            let loaded = rhs.iter().any(|f| f[dof] != 0.);
            if k[(dof, dof)] == 0. {
                if loaded && !model.restrained[node][a] {
                    return Err(singular());
                }
                k[(dof, dof)] = 1.;
                for f in &mut rhs {
                    f[dof] = 0.;
                }
            } else if model.restrained[node][a] {
                for other in 0..dofs {
                    k[(dof, other)] = 0.;
                    k[(other, dof)] = 0.;
                }
                k[(dof, dof)] = 1.;
                for f in &mut rhs {
                    f[dof] = 0.;
                }
            }
        }
    }
    let factor = Cholesky::new(k).ok_or_else(singular)?;
    let mut displacements = Vec::with_capacity(cases.len());
    let mut compliances = Vec::with_capacity(cases.len());
    for f in &rhs {
        let u = factor.solve(f);
        let compliance = f.dot(&u);
        if !compliance.is_finite() || compliance < 0. || u.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        compliances.push(compliance);
        displacements.push(u);
    }
    Ok(Analysis {
        displacements,
        compliances,
    })
}

fn elongation(bar: &Bar, u: &DVector<f64>) -> f64 {
    let [i, j] = bar.nodes;
    (0..3)
        .map(|a| bar.direction[a] * (u[3 * j + a] - u[3 * i + a]))
        .sum()
}

/// Runs the SIMP optimization. See the module docs for the exact model.
pub fn optimize(
    model: &Model,
    cases: &[LoadCase],
    options: &TopologyOptions,
) -> Result<TopologyReport> {
    let bars = validate(model, cases, options)?;
    let full_volume: f64 = bars.iter().map(|b| b.volume).sum();
    let target_volume = full_volume * options.volume_fraction;
    if !target_volume.is_finite() {
        return Err(numeric());
    }
    let m = bars.len();
    let mut densities = vec![options.volume_fraction; m];
    let mut analysis = analyze(&bars, &densities, model, cases, options.penalty)?;
    let weighted = |compliances: &[f64]| -> f64 {
        compliances
            .iter()
            .zip(cases)
            .map(|(c, case)| c * case.weight)
            .sum()
    };
    let initial_compliance = weighted(&analysis.compliances);
    if !initial_compliance.is_finite() {
        return Err(numeric());
    }
    let mut iterations = 0;
    let mut converged = false;
    let mut max_change = f64::INFINITY;
    let mut previous_change = f64::INFINITY;
    // OC can cycle on small determinate layouts: halve the move limit
    // whenever the density change stops decreasing. As the move limit
    // falls below the tolerance the iteration provably terminates.
    let mut move_limit = options.move_limit;
    for iteration in 0..options.max_iterations {
        iterations = iteration + 1;
        // Unit-density strain energies drive the OC sensitivity.
        let mut sensitivity = vec![0.; m];
        for (case_index, case) in cases.iter().enumerate() {
            let u = &analysis.displacements[case_index];
            for (e, bar) in bars.iter().enumerate() {
                let delta = elongation(bar, u);
                sensitivity[e] += case.weight * bar.k0 * delta * delta;
            }
        }
        let volume_of = |trial: &[f64]| -> f64 {
            trial
                .iter()
                .zip(&bars)
                .map(|(rho, bar)| rho * bar.volume)
                .sum()
        };
        let update = |lambda: f64, out: &mut Vec<f64>| {
            out.clear();
            for (e, &rho) in densities.iter().enumerate() {
                let candidate = if sensitivity[e] > 0. && lambda > 0. {
                    rho * (options.penalty * rho.powf(options.penalty - 1.) * sensitivity[e]
                        / lambda)
                        .sqrt()
                } else {
                    0.
                };
                let moved = candidate.clamp(rho - move_limit, rho + move_limit);
                out.push(moved.clamp(MIN_DENSITY, 1.));
            }
        };
        let mut trial = Vec::with_capacity(m);
        // Volume is nonincreasing in lambda: bisect the multiplier.
        let mut low = 0.;
        let mut high = 1.;
        update(high, &mut trial);
        let mut expansions = 0;
        while volume_of(&trial) > target_volume {
            high *= 2.;
            expansions += 1;
            if expansions > 200 {
                return Err(numeric());
            }
            update(high, &mut trial);
        }
        if volume_of(&trial) <= target_volume {
            for _ in 0..80 {
                let mid = 0.5 * (low + high);
                update(mid, &mut trial);
                if volume_of(&trial) > target_volume {
                    low = mid;
                } else {
                    high = mid;
                }
            }
            update(high, &mut trial);
        }
        max_change = densities
            .iter()
            .zip(&trial)
            .map(|(a, b)| (a - b).abs())
            .fold(0., f64::max);
        densities.copy_from_slice(&trial);
        analysis = analyze(&bars, &densities, model, cases, options.penalty)?;
        if max_change >= previous_change {
            move_limit = (move_limit * 0.5).max(1e-6);
        }
        previous_change = max_change;
        if max_change < options.tolerance {
            converged = true;
            break;
        }
    }
    let compliance = weighted(&analysis.compliances);
    let volume: f64 = densities
        .iter()
        .zip(&bars)
        .map(|(rho, bar)| rho * bar.volume)
        .sum();
    // Final linear response at physical areas rho*A_ref for screening.
    let linear = analyze(&bars, &densities, model, cases, 1.)?;
    let mut axial_forces_n = Vec::with_capacity(cases.len());
    for case_index in 0..cases.len() {
        let u = &linear.displacements[case_index];
        let forces: Vec<f64> = bars
            .iter()
            .zip(&densities)
            .map(|(bar, &rho)| bar.young * (rho * bar.area_ref) * elongation(bar, u) / bar.length)
            .collect();
        if forces.iter().any(|v| !v.is_finite()) {
            return Err(numeric());
        }
        axial_forces_n.push(forces);
    }
    if !compliance.is_finite() || !volume.is_finite() {
        return Err(numeric());
    }
    Ok(TopologyReport {
        areas_mm2: densities
            .iter()
            .zip(&bars)
            .map(|(rho, bar)| rho * bar.area_ref)
            .collect(),
        active_members: densities
            .iter()
            .enumerate()
            .filter(|&(_, &rho)| rho >= 0.5)
            .map(|(e, _)| e)
            .collect(),
        densities,
        volume_mm3: volume,
        target_volume_mm3: target_volume,
        compliances_nmm: analysis.compliances,
        initial_compliance_nmm: initial_compliance,
        compliance_nmm: compliance,
        iterations,
        converged,
        max_density_change: max_change,
        axial_forces_n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, reference: f64, rel_tol: f64) {
        assert!(
            (actual - reference).abs() <= rel_tol * reference.abs().max(1.),
            "{actual} vs {reference} (±{:.0}%)",
            rel_tol * 100.
        );
    }

    fn bar_model(nodes: Vec<[f64; 3]>, pairs: Vec<[usize; 2]>, restrained: Vec<[bool; 3]>) -> Model {
        let node_count = restrained.len();
        Model {
            nodes_mm: nodes,
            members: pairs
                .into_iter()
                .map(|nodes| crate::truss::Member {
                    nodes,
                    young_mpa: 200_000.,
                    area_mm2: 100.,
                })
                .collect(),
            restrained,
            forces_n: vec![[0.; 3]; node_count],
        }
    }

    #[test]
    fn single_bar_meets_volume_fraction_with_closed_form_compliance() {
        // F = 1000 N, L = 1000 mm, E = 200 GPa, A_ref = 100 mm².
        // C = F²L/(E·A·ρ^p); the volume constraint fixes ρ = 0.4 exactly.
        let model = bar_model(
            vec![[0.; 3], [1000., 0., 0.]],
            vec![[0, 1]],
            vec![[true; 3], [false; 3]],
        );
        let cases = vec![LoadCase {
            forces_n: vec![[0.; 3], [1000., 0., 0.]],
            weight: 1.,
        }];
        for (penalty, fraction) in [(1., 0.4), (3., 0.5)] {
            let report = optimize(
                &model,
                &cases,
                &TopologyOptions {
                    volume_fraction: fraction,
                    penalty,
                    ..TopologyOptions::default()
                },
            )
            .unwrap();
            assert!(report.converged);
            close(report.densities[0], fraction, 1e-3);
            close(report.volume_mm3, report.target_volume_mm3, 1e-6);
            close(
                report.compliance_nmm,
                1000f64.powi(2) * 1000. / (200_000. * 100. * fraction.powf(penalty)),
                1e-6,
            );
            // Linear re-analysis: N = F regardless of area.
            close(report.axial_forces_n[0][0], 1000., 1e-9);
        }
    }

    #[test]
    fn zero_force_member_vanishes_and_compliance_never_grows() {
        // Nodes 0,1 fully fixed; node 2 loaded. Bar 0-1 joins two supports
        // and can never carry force — its density must collapse to the floor.
        let model = bar_model(
            vec![[0.; 3], [1000., 0., 0.], [500., 1000., 0.]],
            vec![[0, 1], [0, 2], [1, 2]],
            vec![[true; 3], [true; 3], [false; 3]],
        );
        let cases = vec![LoadCase {
            forces_n: vec![[0.; 3], [0.; 3], [1000., -500., 0.]],
            weight: 1.,
        }];
        let report = optimize(
            &model,
            &cases,
            &TopologyOptions {
                volume_fraction: 0.5,
                ..TopologyOptions::default()
            },
        )
        .unwrap();
        assert!(
            report.converged,
            "change={} iterations={} densities={:?} compliance={}",
            report.max_density_change, report.iterations, report.densities, report.compliance_nmm
        );
        assert!(report.densities[0] <= 0.01, "{:?}", report.densities);
        assert_eq!(report.active_members, vec![1, 2]);
        assert!(report.compliance_nmm < report.initial_compliance_nmm);
        close(report.volume_mm3, report.target_volume_mm3, 1e-6);
        assert!(report.volume_mm3 <= report.target_volume_mm3 * 1.000001);
        // Screening-compatible linear forces: zero in the vanished bar.
        close(report.axial_forces_n[0][0], 0., 1e-6);
    }

    #[test]
    fn duplicated_cases_with_unit_weights_match_one_double_weight_case() {
        let model = bar_model(
            vec![[0.; 3], [1000., 0., 0.], [500., 1000., 0.]],
            vec![[0, 1], [0, 2], [1, 2]],
            vec![[true; 3], [true; 3], [false; 3]],
        );
        let forces = vec![[0.; 3], [0.; 3], [1000., -500., 0.]];
        let options = TopologyOptions::default();
        let single = optimize(
            &model,
            &[LoadCase {
                forces_n: forces.clone(),
                weight: 2.,
            }],
            &options,
        )
        .unwrap();
        let doubled = optimize(
            &model,
            &[
                LoadCase {
                    forces_n: forces.clone(),
                    weight: 1.,
                },
                LoadCase { forces_n: forces, weight: 1. },
            ],
            &options,
        )
        .unwrap();
        for (a, b) in single.densities.iter().zip(&doubled.densities) {
            close(*a, *b, 1e-9);
        }
        close(single.compliance_nmm, doubled.compliance_nmm, 1e-9);
        assert_eq!(doubled.compliances_nmm.len(), 2);
    }

    #[test]
    fn rejects_invalid_options_unstable_structures_and_bad_members() {
        let model = bar_model(
            vec![[0.; 3], [1000., 0., 0.]],
            vec![[0, 1]],
            vec![[true; 3], [false; 3]],
        );
        let cases = vec![LoadCase {
            forces_n: vec![[0.; 3], [1000., 0., 0.]],
            weight: 1.,
        }];
        for mutate in [
            |o: &mut TopologyOptions| o.volume_fraction = 0.,
            |o: &mut TopologyOptions| o.volume_fraction = 1.5,
            |o: &mut TopologyOptions| o.penalty = 0.5,
            |o: &mut TopologyOptions| o.max_iterations = 0,
            |o: &mut TopologyOptions| o.move_limit = 0.,
            |o: &mut TopologyOptions| o.tolerance = 0.,
        ] {
            let mut options = TopologyOptions::default();
            mutate(&mut options);
            assert_eq!(
                optimize(&model, &cases, &options).unwrap_err().code,
                "TRUSS_TOPOLOGY_INVALID_INPUT"
            );
        }
        assert!(optimize(&model, &[], &TopologyOptions::default()).is_err());
        // Transverse load on a DOF without any stiffness: unstable.
        let transverse = vec![LoadCase {
            forces_n: vec![[0.; 3], [0., 1000., 0.]],
            weight: 1.,
        }];
        assert_eq!(
            optimize(&model, &transverse, &TopologyOptions::default())
                .unwrap_err()
                .code,
            "TRUSS_TOPOLOGY_SINGULAR"
        );
    }
}
