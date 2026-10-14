import type { buildSolidSurface } from '../../services/solidSurfaceConstruction'
import { isSolidNurbsRefit } from '../../services/solidCurveReduction'
import { isSolidSurfaceBuild } from '../../services/solidSurfaceConstruction'
import { isSolidBodyEdit } from '../../services/solidBodyEdit'
import { isSolidProfileEdit } from '../../services/solidProfileEdit'
import { isSolidSceneEdit } from '../../services/solidSceneEdit'
import { type FramedSweepResult, type SurfaceJetBoundary } from '../../services/nurbsConstructors'
import { type Vec3, type SketchPlane } from '../../services/directSketchGeometry'
import { evaluateNurbsCurve, type NurbsCurve } from '../../services/nurbsCurve'
import { isProfileCommand, type Pane } from './helpers'
import { type solidBodyEdges } from '../../services/solidBodyEdges'
import { createSolidPreviewWorker } from '../../services/solidPreviewWorker'
import { type DirectMaterial, type DirectDocument, type DirectSketch, type Point2, type DirectBody } from '../../services/directModeling'
import { type directExtrusionTool } from '../../services/directModelingTools'
import { type SolidNurbsSurface } from '../../services/solidNurbs'
import { computed, ref, shallowRef, onUnmounted, type Ref, type ShallowRef, type ComputedRef } from 'vue'

