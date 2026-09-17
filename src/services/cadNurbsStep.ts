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

export interface DirectStepV3Report extends StepIdentityReport {
  ignoredEntities: string[]
  instanceCount: number
  reachableCount: number
}

export interface DirectStepV3Export extends AnalyticStepExport {
  identity: StepIdentityReport
  ignoredEntities: string[]
  instanceCount: number
  reachableCount: number
}

export interface DirectStepV3Import extends AnalyticStepImport {
  identity: StepIdentityReport
  ignoredEntities: string[]
  instanceCount: number
  reachableCount: number
}

/** Direct bounded `step-interchange/3` export; never constructor/AABB recognition. */
export function exportDirectStepV3(model: NurbsBrep): DirectStepV3Export {
  return callGeometryRust('brep_nurbs_export_step_v3', { model })
}

/** Direct bounded `step-interchange/3` topology import with explicit identity reporting. */
export function importDirectStepV3(text: string): DirectStepV3Import {
  if (/\b(FACETED_BREP|TESSELLATED_|CSG_SOLID|OPEN_SHELL)\b/i.test(text)) {
    throw new Error('step-interchange/3 refuses faceted, tessellated, CSG, and open-shell roots')
  }
  return callGeometryRust('brep_nurbs_import_step_v3', { text })
}

export interface DirectStepV4Certificate extends Omit<AnalyticStepCertificate,'capability'> {
  capability:'step-interchange/4'
}
export interface DirectStepV4Export extends Omit<DirectStepV3Export,'certificate'> {
  certificate:DirectStepV4Certificate
}
export interface DirectStepV4Import extends Omit<DirectStepV3Import,'certificate'> {
  certificate:DirectStepV4Certificate
}

/** Direct `/4` successor export; retains `/3` graph identity and finite limits. */
export const exportDirectStepV4=(model:NurbsBrep):DirectStepV4Export=>
  callGeometryRust('brep_nurbs_export_step_v4',{model})

/** `/4` additionally admits exact endpoint point selectors and finite rational analytic carriers. */
export function importDirectStepV4(text:string):DirectStepV4Import {
  if (/\b(FACETED_BREP|TESSELLATED_|CSG_SOLID|OPEN_SHELL|EXTERNALLY_DEFINED_)\b/i.test(text)) {
    throw new Error('step-interchange/4 refuses faceted, tessellated, CSG, open-shell, and external-reference roots')
  }
  return callGeometryRust('brep_nurbs_import_step_v4',{text})
}
