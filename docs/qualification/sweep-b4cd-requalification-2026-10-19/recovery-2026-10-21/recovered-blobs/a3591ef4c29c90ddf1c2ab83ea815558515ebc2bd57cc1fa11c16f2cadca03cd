import {importFacetedStep,importTessellatedStepV10,type StepTessellatedPresentation} from './cadStep'
import {exportDirectStepV9,exportDirectStepV10,importDirectStepV10,type DirectStepV10Document} from './cadNurbsStep'
import {uncheckedStepConformance,type StepSemanticConformance} from './cadStepConformance'
import {tessellateNurbsBrep,type NurbsBrep} from './geometry/brep'
import {inspectNurbsBrep} from './geometry/brep'
import type {DirectBody} from './directModeling'
import {storageGet,storageRemove,storageSet} from './safeStorage'

export type StepRoute='retained-ap242-brep'|'faceted-brep'|'tessellated-shape'|'semantic-csg'|'unsupported-product'
export interface StepCapabilityReport {
  route:StepRoute
  retained:boolean
  refusalBoundary:string[]
  identityLoss:string[]
  metadataLoss:string[]
  definitionIdentities:string[]
  occurrenceIdentities:string[]
  productHierarchy:string[]
  externalReferences:string[]
  semanticConformance:StepSemanticConformance
}
export interface RoutedStepImport {
  bodies:DirectBody[]
  retainedModel?:NurbsBrep
  retainedDocument?:DirectStepV10Document
  tessellatedPresentation?:StepTessellatedPresentation[]
  report:StepCapabilityReport
}
const RETAINED_STEP_KEY='open-scad-viewer.retained-step-v1'
export const MAX_RETAINED_STEP_STORE_BYTES=16*1024*1024
interface RetainedStepRecord {version:1|2;model:NurbsBrep;document?:DirectStepV10Document;preview:{positions:number[];indices:number[]};report:StepCapabilityReport}
const utf8Bytes=(text:string)=>new TextEncoder().encode(text).byteLength
export const retainedStepStoreAcceptsEncodedBytes=(text:string)=>
  utf8Bytes(text)<=MAX_RETAINED_STEP_STORE_BYTES
const validRecord=(value:unknown):value is RetainedStepRecord=>{
  if(!value||typeof value!=='object'||![1,2].includes((value as RetainedStepRecord).version))return false
  const record=value as RetainedStepRecord
  try{
    inspectNurbsBrep(record.model)
    return Array.isArray(record.preview?.positions)&&Array.isArray(record.preview?.indices)
      &&record.preview.positions.length<=150_000&&record.preview.indices.length<=150_000
  }catch{return false}
}
const decodeRecord=(text:string|null):RetainedStepRecord|undefined=>{
  if(!text||!retainedStepStoreAcceptsEncodedBytes(text))return
  try{
    const value=JSON.parse(text) as RetainedStepRecord
    if(value?.report)for(const field of ['definitionIdentities','occurrenceIdentities','productHierarchy','externalReferences'] as const)value.report[field]??=[]
    if(value?.report)value.report.semanticConformance=uncheckedStepConformance()
    return validRecord(value)?value:undefined
  }catch{return}
}

