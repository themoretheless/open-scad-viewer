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

export interface DirectStepV5Certificate extends Omit<AnalyticStepCertificate,'capability'> {
  capability:'step-interchange/5'
}
export interface DirectStepV5Export extends Omit<DirectStepV3Export,'certificate'> {
  certificate:DirectStepV5Certificate
  metadataLoss:string[]
}
export interface DirectStepV5Import extends Omit<DirectStepV3Import,'certificate'> {
  certificate:DirectStepV5Certificate
  metadataLoss:string[]
}

/** Standards-conformant AP242 retained B-rep successor. */
export const exportDirectStepV5=(model:NurbsBrep):DirectStepV5Export=>
  callGeometryRust('brep_nurbs_export_step_v5',{model})

/** Strict selected-product AP242 import; never falls back to orphan units or bodies. */
export function importDirectStepV5(text:string):DirectStepV5Import {
  if (/\b(FACETED_BREP|TESSELLATED_|CSG_SOLID|OPEN_SHELL|EXTERNALLY_DEFINED_)\b/i.test(text)) {
    throw new Error('step-interchange/5 refuses faceted, tessellated, CSG, open-shell, and external-reference roots')
  }
  return callGeometryRust('brep_nurbs_import_step_v5',{text})
}

export interface DirectStepV6Certificate extends Omit<AnalyticStepCertificate,'capability'> {
  capability:'step-interchange/6'
}
export interface DirectStepV6Report {
  definitionIdentities:string[]
  occurrenceIdentities:string[]
  productHierarchy:string[]
  externalReferences:string[]
}
export interface DirectStepV6Export extends Omit<DirectStepV5Export,'certificate'>,DirectStepV6Report {
  certificate:DirectStepV6Certificate
}
export interface DirectStepV6Import extends Omit<DirectStepV5Import,'certificate'>,DirectStepV6Report {
  certificate:DirectStepV6Certificate
}

export const exportDirectStepV6=(model:NurbsBrep):DirectStepV6Export=>
  callGeometryRust('brep_nurbs_export_step_v6',{model})

/** AP242 pole-safe, assembly-aware affine successor. External documents must be resolved explicitly first. */
export function importDirectStepV6(text:string):DirectStepV6Import {
  if(/\b(FACETED_BREP|TESSELLATED_|CSG_SOLID|OPEN_SHELL)\b/i.test(text)){
    throw Error('step-interchange/6 refuses faceted, tessellated, CSG, and open-shell roots')
  }
  return callGeometryRust('brep_nurbs_import_step_v6',{text})
}

export interface DirectStepV7Certificate extends Omit<AnalyticStepCertificate,'capability'> {
  capability:'step-interchange/7'
}
export interface DirectStepV7Export extends Omit<DirectStepV6Export,'certificate'> {
  certificate:DirectStepV7Certificate
}
export interface DirectStepV7Import extends Omit<DirectStepV6Import,'certificate'> {
  certificate:DirectStepV7Certificate
}

export const exportDirectStepV7=(model:NurbsBrep):DirectStepV7Export=>
  callGeometryRust('brep_nurbs_export_step_v7',{model})

/** `/7` keeps the native bounded direct graph separate from bundle validation and composition. */
export function importDirectStepV7(text:string):DirectStepV7Import {
  if(/\b(FACETED_BREP|TESSELLATED_|CSG_SOLID|OPEN_SHELL)\b/i.test(text)){
    throw Error('step-interchange/7 refuses faceted, tessellated, CSG, and open-shell roots')
  }
  return callGeometryRust('brep_nurbs_import_step_v7',{text})
}

export interface DirectStepV8RegularityEvidence {
  carrier:'cylinder'|'cone'|'sphere'|'torus'
  parameterU:[number,number]
  parameterV:[number,number]
  liftedPeriods:[number,number]
  denominatorLowerBound:number
  jacobianLowerBound:number
  collapsedBoundaries:string[]
  regularOpenDomain:boolean
  identity:string
}
export interface DirectStepV8Certificate {
  capability:'step-interchange/8'
  complete:true
  regularity:DirectStepV8RegularityEvidence[]
  senseLayers:string[]
  coupledSenseCases:128
  notes:string[]
}
export interface DirectStepV8Export extends Omit<DirectStepV7Export,'certificate'> {
  certificate:DirectStepV8Certificate
}
export interface DirectStepV8Import extends Omit<DirectStepV7Import,'certificate'> {
  certificate:DirectStepV8Certificate
}
export const exportDirectStepV8=(model:NurbsBrep):DirectStepV8Export=>
  callGeometryRust('brep_nurbs_export_step_v8',{model})
export function importDirectStepV8(text:string):DirectStepV8Import {
  if(/\b(FACETED_BREP|TESSELLATED_|CSG_SOLID|OPEN_SHELL)\b/i.test(text)){
    throw Error('step-interchange/8 refuses faceted, tessellated, CSG, and open-shell roots')
  }
  return callGeometryRust('brep_nurbs_import_step_v8',{text})
}

export interface DirectStepV9Certificate extends Omit<DirectStepV8Certificate,'capability'> {
  capability:'step-interchange/9'
}
export interface DirectStepV9Export extends Omit<DirectStepV8Export,'certificate'> {
  certificate:DirectStepV9Certificate
}
export interface DirectStepV9Import extends Omit<DirectStepV8Import,'certificate'> {
  certificate:DirectStepV9Certificate
}
/** Append-only retained geometry-product closure. */
export const exportDirectStepV9=(model:NurbsBrep):DirectStepV9Export=>
  callGeometryRust('brep_nurbs_export_step_v9',{model})
export function importDirectStepV9(text:string):DirectStepV9Import {
  if(/\b(FACETED_BREP|TESSELLATED_|CSG_SOLID|CONSTRUCTIVE_SOLID_GEOMETRY_REPRESENTATION)\b/i.test(text)){
    throw Error('step-interchange/9 direct B-rep route refuses faceted, tessellated, and procedural/CSG product roots')
  }
  return callGeometryRust('brep_nurbs_import_step_v9',{text})
}

export interface DirectStepV10Document {
  source:string
  graphIdentity:string
  definitionIdentities:string[]
  occurrenceIdentities:string[]
  productHierarchy:string[]
  operatorIdentities:string[]
  metadataLoss:string[]
}
export interface DirectStepV10Import {
  model:NurbsBrep
  document:DirectStepV10Document
  certificate:{capability:'step-interchange/10';complete:true;notes:string[]}
}
/** `/10` keeps the Part 21 relationship graph authoritative; `model` is an editing/preview projection. */
export const importDirectStepV10=(text:string):DirectStepV10Import=>
  callGeometryRust('brep_nurbs_import_step_v10',{text})
export const exportDirectStepV10=(document:DirectStepV10Document):{text:string;certificate:DirectStepV10Import['certificate']}=>
  callGeometryRust('brep_nurbs_export_step_v10',{source:document.source,graphIdentity:document.graphIdentity})
