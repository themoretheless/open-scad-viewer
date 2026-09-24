import {bondedSolidExample} from '../src/features/bondedSolidExample'
import {Worker} from 'node:worker_threads'
import {afterEach, expect, it, vi} from 'vitest'
import {MainSolidWorkerClient, type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import type {MainSolidRequest} from '../src/services/mainSolidProtocol'
import type {TrussModel} from '../src/services/trussAnalysis'
import {resolveTrussScenario} from '../src/services/trussScenario'
import {extrudeDirectSketch} from '../src/services/directModeling'
import {previewMeshes} from '../src/services/mainModeling'
import type {LighteningOptions} from '../src/services/solidLightening'

class NodePort implements MainSolidPort {
  onmessage:MainSolidPort['onmessage']=null
  onerror:MainSolidPort['onerror']=null
  onmessageerror:MainSolidPort['onmessageerror']=null
  constructor(readonly worker:Worker) {
    worker.on('message',data=>this.onmessage?.({data} as MessageEvent))
    worker.on('error',error=>this.onerror?.({message:error.message} as ErrorEvent))
    worker.on('messageerror',()=>this.onmessageerror?.({} as MessageEvent))
  }
  postMessage(message:MainSolidRequest){this.worker.postMessage(message)}
  terminate(){void this.worker.terminate()}
}
const clients:MainSolidWorkerClient[]=[], workers:Worker[]=[]
const bar=():TrussModel=>({nodesMm:[[0,0,0],[10,0,0]],members:[{nodes:[0,1],youngMpa:2000,areaMm2:2}],restrained:[[true,true,true],[false,true,true]],forcesN:[[0,0,0],[100,0,0]]})
const latticeOptions:LighteningOptions={pattern:'octet',axis:'z',cell:10,rib:2,rim:0,bottom:0,top:0,seed:42,jitter:0,lineWidth:.45,perimeters:3,skin:0,step:1,diagonals:true}
function realWorker(){
  const worker=new Worker(new URL('./fixtures/web-worker-node-harness.mjs',import.meta.url),{
    workerData:{entryUrl:new URL('../src/workers/mainSolid.worker.ts',import.meta.url).href,announceReady:false},
  })
  workers.push(worker);return new NodePort(worker)
}
afterEach(async()=>{clients.splice(0).forEach(c=>c.dispose());await Promise.all(workers.splice(0).map(w=>w.terminate()));vi.unstubAllGlobals()})

it('runs the shipped entry with real WASM, reuses it and preserves typed errors',async()=>{
  const client=new MainSolidWorkerClient(realWorker);clients.push(client)
  let ticks=0
  const timer=setInterval(()=>ticks++,5)
  try {
    const result=await client.run({kind:'truss',model:bar()})
    expect(result.displacementsMm[1][0]).toBeCloseTo(0.25,12)
    expect(result.reactionsN[0][0]).toBeCloseTo(-100,10)
    expect(ticks).toBeGreaterThan(0)
    const unstable=bar();unstable.restrained[1]=[false,false,false]
    await expect(client.run({kind:'truss',model:unstable})).rejects.toMatchObject({name:'GeometryKernelError',code:'TRUSS_SINGULAR'})
    await expect(client.run({kind:'inspect',bodies:[]})).rejects.toMatchObject({code:'GEOMETRY_INVALID_INPUT'})
    await expect(client.run({kind:'main',meshes:[],selected:0,hit:null,operation:'delete',
      parameters:{amount:1,x:0,y:0,z:0,axis:'z',edge:0,shape:'circle',width:2,height:2,cut:false},
    })).rejects.toThrow('Select a body')
    const box=(id:string,x:number)=>extrudeDirectSketch({id:'s',name:'Box',closed:true,
      points:[[x,0],[x+10,0],[x+10,10],[x,10]]},10,id)
    const body=box('a',0), other=box('b',13), before=JSON.stringify(body)
    const inspection=await client.run({kind:'inspect',bodies:[body,other]})
    expect(inspection[0].gapMm).toBeCloseTo(3,10)
    const resized=await client.run({kind:'cad',document:{version:1,sketches:[],bodies:[body]},options:{
      action:'resize',ids:['a'],sketches:[],axis:[0,0,1],origin:[0,0,0],amount:1,count:1,
      width:20,height:10,depth:10,pitch:1,secondary:1,mode:'min',pathId:'',profileIds:[],
    }})
    const xs=resized.bodies[0].mesh.positions.filter((_,i)=>i%3===0)
    expect(Math.max(...xs)-Math.min(...xs)).toBeCloseTo(20,10)
    expect(JSON.stringify(body)).toBe(before)
    const removed=await client.run({kind:'main',meshes:previewMeshes({version:1,sketches:[],bodies:[body]}),
      selected:0,hit:null,operation:'delete',parameters:{amount:1,x:0,y:0,z:0,axis:'z',edge:0,shape:'circle',width:2,height:2,cut:false},
    })
    expect(removed.bodies).toHaveLength(0)
    const recovered=await client.run({kind:'truss',model:bar()})
    expect(recovered.axialForcesN[0]).toBeCloseTo(100,10)
    const {forcesN:_,...structure}=bar()
    const loads=[{nodes:[1],originMm:[10,0,0] as [number,number,number],forceN:[100,0,0] as [number,number,number],momentNmm:[0,0,100] as [number,number,number]}]
    await expect(client.run({kind:'truss',model:{...structure,loads}})).rejects.toMatchObject({code:'TRUSS_LOAD_UNREALIZABLE'})
    loads[0].momentNmm=[0,0,0]
    expect((await client.run({kind:'truss',model:{...structure,loads}})).axialForcesN[0]).toBeCloseTo(100,10)
    const scenario=resolveTrussScenario({nodesMm:structure.nodesMm,members:structure.members,cases:[
      {id:'pull',restrained:structure.restrained,loads},
      {id:'reverse',restrained:structure.restrained,loads},
    ],combinations:[{id:'combined',terms:[{caseId:'pull',factor:1.2},{caseId:'reverse',factor:-0.5}]}],activeId:'combined'})
    expect((await client.run({kind:'truss',model:scenario.model})).axialForcesN[0]).toBeCloseTo(70,10)
    expect(workers).toHaveLength(1)
  } finally {clearInterval(timer)}
},30000)

it('runs the public scenario API in the shared real worker and returns the exact input snapshot',async()=>{
  vi.stubGlobal('Worker',class {constructor(){return realWorker()}})
  const {computeTrussScenario}=await import('../src/services/mainSolidWorker')
  const {forcesN:_,...structure}=bar()
  const input={nodesMm:structure.nodesMm,members:structure.members,cases:[{id:'pull',restrained:structure.restrained,
    loads:[{nodes:[1],originMm:[10,0,0] as [number,number,number],forceN:[100,0,0] as [number,number,number],momentNmm:[0,0,0] as [number,number,number]}]}],
    combinations:[],activeId:'pull'}
  const pending=computeTrussScenario(input)
  await expect(computeTrussScenario({...input,activeId:'missing'})).rejects.toMatchObject({code:'TRUSS_SCENARIO_INVALID'})
  input.nodesMm[1][0]=20;input.cases[0].loads[0].forceN[0]=200;input.activeId='missing'
  const resolved=await pending
  expect(resolved.result.displacementsMm[1][0]).toBeCloseTo(.25,12)
  expect(resolved.model.nodesMm[1][0]).toBe(10)
  expect(resolved.model.loads[0].forceN[0]).toBe(100)
  expect(resolved.activeId).toBe('pull');expect(resolved.terms).toEqual([{caseId:'pull',factor:1}])
  const controller=new AbortController();controller.abort()
  input.activeId='pull'
  await expect(computeTrussScenario(input,{signal:controller.signal})).rejects.toMatchObject({name:'AbortError'})
  expect((await computeTrussScenario(input)).result.displacementsMm[1][0]).toBeCloseTo(1,12)
  expect(workers).toHaveLength(1)
},30000)

it('terminates an entered noncooperative call and recovers with a real CAD worker',async()=>{
  const state=new Int32Array(new SharedArrayBuffer(4))
  const blocked=new Worker('const {parentPort,workerData}=require("node:worker_threads");parentPort.on("message",()=>{Atomics.store(new Int32Array(workerData),0,1);for(;;){}})',{eval:true,workerData:state.buffer})
  workers.push(blocked)
  await new Promise<void>((resolve,reject)=>{blocked.once('online',resolve);blocked.once('error',reject)})
  let first=true
  const client=new MainSolidWorkerClient(()=>{
    if(!first)return realWorker()
    first=false
    return new NodePort(blocked)
  });clients.push(client)
  await expect(client.run({kind:'truss',model:bar()},{timeoutMs:1000})).rejects.toMatchObject({code:'CAD_TIMEOUT'})
  expect(Atomics.load(state,0)).toBe(1)
  const recovered=await client.run({kind:'truss',model:bar()})
  expect(recovered.axialForcesN[0]).toBeCloseTo(100,10)
  expect(workers).toHaveLength(2)
},30000)

it('generates nominal graphs off-thread and exposes, rather than hides, their bounding-box scope',async()=>{
  const client=new MainSolidWorkerClient(realWorker);clients.push(client)
  const mesh=(points:[number,number][])=>previewMeshes({version:1,sketches:[],bodies:[extrudeDirectSketch({id:'s',name:'Box',closed:true,points},10,'0')]})[0]
  const box=mesh([[0,0],[10,0],[10,10],[0,10]]),triangle=mesh([[0,0],[10,0],[0,10]])
  const graph=await client.run({kind:'latticeGraph',mesh:box,options:latticeOptions})
  expect(graph.modelKind).toBe('nominal-bounding-box-axial')
  expect(graph.nodes).toHaveLength(14);expect(graph.edges).toHaveLength(36)
  expect(await client.run({kind:'latticeGraph',mesh:triangle,options:{...latticeOptions,skin:2,wallDepth:3,keepCore:true}})).toEqual(graph)
  const restrained=graph.nodes.map(point=>point[2]===0?[true,true,true]:[false,false,false]) as [boolean,boolean,boolean][]
  const top=graph.nodes.flatMap((point,i)=>point[2]===10?[i]:[])
  const model={nodesMm:graph.nodes,members:graph.edges.map(nodes=>({nodes,youngMpa:2000,areaMm2:2})),restrained,
    loads:[{nodes:top,originMm:[5,5,10] as [number,number,number],forceN:[0,0,-100] as [number,number,number],momentNmm:[0,0,0] as [number,number,number]}]}
  const result=await client.run({kind:'truss',model})
  expect(result.maxDeflectionMm).toBeGreaterThan(0)
  expect(result.reactionsN.reduce((sum,force)=>sum+force[2],0)).toBeCloseTo(100,9)
  expect(box.vertices.byteLength).toBeGreaterThan(0)
  const flat={...box,vertices:box.vertices.map((value,i)=>i%6===2?0:value)}
  await expect(client.run({kind:'latticeGraph',mesh:flat,options:latticeOptions})).rejects.toThrow('three-dimensional bounds')
  expect(await client.run({kind:'latticeGraph',mesh:box,options:latticeOptions})).toEqual(graph)
  expect(workers).toHaveLength(1)
},30000)

it('transfers loads across explicit solid bonds in the real worker and preserves solver refusal',async()=>{
  const client=new MainSolidWorkerClient(realWorker);clients.push(client)
  const result=await client.run({kind:'bondedSolid',inputJson:JSON.stringify(bondedSolidExample)})
  expect(result.bonds[0].forceOnShellN[2]).toBeCloseTo(3,10)
  expect(result.bonds[0].utilization).toBeCloseTo(.6,10)
  const unstable=structuredClone(bondedSolidExample);unstable.restrained.fill([false,false,false])
  await expect(client.run({kind:'bondedSolid',inputJson:JSON.stringify(unstable)})).rejects.toMatchObject({code:'BONDED_SOLID_SOLVE'})
},30000)
