//! Boolean operations with a manifoldness guarantee.
//!
//! Facade over the `polygon-core` BSP boolean kernels with the contract of
//! the manifold library:
//!
//! 1. **Pre-check** — each operand must be a strict 2-manifold. Non-manifold
//!    input is repaired first (`manifold_core::repair_with_mode`, Full mode);
//!    unrepairable input is rejected with [`BooleanError::UnrepairableInput`].
//! 2. **BSP boolean** — `polygon_core::solid::boolean::boolean` with default
//!    tolerance/work budgets.
//! 3. **Post-check** — the result must be strictly manifold. A small
//!    conservative repair pass (weld + orientation) handles BSP seam
//!    duplicates; anything worse is an error, never a silent non-manifold
//!    result.
//!
//! This crate sits above `manifold-core` because `polygon-core` itself
//! depends on `manifold-core` — the facade cannot live in the core without
//! a dependency cycle.

use manifold_core::{ManifoldReport, RepairMode, RepairReport};
use polygon_core::Mesh;
use polygon_core::solid::boolean;

/// Boolean operation kind (mirrors `polygon_core` `Operation`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOp {
    Union,
    Intersection,
    Difference,
}

impl From<BooleanOp> for boolean::Operation {
    fn from(op: BooleanOp) -> Self {
        match op {
            BooleanOp::Union => boolean::Operation::Union,
            BooleanOp::Intersection => boolean::Operation::Intersection,
            BooleanOp::Difference => boolean::Operation::Difference,
        }
    }
}

/// Per-operand pre-repair statistics (present only when repair was needed).
#[derive(Debug, Clone)]
pub struct OperandRepair {
    pub report: RepairReport,
}

/// Successful boolean result; `positions`/`indices` are strictly manifold.
#[derive(Debug, Clone)]
pub struct BooleanOutcome {
    pub positions: Vec<f64>,
    pub indices: Vec<usize>,
    /// BSP kernel report (tolerance, work, fragments).
    pub kernel_report: boolean::BooleanReport,
    /// Pre-repair of operands, when their input check failed.
    pub operand_repairs: [Option<RepairReport>; 2],
    /// Manifold report computed on the returned mesh (strictly manifold).
    pub output_report: ManifoldReport,
}

/// Why a guaranteed-manifold boolean failed.
#[derive(Debug)]
pub enum BooleanError {
    /// An operand stayed non-manifold even after Full repair.
    UnrepairableInput {
        operand: usize,
        residual: Box<ManifoldReport>,
    },
    /// The BSP kernel rejected the input or exhausted a budget.
    Kernel(math_core::Error),
    /// The kernel result was not manifold and conservative repair could not
    /// make it so. This is a kernel defect, not an input problem.
    NonManifoldOutput { residual: Box<ManifoldReport> },
}

impl std::fmt::Display for BooleanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnrepairableInput { operand, residual } => write!(
                f,
                "operand {operand} is not repairable to a manifold solid \
                 ({} non-manifold edges, {} pinched vertices, {} degenerate triangles)",
                residual.non_manifold_edges.len(),
                residual.non_manifold_vertices.len(),
                residual.degenerate_triangles.len(),
            ),
            Self::Kernel(e) => write!(f, "boolean kernel: {e}"),
            Self::NonManifoldOutput { residual } => write!(
                f,
                "boolean result is not manifold ({} non-manifold edges, \
                 {} orientation conflicts)",
                residual.non_manifold_edges.len(),
                residual.orientation_edges.len(),
            ),
        }
    }
}

impl std::error::Error for BooleanError {}

/// Default weld epsilon for pre/post repair (exact bitwise weld).
const WELD_EPSILON: f64 = 0.0;

/// Ensure `positions`/`indices` are strictly manifold, repairing in Full
/// mode when necessary.
fn admit(operand: usize, positions: &[f64], indices: &[usize]) -> Result<Admitted, BooleanError> {
    let first = manifold_core::check(positions, indices);
    if first.is_manifold() {
        return Ok(Admitted {
            positions: positions.to_vec(),
            indices: indices.to_vec(),
            repair: None,
        });
    }
    let out = manifold_core::repair_with_mode(positions, indices, WELD_EPSILON, RepairMode::Full);
    if !out.is_manifold() {
        return Err(BooleanError::UnrepairableInput {
            operand,
            residual: Box::new(out.report.residual),
        });
    }
    Ok(Admitted {
        positions: out.positions,
        indices: out.indices,
        repair: Some(out.report),
    })
}

struct Admitted {
    positions: Vec<f64>,
    indices: Vec<usize>,
    repair: Option<RepairReport>,
}

