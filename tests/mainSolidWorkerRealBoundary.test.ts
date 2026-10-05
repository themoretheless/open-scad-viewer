import {bondedSolidExample} from '../src/features/bondedSolidExample'
import {Worker} from 'node:worker_threads'
import {afterEach, expect, it, vi} from 'vitest'
import {MainSolidWorkerClient, type MainSolidPort} from '../src/services/mainSolidWorkerClient'
import type {MainSolidRequest} from '../src/services/mainSolidProtocol'
import type {TrussModel} from '../src/services/trussAnalysis'
import {resolveTrussScenario} from '../src/services/trussScenario'
import {serializeDirectDocument,extrudeDirectSketch} from '../src/services/directModeling'
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

it('rounds all rotated cuboid edges with endpoint radii through the real body-edit worker',async()=>{
  const {createBrepBox,transformNurbsBrep,tessellateNurbsBrep,analyzeNurbsBrep}=await import('../src/services/geometry/brep')
  const source=createBrepBox([-7,3,-2],[3,11,4]),a=.37,b=-.61
  const brep=transformNurbsBrep(source,[
    [Math.cos(a)*Math.cos(b),-Math.sin(a),Math.cos(a)*Math.sin(b),17],
    [Math.sin(a)*Math.cos(b),Math.cos(a),Math.sin(a)*Math.sin(b),-9],
    [-Math.sin(b),0,Math.cos(b),23],[0,0,0,1],
  ])
  const body={id:'rotated',name:'Rotated',brep,mesh:tessellateNurbsBrep(brep)}
  const document={version:1 as const,sketches:[],bodies:[body]},before=JSON.stringify(document)
  const client=new MainSolidWorkerClient(realWorker);clients.push(client)
  for(let edge=0;edge<brep.edges.length;edge++){
    const result=await client.run({kind:'bodyEdit',document,options:{operation:'edge-fillet',id:body.id,face:0,edges:[edge],openings:[],segments:8,distance:0,radius:.5,endRadius:1.5,filletMode:'variable',axis:'z'}})
    const [p,q]=source.edges[edge].vertices.map(i=>source.vertices[i].point)
    const length=Math.hypot(...p.map((x,i)=>x-q[i]))
    expect(result.bodies[0].id).toBe(body.id)
    expect(analyzeNurbsBrep(result.bodies[0].brep!).signedVolumeMm3).toBeCloseTo(480-(1-Math.PI/4)*length*(.25+.75+2.25)/3,4)
  }
  expect(JSON.stringify(document)).toBe(before)
},60_000)

it('prepares and extrudes polynomial rounded profiles through the shipped worker',async()=>{
  const client=new MainSolidWorkerClient(realWorker);clients.push(client)
  const points=[[[3,0],[3,3],[0,3]],[[0,3],[-3,3],[-3,0]],[[-3,0],[-3,-3],[0,-3]],[[0,-3],[3,-3],[3,0]]]
  const document={version:1 as const,sketches:[],bodies:[],curves:points.map((controlPoints,i)=>({id:`curve-${i}`,name:`Curve ${i}`,curve:{degree:2,knots:[0,0,0,1,1,1],controlPoints,weights:[1,1,1]}}))}
  const before=JSON.stringify(document)
  const prepared=await client.run({kind:'profilePrepare',document,ids:document.curves.map(c=>c.id),tolerance:0})
  expect(prepared.report.accepted).toBe(true)
  const extruded=await client.run({kind:'extrusion',document:prepared.document,options:{sketchIds:[prepared.document.sketches[0].id],height:5,offset:-2,operation:'new',targetId:'',id:'rounded'}})
  const {analyzeNurbsBrep}=await import('../src/services/geometry/brep')
  expect(analyzeNurbsBrep(extruded.bodies[0].brep!).signedVolumeMm3).toBeCloseTo(150,6)
  expect(JSON.stringify(document)).toBe(before)
},30000)

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
  await expect(client.run({kind:'latticeGraph',mesh:flat,options:latticeOptions})).rejects.toThrow(/three-dimensional bounds|3D bounds/)
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

