/**
 * Analytic STEP AP214/AP242 peer for constructor B-rep solids.
 * Faceted STEP stays in `cadStep.ts` — do not mix the two importers.
 */
import { callGeometryRust } from './geometry/kernel'
import type { NurbsBrep } from './geometry/brep'

export interface AnalyticStepCertificate {
  capability: string
  complete: boolean
  notes: string[]
}

export interface AnalyticStepExport {
  text: string
  certificate: AnalyticStepCertificate
}

export interface AnalyticStepImport {
  model: NurbsBrep
  certificate: AnalyticStepCertificate
}

export interface StepIdentityReport {
  preserved: boolean
  source: 'internal-metadata' | 'external-step'
  preservedCount: number
  createdCount: number
  lostCount: number
}

export interface AnalyticStepExportV2 extends AnalyticStepExport {
  identity: StepIdentityReport
}

export interface AnalyticStepImportV2 extends AnalyticStepImport {
  identity: StepIdentityReport
}

/** Export an analytic constructor solid as linked ADVANCED_FACE STEP. */
export function exportAnalyticStep(model: NurbsBrep): AnalyticStepExport {
  return callGeometryRust('brep_nurbs_export_step', { model })
}

/**
 * Import analytic STEP (surfaces + AXIS2 + CIRCLE rings → constructors).
 * Refuses FACETED-only / STL / OBJ / incomplete graphs.
 */
export function importAnalyticStep(text: string): AnalyticStepImport {
  if (/\bFACETED_BREP\s*\(/i.test(text) && !/\bADVANCED_FACE\s*\(/i.test(text)) {
    throw new Error(
      'Analytic STEP importer refuses FACETED_BREP; use importFacetedStep from cadStep.ts',
    )
  }
  return callGeometryRust('brep_nurbs_import_step', { text })
}

/** Strict finite `step-interchange/2` export with SI context and TopoId metadata. */
export function exportAnalyticStepV2(model: NurbsBrep): AnalyticStepExportV2 {
  return callGeometryRust('brep_nurbs_export_step_v2', { model })
}

/**
 * Strict finite `step-interchange/2` import. Internal metadata preserves
 * opaque 128-bit IDs; metadata-free third-party input reports identity loss.
 */
export function importAnalyticStepV2(text: string): AnalyticStepImportV2 {
  if (/\bFACETED_BREP\s*\(/i.test(text) && !/\bADVANCED_FACE\s*\(/i.test(text)) {
    throw new Error(
      'Analytic STEP importer refuses FACETED_BREP; use importFacetedStep from cadStep.ts',
    )
  }
  return callGeometryRust('brep_nurbs_import_step_v2', { text })
}
