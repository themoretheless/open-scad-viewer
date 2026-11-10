import {importFacetedStep,importTessellatedStepV10,type StepTessellatedPresentation} from './cadStep'
import {
  exportDirectStepV9,exportDirectStepV10,importDirectStepV10,importDirectStepScene,importDirectStepLod0,
  type DirectStepV10Document,type DirectStepSceneTree,type DirectStepDisplay,type DirectStepSceneProduct,
  type StepRgba,
} from './cadNurbsStep'
import {uncheckedStepConformance,type StepSemanticConformance} from './cadStepConformance'
import {tessellateNurbsBrep,type NurbsBrep} from './geometry/brep'
import {inspectNurbsBrep} from './geometry/brep'
import type {DirectBody} from '../services/directModeling'
import {storageGet,storageRemove,storageSet} from '../services/safeStorage'
import {stepLod0Cache,type CachedStepLod0Import} from '../services/stepLod0Cache'

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
  sceneTree?:DirectStepSceneTree
  display?:DirectStepDisplay
  /** XDE product records are the authoritative product subset of sceneTree. */
  xde?:DirectStepSceneProduct[]
  lod0CacheKey?:string
  lod0ShellCacheKeys?:string[]
}
export interface RoutedStepImport {
  bodies:DirectBody[]
  retainedModel?:NurbsBrep
  retainedDocument?:DirectStepV10Document
  tessellatedPresentation?:StepTessellatedPresentation[]
  sceneTree?:DirectStepSceneTree
  display?:DirectStepDisplay
  xde?:DirectStepSceneProduct[]
  lod0?:CachedStepLod0Import
  report:StepCapabilityReport
}
const RETAINED_STEP_KEY='open-scad-viewer.retained-step-v1'
export const MAX_RETAINED_STEP_STORE_BYTES=16*1024*1024
interface RetainedStepRecord {version:1|2;model:NurbsBrep;document?:DirectStepV10Document;preview:{positions:number[];indices:number[]};report:StepCapabilityReport}
const UTF8_ENCODER=new TextEncoder()
const utf8Bytes=(text:string)=>UTF8_ENCODER.encode(text).byteLength
export const retainedStepStoreAcceptsEncodedBytes=(text:string)=>
  utf8Bytes(text)<=MAX_RETAINED_STEP_STORE_BYTES
