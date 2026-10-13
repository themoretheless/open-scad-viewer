import type { OrbitCamera } from '../../services/directModelingTools'
import { createSolidPreviewWorker } from '../../services/solidPreviewWorker'
import { SourceBodyDisplay } from '../../services/sourceBodyDisplay'
import { type SourceBodyResult } from '../../services/sourceBody'
import { projectDirectPoint } from '../../services/directModelingTools'
import { type Vec3 } from '../../services/directSketchGeometry'
import { type DirectDocument, type Point2 } from '../../services/directModeling'
import { type Pane } from './helpers'
import { computed, ref, shallowRef, watch, onUnmounted, type Ref, type ShallowRef } from 'vue'

export interface SourceDisplayOptions {
  camera: Ref<OrbitCamera>
  cancelCommandState: () => void
  document: ShallowRef<DirectDocument>
  edgeIndex: Ref<number>
  edgeIndexes: Ref<number[]>
  extraSelection: Ref<string[]>
  faceIndex: Ref<number>
  /** Read lazily: declared after this composable runs. */
  hovered: () => Ref<string>
  /** Read lazily: declared after this composable runs. */
  kernelReady: () => Ref<boolean>
  mode: Ref<"2d" | "3d">
  objectInView: (id: string) => boolean
  objectSelectable: (id: string) => boolean
  openingFaces: Ref<number[]>
  project: (p: ArrayLike<number>, pane: Pane) => Point2
  props: Readonly<{ open: boolean }>
  selection: Ref<string>
  tool: Ref<"trim" | "select" | "slot" | "circle" | "polyline" | "rectangle" | "arc">
}