export function inspectStepRoute(text:string):StepRoute {
  if(/\b(TESSELLATED_SHAPE_REPRESENTATION|TESSELLATED_SHELL|TESSELLATED_SOLID)\s*\(/i.test(text))
    return 'tessellated-shape'
  if(/\b(FACETED_BREP|FACETED_BREP_SHAPE_REPRESENTATION)\s*\(/i.test(text)
    && !/\b(ADVANCED_FACE|MANIFOLD_SOLID_BREP|BREP_WITH_VOIDS)\s*\(/i.test(text)) return 'faceted-brep'
  if(/\b(CSG_SOLID|CONSTRUCTIVE_SOLID_GEOMETRY_REPRESENTATION|BOOLEAN_RESULT|BLOCK|RIGHT_CIRCULAR_CYLINDER)\s*\(/i.test(text))
    return 'semantic-csg'
  if(!/\b(ADVANCED_BREP_SHAPE_REPRESENTATION|MANIFOLD_SURFACE_SHAPE_REPRESENTATION)\s*\(/i.test(text))
    return 'unsupported-product'
  return 'retained-ap242-brep'
}

/** Content-aware workbench import. The retained model remains authoritative; the mesh is preview-only. */
export function importStepForWorkbench(text:string):RoutedStepImport {
  const route=inspectStepRoute(text)
  if(route==='faceted-brep') return {
    bodies:importFacetedStep(text),
    report:{route,retained:false,refusalBoundary:['analytic surfaces','void shells','assemblies'],identityLoss:['all STEP topology identity'],metadataLoss:['all product and presentation metadata'],
      definitionIdentities:[],occurrenceIdentities:[],productHierarchy:[],externalReferences:[],
      semanticConformance:uncheckedStepConformance()},
  }
  if(route==='tessellated-shape')return {
    ...(()=>{const imported=importTessellatedStepV10(text);return {bodies:imported.bodies,tessellatedPresentation:imported.presentation}})(),
    report:{route,retained:false,refusalBoundary:['analytic B-rep','procedural/CSG evaluation'],
      identityLoss:[],metadataLoss:[],
      definitionIdentities:[],occurrenceIdentities:[],productHierarchy:[],externalReferences:[],
      semanticConformance:uncheckedStepConformance()},
  }
  if(route==='semantic-csg')throw Error(
    'STEP product route semantic-csg is recognized, but this procedural entity set has no explicit semantic evaluator; no mesh relabeling was performed.',
  )
  if(route==='unsupported-product')throw Error(
    'STEP product root is outside retained B-rep, faceted B-rep, tessellated shape, and recognized semantic CSG routes.',
  )
  const imported=importDirectStepV10(text)
  if(/\bDOCUMENT_FILE\s*\(/i.test(text)){
    throw Error('Retained STEP import contains external references; resolve them through an explicitly authorized resolver before workbench import.')
  }
  const preview=tessellateNurbsBrep(imported.model,8)
  return {
    retainedModel:imported.model,
    bodies:[{id:crypto.randomUUID(),name:'Retained AP242 STEP',brep:imported.model,mesh:{positions:preview.positions,indices:preview.indices}}],
    report:{
      route,retained:true,
      refusalBoundary:['unresolved/untrusted external references','singular transforms','assembly graph beyond configured budgets'],
      identityLoss:[],
      metadataLoss:imported.document.metadataLoss,
      definitionIdentities:imported.document.definitionIdentities,
      occurrenceIdentities:imported.document.occurrenceIdentities,
      productHierarchy:imported.document.productHierarchy,
      externalReferences:[],
      semanticConformance:uncheckedStepConformance(),
    },retainedDocument:imported.document,
  }
}

export function exportRetainedStepForWorkbench(model:NurbsBrep,document?:DirectStepV10Document){
  return document?exportDirectStepV10(document):exportDirectStepV9(model)
}

let retainedRecord=decodeRecord(storageGet(RETAINED_STEP_KEY))
export const retainedStepSession={
  get:()=>retainedRecord?.model,
  body:(candidate:DirectBody):DirectBody=>{
    const record=retainedRecord
    if(!record)return candidate
    const same=(a:ArrayLike<number>,b:ArrayLike<number>)=>a.length===b.length&&Array.from(a).every((value,index)=>Object.is(value,b[index]))
    return same(candidate.mesh.positions,record.preview.positions)&&same(candidate.mesh.indices,record.preview.indices)
      ?{...candidate,name:'Retained AP242 STEP',brep:record.model}:candidate
  },
  set:(model:NurbsBrep|undefined,report?:StepCapabilityReport,document?:DirectStepV10Document)=>{
    if(!model){retainedRecord=undefined;storageRemove(RETAINED_STEP_KEY);return false}
    inspectNurbsBrep(model)
    const preview=tessellateNurbsBrep(model,8)
    const record:RetainedStepRecord={version:2,model,document:document??retainedRecord?.document,preview:{positions:Array.from(preview.positions),indices:Array.from(preview.indices)},
      report:report??retainedRecord?.report??{route:'retained-ap242-brep',retained:true,refusalBoundary:[],identityLoss:[],metadataLoss:[],
        definitionIdentities:[],occurrenceIdentities:[],productHierarchy:[],externalReferences:[],
        semanticConformance:uncheckedStepConformance()}}
    const encoded=JSON.stringify(record)
    if(!retainedStepStoreAcceptsEncodedBytes(encoded))throw Error('Retained STEP document exceeds 16 MiB durable-store boundary.')
    if(!storageSet(RETAINED_STEP_KEY,encoded))throw Error('Retained STEP could not be saved durably.')
    retainedRecord=record;return true
  },
  reload:()=>{retainedRecord=decodeRecord(storageGet(RETAINED_STEP_KEY));return retainedRecord?.model},
  report:()=>retainedRecord?.report,
  document:()=>retainedRecord?.document,
  clear:()=>{retainedRecord=undefined;return storageRemove(RETAINED_STEP_KEY)},
}