const validRecord=(value:unknown):value is RetainedStepRecord=>{
  if(!value||typeof value!=='object')return false
  const v=(value as RetainedStepRecord).version
  if(v!==1&&v!==2)return false
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

const TESSELLATED_STEP_RE=/\b(TESSELLATED_SHAPE_REPRESENTATION|TESSELLATED_SHELL|TESSELLATED_SOLID)\s*\(/i
const FACETED_STEP_RE=/\b(FACETED_BREP|FACETED_BREP_SHAPE_REPRESENTATION)\s*\(/i
const EXACT_BREP_STEP_RE=/\b(ADVANCED_FACE|MANIFOLD_SOLID_BREP|BREP_WITH_VOIDS)\s*\(/i
const CSG_STEP_RE=/\b(CSG_SOLID|CONSTRUCTIVE_SOLID_GEOMETRY_REPRESENTATION|BOOLEAN_RESULT|BLOCK|RIGHT_CIRCULAR_CYLINDER)\s*\(/i
const RETAINED_REP_STEP_RE=/\b(ADVANCED_BREP_SHAPE_REPRESENTATION|MANIFOLD_SURFACE_SHAPE_REPRESENTATION)\s*\(/i
const DOCUMENT_FILE_RE=/\bDOCUMENT_FILE\s*\(/i

export function inspectStepRoute(text:string):StepRoute {
  // AP242 may carry baked tessellation beside the authoritative exact B-rep.
  if(EXACT_BREP_STEP_RE.test(text)&&RETAINED_REP_STEP_RE.test(text))return 'retained-ap242-brep'
  if(TESSELLATED_STEP_RE.test(text))return 'tessellated-shape'
  if(FACETED_STEP_RE.test(text))return 'faceted-brep'
  if(CSG_STEP_RE.test(text))return 'semantic-csg'
  if(!RETAINED_REP_STEP_RE.test(text))return 'unsupported-product'
  return 'retained-ap242-brep'
}

const errorText=(error:unknown)=>error instanceof Error?error.message:String(error)
const unique=(values:string[])=>[...new Set(values)]
const sceneMetadataLoss=(scene:DirectStepSceneTree)=>[
  ...scene.unsupported.presentation.map(item=>`Unsupported STEP presentation ${item.code}${item.entity===null?'':` #${item.entity}`}: ${item.detail}`),
  ...scene.unsupported.xde.map(item=>`Unsupported STEP XDE ${item.code}${item.entity===null?'':` #${item.entity}`}: ${item.detail}`),
]
const lodMetadataLoss=(lod:CachedStepLod0Import)=>[
  ...lod.diagnostics.map(item=>`AP242 LOD-0 ${item.severity} ${item.code}: ${item.message}`),
  ...lod.schemaDiagnostics.map(item=>`AP242 schema ${item.severity} ${item.code}: ${item.message}`),
]
const combineLod0=(lod:CachedStepLod0Import)=>{
  if(!lod.lod0Shells.length)throw Error('AP242 LOD-0 contains no display shells.')
  const positions:number[]=[],indices:number[]=[]
  for(const shell of lod.lod0Shells){
    const offset=positions.length/3
    if(positions.length+shell.mesh.positions.length>300_000||indices.length+shell.mesh.indices.length>300_000)
      throw Error('AP242 LOD-0 exceeds the direct document mesh budget.')
    for(const position of shell.mesh.positions)positions.push(position)
    for(const index of shell.mesh.indices)indices.push(index+offset)
  }
  return {positions:Float64Array.from(positions),indices:Uint32Array.from(indices)}
}
const color=(rgba:StepRgba|null)=>{
  if(!rgba)return
  const channel=(value:number)=>Math.max(0,Math.min(255,Math.round(value))).toString(16).padStart(2,'0')
  return `#${channel(rgba[0])}${channel(rgba[1])}${channel(rgba[2])}`
}
const applySceneToBody=(body:DirectBody,scene:DirectStepSceneTree,display:DirectStepDisplay,lod?:CachedStepLod0Import)=>{
  const product=scene.products.length===1?scene.products[0]:undefined
  const name=product?.name.trim()||product?.shapeName?.trim()||(lod?.lod0Shells.length===1?lod.lod0Shells[0].name.trim():'')
  if(name)body.name=name.slice(0,100)
  const bodyColors=display.bodies.map(item=>item.rgba).filter((rgba):rgba is StepRgba=>!!rgba)
  const firstBodyColor=bodyColors[0]
  const unambiguousBodyColor=firstBodyColor&&bodyColors.every(rgba=>rgba.every((channel,index)=>channel===firstBodyColor[index]))?firstBodyColor:undefined
  const rgba=unambiguousBodyColor??product?.display.rgba??null
  const hex=color(rgba)
  if(hex)body.material={name:product?.material?.trim().slice(0,100)||'STEP product',color:hex,...(rgba&&rgba[3]<255?{opacity:rgba[3]/255}: {})}
  // Face display records remain in report.display; DirectBody/renderer material is per-body only.
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
  if(route==='tessellated-shape'){
    const imported=importTessellatedStepV10(text)
    return {bodies:imported.bodies,tessellatedPresentation:imported.presentation,
      report:{route,retained:false,refusalBoundary:['analytic B-rep','procedural/CSG evaluation'],
        identityLoss:[],metadataLoss:imported.metadataLoss,
        definitionIdentities:[],occurrenceIdentities:[],productHierarchy:[],externalReferences:[],
        semanticConformance:uncheckedStepConformance()}}
  }
  if(route==='semantic-csg')throw Error(
    'STEP product route semantic-csg is recognized, but this procedural entity set has no explicit semantic evaluator; no mesh relabeling was performed.',
  )
  if(route==='unsupported-product')throw Error(
    'STEP product root is outside retained B-rep, faceted B-rep, tessellated shape, and recognized semantic CSG routes.',
  )
  const imported=importDirectStepV10(text)
  if(DOCUMENT_FILE_RE.test(text)){
    throw Error('Retained STEP import contains external references; resolve them through an explicitly authorized resolver before workbench import.')
  }
  const metadataLoss=[...imported.document.metadataLoss]
  let sceneTree:DirectStepSceneTree|undefined,display:DirectStepDisplay|undefined,xde:DirectStepSceneProduct[]|undefined
  try{
    const scene=importDirectStepScene(text)
    sceneTree=scene.sceneTree;display=scene.display;xde=scene.sceneTree.products
    metadataLoss.push(...scene.report.metadataLoss,...sceneMetadataLoss(scene.sceneTree))
  }catch(error){
    metadataLoss.push(`STEP scene/XDE metadata unavailable: ${errorText(error)}`)
  }
  let lod0:CachedStepLod0Import|undefined,mesh:DirectBody['mesh']|undefined
  if(TESSELLATED_STEP_RE.test(text)){
    try{
      lod0=stepLod0Cache.getDocument(text)??stepLod0Cache.putDocument(text,importDirectStepLod0(text))
      metadataLoss.push(...lodMetadataLoss(lod0))
      mesh=combineLod0(lod0)
    }catch(error){
      metadataLoss.push(`AP242 LOD-0 display unavailable; exact B-rep was tessellated instead: ${errorText(error)}`)
    }
  }
  mesh??=(()=>{const preview=tessellateNurbsBrep(imported.model,8);return {positions:preview.positions,indices:preview.indices}})()
  const body:DirectBody={id:crypto.randomUUID(),name:'Retained AP242 STEP',brep:imported.model,mesh}
  if(sceneTree&&display)applySceneToBody(body,sceneTree,display,lod0)
  return {
    retainedModel:imported.model,
    bodies:[body],
    report:{
      route,retained:true,
      refusalBoundary:['unresolved/untrusted external references','singular transforms','assembly graph beyond configured budgets'],
      identityLoss:[],
      metadataLoss:unique(metadataLoss),
      definitionIdentities:imported.document.definitionIdentities,
      occurrenceIdentities:imported.document.occurrenceIdentities,
      productHierarchy:imported.document.productHierarchy,
      externalReferences:[],
      semanticConformance:uncheckedStepConformance(),
      ...(sceneTree?{sceneTree,display,xde}: {}),
      ...(lod0?{lod0CacheKey:lod0.cacheKey,lod0ShellCacheKeys:lod0.lod0Shells.map(shell=>shell.cacheKey)}: {}),
    },retainedDocument:imported.document,sceneTree,display,xde,lod0,
  }
}

export function exportRetainedStepForWorkbench(model:NurbsBrep,document?:DirectStepV10Document){
  return document?exportDirectStepV10(document):exportDirectStepV9(model)
}

const sameArrayValues=(a:ArrayLike<number>,b:ArrayLike<number>):boolean=>{
  if(a.length!==b.length)return false
  for(let i=0;i<a.length;i++)if(!Object.is(a[i],b[i]))return false
  return true
}

let retainedRecord=decodeRecord(storageGet(RETAINED_STEP_KEY))
export const retainedStepSession={
  get:()=>retainedRecord?.model,
  body:(candidate:DirectBody):DirectBody=>{
    const record=retainedRecord
    if(!record)return candidate
    return sameArrayValues(candidate.mesh.positions,record.preview.positions)&&sameArrayValues(candidate.mesh.indices,record.preview.indices)
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
