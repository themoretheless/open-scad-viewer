import {Worker} from 'node:worker_threads'
import {afterEach, expect, it} from 'vitest'
import {MainSolidWorkerClient, type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import type {MainSolidRequest} from '../src/services/mainSolidProtocol'
import type {TrussModel} from '../src/services/trussAnalysis'
import {extrudeDirectSketch} from '../src/services/directModeling'
import {previewMeshes} from '../src/services/mainModeling'

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
function realWorker(){
  const worker=new Worker(new URL('./fixtures/web-worker-node-harness.mjs',import.meta.url),{
    workerData:{entryUrl:new URL('../src/workers/mainSolid.worker.ts',import.meta.url).href,announceReady:false},
  })
  workers.push(worker);return new NodePort(worker)
}
afterEach(async()=>{clients.splice(0).forEach(c=>c.dispose());await Promise.all(workers.splice(0).map(w=>w.terminate()))})

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
    expect(workers).toHaveLength(1)
  } finally {clearInterval(timer)}
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
