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
import {MainSolidWorkerPool,type MainSolidPoolRunOptions} from './mainSolidWorkerPool'

const createWorker=()=>new Worker(new URL('../workers/mainSolid.worker.ts',import.meta.url),{type:'module'})

// Primary interactive modeling worker: a superseding modeling call terminates
// the previous interactive modeling step so rapid UI edits don't pile up.
const shared=new MainSolidWorkerClient(createWorker)

// Multi-worker pool for concurrent analysis, inspection, truss FEA, lattice
// generation, and structural sectioning so concurrent jobs run in parallel
// across CPU cores without aborting each other or the modeling worker.
export const sharedSolidPool=new MainSolidWorkerPool(createWorker)

export function cancelMainSolid(){
 shared.cancel()
 sharedSolidPool.cancelAll()
}

export function computeMainSolid(meshes:MeshData[],selected:number,hit:PickHit|null,operation:MainOperation,parameters:MainParameters):Promise<DirectDocument>{
 return shared.run({kind:'main',meshes,selected,hit,operation,parameters})
}

export function computeCadOperation(document:DirectDocument,options:CadOptions):Promise<DirectDocument>{
 return shared.run({kind:'cad',document,options})
}

export function computeCadInspection(bodies:import('./directModeling').DirectBody[],options:MainSolidPoolRunOptions={}):Promise<import('./cadInspection').CadPairReport[]>{
 return sharedSolidPool.run({kind:'inspect',bodies},options)
}

export function computeTrussAnalysis(model:TrussInput,options:MainSolidPoolRunOptions={}){
 return sharedSolidPool.run({kind:'truss',model},{...options,timeoutMs:options.timeoutMs??30000})
}

export async function computeTrussScenario(scenario:TrussScenario,options:MainSolidPoolRunOptions={}){
 const resolved=resolveTrussScenario(scenario)
 const result=await computeTrussAnalysis(resolved.model,options)
 return {...resolved,result}
}

export async function computeNominalLatticeGraph(mesh:LatticeGraphMesh,options:LighteningOptions,runOptions:MainSolidPoolRunOptions={}){
 checkLatticeGraphInput(mesh,options)
 return sharedSolidPool.run({kind:'latticeGraph',mesh:{vertices:mesh.vertices,indices:mesh.indices,transform:mesh.transform},options:{...options}},
  {...runOptions,timeoutMs:runOptions.timeoutMs??30000})
}

export function computeStructuralSections(mesh:LatticeGraphMesh,axis:'x'|'y'|'z',stations:number[],options:MainSolidPoolRunOptions={}){
 checkLatticeGraphMeshInput(mesh)
 if(!Array.isArray(stations)||!stations.length||stations.length>64)throw new Error('Provide 1-64 section positions.')
 return sharedSolidPool.run({kind:'structuralSections',mesh:{vertices:mesh.vertices,indices:mesh.indices,transform:mesh.transform},axis,stations:[...stations]},
  {...options,timeoutMs:options.timeoutMs??30000})
}

export function computeBondedSolid(inputJson:string,options:MainSolidPoolRunOptions={}){
 parseBondedSolidInput(inputJson)
 return sharedSolidPool.run({kind:'bondedSolid',inputJson},{...options,timeoutMs:options.timeoutMs??30000})
}