export interface AdvancedPreviewOptions {
  activePlane: Ref<{ origin: [number, number, number]; u: [number, number, number]; v: [number, number, number]; }, SketchPlane | { origin: [number, number, number]; u: [number, number, number]; v: [number, number, number]; }>
  advanced: Ref<{ offsetJoin: "smooth" | "bevel" | "trim-nonzero" | "trim-evenodd"; patchPrepare: boolean; patchError: number; profileGap: number; curveEndA: "start" | "end"; curveEndB: "start" | "end"; curveAngle: number; prepareOpenPeriodic: boolean; matchOrder: 1 | 2; matchBoundaryA: SurfaceJetBoundary; matchBoundaryB: SurfaceJetBoundary; matchScale: number; matchReverse: boolean; matchError: number; sweepMode: "translation" | "framed"; sweepSections: number; sweepDeviation: number; sweepNormalX: number; sweepNormalY: number; sweepNormalZ: number; surfaceAxis: "u" | "v"; rebuildControls: number; reduceDegree: number; maxError: number; distance: number; radius: number; endRadius: number; filletMode: "constant" | "variable" | "corner" | "partial-preview"; axis: "x" | "y" | "z"; x: number; y: number; z: number; angle: number; scale: number; cx: number; cy: number; start: number; sweep: number; end: "start" | "end"; }, { offsetJoin: "smooth" | "bevel" | "trim-nonzero" | "trim-evenodd"; patchPrepare: boolean; patchError: number; profileGap: number; curveEndA: "start" | "end"; curveEndB: "start" | "end"; curveAngle: number; prepareOpenPeriodic: boolean; matchOrder: 1 | 2; matchBoundaryA: SurfaceJetBoundary; matchBoundaryB: SurfaceJetBoundary; matchScale: number; matchReverse: boolean; matchError: number; sweepMode: "translation" | "framed"; sweepSections: number; sweepDeviation: number; sweepNormalX: number; sweepNormalY: number; sweepNormalZ: number; surfaceAxis: "u" | "v"; rebuildControls: number; reduceDegree: number; maxError: number; distance: number; radius: number; endRadius: number; filletMode: "constant" | "variable" | "corner" | "partial-preview"; axis: "x" | "y" | "z"; x: number; y: number; z: number; angle: number; scale: number; cx: number; cy: number; start: number; sweep: number; end: "start" | "end"; } | { offsetJoin: "smooth" | "bevel" | "trim-nonzero" | "trim-evenodd"; patchPrepare: boolean; patchError: number; profileGap: number; curveEndA: "start" | "end"; curveEndB: "start" | "end"; curveAngle: number; prepareOpenPeriodic: boolean; matchOrder: 1 | 2; matchBoundaryA: SurfaceJetBoundary; matchBoundaryB: SurfaceJetBoundary; matchScale: number; matchReverse: boolean; matchError: number; sweepMode: "translation" | "framed"; sweepSections: number; sweepDeviation: number; sweepNormalX: number; sweepNormalY: number; sweepNormalZ: number; surfaceAxis: "u" | "v"; rebuildControls: number; reduceDegree: number; maxError: number; distance: number; radius: number; endRadius: number; filletMode: "constant" | "variable" | "corner" | "partial-preview"; axis: "x" | "y" | "z"; x: number; y: number; z: number; angle: number; scale: number; cx: number; cy: number; start: number; sweep: number; end: "start" | "end"; }>
  advancedOp: Ref<"split" | "push" | "shell" | "curve" | "transform" | "offset" | "loft" | "chamfer" | "instance-create" | "instance-transform" | "instance-place" | "nurbs-loft" | "nurbs-sweep" | "nurbs-patch" | "nurbs-rebuild" | "nurbs-reduce" | "nurbs-surface-rebuild" | "nurbs-surface-reduce" | "profile-union" | "profile-difference" | "profile-intersection" | "extend" | "edge-fillet" | "profile-prepare" | "nurbs-offset" | "nurbs-point-trim" | "nurbs-match" | "nurbs-prepare" | "nurbs-curve-match" | null>
  bodyEditError: Ref<string>
  bodyEditPending: Ref<boolean>
  bodyEditResult: ShallowRef<DirectDocument | null>
  commit: (next: DirectDocument) => void
  curvePoints: (curve: NurbsCurve) => number[][]
  document: ShallowRef<DirectDocument>
  edgeIndex: Ref<number>
  edgeIndexes: Ref<number[]>
  error: Ref<string>
  extraSelection: Ref<string[]>
  faceIndex: Ref<number>
  isNurbsMatch: (op: unknown) => boolean
  /** Read lazily: declared after this composable runs. */
  kernelReady: () => Ref<boolean>
  loftPreviewId: Ref<string>
  meshPolygons: (b: ReturnType<typeof directExtrusionTool>, preview?: number, projectPoint?: (p: number[]) => [number, number, number], world?: boolean) => { preview: number; surface: boolean; normal: number[]; closed: boolean; vertices: [number, number, number][]; id: string; material: DirectMaterial | undefined; triangle: number; key: string; points: string; shade: number; depth: number; }[]
  openingFaces: Ref<number[]>
  pickMode: Ref<"body" | "vertex" | "edge" | "face">
  pickObject: (id: string, pane: Pane, add?: boolean) => void
  profileLoops: (sketch: DirectSketch) => Point2[][]
  props: Readonly<{ open: boolean }>
  resultAdvanced: (onSweepReport?: (report: FramedSweepResult["report"]) => void) => DirectDocument
  run: (action: () => void) => void
  samePlane: (a?: SketchPlane, b?: SketchPlane) => boolean
  /** Read lazily: declared after this composable runs. */
  sceneBodies: () => ComputedRef<DirectBody[]>
  selectedBody: ComputedRef<DirectBody | undefined>
  selectedIds: ComputedRef<string[]>
  selection: Ref<string>
  surfaceBuildResult: ShallowRef<ReturnType<typeof buildSolidSurface> | null>
  surfaceConstructionError: (error: unknown) => string
  surfaceInputs: Ref<string[]>
  surfacePolygons: (item: SolidNurbsSurface) => { preview: number; surface: boolean; normal: number[]; closed: boolean; vertices: [number, number, number][]; id: string; material: DirectMaterial | undefined; triangle: number; key: string; points: string; shade: number; depth: number; }[]
  surfaceReversed: Ref<boolean[]>
}

