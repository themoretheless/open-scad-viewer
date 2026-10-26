//! Iterative, reversible simplification of nearly planar faces (checklist 1041).
//!
//! Each accepted candidate is snapped independently through 876/874. The next
//! face is considered only after replacement validation, solid audit, volume
//! and shared-vertex checks. Cylinder targets are explicitly outside this
//! iteration: `replace_face` still admits only bilinear plane targets and its
//! cylinder re-trim requires a broader non-planar-neighbor contract.

use super::*;
use crate::TopoId;
use crate::analysis::surface_classify::classify_surface;
use nurbs_core::surface::Surface;
use std::sync::Arc;

pub const SIMPLIFY_INVALID: &str = "BREP_SIMPLIFY_INVALID";
pub const SIMPLIFY_UNKNOWN_FACE: &str = "BREP_SIMPLIFY_UNKNOWN_FACE";
pub const SIMPLIFY_VOLUME_DRIFT: &str = "BREP_SIMPLIFY_VOLUME_DRIFT";
pub const SIMPLIFY_NEIGHBOR_DRIFT: &str = "BREP_SIMPLIFY_NEIGHBOR_DRIFT";
pub const SIMPLIFY_CYLINDER_UNSUPPORTED: &str = "BREP_SIMPLIFY_CYLINDER_UNSUPPORTED";
const BUDGET_EXHAUSTED: &str = "NURBS_RESOURCE_LIMIT";

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SimplifyOptions {
    /// Maximum sampled deviation admitted by the analytic fit.
    pub tolerance_mm: f64,
    /// Per-step and cumulative |ΔV/V| ceiling.
    pub max_relative_volume_delta: f64,
    pub planes: bool,
    /// Reserved for the cylinder-target iteration. `true` currently returns
    /// `BREP_SIMPLIFY_CYLINDER_UNSUPPORTED` rather than silently ignoring it.
    pub cylinders: bool,
}

