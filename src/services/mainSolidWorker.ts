import {parseBondedSolidInput} from './bondedSolidProtocol'
import type {CadOptions} from './cadWorkbench'
import type {MeshData} from '../core/mesh'
import type {PickHit} from './rendererContracts'
import type {DirectDocument} from './directModeling'
import type {MainOperation,MainParameters} from './mainModeling'
import type {TrussInput} from './trussAnalysis'
import {resolveTrussScenario, type TrussScenario} from './trussScenario'
import {checkLatticeGraphInput, checkLatticeGraphMeshInput, type LatticeGraphMesh} from './latticeGraphProtocol'
import type {LighteningOptions} from './solidLightening'
import {MainSolidWorkerClient,type MainSolidRunOptions} from './mainSolidWorkerClient'

// One warm worker serves all main-solid/CAD operations; the geometry kernel is
// initialized once instead of every call paying the WASM inflate+compile.
// A running computation cannot be interrupted in place, so cancellation (or a
// superseding call) terminates the worker and the next operation recreates it.
const shared=new MainSolidWorkerClient(()=>new Worker(new URL('../workers/mainSolid.worker.ts',import.meta.url),{type:'module'}))
export function cancelMainSolid(){shared.cancel()}

export function computeMainSolid(meshes:MeshData[],selected:number,hit:PickHit|null,operation:MainOperation,parameters:MainParameters):Promise<DirectDocument>{
 return shared.run({kind:'main',meshes,selected,hit,operation,parameters})
}

export function computeCadOperation(document:DirectDocument,options:CadOptions):Promise<DirectDocument>{
 return shared.run({kind:'cad',document,options})
}

export function computeCadInspection(bodies:import('./directModeling').DirectBody[]):Promise<import('./cadInspection').CadPairReport[]>{
 return shared.run({kind:'inspect',bodies})
}

export function computeTrussAnalysis(model:TrussInput,options:MainSolidRunOptions={}){
 return shared.run({kind:'truss',model},{...options,timeoutMs:options.timeoutMs??30000})
}

export async function computeTrussScenario(scenario:TrussScenario,options:MainSolidRunOptions={}){
 const resolved=resolveTrussScenario(scenario)
 const result=await computeTrussAnalysis(resolved.model,options)
 return {...resolved,result}
}

export async function computeNominalLatticeGraph(mesh:LatticeGraphMesh,options:LighteningOptions,runOptions:MainSolidRunOptions={}){
 checkLatticeGraphInput(mesh,options)
 return shared.run({kind:'latticeGraph',mesh:{vertices:mesh.vertices,indices:mesh.indices,transform:mesh.transform},options:{...options}},
  {...runOptions,timeoutMs:runOptions.timeoutMs??30000})
}

export function computeStructuralSections(mesh:LatticeGraphMesh,axis:'x'|'y'|'z',stations:number[],options:MainSolidRunOptions={}){
 checkLatticeGraphMeshInput(mesh)
 if(!Array.isArray(stations)||!stations.length||stations.length>64)throw new Error('Provide 1-64 section positions.')
 return shared.run({kind:'structuralSections',mesh:{vertices:mesh.vertices,indices:mesh.indices,transform:mesh.transform},axis,stations:[...stations]},
  {...options,timeoutMs:options.timeoutMs??30000})
}

export function computeBondedSolid(inputJson:string,options:MainSolidRunOptions={}){
 parseBondedSolidInput(inputJson)
 return shared.run({kind:'bondedSolid',inputJson},{...options,timeoutMs:options.timeoutMs??30000})
}