it('retains a bounded NURBS offset through the real worker and preserves source geometry',async()=>{
 const client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const document={version:1 as const,sketches:[],bodies:[],curves:[{id:'source',name:'Line',curve:{degree:1,knots:[0,0,1,1],weights:[1,1],controlPoints:[[0,0,7],[10,0,7]]}}]}
 const before=structuredClone(document)
 const result=await client.run({kind:'curveOffset',document,options:{id:'source',createdId:'offset',distance:2,toleranceMm:.01,maxCells:256,maxPairs:1000}})
 expect(document).toEqual(before);expect(result.document.curves![0]).toEqual(before.curves[0])
 expect(result.document.curves![1]).toMatchObject({id:'offset',curve:{degree:1}})
 const points=result.document.curves![1]!.curve.controlPoints
 expect(points).toHaveLength(2)
 for(const [i,point] of points.entries()){
  expect(Math.hypot(point[0]!-i*10,point[1]!-2)).toBeLessThanOrEqual(result.report.errorUpperMm)
  expect(point[2]).toBe(7)
 }
 expect(result.report).toMatchObject({accepted:true,wholeCurve:true,regionTopologyCertified:false,offsetRegularityCertified:false})
 expect(result.report.errorUpperMm).toBeLessThanOrEqual(.01)
 const corner={...document,curves:[{...document.curves[0]!,curve:{degree:1,knots:[0,0,.5,1,1],weights:[1,1,1],controlPoints:[[0,0,7],[10,0,7],[10,10,7]]}}]}
 await expect(client.run({kind:'curveOffset',document:corner,options:{id:'source',createdId:'corner-offset',distance:2,toleranceMm:.01,maxCells:256,maxPairs:1000}})).rejects.toThrow('explicit profile join')
 const zero=await client.run({kind:'curveOffset',document,options:{id:'source',createdId:'zero',distance:0,toleranceMm:.01,maxCells:256,maxPairs:1000}})
 expect(zero.document.curves![1]!.curve).toEqual(document.curves[0]!.curve)
})

it('retains a periodic bevel offset through real WASM with explicit source roles',async()=>{
 const client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const document={version:1 as const,sketches:[],bodies:[],curves:[{id:'square',name:'Periodic square',curve:{degree:1,knots:[0,1,2,3,4,5,6],weights:[1,1,1,1,1],controlPoints:[[0,0,3],[10,0,3],[10,10,3],[0,10,3],[0,0,3]],periodic:true}}]}
 const before=structuredClone(document)
 const result=await client.run({kind:'curveOffset',document,options:{id:'square',createdId:'bevel',distance:-2,toleranceMm:1e-6,maxCells:256,maxPairs:1000,join:'bevel'}})
 expect(document).toEqual(before);expect(result.document.curves![0]).toEqual(document.curves[0])
 expect(result.report).toMatchObject({accepted:true,closed:true,wholeCurve:false,wholeWire:true,regionTrimmed:false,regionTopologyCertified:false,chainDiagnostics:{complete:true,simple:true}})
 expect(result.report.cells.filter(c=>c.source?.kind==='bevel')).toHaveLength(4)
 const points=result.document.curves![1]!.curve.controlPoints
 expect(points[0]).toEqual(points.at(-1));expect(points.every(p=>p[2]===3)).toBe(true)
})

it('reinspects edited current chords through the real worker without construction metadata',async()=>{
 const client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const curve={degree:1,knots:[0,0,1,2,3,4,4],weights:[1,1,1,1,1],controlPoints:[[0,0,7],[2,2,7],[0,2,7],[2,0,7],[0,0,7]]}
 const document={version:1 as const,bodies:[],sketches:[],curves:[{id:'chain',name:'Current chain',curve}]}
 const before=structuredClone(document)
 const report=await client.run({kind:'curveChainInspection',document,ids:['chain'],maxPairs:100})
 expect(document).toEqual(before)
 expect(report).toMatchObject({complete:true,crossings:[[0,2]],originalOffsetTopologyCertified:false})
 const corrected=structuredClone(document);corrected.curves[0]!.curve.controlPoints=[[0,0,7],[2,0,7],[2,2,7],[0,2,7],[0,0,7]]
 expect(await client.run({kind:'curveChainInspection',document:corrected,ids:['chain'],maxPairs:100})).toMatchObject({complete:true,simple:true,crossings:[]})
 expect(await client.run({kind:'curveChainInspection',document,ids:['chain'],maxPairs:1})).toMatchObject({complete:false,checks:1})
})