export function useAdvancedPreview(options: AdvancedPreviewOptions) {
  const { activePlane, advanced, advancedOp, bodyEditError, bodyEditPending, bodyEditResult, commit, curvePoints, document, edgeIndex, edgeIndexes, error, extraSelection, faceIndex, isNurbsMatch, kernelReady, loftPreviewId, meshPolygons, openingFaces, pickMode, pickObject, profileLoops, props, resultAdvanced, run, samePlane, sceneBodies, selectedBody, selectedIds, selection, surfaceBuildResult, surfaceConstructionError, surfaceInputs, surfacePolygons, surfaceReversed } = options
const advancedPreview = computed(() => {
 if(!advancedOp.value||(advancedOp.value==='profile-prepare'||isSolidNurbsRefit(advancedOp.value)||isNurbsMatch(advancedOp.value))&&bodyEditPending.value)return {document:null,error:'',rawError:'',sweepReport:null}
 if(isSolidSurfaceBuild(advancedOp.value)){const r=surfaceBuildResult.value,error=r?.error||bodyEditError.value;return {document:r?.document??null,error:error?surfaceConstructionError(error):'',rawError:error,sweepReport:r?.report??null}}
 if(advancedOp.value==='nurbs-offset'||advancedOp.value==='nurbs-point-trim'||isSolidBodyEdit(advancedOp.value)||isSolidProfileEdit(advancedOp.value)||isSolidSceneEdit(advancedOp.value))return {document:bodyEditResult.value,error:bodyEditError.value?surfaceConstructionError(bodyEditError.value):'',rawError:bodyEditError.value,sweepReport:null}
 // Explicit reactive dependencies: history itself intentionally is not reactive.
 void document.value;void selection.value;void faceIndex.value;void edgeIndex.value;void edgeIndexes.value;void openingFaces.value;void advanced.value
 const report:{value:FramedSweepResult['report']|null}={value:null}
 try{const result=resultAdvanced(value=>report.value=value);return {document:result,error:'',rawError:'',sweepReport:report.value}}catch(e){return {document:null,error:surfaceConstructionError(e),rawError:e instanceof Error?e.message:String(e),sweepReport:report.value}}
})
const pointTrimPreviewPoints=computed(()=>{
 if(advancedOp.value!=='nurbs-point-trim')return []
 const curve=advancedPreview.value.document?.curves?.find(c=>c.id===selection.value)?.curve
 return curve?curvePoints(curve).map(p=>[p[0],p[1],p[2]??0] as Vec3):[]
})
const pointTrimCut=computed(()=>{
 if(advancedOp.value!=='nurbs-point-trim')return undefined
 const curve=advancedPreview.value.document?.curves?.find(c=>c.id===selection.value)?.curve
 // Native trim inserts the cut knot, making this endpoint a control point.
 const p=advanced.value.end==='start'?curve?.controlPoints.at(-1):curve?.controlPoints[0]
 return p?[p[0],p[1],p[2]??0]:undefined
})
const patchBudgetCurve=computed(()=>{
 const role=advancedOp.value==='nurbs-patch'?advancedPreview.value.rawError.match(/PATCH_BUDGET:([0-3])/):null
 return role?surfaceInputs.value[Number(role[1])]:undefined
})
const patchGapPoints=computed(()=>{
 if(advancedOp.value!=='nurbs-patch')return []
 const match=advancedPreview.value.rawError.match(/Coons corner ([1-4]) does not coincide/)
 if(!match)return []
 const pairs=[[[0,0],[2,0]],[[0,1],[3,0]],[[1,0],[2,1]],[[1,1],[3,1]]]
 try{return pairs[Number(match[1])-1].map(([role,end])=>{
  const curve=document.value.curves!.find(c=>c.id===surfaceInputs.value[role])!.curve
  const last=surfaceReversed.value[role]?!end:!!end
  return evaluateNurbsCurve(curve,curve.knots[last?curve.controlPoints.length:curve.degree]).point as Vec3
 })}catch{return []}
})
function applyAdvanced(){run(()=>{
 if(!advancedPreview.value.document)return
 const d=structuredClone(advancedPreview.value.document),splitBody=d.bodies.find(b=>b.id==='preview-split');if(splitBody)splitBody.id=crypto.randomUUID()
 const prepared=advancedOp.value==='nurbs-prepare'
 const profileId=(isProfileCommand(advancedOp.value))?surfaceInputs.value[0]:''
 const created=(advancedOp.value==='instance-create'||advancedOp.value==='nurbs-offset'||advancedOp.value==='loft')?loftPreviewId.value:''
 commit(d);advancedOp.value=null;if(profileId){selection.value=profileId;extraSelection.value=[]}if(prepared)advanced.value.matchReverse=false;if(created)pickObject(created,'3d');faceIndex.value=edgeIndex.value=-1;edgeIndexes.value=[];openingFaces.value=[]
})}
const axisVector=(axis:string):Vec3=>axis==='x'?[1,0,0]:axis==='y'?[0,1,0]:[0,0,1]
const advancedBodyIds=computed(()=>new Set([...selectedIds.value,...sceneBodies().value.filter(b=>b.instance&&selectedIds.value.includes(b.instance.sourceId)).map(b=>b.id),'preview-split',loftPreviewId.value]))
const advancedPolygons=computed(()=>advancedPreview.value.document?.bodies.filter(b=>advancedBodyIds.value.has(b.id)).flatMap(b=>meshPolygons(b,1)).sort((a,b)=>a.depth-b.depth)??[])
const advancedSurfaceIds=computed(()=>advancedOp.value==='nurbs-prepare'?surfaceInputs.value:advancedOp.value==='nurbs-match'?[surfaceInputs.value[1]]:advancedOp.value==='nurbs-surface-reduce'||advancedOp.value==='nurbs-surface-rebuild'?[selection.value]:[loftPreviewId.value])
const advancedSurfacePolygons=computed(()=>advancedPreview.value.document?.surfaces?.filter(item=>advancedSurfaceIds.value.includes(item.id)).flatMap(item=>surfacePolygons(item)).sort((a,b)=>a.depth-b.depth)??[])
const advancedSketches=computed(()=>advancedPreview.value.document?.sketches.filter(s=>selectedIds.value.includes(s.id)&&samePlane(s.plane,activePlane.value))??[])
const advancedSketchDisplay=computed(()=>new Map(advancedSketches.value.map(s=>[s.id,profileLoops(s)])))
const authoredEdges=shallowRef<ReturnType<typeof solidBodyEdges>>([]),bodyEdgesPending=ref(false)
const bodyEdgesWorker=createSolidPreviewWorker()
let bodyEdgesGeneration=0
function cancelBodyEdges(){bodyEdgesGeneration++;bodyEdgesWorker.cancel();bodyEdgesPending.value=false}
onUnmounted(()=>{cancelBodyEdges();bodyEdgesWorker.dispose()})
async function refreshBodyEdges(){
 cancelBodyEdges();authoredEdges.value=[]
 const body=selectedBody.value,source=document.value,generation=bodyEdgesGeneration
 if(!props.open||!kernelReady().value||!body||pickMode.value!=='edge')return
 bodyEdgesPending.value=true
 try{
  const edges=await bodyEdgesWorker.run({kind:'bodyEdges',body})
  if(generation===bodyEdgesGeneration&&document.value===source&&selectedBody.value===body)authoredEdges.value=edges
 }catch(e){if(generation===bodyEdgesGeneration)error.value=e instanceof Error?e.message:String(e)}
 finally{if(generation===bodyEdgesGeneration)bodyEdgesPending.value=false}
}
  return { advancedPolygons, advancedSurfacePolygons, advancedPreview, pointTrimPreviewPoints, pointTrimCut, patchBudgetCurve, patchGapPoints, applyAdvanced, axisVector, advancedBodyIds, advancedSurfaceIds, advancedSketches, advancedSketchDisplay, authoredEdges, bodyEdgesPending, cancelBodyEdges, refreshBodyEdges }
}
