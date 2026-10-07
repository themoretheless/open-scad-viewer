//! Shared singularity diagnostics for the frame and truss solvers.
//!
//! Both solvers refuse a singular or numerically unstable stiffness matrix,
//! but a bare refusal cannot say which degrees of freedom are free to move.
//! This module answers that. Zero diagonals identify completely unrestrained
//! DOFs (isolated nodes, fully released directions). A manual LDLᵀ run on the
//! equilibrated reduced matrix finds the first failing pivot and ranks the
//! DOFs dominating its null mode (mechanisms: hinge chains, collinear bars,
//! rigid-body modes).
//!
//! Diagnostics run only on the error path or on an explicit `diagnose` call;
//! the fast solve path keeps the nalgebra Cholesky untouched.

use nalgebra::DMatrix;

/// Normalized-pivot refusal threshold shared by both solvers. The reduced
/// matrix is diagonally equilibrated, so its pivots are dimensionless.
pub(crate) const MIN_NORMALIZED_PIVOT: f64 = 1e-12;

/// DOFs named in one report; keeps the report and error messages compact.
const MAX_REPORTED_DOFS: usize = 12;
/// DOFs listed verbatim in an error message before "+N more".
const MAX_LISTED_DOFS: usize = 6;
/// Null-mode entries below this fraction of the largest are not reported.
const NULL_MODE_CUTOFF: f64 = 0.25;

/// Why one DOF is implicated in a singular stiffness matrix.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DofIssue {
    /// No stiffness on the diagonal: isolated node or fully released direction.
    Unrestrained,
    /// Connected, but the factorization breaks down: a mechanism such as a
    /// hinge chain, collinear bars, or a rigid-body mode.
    Mechanism,
}

/// One DOF implicated in a singular stiffness matrix.
#[derive(Clone, Debug, PartialEq)]
pub struct DofDiagnosis {
    pub node: usize,
    pub dof: usize,
    pub dof_name: &'static str,
    pub issue: DofIssue,
}

/// Structured answer to "why is this structure singular".
#[derive(Clone, Debug, PartialEq)]
pub struct SingularityDiagnosis {
    pub issues: Vec<DofDiagnosis>,
    /// Smallest normalized LDLᵀ pivot over the free DOFs; a small value marks
    /// a near-mechanism even in a stable structure. `None` when a zero
    /// diagonal stopped assembly before factorization.
    pub min_normalized_pivot: Option<f64>,
}

impl SingularityDiagnosis {
    /// No implicated DOFs: the factorization stands.
    pub fn is_stable(&self) -> bool {
        self.issues.is_empty()
    }
}

fn dof_diagnosis(global: usize, dof_names: &'static [&'static str], issue: DofIssue) -> DofDiagnosis {
    let per_node = dof_names.len();
    DofDiagnosis {
        node: global / per_node,
        dof: global % per_node,
        dof_name: dof_names[global % per_node],
        issue,
    }
}

/// Diagnose an assembled stiffness matrix from its free-DOF list. Zero
/// diagonals are reported first; otherwise a full LDLᵀ runs, so a stable
/// structure still reports its stability margin.
pub(crate) fn diagnose_singular(
    stiffness: &DMatrix<f64>,
    free: &[usize],
    dof_names: &'static [&'static str],
) -> SingularityDiagnosis {
    let mut issues: Vec<DofDiagnosis> = free
        .iter()
        .filter(|&&dof| stiffness[(dof, dof)] <= 0.)
        .map(|&dof| dof_diagnosis(dof, dof_names, DofIssue::Unrestrained))
        .collect();
    if !issues.is_empty() {
        issues.truncate(MAX_REPORTED_DOFS);
        return SingularityDiagnosis {
            issues,
            min_normalized_pivot: None,
        };
    }
    ldl_diagnosis(stiffness, free, dof_names)
}

/// Equilibrated LDLᵀ without pivoting over the free DOFs. The pivot order is
/// deterministic, so the first failing pivot names a DOF inside the null mode;
/// the null-mode vector ranks the rest.
fn ldl_diagnosis(
    stiffness: &DMatrix<f64>,
    free: &[usize],
    dof_names: &'static [&'static str],
) -> SingularityDiagnosis {
    let n = free.len();
    let scales: Vec<f64> = free.iter().map(|&i| stiffness[(i, i)].sqrt()).collect();
    let a = |i: usize, j: usize| stiffness[(free[i], free[j])] / scales[i] / scales[j];
    let mut l = DMatrix::<f64>::zeros(n, n);
    let mut d = vec![0f64; n];
    let mut min_pivot = f64::INFINITY;
    for k in 0..n {
        let mut dk = a(k, k);
        for j in 0..k {
            dk -= l[(k, j)] * l[(k, j)] * d[j];
        }
        d[k] = dk;
        min_pivot = min_pivot.min(dk);
        if !dk.is_finite() || dk <= MIN_NORMALIZED_PIVOT {
            return null_mode_report(&l, k, free, dof_names, dk);
        }
        for i in (k + 1)..n {
            let mut lik = a(i, k);
            for j in 0..k {
                lik -= l[(i, j)] * d[j] * l[(k, j)];
            }
            l[(i, k)] = lik / dk;
        }
    }
    SingularityDiagnosis {
        issues: Vec::new(),
        min_normalized_pivot: min_pivot.is_finite().then_some(min_pivot),
    }
}