it('constructs trimmed offset loops through real WASM and refuses incomplete arrangements',async()=>{
 const client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const document={version:1 as const,bodies:[],sketches:[],curves:[{id:'square',name:'Square',curve:{degree:1,knots:[0,0,1,2,3,4,4],weights:[1,1,1,1,1],controlPoints:[[0,0,7],[4,0,7],[4,4,7],[0,4,7],[0,0,7]]}}]}
 const options={id:'square',createdId:'trimmed',distance:-.5,toleranceMm:1e-4,maxCells:1024,maxPairs:10000,maxWitnessChecks:100000,intersectionToleranceMm:1e-6,fillRule:'nonzero' as const}
 const before=structuredClone(document)
 const result=await client.run({kind:'trimmedCurveOffset',document,options})
 expect(document).toEqual(before);expect(result.document.curves![0]).toEqual(before.curves[0])
 expect(result.loopIds).toEqual([['trimmed:0:0']])
 const points=result.document.curves![1]!.curve.controlPoints
 expect(points[0]).toEqual(points.at(-1));expect(points.every(p=>p[2]===7)).toBe(true)
 expect(result.report).toMatchObject({regionTrimmed:true,originalOffsetTopologyCertified:false,topologyScope:'represented-reconstructed-chord-graph'})
 const crossed=structuredClone(document);crossed.curves[0]!.curve.controlPoints=[[0,0,7],[4,4,7],[0,4,7],[4,0,7],[0,0,7]]
 const pieces=await client.run({kind:'trimmedCurveOffset',document:crossed,options:{...options,distance:.1}})
 expect(pieces.loopIds.length).toBeGreaterThanOrEqual(2)
 for(const ids of pieces.loopIds){const curves=ids.map(id=>pieces.document.curves!.find(c=>c.id===id)!);expect(curves[0]!.curve.controlPoints[0]).toEqual(curves.at(-1)!.curve.controlPoints.at(-1));expect(curves.every(c=>c.offsetRegion?.scope==='at-construction')).toBe(true)}

 await expect(client.run({kind:'trimmedCurveOffset',document,options:{...options,maxPairs:1}})).rejects.toThrow()
 expect(document).toEqual(before)
})

it('proves diagonal overlap contacts through the real worker without mutating the source',async()=>{
 const client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const controlPoints=[[0,0,7],[6,6,7],[2,2,7],[2,5,7],[0,0,7]]
 const curve={degree:1,knots:[0,0,1,2,3,4,4],weights:[1,1,1,1,1],controlPoints}
 const document={version:1 as const,bodies:[],sketches:[],curves:[{id:'contact-chain',name:'Overlap',curve}]}
 const before=structuredClone(document)
 const report=await client.run({kind:'curveChainInspection',document,ids:['contact-chain'],maxPairs:100})
 expect(report).toMatchObject({method:'outward-line-pair-interval-exact/2',complete:true,simple:false,uncertain:[],originalOffsetTopologyCertified:false})
 expect(report.contacts.length).toBeGreaterThan(0)
 const limited=await client.run({kind:'curveChainInspection',document,ids:['contact-chain'],maxPairs:1})
 expect(limited).toMatchObject({complete:false,enumerationComplete:false,checks:1,simple:false})
 expect(document).toEqual(before)
})

it('distinguishes an exact interior endpoint contact from a nearby separated chord',async()=>{
 const client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const document={version:1 as const,bodies:[],sketches:[],curves:[{id:'touch',name:'Touch',curve:{degree:1,knots:[0,0,1,2,3,4,4],weights:[1,1,1,1,1],controlPoints:[[0,0,0],[4,0,0],[4,4,0],[2,0,0],[0,0,0]]}}]}
 const contact=await client.run({kind:'curveChainInspection',document,ids:['touch'],maxPairs:100})
 expect(contact).toMatchObject({complete:true,simple:false,uncertain:[]})
 expect(contact.contacts).toContainEqual([0,2])
 const translated=structuredClone(document)
 translated.curves[0]!.curve.controlPoints=translated.curves[0]!.curve.controlPoints.map(([x,y,z])=>[x!+100000000,y!-100000000,z!])
 const moved=await client.run({kind:'curveChainInspection',document:translated,ids:['touch'],maxPairs:100})
 expect(moved.contacts).toEqual(contact.contacts);expect(moved.uncertain).toEqual([])
 const separated=structuredClone(document);separated.curves[0]!.curve.controlPoints[3]=[2,.00001,0]
 const clear=await client.run({kind:'curveChainInspection',document:separated,ids:['touch'],maxPairs:100})
 expect(clear).toMatchObject({complete:true,simple:true,contacts:[],crossings:[],uncertain:[]})
})

