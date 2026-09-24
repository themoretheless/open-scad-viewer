import {parseBondedSolidInput,isBondedSolidResult,type BondedSolidResult} from './bondedSolidProtocol'
import type {StructuralSections,BoundaryConnectivity} from './structuralSections'
import type {MeshData} from '../core/mesh'
import type {CadOptions} from './cadWorkbench'
import type {CadPairReport} from './cadInspection'
import type {DirectBody, DirectDocument} from './directModeling'
import type {MainOperation, MainParameters} from './mainModeling'
import type {PickHit} from './rendererContracts'
import type {TrussInput, TrussResponse} from './trussAnalysis'
import {isNominalLatticeGraph, type LatticeGraphMesh, type NominalLatticeGraph} from './latticeGraphProtocol'
import type {LighteningOptions} from './solidLightening'

export type MainSolidJob =
  | {kind:'main'; meshes:MeshData[]; selected:number; hit:PickHit|null; operation:MainOperation; parameters:MainParameters}
  | {kind:'cad'; document:DirectDocument; options:CadOptions}
  | {kind:'inspect'; bodies:DirectBody[]}
  | {kind:'truss'; model:TrussInput}
  | {kind:'bondedSolid';inputJson:string}
  | {kind:'structuralSections'; mesh:LatticeGraphMesh; axis:'x'|'y'|'z'; stations:number[]}
  | {kind:'latticeGraph'; mesh:LatticeGraphMesh; options:LighteningOptions}
export interface MainSolidResults {bondedSolid:BondedSolidResult; main:DirectDocument; cad:DirectDocument; inspect:CadPairReport[]; truss:TrussResponse; latticeGraph:NominalLatticeGraph; structuralSections:StructuralSections}
export type MainSolidRequest = {version:1; id:number; job:MainSolidJob}
export type MainSolidResponse = {version:1; id:number; kind:MainSolidJob['kind']} & (
  | {ok:true; result:MainSolidResults[keyof MainSolidResults]}
  | {ok:false; error:{name:string; message:string; code?:string}}
)

const finite = (v:unknown): v is number => typeof v==='number' && Number.isFinite(v)
function arrayOf(value:unknown, length:number, check:(v:unknown)=>boolean):boolean {
  if(!Array.isArray(value)||value.length!==length)return false
  for(const item of value)if(!check(item))return false
  return true
}
const vector = (v:unknown) => arrayOf(v,3,finite)
export type MainSolidExpectation = {kind:'truss'; nodes:number; members:number}
  | {kind:'structuralSections';axis:'x'|'y'|'z';stations:number[]}
  | {kind:'bondedSolid';nodes:number;tets:number;bonds:number}
  | {kind:Exclude<MainSolidJob['kind'],'truss'|'structuralSections'|'bondedSolid'>}
export function mainSolidExpectation(job:MainSolidJob):MainSolidExpectation {
  if(job.kind==='bondedSolid'){
    const m=parseBondedSolidInput(job.inputJson)
    return {kind:job.kind,nodes:m.nodesMm.length,tets:m.tets.length,bonds:m.bonds.length}
  }
  return job.kind==='truss' ? {kind:job.kind,nodes:job.model.nodesMm.length,members:job.model.members.length} : job.kind==='structuralSections'?{kind:job.kind,axis:job.axis,stations:[...job.stations]}:{kind:job.kind}
}

/** Admit the result for this request, not merely any object with a result field. */
export function mainSolidResult(job:MainSolidExpectation, value:unknown): boolean {
  if (!value || typeof value!=='object') return false
  if (job.kind==='bondedSolid')return isBondedSolidResult(value,job.nodes,job.tets,job.bonds)
  if (job.kind==='latticeGraph') return isNominalLatticeGraph(value)
  if (job.kind==='structuralSections') {
    const v=value as StructuralSections
    const planeAxes={x:[1,2],y:[2,0],z:[0,1]}[job.axis]
    return v.modelKind==='finished-mesh-sections-v1'&&v.axis===job.axis
      &&arrayOf(v.planeAxes,2,n=>Number.isInteger(n)&&Number(n)>=0&&Number(n)<3)
      &&v.planeAxes.every((axis,i)=>axis===planeAxes[i])
      &&(v.selfIntersections==='not-checked'||v.selfIntersections==='checked-at-tolerance')
      &&!!v.sourceMesh&&Array.isArray(v.sourceMesh.positions)&&v.sourceMesh.positions.length<=900000
      &&v.sourceMesh.positions.length%3===0&&arrayOf(v.sourceMesh.positions,v.sourceMesh.positions.length,finite)
      &&Array.isArray(v.sourceMesh.indices)&&v.sourceMesh.indices.length===v.triangleCount*3
      &&arrayOf(v.sourceMesh.indices,v.sourceMesh.indices.length,n=>Number.isInteger(n)&&Number(n)>=0&&Number(n)<v.sourceMesh.positions.length/3)
      &&finite(v.volumeMm3)&&v.volumeMm3>0&&Number.isInteger(v.triangleCount)&&v.triangleCount>0&&v.triangleCount<=100000
      &&isBoundaryConnectivity(v.connectivity,v.triangleCount,v.sourceMesh.positions.length/3)
      &&isMaterialAudit(v)
      &&Array.isArray(v.sections)&&v.sections.length===job.stations.length&&v.sections.length>0&&v.sections.length<=64
      &&Array.from(v.sections).every((s,i)=>s&&s.positionMm===job.stations[i]&&finite(s.positionMm)&&typeof s.material==='boolean'&&Array.isArray(s.contours)
        &&(s.material?!!s.properties&&finite(s.properties.areaMm2)&&s.properties.areaMm2>0
          &&arrayOf(s.properties.centroidMm,2,finite)&&finite(s.properties.iuuMm4)&&s.properties.iuuMm4>0
          &&finite(s.properties.ivvMm4)&&s.properties.ivvMm4>0&&finite(s.properties.iuvMm4):s.properties===null))
  }
  if (job.kind==='truss') {
    const v=value as TrussResponse, {nodes,members}=job
    return nodes>0 && nodes<=125 && members>0 && members<=400
      && arrayOf(v.displacementsMm,nodes,vector)
      && arrayOf(v.reactionsN,nodes,vector)
      && arrayOf(v.axialForcesN,members,finite)
      && arrayOf(v.axialStressesMpa,members,finite)
      && finite(v.maxDeflectionMm) && v.maxDeflectionMm>=0
      && finite(v.maxRelativeResidual) && v.maxRelativeResidual>=0 && v.maxRelativeResidual<=1e-9
      && Number.isInteger(v.freeDofs) && v.freeDofs>=0 && v.freeDofs<=3*nodes
  }
  if (job.kind==='inspect') return Array.isArray(value) && value.every(v => v
    && typeof v.a==='string' && typeof v.b==='string' && finite(v.overlapMm3) && finite(v.gapMm))
  const v=value as DirectDocument
  return v.version===1 && Array.isArray(v.sketches) && Array.isArray(v.bodies)
}