/// Boolean `a OP b` with guaranteed strictly manifold output.
pub fn boolean_manifold(
    a_positions: &[f64],
    a_indices: &[usize],
    b_positions: &[f64],
    b_indices: &[usize],
    op: BooleanOp,
) -> Result<BooleanOutcome, BooleanError> {
    let a = admit(0, a_positions, a_indices)?;
    let b = admit(1, b_positions, b_indices)?;

    let to_mesh = |positions: Vec<f64>, indices: Vec<usize>| Mesh {
        positions,
        indices,
        uv: None,
    };
    let built = boolean::boolean(
        &to_mesh(a.positions, a.indices),
        &to_mesh(b.positions, b.indices),
        op.into(),
        &boolean::Options::default(),
    )
    .map_err(BooleanError::Kernel)?;

    // Post-check: the result must be strictly manifold. BSP seam vertices
    // can duplicate bitwise, so one conservative weld pass is legitimate.
    let report = manifold_core::check(&built.mesh.positions, &built.mesh.indices);
    let (positions, indices, output_report) = if report.is_manifold() {
        (built.mesh.positions, built.mesh.indices, report)
    } else {
        let out = manifold_core::repair(&built.mesh.positions, &built.mesh.indices, WELD_EPSILON);
        if !out.is_manifold() {
            return Err(BooleanError::NonManifoldOutput {
                residual: Box::new(out.report.residual),
            });
        }
        (out.positions, out.indices, out.report.residual)
    };

    Ok(BooleanOutcome {
        positions,
        indices,
        kernel_report: built.report.boolean.unwrap_or(boolean::BooleanReport {
            operation: op.into(),
            tolerance_mm: 0.0,
            work: 0,
            fragments: 0,
            input_triangles: [0, 0],
        }),
        operand_repairs: [a.repair, b.repair],
        output_report,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cube(origin: [f64; 3], size: f64) -> (Vec<f64>, Vec<usize>) {
        let [x, y, z] = origin;
        let s = size;
        #[rustfmt::skip]
        let positions = vec![
            x, y, z, x + s, y, z, x + s, y + s, z, x, y + s, z,
            x, y, z + s, x + s, y, z + s, x + s, y + s, z + s, x, y + s, z + s,
        ];
        let indices = vec![
            0, 2, 1, 0, 3, 2, // bottom
            4, 5, 6, 4, 6, 7, // top
            0, 1, 5, 0, 5, 4, // front
            2, 3, 7, 2, 7, 6, // back
            1, 2, 6, 1, 6, 5, // right
            3, 0, 4, 3, 4, 7, // left
        ];
        (positions, indices)
    }

    #[test]
    fn union_of_overlapping_cubes_is_manifold() {
        let (ap, ai) = cube([0.0, 0.0, 0.0], 1.0);
        let (bp, bi) = cube([0.5, 0.5, 0.5], 1.0);
        let out = boolean_manifold(&ap, &ai, &bp, &bi, BooleanOp::Union).unwrap();
        assert!(out.output_report.is_manifold());
        assert!(out.operand_repairs.iter().all(|r| r.is_none()));
    }

    #[test]
    fn intersection_of_overlapping_cubes_is_manifold() {
        let (ap, ai) = cube([0.0, 0.0, 0.0], 1.0);
        let (bp, bi) = cube([0.5, 0.5, 0.5], 1.0);
        let out = boolean_manifold(&ap, &ai, &bp, &bi, BooleanOp::Intersection).unwrap();
        assert!(out.output_report.is_manifold());
        assert!(!out.indices.is_empty());
    }

    #[test]
    fn difference_of_overlapping_cubes_is_manifold() {
        let (ap, ai) = cube([0.0, 0.0, 0.0], 1.0);
        let (bp, bi) = cube([0.5, 0.5, 0.5], 1.0);
        let out = boolean_manifold(&ap, &ai, &bp, &bi, BooleanOp::Difference).unwrap();
        assert!(out.output_report.is_manifold());
    }

    #[test]
    fn unwelded_input_is_repaired_first() {
        let (ap, ai) = cube([0.0, 0.0, 0.0], 1.0);
        // Second cube as unwelded face soup (24 duplicated vertices).
        let (bp, bi) = cube([0.5, 0.5, 0.5], 1.0);
        let mut soup_p = Vec::new();
        let mut soup_i = Vec::new();
        for t in bi.as_chunks::<3>().0 {
            for &corner in t {
                soup_i.push(soup_p.len() / 3);
                soup_p.extend_from_slice(&bp[corner * 3..corner * 3 + 3]);
            }
        }
        assert!(!manifold_core::is_manifold(&soup_p, &soup_i));
        let out = boolean_manifold(&ap, &ai, &soup_p, &soup_i, BooleanOp::Union).unwrap();
        assert!(out.output_report.is_manifold());
        assert!(out.operand_repairs[1].is_some());
    }

    #[test]
    fn unrepairable_input_is_rejected() {
        let (ap, ai) = cube([0.0, 0.0, 0.0], 1.0);
        // Bowtie: two triangles sharing only vertex 0. The boundary branches
        // at the shared vertex, so loops are ambiguous and cannot be filled
        // without dropping faces.
        let bp = vec![
            2.0, 0.0, 0.0, 3.0, 0.0, 0.0, 2.0, 1.0, 0.0, 2.0, 0.0, 1.0, 2.0, -1.0, 0.0,
        ];
        let bi = vec![0, 1, 2, 0, 3, 4];
        let err = boolean_manifold(&ap, &ai, &bp, &bi, BooleanOp::Union).unwrap_err();
        assert!(matches!(
            err,
            BooleanError::UnrepairableInput { operand: 1, .. }
        ));
    }
}