it('transforms typed scene meshes through postMessage without detaching or changing source buffers',async()=>{
 const client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const body=extrudeDirectSketch({id:'s',name:'Box',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]},10,'a')
 const positions=Float64Array.from(body.mesh.positions),indices=Uint32Array.from(body.mesh.indices)
 body.mesh={positions,indices};const before=Array.from(positions)
 const result=await client.run({kind:'sceneEdit',document:{version:1,sketches:[],bodies:[body]},options:{operation:'transform',id:'a',ids:['a'],createdId:'',x:5,y:0,z:0,axis:'z',angle:0,scale:1}})
 expect(Array.from(positions)).toEqual(before);expect(indices.byteLength).toBeGreaterThan(0)
 const xs=Array.from(result.bodies[0].mesh.positions).filter((_,i)=>i%3===0)
 expect(Math.min(...xs)).toBe(5);expect(Math.max(...xs)).toBe(15)
 expect(result.bodies[0].id).toBe('a')
 const repeated=await client.run({kind:'sceneEdit',document:result,options:{operation:'transform',id:'a',ids:['a'],createdId:'',x:5,y:0,z:0,axis:'z',angle:0,scale:1}})
 expect(Math.min(...Array.from(repeated.bodies[0].mesh.positions).filter((_,i)=>i%3===0))).toBe(10)
 expect(Math.min(...Array.from(result.bodies[0].mesh.positions).filter((_,i)=>i%3===0))).toBe(5)
 const linked=await client.run({kind:'sceneEdit',document:result,options:{operation:'instance-create',id:'a',ids:['a'],createdId:'linked',x:20,y:0,z:0,axis:'z',angle:0,scale:1}})
 const options={operation:'transform' as const,id:'a',ids:['a'],createdId:'',x:1,y:0,z:0,axis:'z' as const,angle:0,scale:1}
 const first=await client.run({kind:'sceneEdit',document:linked,options})
 const second=await client.run({kind:'sceneEdit',document:linked,options})
 expect(Array.from(second.bodies[1].mesh.positions)).toEqual(Array.from(first.bodies[1].mesh.positions))
 expect(second.bodies[1].instance?.sourceId).toBe('a')
 expect(Math.min(...Array.from(second.bodies[1].mesh.positions).filter((_,i)=>i%3===0))).toBe(26)
 const beforeDetach=structuredClone(second)
 const detached=await client.run({kind:'sceneEdit',document:second,options:{...options,operation:'instance-detach',id:'linked',ids:['linked']}})
 expect(detached.bodies[1].instance).toBeUndefined()
 expect(detached.bodies[1].id).toBe('linked')
 expect(detached.bodies[1].brep).toEqual(second.bodies[1].brep)
 expect(Array.from(detached.bodies[1].mesh.positions)).toEqual(Array.from(second.bodies[1].mesh.positions))
 expect(second).toEqual(beforeDetach)
 const changedSource=await client.run({kind:'sceneEdit',document:detached,options})
 expect(changedSource.bodies[1]).toEqual(detached.bodies[1])
 expect(Array.from(changedSource.bodies[0].mesh.positions)).not.toEqual(Array.from(detached.bodies[0].mesh.positions))
},30000)

it('changes groups through the worker without changing geometry, identities or instance links',async()=>{
 const client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const source=extrudeDirectSketch({id:'sketch',name:'Box',closed:true,points:[[0,0],[2,0],[2,3],[0,3]]},4,'source')
 const options={operation:'instance-create' as const,id:'source',ids:['source'],createdId:'linked',x:10,y:0,z:0,axis:'z' as const,angle:0,scale:1}
 const linked=await client.run({kind:'sceneEdit',document:{version:1,sketches:[],bodies:[source]},options})
 const before=structuredClone(linked)
 const grouped=await client.run({kind:'sceneEdit',document:linked,options:{...options,operation:'group-move',ids:['linked'],group:'Assembly'}})
 expect(grouped.bodies[0]).toEqual(linked.bodies[0])
 expect(grouped.bodies[1]).toEqual({...linked.bodies[1],group:'Assembly'})
 expect(linked).toEqual(before)
 const created=await client.run({kind:'sceneEdit',document:grouped,options:{...options,operation:'group-create',group:'Empty'}})
 expect(created.groups).toEqual([{name:'Empty',source:''}]);expect(created.bodies).toEqual(grouped.bodies)
 const ungrouped=await client.run({kind:'sceneEdit',document:created,options:{...options,operation:'group-move',ids:['linked'],group:''}})
 expect(ungrouped.bodies).toEqual(linked.bodies)
 await expect(client.run({kind:'sceneEdit',document:created,options:{...options,operation:'group-create',group:'Empty'}})).rejects.toThrow('Group already exists')
 await expect(client.run({kind:'sceneEdit',document:created,options:{...options,operation:'group-create',group:''}})).rejects.toThrow('Invalid object group')
})