export function useSourceDisplay(options: SourceDisplayOptions) {
  const { camera, cancelCommandState, document, edgeIndex, edgeIndexes, extraSelection, faceIndex, hovered, kernelReady, mode, objectInView, objectSelectable, openingFaces, project, props, selection, tool } = options
const sourceDisplayWorker=createSolidPreviewWorker(),sourceDisplay=new SourceBodyDisplay(sourceDisplayWorker)
const sourceDisplayResults=shallowRef<Record<string,SourceBodyResult>>({}),sourceDisplayErrors=shallowRef<Record<string,string>>({})
const sourceDisplayPending=ref(false),sourceEdgeSelection=ref(''),sourceEdgeHover=ref('')
let sourceDisplayGeneration=0
function cancelSourceDisplay(){sourceDisplayGeneration++;sourceDisplay.cancel();sourceDisplayPending.value=false}
async function refreshSourceDisplay(){
 cancelSourceDisplay();sourceDisplayResults.value={};sourceDisplayErrors.value={};sourceEdgeSelection.value=''
 if(!props.open||!kernelReady().value)return
 const source=document.value,generation=sourceDisplayGeneration
 sourceDisplayPending.value=true
 await sourceDisplay.prepare(source.sourceBodies??[],(id,result)=>{
  if(generation===sourceDisplayGeneration&&document.value===source)sourceDisplayResults.value={...sourceDisplayResults.value,[id]:result}
 },(id,e)=>{
  if(generation===sourceDisplayGeneration&&document.value===source)sourceDisplayErrors.value={...sourceDisplayErrors.value,[id]:e instanceof Error?e.message:String(e)}
 })
 if(generation===sourceDisplayGeneration)sourceDisplayPending.value=false
}
function pickSourceEdge(id:string,index:number){
 if(!objectSelectable(id)||!sourceDisplayResults.value[id]?.edges.some(edge=>edge.index===index))return
 cancelCommandState();selection.value='';extraSelection.value=[];hovered().value=''
 sourceEdgeSelection.value=id+':'+index;mode.value='3d';tool.value='select'
 faceIndex.value=edgeIndex.value=-1;edgeIndexes.value=[];openingFaces.value=[]
}
function pickSourceBody(id:string){
 const edge=sourceDisplayResults.value[id]?.edges[0]
 if(edge)pickSourceEdge(id,edge.index)
}
watch(selection,id=>{if(id)sourceEdgeSelection.value=''})
const sourceDisplayEdges=computed(()=>(document.value.sourceBodies??[]).filter(item=>objectInView(item.id)).flatMap(item=>
 (sourceDisplayResults.value[item.id]?.edges??[]).map(edge=>({key:item.id+':'+edge.index,body:item.id,name:item.name,index:edge.index,selectable:objectSelectable(item.id),
  points:(edge.displaySegments??[]).flatMap((segment,i)=>i===0?segment[1]:[segment[1][1]]).map(p=>project(p,'3d').join(',')).join(' ')}))))
const sourceDisplayFaces=computed(()=>Object.entries(sourceDisplayResults.value).filter(([id])=>objectInView(id)).flatMap(([id,result])=>
 (result.displayFaces??[]).flatMap(face=>face.tiles.flatMap((tile,i)=>[[0,1,2],[0,2,3]].map((indexes,k)=>({key:id+':'+face.index+':'+i+':'+k,body:id,face:face.index,
 points:indexes.map(index=>project(tile.corners[index]!,'3d').join(',')).join(' '),
 depth:indexes.reduce((sum,index)=>sum+projectDirectPoint(tile.corners[index]! as Vec3,camera.value)[2],0)/3}))))).sort((a,b)=>a.depth-b.depth))
const sourceUnresolvedCount=computed(()=>Object.entries(sourceDisplayResults.value).filter(([id])=>objectInView(id)).reduce((sum,[,result])=>sum+(result.displayFaces??[]).reduce((n,face)=>n+face.unresolved.length,0),0))
const sourceUnresolvedBoxes=computed(()=>Object.entries(sourceDisplayResults.value).filter(([id])=>objectInView(id)).flatMap(([id,result])=>
 (result.displayFaces??[]).flatMap(face=>face.unresolvedBoxes.map((box,i)=>{
 const points=Array.from({length:8},(_,k)=>project([box[0][k&1],box[1][(k>>1)&1],box[2][(k>>2)&1]],'3d'))
 const x=Math.min(...points.map(p=>p[0])),y=Math.min(...points.map(p=>p[1]))
 return {key:id+':'+face.index+':'+i,body:id,face:face.index,uv:face.unresolved[i],x,y,width:Math.max(...points.map(p=>p[0]))-x,height:Math.max(...points.map(p=>p[1]))-y}
 }))))
const sourceDisplayEndpointBoxes=computed(()=>Object.entries(sourceDisplayResults.value).flatMap(([id,result])=>
 result.edges.filter(edge=>id+':'+edge.index===sourceEdgeSelection.value&&objectInView(id)).flatMap(edge=>edge.endpointBoxes.map((box,end)=>{
  const corners=Array.from({length:8},(_,i)=>project([box[0][i&1],box[1][(i>>1)&1],box[2][(i>>2)&1]],'3d'))
  const x=Math.min(...corners.map(p=>p[0])),y=Math.min(...corners.map(p=>p[1]))
  return {key:id+':'+edge.index+':'+end,x,y,width:Math.max(...corners.map(p=>p[0]))-x,height:Math.max(...corners.map(p=>p[1]))-y,
   bounds:box.map(b=>'['+b.join(', ')+']').join(' ')}
 }))))
onUnmounted(()=>{cancelSourceDisplay();sourceDisplayWorker.dispose()})
  return { sourceDisplayResults, sourceDisplayErrors, sourceDisplayPending, sourceEdgeSelection, sourceEdgeHover, cancelSourceDisplay, refreshSourceDisplay, pickSourceEdge, pickSourceBody, sourceDisplayEdges, sourceDisplayFaces, sourceUnresolvedCount, sourceUnresolvedBoxes, sourceDisplayEndpointBoxes }
}
