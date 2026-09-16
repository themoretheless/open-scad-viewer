/**
 * Freeform NURBS STEP peer.
 * - `nurbs-step-bicubic-face/1` — untrimmed open face
 * - `nurbs-step-trimmed-bicubic/1` — trimmed open face (outer + hole)
 * - `nurbs-step-solid/1` — closed freeform cuboid / bump / cavity solid
 * Constructor analytic STEP stays in `cadAnalyticStep.ts`.
 * Faceted STEP stays in `cadStep.ts`.
 */
import { callGeometryRust } from './geometry/kernel'
import type { NurbsBrep } from './geometry/brep'
import type {
  AnalyticStepCertificate,
  AnalyticStepExport,
  AnalyticStepExportV2,
  AnalyticStepImport,
  AnalyticStepImportV2,
  StepIdentityReport,
} from './cadAnalyticStep'

export type {
  AnalyticStepCertificate,
  AnalyticStepExport,
  AnalyticStepExportV2,
  AnalyticStepImport,
  AnalyticStepImportV2,
  StepIdentityReport,
}

/** Export a bicubic open-face B-rep as B_SPLINE STEP (open shell). */
export function exportNurbsStepFreeform(model: NurbsBrep): AnalyticStepExport {
  return callGeometryRust('brep_nurbs_export_step_freeform', { model })
}

/**
 * Import freeform bicubic open-face STEP.
 * Refuses MANIFOLD_SOLID_BREP, FACETED-only, STL/OBJ, constructor graphs without B_SPLINE.
 */
export function importNurbsStepFreeform(text: string): AnalyticStepImport {
  if (/\bFACETED_BREP\s*\(/i.test(text) && !/\bADVANCED_FACE\s*\(/i.test(text)) {
    throw new Error(
      'Freeform NURBS STEP importer refuses FACETED_BREP; use importFacetedStep from cadStep.ts',
    )
  }
  if (/\bMANIFOLD_SOLID_BREP\s*\(/i.test(text) || /\bBREP_WITH_VOIDS\s*\(/i.test(text)) {
    throw new Error(
      'nurbs-step-bicubic-face/1 refuses MANIFOLD_SOLID_BREP; use importNurbsStepSolid or importAnalyticStep',
    )
  }
  return callGeometryRust('brep_nurbs_import_step_freeform', { text })
}

/** Export trimmed bicubic open face (FACE_OUTER_BOUND + FACE_BOUND + OPEN_SHELL). */
export function exportNurbsStepTrimmed(model: NurbsBrep): AnalyticStepExport {
  return callGeometryRust('brep_nurbs_export_step_trimmed', { model })
}

/** Import trimmed bicubic open-face STEP; refuses solids. */
export function importNurbsStepTrimmed(text: string): AnalyticStepImport {
  if (/\bMANIFOLD_SOLID_BREP\s*\(/i.test(text) || /\bBREP_WITH_VOIDS\s*\(/i.test(text)) {
    throw new Error(
      'nurbs-step-trimmed-bicubic/1 refuses MANIFOLD_SOLID_BREP; use importNurbsStepSolid',
    )
  }
  return callGeometryRust('brep_nurbs_import_step_trimmed', { text })
}

/** Export freeform closed solid (MANIFOLD_SOLID_BREP / BREP_WITH_VOIDS + B_SPLINE faces). */
export function exportNurbsStepSolid(model: NurbsBrep): AnalyticStepExport {
  return callGeometryRust('brep_nurbs_export_step_solid', { model })
}

/** Import freeform solid STEP (cuboid / bump / planar cavity). */
export function importNurbsStepSolid(text: string): AnalyticStepImport {
  if (/\bFACETED_BREP\s*\(/i.test(text) && !/\bADVANCED_FACE\s*\(/i.test(text)) {
    throw new Error(
      'Freeform NURBS solid importer refuses FACETED_BREP; use importFacetedStep from cadStep.ts',
    )
  }
  return callGeometryRust('brep_nurbs_import_step_solid', { text })
}

/** `step-interchange/2` freeform solid export; admits multiple bodies and one void per body. */
export function exportNurbsStepSolidV2(model: NurbsBrep): AnalyticStepExportV2 {
  return callGeometryRust('brep_nurbs_export_step_solid_v2', { model })
}

/** Strict successor import with SI, placement, pcurve and identity reporting. */
export function importNurbsStepSolidV2(text: string): AnalyticStepImportV2 {
  if (/\bFACETED_BREP\s*\(/i.test(text) && !/\bADVANCED_FACE\s*\(/i.test(text)) {
    throw new Error(
      'Freeform NURBS solid importer refuses FACETED_BREP; use importFacetedStep from cadStep.ts',
    )
  }
  return callGeometryRust('brep_nurbs_import_step_solid_v2', { text })
}