it('accepts compact scene snapshots with exact parity and rejects invalid geometry before metadata edits',async()=>{
 const client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const source=extrudeDirectSketch({id:'sketch',name:'Box',closed:true,points:[[0,0],[2,0],[2,3],[0,3]]},4,'source')
 const options={operation:'instance-create' as const,id:'source',ids:['source'],createdId:'linked',x:10,y:0,z:0,axis:'z' as const,angle:0,scale:1}
 const linked=await client.run({kind:'sceneEdit',document:{version:1,sketches:[],bodies:[source]},options})
 const text=serializeDirectDocument(linked),compact=JSON.parse(text)
 expect(compact.bodies[1].mesh).toBeUndefined();expect(compact.bodies[1].brep).toBeUndefined()
 for(const operation of ['group-move','group-create','instance-detach','transform','instance-transform','instance-create','instance-place'] as const){
  const edit={...options,operation,id:operation==='instance-transform'||operation==='instance-place'?'linked':'source',createdId:'another-link',ids:['linked'],group:'Assembly'}
  const full=await client.run({kind:'sceneEdit',document:linked,options:edit})
  const restored=await client.run({kind:'sceneEdit',document:text,options:edit})
  expect(restored).toEqual(full)
 }
 compact.bodies[0].mesh.indices[0]=99999
 await expect(client.run({kind:'sceneEdit',document:JSON.stringify(compact),options:{...options,operation:'group-create',group:'Unsafe'}})).rejects.toThrow()
 expect(linked.bodies[1].instance?.sourceId).toBe('source')
})
it('revolves retained holed profiles through postMessage and preserves source controls and identities',async()=>{
 const {warmGeometryKernel}=await import('../src/services/geometry/kernel')
 const {authorBrepProfile}=await import('../src/services/geometry/brepProfile')
 const {withRetainedProfile}=await import('../src/services/retainedSketchProfile')
 const {applySolidRevolve}=await import('../src/services/solidRevolve')
 await warmGeometryKernel()
 const profile=authorBrepProfile({kind:'polygon',rings:[[[3,0],[6,0],[6,4],[3,4]],[[4,1],[4,3],[5,3],[5,1]]]})
 const sketch=withRetainedProfile({id:'profile',name:'Holed',closed:true,points:[]},profile)
 const document={version:1 as const,bodies:[],sketches:[sketch]},before=structuredClone(document)
 const options={sketchId:'profile',geometry:'exact' as const,operation:'new' as const,targetId:'',id:'result',name:'Revolved',tessellation:2,axis:'y' as const,offset:0,angle:90,segments:32}
 const client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const result=await client.run({kind:'revolve',document,options})
 expect(result).toEqual(applySolidRevolve(document,options))
 expect(result.bodies[0].id).toBe('result')
 expect(result.bodies[0].brep!.faces.filter(face=>face.holes.length===1)).toHaveLength(2)
 expect(result.sketches[0].retainedProfile).toEqual(profile)
 expect(document).toEqual(before)
 await expect(client.run({kind:'revolve',document,options:{...options,offset:4}})).rejects.toThrow(/one side of its axis/)
 expect(document).toEqual(before)
})
it('builds a retained holed loft through postMessage without changing source profiles',async()=>{
 const {retainedLoftFixture}=await import('./support/retainedLoftFixture')
 const {applySolidSceneEdit}=await import('../src/services/solidSceneEdit')
 const sketches=await retainedLoftFixture()
 const document={version:1 as const,bodies:[],sketches},before=structuredClone(document)
 const options={operation:'loft' as const,id:'base',ids:['base','top'],createdId:'loft',x:0,y:0,z:0,axis:'z' as const,angle:0,scale:1}
 const client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const result=await client.run({kind:'sceneEdit',document,options})
 expect(result).toEqual(applySolidSceneEdit(document,options))
 expect(result.bodies[0].id).toBe('loft')
 expect(result.bodies[0].brep!.faces.filter(face=>face.holes.length===1)).toHaveLength(2)
 expect(result.sketches).toEqual(sketches)
 expect(document).toEqual(before)
 await expect(client.run({kind:'sceneEdit',document,options:{...options,ids:['top','base']}})).rejects.toThrow(/positive sketch normal/)
 expect(document).toEqual(before)
})

