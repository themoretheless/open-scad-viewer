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
    /// Symmetric difference `(A ∪ B) − (A ∩ B)`. Overlapping operands whose
    /// surfaces cross leave the two parts sharing only the crossing curve;
    /// such a result is not a 2-manifold and is rejected with an error
    /// (kernel validity or [`BooleanError::NonManifoldOutput`]), never
    /// returned.
    Xor,
}

impl BooleanOp {
    fn kernel(self) -> Option<boolean::Operation> {
        match self {
            BooleanOp::Union => Some(boolean::Operation::Union),
            BooleanOp::Intersection => Some(boolean::Operation::Intersection),
            BooleanOp::Difference => Some(boolean::Operation::Difference),
            BooleanOp::Xor => None,
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

impl BooleanOutcome {
    /// Borrow or convert the boolean result into a `polygon_core::Mesh`.
    pub fn to_mesh(&self) -> Mesh {
        Mesh {
            positions: self.positions.clone(),
            indices: self.indices.clone(),
            uv: None,
        }
    }
    /// Total surface area of the manifold boolean result.
    pub fn surface_area(&self) -> f64 {
        self.to_mesh().surface_area()
    }
    /// Signed enclosed volume of the manifold boolean result.
    pub fn volume(&self) -> f64 {
        self.to_mesh().volume()
    }
    /// Volume-weighted center of mass of the manifold boolean result.
    pub fn center_of_mass(&self) -> [f64; 3] {
        self.to_mesh().center_of_mass()
    }
    /// 3x3 inertia tensor relative to `center_of_mass()` (assuming unit density).
    pub fn inertia_tensor(&self) -> [[f64; 3]; 3] {
        self.to_mesh().inertia_tensor()
    }
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

/// Validated tolerances for operand repair and BSP classification.
#[derive(Clone, Copy, Debug)]
pub struct BooleanOptions {
    /// Absolute mesh-unit epsilon used only when repair is necessary.
    pub weld_epsilon: f64,
    /// BSP tolerance relative to the longest combined input-bound side.
    pub relative_tolerance: f64,
    /// BSP work budget per kernel call (`1..=MAX_WORK_BUDGET`).
    pub max_work: usize,
}

/// Default BSP work budget, equal to the kernel default.
pub const DEFAULT_WORK_BUDGET: usize = 8_000_000;
/// Largest accepted BSP work budget.
pub const MAX_WORK_BUDGET: usize = 64_000_000;

impl Default for BooleanOptions {
    fn default() -> Self {
        Self { weld_epsilon: 0.0, relative_tolerance: 1e-9, max_work: DEFAULT_WORK_BUDGET }
    }
}

fn invalid_options(message: &'static str) -> BooleanError {
    BooleanError::Kernel(math_core::Error { code: "GEOMETRY_INVALID_INPUT", message: message.into() })
}

/// Ensure `positions`/`indices` are strictly manifold, repairing in Full
/// mode when necessary.
fn admit(operand: usize, positions: &[f64], indices: &[usize], weld_epsilon: f64) -> Result<Admitted, BooleanError> {
    let first = manifold_core::check(positions, indices);
    let mut admitted = if first.is_manifold() {
        Admitted { positions: positions.to_vec(), indices: indices.to_vec(), repair: None }
    } else {
        let out = manifold_core::repair_with_mode(positions, indices, weld_epsilon, RepairMode::Full);
        if !out.is_manifold() {
            return Err(BooleanError::UnrepairableInput {
                operand,
                residual: Box::new(out.report.residual),
            });
        }
        Admitted { positions: out.positions, indices: out.indices, repair: Some(out.report) }
    };
    // A consistently oriented closed mesh can still face inward (an
    // inside-out solid, or orientation unified towards a flipped seed
    // triangle). Reverse it so the kernel receives an outward solid.
    let mesh = Mesh { positions: admitted.positions, indices: admitted.indices, uv: None };
    let inverted = mesh.volume() < 0.0;
    let Mesh { positions, mut indices, .. } = mesh;
    if inverted {
        for triangle in indices.chunks_mut(3) {
            triangle.swap(1, 2);
        }
        if admitted.repair.is_none() {
            admitted.repair = Some(manifold_core::repair_with_mode(&positions, &indices, weld_epsilon, RepairMode::Full).report);
        }
    }
    admitted.positions = positions;
    admitted.indices = indices;
    Ok(admitted)
}

struct Admitted {
    positions: Vec<f64>,
    indices: Vec<usize>,
    repair: Option<RepairReport>,
}

/// Boolean `a OP b` with guaranteed strictly manifold output and legacy tolerances.
pub fn boolean_manifold(
    a_positions: &[f64],
    a_indices: &[usize],
    b_positions: &[f64],
    b_indices: &[usize],
    op: BooleanOp,
) -> Result<BooleanOutcome, BooleanError> {
    boolean_manifold_with_options(a_positions, a_indices, b_positions, b_indices, op, BooleanOptions::default())
}

/// Boolean with explicit repair and BSP tolerances; all geometry stays native.
pub fn boolean_manifold_with_options(
    a_positions: &[f64],
    a_indices: &[usize],
    b_positions: &[f64],
    b_indices: &[usize],
    op: BooleanOp,
    options: BooleanOptions,
) -> Result<BooleanOutcome, BooleanError> {
    if !options.weld_epsilon.is_finite() || !(0.0..=1.0).contains(&options.weld_epsilon) {
        return Err(invalid_options("Boolean weld epsilon must be finite and within 0..=1 mm"));
    }
    if !(1e-12..=1e-5).contains(&options.relative_tolerance) {
        return Err(invalid_options("Boolean relative tolerance must be within 1e-12..=1e-5"));
    }
    if !(1..=MAX_WORK_BUDGET).contains(&options.max_work) {
        return Err(invalid_options("Boolean work budget must be within 1..=64000000"));
    }
    let a = admit(0, a_positions, a_indices, options.weld_epsilon)?;
    let b = admit(1, b_positions, b_indices, options.weld_epsilon)?;

    let operand_repairs = [a.repair, b.repair];
    let a = Mesh { positions: a.positions, indices: a.indices, uv: None };
    let b = Mesh { positions: b.positions, indices: b.indices, uv: None };
    let kernel_options = boolean::Options {
        relative_tolerance: options.relative_tolerance,
        max_work: options.max_work,
        ..boolean::Options::default()
    };
    let run = |x: &Mesh, y: &Mesh, op: boolean::Operation| {
        boolean::boolean(x, y, op, &kernel_options).map_err(BooleanError::Kernel)
    };
    let report_of = |built: &polygon_core::BuiltMesh, op: boolean::Operation| {
        built.report.boolean.clone().unwrap_or(boolean::BooleanReport {
            operation: op,
            tolerance_mm: 0.0,
            work: 0,
            fragments: 0,
            input_triangles: [0, 0],
        })
    };
    let (built_mesh, kernel_report) = match op.kernel() {
        Some(kop) => {
            let built = run(&a, &b, kop)?;
            let report = report_of(&built, kop);
            (built.mesh, report)
        }
        None => {
            let union = run(&a, &b, boolean::Operation::Union)?;
            let common = run(&a, &b, boolean::Operation::Intersection)?;
            let built = run(&union.mesh, &common.mesh, boolean::Operation::Difference)?;
            let parts = [
                report_of(&union, boolean::Operation::Union),
                report_of(&common, boolean::Operation::Intersection),
                report_of(&built, boolean::Operation::Difference),
            ];
            let report = boolean::BooleanReport {
                operation: boolean::Operation::Difference,
                tolerance_mm: parts.iter().map(|r| r.tolerance_mm).fold(0.0, f64::max),
                work: parts.iter().map(|r| r.work).sum(),
                fragments: parts.iter().map(|r| r.fragments).sum(),
                input_triangles: [a.indices.len() / 3, b.indices.len() / 3],
            };
            (built.mesh, report)
        }
    };

    // Post-check: the result must be strictly manifold. BSP seam vertices
    // can duplicate bitwise, so one conservative weld pass is legitimate.
    let report = manifold_core::check(&built_mesh.positions, &built_mesh.indices);
    let (positions, indices, output_report) = if report.is_manifold() {
        (built_mesh.positions, built_mesh.indices, report)
    } else {
        let out = manifold_core::repair(&built_mesh.positions, &built_mesh.indices, options.weld_epsilon);
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
        kernel_report,
        operand_repairs,
        output_report,
    })
}

/// One operand of an n-ary boolean: flat `positions` and triangle `indices`.
pub type Operand<'a> = (&'a [f64], &'a [usize]);

/// Result of [`boolean_manifold_many`]: the final mesh plus totals over every
/// pairwise step.
#[derive(Debug, Clone)]
pub struct ManyOutcome {
    pub outcome: BooleanOutcome,
    /// Pre-repair of each input operand, in input order.
    pub operand_repairs: Vec<Option<RepairReport>>,
    /// Kernel work summed over all pairwise steps.
    pub total_work: usize,
    /// Number of pairwise kernel steps evaluated.
    pub steps: usize,
}

/// N-ary boolean folded left to right: union of all, intersection of all, or
/// the first operand minus every other one. Every operand is checked (and
/// repaired once, if needed) before any boolean runs; `max_work` bounds each
/// step. `Xor` folds as an odd-parity symmetric difference. A single operand
/// is returned after admission; an empty list is invalid input.
pub fn boolean_manifold_many(
    operands: &[Operand<'_>],
    op: BooleanOp,
    options: BooleanOptions,
) -> Result<ManyOutcome, BooleanError> {
    if operands.is_empty() {
        return Err(invalid_options("N-ary boolean needs at least one operand"));
    }
    let mut admitted = Vec::with_capacity(operands.len());
    for (slot, (positions, indices)) in operands.iter().enumerate() {
        admitted.push(admit(slot, positions, indices, options.weld_epsilon)?);
    }
    let operand_repairs: Vec<_> = admitted.iter().map(|a| a.repair.clone()).collect();
    let mut iter = admitted.into_iter();
    let first = iter.next().expect("non-empty");
    let mut outcome = BooleanOutcome {
        output_report: manifold_core::check(&first.positions, &first.indices),
        positions: first.positions,
        indices: first.indices,
        kernel_report: boolean::BooleanReport {
            operation: op.kernel().unwrap_or(boolean::Operation::Difference),
            tolerance_mm: 0.0,
            work: 0,
            fragments: 0,
            input_triangles: [0, 0],
        },
        operand_repairs: [None, None],
    };
    let (mut total_work, mut steps) = (0, 0);
    for next in iter {
        let step = boolean_manifold_with_options(&outcome.positions, &outcome.indices, &next.positions, &next.indices, op, options)?;
        total_work += step.kernel_report.work;
        steps += 1;
        outcome = step;
    }
    Ok(ManyOutcome { outcome, operand_repairs, total_work, steps })
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
        assert!((out.volume() - 1.875).abs() < 1e-9);
        assert!(out.surface_area() > 6.0);
        let com = out.center_of_mass();
        assert!((com[0] - 0.75).abs() < 1e-9);
        assert!((com[1] - 0.75).abs() < 1e-9);
        assert!((com[2] - 0.75).abs() < 1e-9);
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

    #[test]
    fn explicit_tolerances_reach_bsp_and_invalid_values_are_rejected() {
        let (ap, ai) = cube([0.0, 0.0, 0.0], 10.0);
        let (bp, bi) = cube([5.0, 0.0, 0.0], 10.0);
        let out = boolean_manifold_with_options(
            &ap, &ai, &bp, &bi, BooleanOp::Union,
            BooleanOptions { weld_epsilon: 1e-4, relative_tolerance: 1e-7, ..BooleanOptions::default() },
        ).unwrap();
        assert!((out.kernel_report.tolerance_mm - 1.5e-6).abs() < 1e-12);
        for options in [
            BooleanOptions { weld_epsilon: f64::NAN, ..BooleanOptions::default() },
            BooleanOptions { weld_epsilon: 1.1, ..BooleanOptions::default() },
            BooleanOptions { relative_tolerance: 0.0, ..BooleanOptions::default() },
            BooleanOptions { relative_tolerance: 1e-4, ..BooleanOptions::default() },
        ] {
            let error = boolean_manifold_with_options(&ap, &ai, &bp, &bi, BooleanOp::Union, options).unwrap_err();
            assert!(matches!(error, BooleanError::Kernel(ref e) if e.code == "GEOMETRY_INVALID_INPUT"));
        }
    }

    fn volume_of(out: &BooleanOutcome) -> f64 {
        out.volume()
    }

    #[test]
    fn xor_of_nested_cubes_is_a_hollow_shell() {
        let (ap, ai) = cube([0.0; 3], 4.0);
        let (bp, bi) = cube([1.0; 3], 2.0);
        let out = boolean_manifold(&ap, &ai, &bp, &bi, BooleanOp::Xor).unwrap();
        assert!(out.output_report.is_manifold());
        assert!((volume_of(&out) - 56.0).abs() < 1e-9);
    }

    #[test]
    fn xor_of_disjoint_cubes_keeps_both() {
        let (ap, ai) = cube([0.0; 3], 1.0);
        let (bp, bi) = cube([3.0, 0.0, 0.0], 1.0);
        let out = boolean_manifold(&ap, &ai, &bp, &bi, BooleanOp::Xor).unwrap();
        assert!((volume_of(&out) - 2.0).abs() < 1e-12);
    }

    #[test]
    fn xor_of_identical_cubes_is_empty() {
        let (ap, ai) = cube([0.0; 3], 1.0);
        let out = boolean_manifold(&ap, &ai, &ap, &ai, BooleanOp::Xor).unwrap();
        assert!(out.indices.is_empty());
    }

    #[test]
    fn xor_of_crossing_cubes_is_refused_not_returned_non_manifold() {
        let (ap, ai) = cube([0.0; 3], 2.0);
        let (bp, bi) = cube([1.0; 3], 2.0);
        match boolean_manifold(&ap, &ai, &bp, &bi, BooleanOp::Xor) {
            Ok(out) => {
                assert!(out.output_report.is_manifold());
                assert!((volume_of(&out) - 14.0).abs() < 1e-9);
            }
            Err(BooleanError::NonManifoldOutput { .. }) => {}
            Err(BooleanError::Kernel(e)) if e.code != "GEOMETRY_INVALID_INPUT" || e.message.contains("not a valid solid") => {}
            Err(e) => panic!("unexpected error {e}"),
        }
    }

    #[test]
    fn work_budget_is_validated_and_enforced() {
        let (ap, ai) = cube([0.0; 3], 2.0);
        let (bp, bi) = cube([1.0; 3], 2.0);
        for max_work in [0, MAX_WORK_BUDGET + 1] {
            let options = BooleanOptions { max_work, ..BooleanOptions::default() };
            assert!(boolean_manifold_with_options(&ap, &ai, &bp, &bi, BooleanOp::Union, options).is_err());
        }
        let options = BooleanOptions { max_work: 1, ..BooleanOptions::default() };
        match boolean_manifold_with_options(&ap, &ai, &bp, &bi, BooleanOp::Union, options) {
            Err(BooleanError::Kernel(e)) => assert_ne!(e.code, "GEOMETRY_INVALID_INPUT"),
            other => panic!("tiny work budget must exhaust the kernel, got {other:?}"),
        }
    }

    #[test]
    fn n_ary_union_intersection_and_difference_fold_all_operands() {
        let cubes: Vec<_> = (0..4).map(|i| cube([i as f64 * 1.5, 0.0, 0.0], 2.0)).collect();
        let operands: Vec<Operand<'_>> = cubes.iter().map(|(p, i)| (p.as_slice(), i.as_slice())).collect();
        let union = boolean_manifold_many(&operands, BooleanOp::Union, BooleanOptions::default()).unwrap();
        assert_eq!(union.steps, 3);
        assert!(union.total_work > 0);
        assert!((union.outcome.volume() - 2.0 * 2.0 * 6.5).abs() < 1e-9);
        let common = boolean_manifold_many(&operands[..2], BooleanOp::Intersection, BooleanOptions::default()).unwrap();
        assert!((common.outcome.volume() - 0.5 * 4.0).abs() < 1e-9);
        let (bp, bi) = cube([-1.0; 3], 10.0);
        let mut cut: Vec<Operand<'_>> = vec![(bp.as_slice(), bi.as_slice())];
        cut.extend_from_slice(&operands);
        let diff = boolean_manifold_many(&cut, BooleanOp::Difference, BooleanOptions::default()).unwrap();
        assert!((diff.outcome.volume() - (1000.0 - 26.0)).abs() < 1e-9);
        assert_eq!(diff.operand_repairs.len(), 5);
        assert!(boolean_manifold_many(&[], BooleanOp::Union, BooleanOptions::default()).is_err());
        let single = boolean_manifold_many(&operands[..1], BooleanOp::Union, BooleanOptions::default()).unwrap();
        assert_eq!(single.steps, 0);
    }

    #[test]
    fn inside_out_and_partly_flipped_operands_are_reoriented() {
        let (ap, ai) = cube([0.0; 3], 2.0);
        let (bp, bi) = cube([1.0; 3], 2.0);
        let mut inverted = ai.clone();
        inverted.chunks_mut(3).for_each(|t| t.swap(1, 2));
        let mut flipped = ai.clone();
        flipped[..6].chunks_mut(3).for_each(|t| t.swap(1, 2));
        for indices in [inverted, flipped] {
            let out = boolean_manifold(&ap, &indices, &bp, &bi, BooleanOp::Union).unwrap();
            assert!(out.operand_repairs[0].is_some());
            assert!((out.volume() - 15.0).abs() < 1e-9);
        }
    }
}
