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