it('preserves retained profile segment provenance across the real worker boundary',async()=>{
 const {prepareSolidProfile}=await import('../src/services/solidProfilePreparation')
 const {warmGeometryKernel}=await import('../src/services/geometry/kernel')
 await warmGeometryKernel()
 const document={version:1 as const,bodies:[],sketches:[
  {id:'arc',name:'Arc',closed:false,points:[[999,999],[998,998]] as [number,number][],analytic:{kind:'arc' as const,center:[0,0] as [number,number],radius:2,start:0,sweep:-180}},
  {id:'line',name:'Line',closed:false,points:[[-2,0],[2,0]] as [number,number][]},
 ]},before=structuredClone(document)
 const client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const result=await client.run({kind:'profilePrepare',document,ids:['arc','line'],tolerance:0})
 expect(result).toEqual(prepareSolidProfile(document,['arc','line'],0))
 expect(result.report.accepted).toBe(true)
 expect(result.report.curveSources).toHaveLength(3)
 expect(result.report.curveSources!.filter(s=>s.chain===0).every(s=>s.reversed)).toBe(true)
 expect(result.document.sketches[0].id).toBe('arc')
 expect(document).toEqual(before)
})

it('retains multispan trim proofs and incomplete budgets across real postMessage',async()=>{
 const {readFileSync}=await import('node:fs')
 const cases=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/solid-distance-2026-09-30/contract-fixtures.json',import.meta.url),'utf8')).cases
 const options=structuredClone(cases[3].request)
 const coedge=options.a.loops[options.a.faces[0].outer].coedges[0],curve=coedge.pcurve,[a,b]=curve.controlPoints
 coedge.pcurve={degree:2,knots:[0,0,0,.5,1,1,1],controlPoints:[0,.25,.75,1].map(t=>a.map((x:number,i:number)=>x*(1-t)+b[i]*t)),weights:[1,1,1,1],periodic:false}
 const before=structuredClone(options),client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const result=await client.run({kind:'solidDistance',options})
 expect(result.validity[0].trimValid).toBe(true)
 const limited=await client.run({kind:'solidDistance',options:{...options,validityLimits:{...options.validityLimits,trimCells:1}}})
 expect(limited.validity[0].trimValid).toBe(false)
 expect(limited.converged).toBe(false)
 expect(limited.reason).toBe('volume-validity-unproven')
 expect(options).toEqual(before)
})

it('prepares original general NURBS through the real worker boundary',async()=>{
 const {emptyDirectDocument}=await import('../src/services/directModeling')
 const {prepareSolidProfile}=await import('../src/services/solidProfilePreparation')
 const {warmGeometryKernel}=await import('../src/services/geometry/kernel');await warmGeometryKernel()
 const document=emptyDirectDocument()
 document.curves=[{id:'nurbs',name:'NURBS',curve:{degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0],[1,-1],[2,0]],weights:[1,1,1]}}]
 document.sketches=[{id:'line',name:'Line',closed:false,points:[[2,0],[2,2],[0,2],[0,0]]}]
 const before=structuredClone(document),client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const result=await client.run({kind:'profilePrepare',document,ids:['nurbs','line'],tolerance:0})
 expect(result).toEqual(prepareSolidProfile(document,['nurbs','line'],0))
 expect(result.report.accepted).toBe(true)
 expect(result.document.sketches[0].id).toBe('nurbs')
 expect(result.document.curves).toEqual([])
 expect(document).toEqual(before)
})