impl Default for SimplifyOptions {
    fn default() -> Self {
        Self {
            tolerance_mm: 1e-3,
            max_relative_volume_delta: 1e-6,
            planes: true,
            cylinders: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimplificationSkip {
    pub face_id: TopoId,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimplifyReport {
    pub analytic_faces_before: usize,
    pub analytic_faces_after: usize,
    pub total_faces: usize,
    pub plane_replacements: usize,
    pub cylinder_replacements: usize,
    pub volume_before_mm3: f64,
    pub volume_after_mm3: f64,
    pub abs_relative_delta_volume: f64,
    pub max_deviation_mm: f64,
    pub max_boundary_deviation_mm: f64,
    pub max_neighbor_vertex_displacement_mm: f64,
    pub skipped: Vec<SimplificationSkip>,
}

/// One reversible replacement. The public original surface is accompanied by
/// the complete pre-step state so rollback also restores compatible pcurves,
/// edge curves and vertices, not merely an isolated surface definition.
#[derive(Clone, Debug)]
pub struct SimplificationRecord {
    pub face_id: TopoId,
    pub face_index: usize,
    pub original_surface: Surface,
    pub target: SurfaceClass,
    pub max_deviation_mm: f64,
    pub max_boundary_deviation_mm: f64,
    pub max_neighbor_vertex_displacement_mm: f64,
    before: Arc<Model>,
}

#[derive(Clone, Debug, Default)]
pub struct SimplificationLog {
    pub replacements: Vec<SimplificationRecord>,
}

impl SimplificationLog {
    pub fn len(&self) -> usize {
        self.replacements.len()
    }

    pub fn is_empty(&self) -> bool {
        self.replacements.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct SimplifyRun {
    pub model: Model,
    pub report: SimplifyReport,
    pub log: SimplificationLog,
}

fn relative_delta(before: f64, after: f64) -> Result<f64> {
    if !before.is_finite() || !after.is_finite() || before.abs() <= f64::MIN_POSITIVE {
        return Err(error(
            SIMPLIFY_INVALID,
            "Simplification needs finite, non-zero signed volume",
        ));
    }
    Ok((after - before).abs() / before.abs())
}

fn analytic_face_count(model: &Model, budget: &Budget) -> Result<usize> {
    let mut count = 0;
    for face in &model.faces {
        let class = classify_surface(&face.surface, model.tolerance_mm.max(1e-7), budget)?;
        count += usize::from(class.class != SurfaceClass::Freeform);
    }
    Ok(count)
}

fn boundary_vertices(model: &Model, face: usize) -> BTreeSet<usize> {
    let mut vertices = BTreeSet::new();
    for &ring in std::iter::once(&model.faces[face].outer).chain(&model.faces[face].holes) {
        for coedge in &model.loops[ring].coedges {
            vertices.extend(model.edges[coedge.edge].vertices);
        }
    }
    vertices
}

fn neighbor_vertex_displacement(before: &Model, after: &Model, face: usize) -> f64 {
    boundary_vertices(before, face)
        .into_iter()
        .map(|vertex| dist(before.vertices[vertex].point, after.vertices[vertex].point))
        .fold(0., f64::max)
}

/// Restore the valid state immediately before the replacement of `face`.
/// Later replacements are rolled back with it, giving deterministic stepwise
/// undo while preserving every earlier accepted simplification.
pub fn rollback_simplification(run: &SimplifyRun, face: TopoId) -> Result<Model> {
    let record = run
        .log
        .replacements
        .iter()
        .find(|record| record.face_id == face)
        .ok_or_else(|| {
            error(
                SIMPLIFY_UNKNOWN_FACE,
                format!("No simplification record for topology {face}"),
            )
        })?;
    let restored = record.before.as_ref().clone();
    restored.validate()?;
    crate::solid_audit::audit_solid(&restored)?;
    Ok(restored)
}

/// Simplify nearly planar freeform faces one at a time. Failed candidates are
/// reported and not committed; resource exhaustion and invalid input remain
/// hard errors. Plane-only scope is explicit through `SimplifyOptions`.
pub fn simplify_to_analytic(
    model: &Model,
    options: SimplifyOptions,
    budget: &Budget,
) -> Result<SimplifyRun> {
    if !options.tolerance_mm.is_finite()
        || options.tolerance_mm <= 0.
        || !options.max_relative_volume_delta.is_finite()
        || options.max_relative_volume_delta < 0.
    {
        return Err(error(
            SIMPLIFY_INVALID,
            "Simplification tolerance must be positive and the volume ceiling finite and non-negative",
        ));
    }
    if options.cylinders {
        return Err(error(
            SIMPLIFY_CYLINDER_UNSUPPORTED,
            "Cylinder simplification is deferred: replace-face currently accepts only bilinear plane targets in the validated neighbor subset",
        ));
    }
    model.validate()?;
    crate::solid_audit::audit_solid(model)?;
    let analytic_faces_before = analytic_face_count(model, budget)?;
    let initial_volume = volume_of(model)?;
    let mut staged = model.clone();
    let mut staged_volume = initial_volume;
    let mut log = SimplificationLog::default();
    let mut skipped = Vec::new();
    let mut max_deviation_mm = 0f64;
    let mut max_boundary_deviation_mm = 0f64;
    let mut max_neighbor_vertex_displacement_mm = 0f64;
    let mut guard = budget.guard("simplify-to-analytic");

    for face in 0..model.faces.len() {
        guard.tick()?;
        let face_id = *model.1.faces.get(face).ok_or_else(|| {
            error(
                SIMPLIFY_INVALID,
                format!("No persistent id for face {face}"),
            )
        })?;
        let strict = classify_surface(
            &staged.faces[face].surface,
            staged.tolerance_mm.max(1e-7),
            budget,
        )?;
        if strict.class != SurfaceClass::Freeform {
            continue;
        }
        let candidate =
            classify_surface(&staged.faces[face].surface, options.tolerance_mm, budget)?;
        if candidate.class == SurfaceClass::Cylinder {
            skipped.push(SimplificationSkip {
                face_id,
                code: SIMPLIFY_CYLINDER_UNSUPPORTED.into(),
                message: "Near-cylinder replacement is outside the validated plane-only scope"
                    .into(),
            });
            continue;
        }
        if candidate.class != SurfaceClass::Plane || !options.planes {
            continue;
        }

        let before = staged.clone();
        let snapped = match snap_face_to_analytic(&before, face, options.tolerance_mm, budget) {
            Ok(report) => report,
            Err(failure) => {
                if failure.code == BUDGET_EXHAUSTED {
                    return Err(failure);
                }
                skipped.push(SimplificationSkip {
                    face_id,
                    code: failure.code.to_string(),
                    message: failure.message,
                });
                continue;
            }
        };
        let after = snapped.model;
        after.validate()?;
        crate::solid_audit::audit_solid(&after)?;
        let after_volume = volume_of(&after)?;
        let step_delta = relative_delta(staged_volume, after_volume)?;
        let cumulative_delta = relative_delta(initial_volume, after_volume)?;
        if step_delta > options.max_relative_volume_delta
            || cumulative_delta > options.max_relative_volume_delta
        {
            skipped.push(SimplificationSkip {
                face_id,
                code: SIMPLIFY_VOLUME_DRIFT.into(),
                message: format!(
                    "Relative volume delta {step_delta} step / {cumulative_delta} cumulative exceeds {}",
                    options.max_relative_volume_delta
                ),
            });
            continue;
        }
        let neighbor_displacement = neighbor_vertex_displacement(&before, &after, face);
        let neighbor_surface_changed = before
            .faces
            .iter()
            .zip(&after.faces)
            .enumerate()
            .any(|(index, (old, new))| index != face && old.surface != new.surface);
        if neighbor_surface_changed || neighbor_displacement > before.tolerance_mm.max(1e-7) {
            skipped.push(SimplificationSkip {
                face_id,
                code: SIMPLIFY_NEIGHBOR_DRIFT.into(),
                message: format!(
                    "Shared vertex displacement {neighbor_displacement} or neighbor surface change exceeds the replacement contract"
                ),
            });
            continue;
        }

        max_deviation_mm = max_deviation_mm.max(snapped.max_deviation_mm);
        max_boundary_deviation_mm =
            max_boundary_deviation_mm.max(snapped.max_boundary_deviation_mm);
        max_neighbor_vertex_displacement_mm =
            max_neighbor_vertex_displacement_mm.max(neighbor_displacement);
        log.replacements.push(SimplificationRecord {
            face_id,
            face_index: face,
            original_surface: before.faces[face].surface.clone(),
            target: SurfaceClass::Plane,
            max_deviation_mm: snapped.max_deviation_mm,
            max_boundary_deviation_mm: snapped.max_boundary_deviation_mm,
            max_neighbor_vertex_displacement_mm: neighbor_displacement,
            before: Arc::new(before),
        });
        staged = after;
        staged_volume = after_volume;
        guard.check()?;
    }

    staged.validate()?;
    crate::solid_audit::audit_solid(&staged)?;
    let analytic_faces_after = analytic_face_count(&staged, budget)?;
    let abs_relative_delta_volume = relative_delta(initial_volume, staged_volume)?;
    Ok(SimplifyRun {
        model: staged,
        report: SimplifyReport {
            analytic_faces_before,
            analytic_faces_after,
            total_faces: model.faces.len(),
            plane_replacements: log.len(),
            cylinder_replacements: 0,
            volume_before_mm3: initial_volume,
            volume_after_mm3: staged_volume,
            abs_relative_delta_volume,
            max_deviation_mm,
            max_boundary_deviation_mm,
            max_neighbor_vertex_displacement_mm,
            skipped,
        },
        log,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn budget() -> Budget {
        Budget::new(30_000_000, 8, 120_000).unwrap()
    }

    fn noisy_cuboid(noisy_faces: &[usize]) -> Model {
        let mut model = crate::cuboid([0.; 3], [4., 3., 2.]).unwrap();
        for &face in noisy_faces {
            model.faces[face].surface =
                crate::nurbs_step_shared::as_uniform_bicubic(&model.faces[face].surface).unwrap();
            for (i, row) in model.faces[face]
                .surface
                .control_points
                .iter_mut()
                .enumerate()
            {
                for (j, point) in row.iter_mut().enumerate() {
                    if (1..=2).contains(&i) && (1..=2).contains(&j) {
                        let sign = if (i + j) % 2 == 0 { 1. } else { -1. };
                        point[2] += sign * 1e-5;
                    }
                }
            }
        }
        model.validate().unwrap();
        model
    }

    #[test]
    fn iterative_plane_simplification_preserves_volume_and_supports_step_rollback() {
        let model = noisy_cuboid(&[0, 1]);
        let run = simplify_to_analytic(&model, SimplifyOptions::default(), &budget()).unwrap();
        println!(
            "analytic faces: {} -> {}; |dV/V| = {:.17e}",
            run.report.analytic_faces_before,
            run.report.analytic_faces_after,
            run.report.abs_relative_delta_volume
        );
        assert_eq!(run.report.total_faces, 6);
        assert_eq!(run.report.analytic_faces_before, 4);
        assert_eq!(run.report.analytic_faces_after, 6, "{:#?}", run.report);
        assert!(run.report.analytic_faces_after > run.report.analytic_faces_before);
        assert_eq!(run.report.plane_replacements, 2);
        assert_eq!(run.report.cylinder_replacements, 0);
        assert_eq!(run.log.len(), 2);
        assert!(run.report.abs_relative_delta_volume < 1e-6);
        assert!(run.report.max_neighbor_vertex_displacement_mm <= model.tolerance_mm.max(1e-7));
        assert!(run.skipped_is_empty());

        for record in &run.log.replacements {
            let restored = rollback_simplification(&run, record.face_id).unwrap();
            restored.validate().unwrap();
            crate::solid_audit::audit_solid(&restored).unwrap();
            assert_eq!(restored.1.faces[record.face_index], record.face_id);
            assert_eq!(
                restored.faces[record.face_index].surface,
                record.original_surface
            );
        }
        let first = rollback_simplification(&run, run.log.replacements[0].face_id).unwrap();
        assert_eq!(first, model);
    }

    #[test]
    fn cylinder_request_has_explicit_typed_unsupported_scope() {
        let model = noisy_cuboid(&[0]);
        let options = SimplifyOptions {
            cylinders: true,
            ..SimplifyOptions::default()
        };
        let failure = simplify_to_analytic(&model, options, &budget()).unwrap_err();
        assert_eq!(failure.code, SIMPLIFY_CYLINDER_UNSUPPORTED);
    }

    impl SimplifyRun {
        fn skipped_is_empty(&self) -> bool {
            self.report.skipped.is_empty()
        }
    }
}