/// DOFs dominating the null mode exposed by a failing pivot at step k.
/// The leading (k+1)-principal submatrix is LDLᵀ with d_k ≈ 0, so the vector
/// v = (Lᵀ)⁻¹ e_k annihilates it; its largest entries name the mechanism.
fn null_mode_report(
    l: &DMatrix<f64>,
    k: usize,
    free: &[usize],
    dof_names: &'static [&'static str],
    pivot: f64,
) -> SingularityDiagnosis {
    let mut v = vec![0f64; k + 1];
    v[k] = 1.;
    for j in (0..k).rev() {
        v[j] = -(j + 1..=k).map(|i| l[(i, j)] * v[i]).sum::<f64>();
    }
    let peak = v.iter().map(|x| x.abs()).fold(0., f64::max);
    let mut ranked: Vec<usize> = (0..=k)
        .filter(|&i| v[i].abs() >= NULL_MODE_CUTOFF * peak)
        .collect();
    ranked.sort_by(|&i, &j| v[j].abs().total_cmp(&v[i].abs()));
    ranked.truncate(MAX_REPORTED_DOFS);
    ranked.sort_unstable();
    SingularityDiagnosis {
        issues: ranked
            .into_iter()
            .map(|i| dof_diagnosis(free[i], dof_names, DofIssue::Mechanism))
            .collect(),
        min_normalized_pivot: pivot.is_finite().then_some(pivot),
    }
}

/// Compact one-line diagnosis appended to singular solver errors.
pub(crate) fn summarize(diagnosis: &SingularityDiagnosis) -> String {
    if diagnosis.issues.is_empty() {
        return "no specific DOF identified".to_string();
    }
    let mut parts: Vec<String> = diagnosis
        .issues
        .iter()
        .take(MAX_LISTED_DOFS)
        .map(|d| {
            let what = match d.issue {
                DofIssue::Unrestrained => "unrestrained",
                DofIssue::Mechanism => "in a mechanism",
            };
            format!("node {} {} {}", d.node, d.dof_name, what)
        })
        .collect();
    let extra = diagnosis.issues.len() - parts.len();
    if extra > 0 {
        parts.push(format!("+{extra} more DOFs"));
    }
    let mut text = parts.join("; ");
    if let Some(pivot) = diagnosis.min_normalized_pivot {
        text.push_str(&format!(" (normalized pivot {pivot:.1e})"));
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;

    const NAMES: [&str; 3] = ["x", "y", "z"];

    #[test]
    fn zero_diagonals_report_unrestrained_dofs() {
        let stiffness = DMatrix::from_diagonal(&nalgebra::DVector::from_vec(vec![
            1., 0., 2., 3., 0., 4.,
        ]));
        let free: Vec<usize> = (0..6).collect();
        let diagnosis = diagnose_singular(&stiffness, &free, &NAMES);
        assert_eq!(
            diagnosis.issues,
            vec![
                DofDiagnosis {
                    node: 0,
                    dof: 1,
                    dof_name: "y",
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
        assert_eq!(diagnosis.min_normalized_pivot, None);
        assert!(!diagnosis.is_stable());
    }

    #[test]
    fn rigid_body_mode_reports_mechanism_dofs_and_pivot() {
        // Two-node bar along z with weak springs on x/y: every diagonal is
        // positive, but the axial rigid mode between the nodes is singular.
        let k = 2.;
        let mut stiffness = DMatrix::<f64>::zeros(6, 6);
        for node in 0..2 {
            for axis in 0..2 {
                stiffness[(3 * node + axis, 3 * node + axis)] = 1.;
            }
        }
        for (i, sign) in [(0usize, 1.), (1, -1.)] {
            for (j, sign_j) in [(0usize, 1.), (1, -1.)] {
                stiffness[(3 * i + 2, 3 * j + 2)] += sign * sign_j * k;
            }
        }
        let free: Vec<usize> = (0..6).collect();
        let diagnosis = diagnose_singular(&stiffness, &free, &NAMES);
        assert!(!diagnosis.is_stable());
        assert!(
            diagnosis
                .issues
                .iter()
                .all(|d| d.issue == DofIssue::Mechanism)
        );
        assert!(diagnosis.issues.iter().any(|d| d.node == 0 && d.dof == 2));
        assert!(diagnosis.issues.iter().any(|d| d.node == 1 && d.dof == 2));
        let pivot = diagnosis.min_normalized_pivot.unwrap();
        assert!(pivot <= MIN_NORMALIZED_PIVOT);
        let message = summarize(&diagnosis);
        assert!(message.contains("node 0 z in a mechanism"));
        assert!(message.contains("normalized pivot"));
    }

    #[test]
    fn stable_matrix_reports_empty_issues_and_margin() {
        let stiffness = DMatrix::<f64>::identity(3, 3) * 7.;
        let diagnosis = diagnose_singular(&stiffness, &[0, 1, 2], &NAMES);
        assert!(diagnosis.is_stable());
        let pivot = diagnosis.min_normalized_pivot.unwrap();
        assert!((pivot - 1.).abs() < 1e-12);
    }

    #[test]
    fn summary_caps_the_listed_dofs() {
        let diagnosis = SingularityDiagnosis {
            issues: (0..9)
                .map(|i| DofDiagnosis {
                    node: i,
                    dof: 0,
                    dof_name: "x",
                    issue: DofIssue::Unrestrained,
                })
                .collect(),
            min_normalized_pivot: None,
        };
        let message = summarize(&diagnosis);
        assert!(message.contains("+3 more DOFs"));
        assert!(!message.contains("node 6"));
    }
}