it('converges oblique sphere distances across the real worker boundary',async()=>{
 const {obliqueSphereRequests}=await import('./fixtures/oblique-sphere-distance')
 const {validSolidDistance,solidDistanceExpectation}=await import('../src/services/solidDistance')
 const client=new MainSolidWorkerClient(realWorker);clients.push(client)
 for(const {options,expected} of await obliqueSphereRequests()){
  const before=structuredClone(options),r=await client.run({kind:'solidDistance',options})
  expect(validSolidDistance(solidDistanceExpectation(options),r)).toBe(true)
  expect(r.converged,JSON.stringify(r)).toBe(true)
  expect(r.distanceIntervalMm![0]).toBeLessThanOrEqual(expected)
  expect(r.distanceIntervalMm![1]).toBeGreaterThanOrEqual(expected)
  expect(r.distanceIntervalMm![1]-r.distanceIntervalMm![0]).toBeLessThanOrEqual(options.toleranceMm)
  expect(r.separationWitness).not.toBeNull()
  expect(options).toEqual(before)
 }
})

it('certifies every parameter of an offset contact band through the actual worker and WASM',async()=>{
 const a={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false}
 const b=structuredClone(a);for(const row of b.controlPoints)for(const p of row){const z=p[1];p[1]=.5;p[2]=z}
 const options={a,b,distances:[.2,.2] as [number,number],fixedAxis:0 as const,fixedInterval:[.35,.39] as [number,number],firstOther:[.25,.35] as [number,number],secondDomain:[[.30,.44],[.15,.25]] as [[number,number],[number,number]],maxSpans:2}
 const before=structuredClone(options),client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const pending=client.run({kind:'offsetContactBand',options})
 // postMessage and the expectation must retain their own request snapshot.
 options.fixedInterval[0]=.36
 options.secondDomain[0][0]=.31
 const r=await pending
 expect(r).toMatchObject({status:'continuous-branch',rootForEveryParameterProven:true,continuousBranchProven:true,wholeCurveComplete:false,trimMembershipProven:false,topologyAuthority:false})
 expect(r.witness!.firstUV[0]).toEqual(before.fixedInterval)
 options.fixedInterval[0]=before.fixedInterval[0];options.secondDomain[0][0]=before.secondDomain[0][0]
 expect(options).toEqual(before)
 const narrow=await client.run({kind:'offsetContactBand',options:{...options,secondDomain:[[.36,.38],[.15,.25]]}})
 expect(narrow).toMatchObject({status:'unresolved',rootForEveryParameterProven:false,witness:null})
 await expect(client.run({kind:'offsetContactBand',options:{...options,maxSpans:0}})).rejects.toMatchObject({code:'NURBS_INVALID_INPUT'})
 expect((await client.run({kind:'offsetContactBand',options})).status).toBe('continuous-branch')
 // Capture a genuinely successful worker reply, cancel before its admission,
 // then deliver it through the original callback. Cancellation must stay final.
 const heldPort=realWorker(),heldClient=new MainSolidWorkerClient(()=>heldPort);clients.push(heldClient)
 const cancelled=heldClient.run({kind:'offsetContactBand',options})
 const rejection=expect(cancelled).rejects.toMatchObject({code:'CAD_CANCELLED'})
 const callback=heldPort.onmessage!
 let release!:(event:MessageEvent)=>void
 const captured=new Promise<MessageEvent>(resolve=>{release=resolve})
 heldPort.onmessage=event=>release(event)
 const event=await captured
 expect(event.data.ok).toBe(true)
 heldClient.cancel();callback(event)
 await rejection
 expect((await client.run({kind:'offsetContactBand',options})).status).toBe('continuous-branch')
},30_000)

