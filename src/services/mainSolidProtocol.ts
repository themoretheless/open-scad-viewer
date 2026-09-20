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
  | {kind:'latticeGraph'; mesh:LatticeGraphMesh; options:LighteningOptions}
export interface MainSolidResults {main:DirectDocument; cad:DirectDocument; inspect:CadPairReport[]; truss:TrussResponse; latticeGraph:NominalLatticeGraph}
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
  | {kind:Exclude<MainSolidJob['kind'],'truss'>}
export function mainSolidExpectation(job:MainSolidJob):MainSolidExpectation {
  return job.kind==='truss' ? {kind:job.kind,nodes:job.model.nodesMm.length,members:job.model.members.length} : {kind:job.kind}
}

/** Admit the result for this request, not merely any object with a result field. */
export function mainSolidResult(job:MainSolidExpectation, value:unknown): boolean {
  if (!value || typeof value!=='object') return false
  if (job.kind==='latticeGraph') return isNominalLatticeGraph(value)
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