/** Validate bounded provenance before accepting a worker report. No mechanics in TS. */
export function isBoundaryConnectivity(value:unknown,triangles:number,vertices:number):value is BoundaryConnectivity {
  if(!value||typeof value!=='object')return false
  const v=value as BoundaryConnectivity
  if(v.modelKind!=='indexed-edge-boundary-components-v1'||(v.materialConnectivity!=='not-established'&&v.materialConnectivity!=='classified-at-tolerance')
    ||!Array.isArray(v.components)||!v.components.length||v.components.length>triangles
    ||!Array.isArray(v.sharedVertices)||v.sharedVertices.length>vertices)return false
  const seen=new Set<number>()
  for(let i=0;i<v.components.length;i++){
    const c=v.components[i]
    if(!c||c.id!==i||!Number.isFinite(c.signedVolumeMm3)||!Array.isArray(c.sourceTriangles)
      ||!c.sourceTriangles.length||c.sourceTriangles.length>triangles
      ||!Array.isArray(c.boundsMm)||c.boundsMm.length!==2
      ||!c.boundsMm.every(p=>Array.isArray(p)&&p.length===3&&p.every(Number.isFinite))
      ||c.boundsMm[0].some((x,k)=>x>c.boundsMm[1][k]))return false
    for(const t of c.sourceTriangles){
      if(!Number.isInteger(t)||t<0||t>=triangles||seen.has(t))return false
      seen.add(t)
    }
  }
  if(seen.size!==triangles)return false
  const shared=new Set<number>()
  for(const p of v.sharedVertices){
    if(!p||!Number.isInteger(p.sourceVertex)||p.sourceVertex<0||p.sourceVertex>=vertices||shared.has(p.sourceVertex)
      ||!Array.isArray(p.components)||p.components.length<2||p.components.length>v.components.length
      ||!p.components.every((c,i)=>Number.isInteger(c)&&c>=0&&c<v.components.length&&(i===0||c>p.components[i-1])))return false
    shared.add(p.sourceVertex)
  }
  return true
}

function isMaterialAudit(v:StructuralSections):boolean {
  const a=v.materialAudit
  if(!a||typeof a!=='object')return false
  if(a.status==='unresolved')return v.selfIntersections==='not-checked'
    &&v.connectivity.materialConnectivity==='not-established'
    &&typeof a.code==='string'&&a.code.length>0&&a.code.length<=128
    &&typeof a.message==='string'&&a.message.length>0&&a.message.length<=4096
  if(a.status!=='classified'||v.selfIntersections!=='checked-at-tolerance'
    ||v.connectivity.materialConnectivity!=='classified-at-tolerance'||v.connectivity.sharedVertices.length!==0
    ||!finite(a.toleranceMm)||a.toleranceMm<=0||!Number.isInteger(a.materialRegions)||a.materialRegions<=0
    ||!Array.isArray(a.shells)||a.shells.length!==v.connectivity.components.length||a.shells.length>128)return false
  if(!a.shells.every((s,i)=>s&&s.id===i&&s.firstTriangle===v.connectivity.components[i].sourceTriangles[0]
    &&Number.isInteger(s.depth)&&s.depth>=0&&s.depth<a.shells.length
    &&s.kind===(s.depth%2===0?'material':'cavity')
    &&(s.parent===null?s.depth===0:Number.isInteger(s.parent)&&s.parent>=0&&s.parent<a.shells.length&&s.parent!==i)))return false
  return a.shells.every(s=>s.parent===null||a.shells[s.parent].depth+1===s.depth)
    &&a.materialRegions===a.shells.filter(s=>s.kind==='material').length
}