it('admits a trimmed offset contact band through real WASM and worker with holes and bounded work',async()=>{
 const a={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false}
 const b=structuredClone(a);for(const row of b.controlPoints)for(const p of row){const z=p[1];p[1]=.5;p[2]=z}
 const rectangle=(lo:[number,number],hi:[number,number],reverse=false)=>{
  const p=[lo,[hi[0],lo[1]],hi,[lo[0],hi[1]]];if(reverse)p.reverse()
  return p.map((point,i)=>({degree:1,knots:[0,0,1,1],controlPoints:[point,p[(i+1)%4]],weights:[1,1],periodic:false}))
 }
 const outer=rectangle([0,0],[1,1]),options={a,b,distances:[.2,.2] as [number,number],fixedAxis:0 as const,fixedInterval:[.35,.39] as [number,number],firstOther:[.25,.35] as [number,number],secondDomain:[[.30,.44],[.15,.25]] as [[number,number],[number,number]],maxSpans:2,firstLoops:[outer],secondLoops:[outer],toleranceUv:1e-7,maxPairs:10000,maxCells:10000,maxDomainCells:100000}
 const client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const before=structuredClone(options),r=await client.run({kind:'trimmedOffsetContactBand',options})
 expect(r).toMatchObject({trimMembershipProven:true,continuousBranchProven:true,topologyAuthority:false,worldCoedgeIdentityProven:false})
 const hole=await client.run({kind:'trimmedOffsetContactBand',options:{...options,firstLoops:[outer,rectangle([.34,.29],[.40,.31],true)]}})
 expect(hole).toMatchObject({trimMembershipProven:false,reason:'contact-outside-trim'})
 const crossing=await client.run({kind:'trimmedOffsetContactBand',options:{...options,firstLoops:[rectangle([.36,.1],[.9,.9])]}})
 expect(crossing).toMatchObject({trimMembershipProven:false,reason:'contact-trim-unresolved'})
 const cap=await client.run({kind:'trimmedOffsetContactBand',options:{...options,maxPairs:1,maxCells:1,maxDomainCells:1}})
 expect(cap.trimMembershipProven).toBe(false);expect(cap.cells).toBeLessThanOrEqual(1)
 await expect(client.run({kind:'trimmedOffsetContactBand',options:{...options,toleranceUv:0}})).rejects.toMatchObject({code:'NURBS_INVALID_INPUT'})
 expect((await client.run({kind:'trimmedOffsetContactBand',options})).trimMembershipProven).toBe(true)
 expect(options).toEqual(before)
},30_000)

it('links original source coedges through real WASM and worker without promoting tolerance agreement',async()=>{
 const a={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false}
 const b=structuredClone(a);for(const row of b.controlPoints)for(const p of row){const z=p[1];p[1]=.5;p[2]=z}
 const p=[[0,0],[1,0],[1,1],[0,1]],outer=p.map((point,i)=>({degree:1,knots:[0,0,1,1],controlPoints:[point,p[(i+1)%4]],weights:[1,1],periodic:false}))
 const world=(side:number)=>outer.map(c=>({world:{...c,controlPoints:c.controlPoints.map(uv=>side===0?[uv[0],uv[1],0]:[uv[0],.5,uv[1]])},reversed:false}))
 const options={a,b,distances:[.2,.2] as [number,number],fixedAxis:0 as const,fixedInterval:[.35,.39] as [number,number],firstOther:[.25,.35] as [number,number],secondDomain:[[.30,.44],[.15,.25]] as [[number,number],[number,number]],maxSpans:2,firstLoops:[outer],secondLoops:[outer],firstCoedges:[world(0)],secondCoedges:[world(1)],toleranceUv:1e-7,maxPairs:10000,maxCells:10000,maxDomainCells:100000,toleranceMm:1e-5,maxExactWork:1000000,maxAgreementCells:10000}
 const before=structuredClone(options),client=new MainSolidWorkerClient(realWorker);clients.push(client)
 const r=await client.run({kind:'offsetSourceBoundary',options})
 expect(r).toMatchObject({worldCoedgeIdentityProven:true,worldBoundaryWithinToleranceProven:true,checkedCoedges:8,totalCoedges:8,topologyAuthority:false})
 const changed=structuredClone(options);changed.secondCoedges[0][0].world.controlPoints[0][1]=.501
 const mismatch=await client.run({kind:'offsetSourceBoundary',options:changed})
 expect(mismatch).toMatchObject({worldCoedgeIdentityProven:false,worldBoundaryWithinToleranceProven:false,reason:'spatial-boundary-mismatch'})
 const tolerance=await client.run({kind:'offsetSourceBoundary',options:{...options,maxExactWork:0}})
 expect(tolerance).toMatchObject({worldCoedgeIdentityProven:false,worldBoundaryWithinToleranceProven:true})
 const cap=await client.run({kind:'offsetSourceBoundary',options:{...options,maxExactWork:0,maxAgreementCells:1}})
 expect(cap).toMatchObject({worldCoedgeIdentityProven:false,worldBoundaryWithinToleranceProven:false,reason:'boundary-work-limit'})
 await expect(client.run({kind:'offsetSourceBoundary',options:{...options,secondCoedges:[]}})).rejects.toMatchObject({code:'NURBS_INVALID_INPUT'})
 expect((await client.run({kind:'offsetSourceBoundary',options})).worldCoedgeIdentityProven).toBe(true)
 expect(options).toEqual(before)
},30_000)
