import {expect,it} from 'vitest'
import {createMainSolidWorkerHandler} from '../src/services/mainSolidWorkerRuntime'
import type {MainSolidResponse} from '../src/services/mainSolidProtocol'
import type {TrussModel} from '../src/services/trussAnalysis'

it('rejects overlapping requests during initialization and recovers after typed errors',async()=>{
  const messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
  const model:TrussModel={nodesMm:[[0,0,0],[10,0,0]],members:[{nodes:[0,1],youngMpa:2000,areaMm2:2}],
    restrained:[[true,true,true],[false,true,true]],forcesN:[[0,0,0],[100,0,0]]}
  const first=handle({version:1,id:1,job:{kind:'truss',model}})
  await handle({version:1,id:2,job:{kind:'truss',model}})
  expect(messages[0]).toMatchObject({version:1,id:2,kind:'truss',ok:false,error:{code:'CAD_BUSY'}})
  await first
  expect(messages[1]).toMatchObject({version:1,id:1,kind:'truss',ok:true,result:{freeDofs:1}})
  const unstable={...model,restrained:[[true,true,true],[false,false,false]]}
  await handle({version:1,id:3,job:{kind:'truss',model:unstable}})
  expect(messages[2]).toMatchObject({id:3,ok:false,error:{name:'GeometryKernelError',code:'TRUSS_SINGULAR'}})
  await handle({version:1,id:4,job:{kind:'truss',model}})
  expect(messages[3]).toMatchObject({id:4,ok:true,result:{freeDofs:1}})
  for(const malformed of [null,{}, {version:2,id:5,job:{kind:'truss',model}},
    {version:1,id:-1,job:{kind:'truss',model}}, {version:1,id:5,job:{kind:'unknown'}}])await handle(malformed)
  expect(messages).toHaveLength(4)
})

it('runs contact inspection through the worker handler and warmed WASM',async()=>{
 const messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 const mesh={positions:new Float64Array([0,0,0,2,0,0,0,2,0]),indices:new Uint32Array([0,1,2,0,1,2])}
 await handle({version:1,id:1,job:{kind:'meshContacts',mesh}})
 expect(messages[0]).toMatchObject({ok:true,kind:'meshContacts',result:{contact:{triangles:[0,1]},scope:'display-mesh-all-contacts',complete:true}})
 expect(mesh.indices.byteLength).toBe(24)
})

it('extrudes a document in the worker without mutating its source',async()=>{
 const messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 const document={version:1 as const,sketches:[{id:'profile',name:'Profile',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]}],bodies:[]}
 const before=JSON.stringify(document)
 await handle({version:1,id:1,job:{kind:'extrusion',document,options:{sketchIds:['profile'],height:23,offset:0,operation:'new',targetId:'',id:'created'}}})
 expect(messages[0]).toMatchObject({ok:true,kind:'extrusion',result:{bodies:[{id:'created'}]}})
 expect(JSON.stringify(document)).toBe(before)
})

it('uses the same exact revolve result for new, union and complete subtraction',async()=>{
 const {inspectPolygonMesh}=await import('../src/services/geometry/polygon')
 const messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 const document={version:1 as const,sketches:[{id:'profile',name:'Profile',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]}],bodies:[]}
 const options={sketchId:'profile',axis:'y' as const,offset:0,angle:360,segments:32,geometry:'exact' as const,operation:'new' as const,targetId:'',id:'revolved',name:'Exact revolve',tessellation:8}
 await handle({version:1,id:1,job:{kind:'revolve',document,options}})
 expect(messages[0]).toMatchObject({ok:true,kind:'revolve'})
 const built=(messages[0] as any).result
 expect(built.bodies).toHaveLength(1)
 expect(inspectPolygonMesh(built.bodies[0].mesh).closed).toBe(true)
 const before=JSON.stringify(built)
 await handle({version:1,id:2,job:{kind:'revolve',document:built,options:{...options,operation:'union',targetId:'revolved'}}})
 expect(messages[1]).toMatchObject({ok:true,result:{bodies:[{id:'revolved'}]}})
 await handle({version:1,id:3,job:{kind:'revolve',document:built,options:{...options,operation:'difference',targetId:'revolved'}}})
 expect(messages[2]).toMatchObject({ok:true,result:{bodies:[]}})
 expect(JSON.stringify(built)).toBe(before)
})

it('splits a body in the worker, retaining its identity and source snapshot',async()=>{
 const {extrudeDirectSketch}=await import('../src/services/directModeling')
 const {inspectPolygonMesh}=await import('../src/services/geometry/polygon')
 const sketch={id:'s',name:'Square',closed:true,points:[[0,0],[10,0],[10,10],[0,10]] as [number,number][]}
 const document={version:1 as const,sketches:[sketch],bodies:[extrudeDirectSketch(sketch,10,'stock')]}
 const before=JSON.stringify(document),messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 await handle({version:1,id:1,job:{kind:'bodyEdit',document,options:{operation:'split',id:'stock',face:0,edges:[0],openings:[0],segments:12,distance:5,radius:1,endRadius:2,filletMode:'constant',axis:'z'}}})
 expect(messages[0]).toMatchObject({ok:true,kind:'bodyEdit',result:{bodies:[{id:'stock'},{id:'preview-split'}]}})
 expect((messages[0] as any).result.bodies.map((b:any)=>inspectPolygonMesh(b.mesh).signedVolumeMm3)).toEqual([500,500])
 expect(JSON.stringify(document)).toBe(before)
})

it('computes profile difference and retained offset in the worker with holes and stable IDs',async()=>{
 const {sampleCurve}=await import('../src/services/directSketchGeometry')
 const circle={kind:'circle' as const,center:[4,3] as [number,number],radius:1,start:0,sweep:360}
 const document={version:1 as const,sketches:[{id:'plate',name:'Plate',closed:true,points:[[0,0],[8,0],[8,6],[0,6]]},{id:'hole',name:'Hole',closed:true,analytic:circle,points:sampleCurve(circle)}],bodies:[]}
 const options={operation:'profile-difference',id:'plate',inputs:['plate','hole'],distance:.5,end:'end',cx:0,cy:0,radius:1,start:0,sweep:360}
 const before=JSON.stringify(document),messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 await handle({version:1,id:1,job:{kind:'profileEdit',document,options}})
 expect(messages[0]).toMatchObject({ok:true,kind:'profileEdit'})
 const result=(messages[0] as any).result
 expect(result.sketches).toHaveLength(1);expect(result.sketches[0].id).toBe('plate')
 expect(result.sketches[0].retainedProfile.loops).toHaveLength(2)
 expect(result.sketches[0].retainedProfile.areaMm2).toBeCloseTo(48-Math.PI,9)
 await handle({version:1,id:2,job:{kind:'profileEdit',document:result,options:{...options,operation:'offset'}}})
 expect(messages[1]).toMatchObject({ok:true})
 expect((messages[1] as any).result.sketches[0].retainedProfile.areaMm2).toBeCloseTo(62,9)
 expect(JSON.stringify(document)).toBe(before)
})

it('returns profile gap diagnostics and the accepted contour through the worker protocol',async()=>{
 const {mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const document={version:1 as const,sketches:[{id:'a',name:'A',closed:false,points:[[0,0],[10,0],[10,10]]},{id:'b',name:'B',closed:false,points:[[10,10],[0,10],[0,.005]]}],bodies:[]}
 const before=JSON.stringify(document),messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 for(const [i,tolerance] of [.001,.01].entries())await handle({version:1,id:i+1,job:{kind:'profilePrepare',document,ids:['a','b'],tolerance}})
 expect(messages[0]).toMatchObject({ok:true,kind:'profilePrepare',result:{report:{accepted:false,reason:'endpoint-topology'}}})
 expect(messages[1]).toMatchObject({ok:true,result:{report:{accepted:true,reason:'accepted'},document:{sketches:[{id:'a',closed:true}]}}})
 for(const response of messages){expect(response.ok).toBe(true);if(response.ok)expect(mainSolidResult({kind:'profilePrepare'},response.result)).toBe(true)}
 expect(JSON.stringify(document)).toBe(before)
})

it('returns validated NURBS refit certificates for curve and surface edits',async()=>{
 const {createSolidNurbsCurve}=await import('../src/services/solidNurbs')
 const {mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const curve=createSolidNurbsCurve('curve');curve.curve.controlPoints=[[0,0,0],[1,0,0],[2,0,0],[3,0,0]]
 const surface={id:'surface',name:'Plane',segmentsU:4,segmentsV:4,surface:{degreeU:2,degreeV:2,knotsU:[0,0,0,1,1,1],knotsV:[0,0,0,1,1,1],controlPoints:Array.from({length:3},(_,i)=>Array.from({length:3},(_,j)=>[i,j,0])),weights:Array.from({length:3},()=>[1,1,1])}}
 const document={version:1 as const,sketches:[],bodies:[],curves:[curve],surfaces:[surface]},before=JSON.stringify(document)
 const messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 for(const [index,operation] of ['nurbs-reduce','nurbs-rebuild','nurbs-surface-reduce','nurbs-surface-rebuild'].entries()){
  await handle({version:1,id:index+1,job:{kind:'nurbsRefit',document,options:{operation,id:operation.includes('surface')?'surface':'curve',axis:'v',degree:1,controlCount:6,maxError:.1}}})
  const response=messages.at(-1)!
  expect(response).toMatchObject({ok:true,result:{certificate:{accepted:true,rolledBack:false}}})
  if(response.ok)expect(mainSolidResult({kind:'nurbsRefit'},response.result)).toBe(true)
 }
 expect(JSON.stringify(document)).toBe(before)
})

it('preserves the sweep refinement report on worker refusal and success',async()=>{
 const {mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const curves=[{id:'p',name:'Profile',curve:{degree:1,controlPoints:[[1,0,0],[1.2,0,0]],weights:[1,1],knots:[0,0,1,1]}},{id:'q',name:'Path',curve:{degree:2,controlPoints:[[1,0,0],[1,1,0],[0,1,0]],weights:[1,Math.SQRT1_2,1],knots:[0,0,0,1,1,1]}}]
 const document={version:1,sketches:[],bodies:[],curves},before=JSON.stringify(document)
 const messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 for(const [i,sections] of [3,32].entries()){
  await handle({version:1,id:i+1,job:{kind:'surfaceBuild',document,options:{kind:'nurbs-sweep',ids:['p','q'],id:'surface',reversed:[],sweep:{mode:'framed',normal:[0,0,1],sections,maxDeviation:.01}}}})
  const response=messages.at(-1)!;expect(response.ok).toBe(true)
  if(response.ok){expect(mainSolidResult({kind:'surfaceBuild'},response.result)).toBe(true);expect((response.result as any).report.accepted).toBe(i===1)}
 }
 expect((messages[0] as any).result.document).toBe(null)
 expect((messages[1] as any).result.document.surfaces[0].id).toBe('surface')
 expect(JSON.stringify(document)).toBe(before)
})

it('carries curve, surface and seam proof reports through the actual worker protocol',async()=>{
 const {createSolidNurbsCurve}=await import('../src/services/solidNurbs')
 const {mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const a=createSolidNurbsCurve('a'),b=createSolidNurbsCurve('b');b.curve.controlPoints.forEach(p=>p[0]+=50)
 const surface={degreeU:3,degreeV:3,knotsU:[0,0,0,0,1,1,1,1],knotsV:[0,0,0,0,1,1,1,1],controlPoints:Array.from({length:4},(_,i)=>Array.from({length:4},(_,j)=>[i,j,.15*i*i+.1*i*j+.2*j*j])),weights:Array.from({length:4},(_,i)=>Array.from({length:4},(_,j)=>1+.03*i+.02*j+.01*i*j))}
 const sa={id:'sa',name:'A',surface,segmentsU:4,segmentsV:4},sb=structuredClone(sa);sb.id='sb';sb.surface.controlPoints.forEach(row=>row.forEach(p=>p[0]+=5))
 const document={version:1,sketches:[],bodies:[],curves:[a,b],surfaces:[sa,sb]},before=JSON.stringify(document)
 const messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 const options={referenceBoundary:'uMax',editedBoundary:'uMin',order:2,scale:1,reverse:false,maxError:1e-6}
 const jobs=[{kind:'curveMatch',args:[document,'a','b','end','start',1e-6]}, {kind:'surfaceMatch',args:[document,'sa','sb',options]}, {kind:'seamPrepare',args:[document,'sa','sb',options]}]
 for(const [index,job] of jobs.entries()){
  await handle({version:1,id:index+1,job})
  const response=messages.at(-1)!;expect(response).toMatchObject({ok:true,result:{report:{accepted:true}}})
  if(response.ok)expect(mainSolidResult({kind:job.kind} as any,response.result)).toBe(true)
 }
 expect(JSON.stringify(document)).toBe(before)
})

it('updates linked geometry in the scene worker preview without changing its source snapshot',async()=>{
 const {extrudeDirectSketch}=await import('../src/services/directModeling')
 const sketch={id:'s',name:'Square',closed:true,points:[[0,0],[10,0],[10,10],[0,10]] as [number,number][]}
 const document={version:1 as const,sketches:[sketch],bodies:[extrudeDirectSketch(sketch,10,'source')]},before=JSON.stringify(document)
 const options={operation:'instance-create',id:'source',ids:['source'],createdId:'linked',x:20,y:0,z:0,axis:'z',angle:0,scale:1}
 const messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 await handle({version:1,id:1,job:{kind:'sceneEdit',document,options}})
 expect(messages[0]).toMatchObject({ok:true,result:{bodies:[{id:'source'},{id:'linked',instance:{sourceId:'source'}}]}})
 const linked=(messages[0] as any).result,original=JSON.stringify(linked)
 await handle({version:1,id:2,job:{kind:'sceneEdit',document:linked,options:{...options,operation:'transform',x:5}}})
 const next=(messages[1] as any).result
 const minX=(body:any)=>Math.min(...Array.from(body.mesh.positions as ArrayLike<number>).filter((_,i)=>i%3===0))
 expect(minX(next.bodies[0])).toBeCloseTo(5);expect(minX(next.bodies[1])).toBeCloseTo(25)
 expect(next.bodies[1].instance.matrix[0][3]).toBe(20)
 expect(JSON.stringify(linked)).toBe(original);expect(JSON.stringify(document)).toBe(before)
})

it('builds a ruled solid loft in the worker with its requested identity',async()=>{
 const {analyzeNurbsBrep}=await import('../src/services/geometry/brep')
 const points=[[0,0],[10,0],[10,10],[0,10]]
 const document={version:1,sketches:[{id:'a',name:'A',closed:true,points},{id:'b',name:'B',closed:true,points,plane:{origin:[0,0,5],u:[1,0,0],v:[0,1,0]}}],bodies:[]},before=JSON.stringify(document)
 const messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 await handle({version:1,id:1,job:{kind:'sceneEdit',document,options:{operation:'loft',id:'a',ids:['a','b'],createdId:'loft',x:0,y:0,z:0,axis:'z',angle:0,scale:1}}})
 expect(messages[0]).toMatchObject({ok:true,result:{bodies:[{id:'loft'}]}})
 expect(analyzeNurbsBrep((messages[0] as any).result.bodies[0].brep).signedVolumeMm3).toBeCloseTo(500,8)
 expect(JSON.stringify(document)).toBe(before)
})

it('runs exact Boolean groups in a worker with one anchor ID, empty results and immutable inputs',async()=>{
 const {createBrepBox,tessellateNurbsBrep,analyzeNurbsBrep}=await import('../src/services/geometry/brep')
 const {mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const body=(id:string,min:number,max:number)=>{const brep=createBrepBox([min,0,0],[max,10,10]),mesh=tessellateNurbsBrep(brep,4);return {id,name:id,brep,mesh:{positions:mesh.positions,indices:mesh.indices}}}
 const document={version:1 as const,sketches:[],bodies:[body('A',0,10),body('B',5,15),body('C',15,20)]}
 const before=JSON.stringify(document),messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 const options={operation:'difference' as const,a:['A'],b:['B','C'],segments:4,ru:false}
 await handle({version:1,id:1,job:{kind:'boolean',document,options}})
 expect(messages[0]).toMatchObject({ok:true,result:{resultId:'A',exact:true,toleranceMm:0,document:{bodies:[{id:'A'}]}}})
 const result=(messages[0] as any).result
 expect(result.document.bodies).toHaveLength(1)
 expect(analyzeNurbsBrep(result.document.bodies[0].brep).signedVolumeMm3).toBeCloseTo(500)
 expect(mainSolidResult({kind:'boolean'},result)).toBe(true)
 expect(mainSolidResult({kind:'boolean'},{...result,resultId:'missing'})).toBe(false)
 expect(mainSolidResult({kind:'boolean'},{...result,toleranceMm:NaN})).toBe(false)
 await handle({version:1,id:2,job:{kind:'boolean',document,options:{...options,operation:'intersection',a:['A'],b:['C']}}})
 expect(messages[1]).toMatchObject({ok:true,result:{resultId:null,document:{bodies:[{id:'B'}]}}})
 expect((messages[1] as any).result.document.bodies).toHaveLength(1)
 await handle({version:1,id:3,job:{kind:'boolean',document,options:{...options,b:['A']}}})
 expect(messages[2]).toMatchObject({ok:false,error:{message:'Each body must occur once, in either A or B.'}})
 expect(JSON.stringify(document)).toBe(before)
})

it('builds sketch corners and circular copies with stable IDs without mutating the request',async()=>{
 const document={version:1 as const,sketches:[{id:'s',name:'Square',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]}],bodies:[]}
 const before=JSON.stringify(document),messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 const options={operation:'fillet' as const,id:'s',vertex:0,radius:2,count:4,center:[0,0] as [number,number],sweep:360,copyIds:['c1','c2','c3']}
 await handle({version:1,id:1,job:{kind:'sketchEdit',document,options}})
 expect(messages[0]).toMatchObject({ok:true,result:{sketches:[{id:'s'}]}})
 const points=(messages[0] as any).result.sketches[0].points
 expect(points[0][0]).toBeCloseTo(0);expect(points[0][1]).toBeCloseTo(2)
 expect(points.at(-4)[0]).toBeCloseTo(2);expect(points.at(-4)[1]).toBeCloseTo(0)
 expect(points.length).toBeGreaterThan(4)
 await handle({version:1,id:2,job:{kind:'sketchEdit',document,options:{...options,operation:'dogear'}}})
 expect(messages[1]).toMatchObject({ok:true})
 await handle({version:1,id:3,job:{kind:'sketchEdit',document,options:{...options,operation:'array'}}})
 const copies=(messages[2] as any).result.sketches
 expect(copies.map((s:any)=>s.id)).toEqual(['s','c1','c2','c3'])
 expect(copies[1].points[1][0]).toBeCloseTo(0);expect(copies[1].points[1][1]).toBeCloseTo(10)
 await handle({version:1,id:4,job:{kind:'sketchEdit',document,options:{...options,radius:20}}})
 expect(messages[3]).toMatchObject({ok:false,error:{message:'Radius exceeds the adjacent edges. Use a smaller radius.'}})
 expect(JSON.stringify(document)).toBe(before)
})

it('keeps numeric sketch transforms in the support plane in the worker',async()=>{
 const plane={origin:[7,8,9],u:[0,1,0],v:[0,0,1]}
 const document={version:1 as const,sketches:[{id:'s',name:'Sketch',closed:true,plane,points:[[0,0],[10,0],[10,10],[0,10]]}],bodies:[]}
 const before=JSON.stringify(document),messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 await handle({version:1,id:1,job:{kind:'sceneEdit',document,options:{operation:'sketch-transform',id:'s',ids:['s'],createdId:'',x:3,y:4,z:100,axis:'z',angle:0,scale:1}}})
 expect(messages[0]).toMatchObject({ok:true,result:{sketches:[{id:'s',plane,points:[[3,4],[13,4],[13,14],[3,14]]}]}})
 expect(JSON.stringify(document)).toBe(before)
})

it('edits a CV and its dependent bridge, and moves mesh vertices without changing the source',async()=>{
 const {createSolidNurbsCurve}=await import('../src/services/solidNurbs')
 const {bridgeNurbsCurves}=await import('../src/services/directDimensions')
 const a=createSolidNurbsCurve('a'),b=createSolidNurbsCurve('b')
 b.curve.controlPoints=b.curve.controlPoints.map(p=>[p[0]+50,p[1],p[2]])
 const bridge={id:'bridge',name:'Bridge',bridge:{sourceA:'a',sourceB:'b',endA:'end' as const,endB:'start' as const,tension:1},curve:bridgeNurbsCurves(a.curve,b.curve,'end','start',1)}
 const document={version:1 as const,sketches:[],bodies:[{id:'mesh',name:'Mesh',mesh:{positions:[0,0,0,1,0,0,0,1,0],indices:[0,1,2]}}],curves:[a,b,bridge]}
 const before=JSON.stringify(document),messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message)),u=a.curve.controlPoints.length-1
 const point:[number,number,number]=[31,2,3]
 await handle({version:1,id:1,job:{kind:'pointEdit',document,options:{kind:'cv',id:'a',u,v:0,point}}})
 expect(messages[0]).toMatchObject({ok:true})
 const result=(messages[0] as any).result
 expect(result.curves[0].curve.controlPoints[u]).toEqual(point)
 expect(result.curves[2].curve.controlPoints[0]).toEqual(point)
 expect(result.curves[2].bridge).toEqual(bridge.bridge)
 await handle({version:1,id:2,job:{kind:'pointEdit',document,options:{kind:'vertices',id:'mesh',indices:[0,0,2],delta:[2,3,4]}}})
 expect(messages[1]).toMatchObject({ok:true,result:{bodies:[{id:'mesh',mesh:{positions:[2,3,4,1,0,0,2,4,4]}}]}})
 expect(JSON.stringify(document)).toBe(before)
})

it('edits analytic circle/arc handles and sketch vertices in isolated worker snapshots',async()=>{
 const {sampleCurve}=await import('../src/services/directSketchGeometry')
 const analytic={kind:'arc' as const,center:[2,3] as [number,number],radius:4,start:0,sweep:180}
 const document={version:1 as const,sketches:[{id:'arc',name:'Arc',closed:false,analytic,points:sampleCurve(analytic)},{id:'poly',name:'Polyline',closed:false,points:[[0,0],[10,0]]}],bodies:[]}
 const before=JSON.stringify(document),messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 for(const [i,option] of [{kind:'analytic-handle',id:'arc',handle:'radius',point:[2,9]},{kind:'analytic-handle',id:'arc',handle:'center',point:[5,6]},{kind:'analytic-handle',id:'arc',handle:'start',point:[2,7]},{kind:'analytic-handle',id:'arc',handle:'end',point:[2,-1]},{kind:'sketch-vertex',id:'poly',index:1,point:[7,8]}].entries())await handle({version:1,id:i+1,job:{kind:'pointEdit',document,options:option}})
 expect(messages.every(m=>m.ok)).toBe(true)
 const result=(i:number)=>(messages[i] as any).result
 expect(result(0).sketches[0].analytic.radius).toBe(6)
 expect(result(1).sketches[0].analytic.center).toEqual([5,6])
 expect(result(2).sketches[0].analytic).toMatchObject({start:90,sweep:90})
 expect(result(3).sketches[0].analytic.sweep).toBe(270)
 expect(result(4).sketches[1].points).toEqual([[0,0],[7,8]])
 expect(JSON.stringify(document)).toBe(before)
})

it('runs native NURBS commands with stable identities and preserved knot/elevation geometry',async()=>{
 const {createSolidNurbsCurve,createSolidNurbsSurface}=await import('../src/services/solidNurbs')
 const {evaluateNurbsCurve}=await import('../src/services/nurbsCurve')
 const {evaluateNurbsSurface}=await import('../src/services/nurbsSurface')
 const source={version:1 as const,sketches:[],bodies:[],curves:[createSolidNurbsCurve('curve')],surfaces:[createSolidNurbsSurface('surface')]}
 const snapshot=JSON.stringify(source),messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(m=>messages.push(m))
 async function edit(options:any){await handle({version:1,id:messages.length+1,job:{kind:'nurbsEdit',document:source,options}});const reply=messages.at(-1)!;expect(reply.ok).toBe(true);return (reply as any).result}
 for(const kind of ['knot','elevate']){
  const d=await edit({kind,id:'curve',axis:'curve',value:.4})
  expect(d.curves[0].id).toBe('curve')
  for(const u of [0,.13,.4,.83,1])evaluateNurbsCurve(d.curves[0].curve,u).point.forEach((x,i)=>expect(x).toBeCloseTo(evaluateNurbsCurve(source.curves[0].curve,u).point[i],9))
  for(const axis of ['u','v']){
   const surface=await edit({kind,id:'surface',axis,value:.4})
   for(const [u,v] of [[0,0],[.13,.72],[.4,.4],[1,1]])evaluateNurbsSurface(surface.surfaces[0].surface,u,v).point.forEach((x,i)=>expect(x).toBeCloseTo(evaluateNurbsSurface(source.surfaces[0].surface,u,v).point[i],9))
  }
 }
 const extruded=await edit({kind:'extrude',id:'curve',createdId:'extruded'})
 expect(extruded.surfaces[1].id).toBe('extruded');expect(extruded.curves).toEqual(source.curves)
 const iso=await edit({kind:'iso',id:'surface',axis:'u',value:.4,createdId:'iso'})
 expect(iso.curves[1].id).toBe('iso')
 for(const t of [0,.3,1])evaluateNurbsCurve(iso.curves[1].curve,t).point.forEach((x,i)=>expect(x).toBeCloseTo(evaluateNurbsSurface(source.surfaces[0].surface,.4,t).point[i],9))
 const trimmed=await edit({kind:'trim',id:'surface',bounds:[.2,.8,.1,.9]})
 expect(trimmed.surfaces[0].id).toBe('surface')
 expect(evaluateNurbsSurface(trimmed.surfaces[0].surface,.2,.9)).toMatchObject({domainU:[.2,.8],domainV:[.1,.9]})
 for(const [u,v] of [[.2,.1],[.45,.65],[.8,.9]])evaluateNurbsSurface(trimmed.surfaces[0].surface,u,v).point.forEach((x,i)=>expect(x).toBeCloseTo(evaluateNurbsSurface(source.surfaces[0].surface,u,v).point[i],9))
 for(const id of ['curve','surface']){
  const baked=await edit({kind:'bake',id,createdId:'baked'})
  expect((id==='curve'?baked.sketches:baked.bodies)[0].id).toBe('baked')
 }
 await handle({version:1,id:99,job:{kind:'nurbsEdit',document:source,options:{kind:'extrude',id:'curve',createdId:'surface'}}})
 expect(messages.at(-1)).toMatchObject({ok:false,error:{message:'A new NURBS result needs a unique identity.'}})
 expect(JSON.stringify(source)).toBe(snapshot)
})

it('creates a dependent bridge and runs B-rep properties/detail in the worker',async()=>{
 const {createSolidNurbsCurve}=await import('../src/services/solidNurbs')
 const {evaluateNurbsCurve}=await import('../src/services/nurbsCurve')
 const {createBrepBox,tessellateNurbsBrep}=await import('../src/services/geometry/brep')
 const {mainSolidResult,mainSolidExpectation}=await import('../src/services/mainSolidProtocol')
 const a=createSolidNurbsCurve('a'),b=createSolidNurbsCurve('b');b.curve.controlPoints.forEach(p=>p[0]+=60)
 const brep=createBrepBox([0,0,0],[10,20,30]),mesh=tessellateNurbsBrep(brep,2)
 const source={version:1 as const,curves:[a,b],sketches:[],bodies:[{id:'box',name:'Box',brep,mesh:{positions:mesh.positions,indices:mesh.indices}}]}
 const snapshot=JSON.stringify(source),messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(m=>messages.push(m))
 await handle({version:1,id:1,job:{kind:'nurbsEdit',document:source,options:{kind:'bridge',id:'a',otherId:'b',endA:'end',endB:'start',tension:1,createdId:'bridge'}}})
 expect(messages[0]).toMatchObject({ok:true,result:{curves:[{id:'a'},{id:'b'},{id:'bridge',bridge:{sourceA:'a',sourceB:'b'}}]}})
 const bridge=(messages[0] as any).result.curves[2].curve
 expect(evaluateNurbsCurve(bridge,0).point).toEqual(evaluateNurbsCurve(a.curve,1).point)
 expect(evaluateNurbsCurve(bridge,1).point).toEqual(evaluateNurbsCurve(b.curve,0).point)
 const job={kind:'brepTool' as const,document:source,options:{kind:'mass' as const,id:'box'}}
 await handle({version:1,id:2,job})
 expect(messages[1]).toMatchObject({ok:true,result:{kind:'mass',id:'box'}})
 expect((messages[1] as any).result.mass.signedVolumeMm3).toBeCloseTo(6000,7)
 expect((messages[1] as any).result.mass.surfaceAreaMm2).toBeCloseTo(2200,7)
 const result=(messages[1] as any).result,expectation=mainSolidExpectation(job)
 expect(mainSolidResult(expectation,result)).toBe(true)
 expect(mainSolidResult(expectation,{...result,id:'wrong'})).toBe(false)
 expect(mainSolidResult(expectation,{...result,mass:{...result.mass,centroid:[NaN,0,0]}})).toBe(false)
 await handle({version:1,id:3,job:{...job,options:{kind:'mesh',id:'box',segments:8}}})
 expect(messages[2]).toMatchObject({ok:true,result:{kind:'mesh',document:{bodies:[{id:'box',brep}]}}})
 const displayJob={...job,options:{kind:'display' as const,id:'box',offset:5,normal:[0,0,1]}}
 await handle({version:1,id:4,job:displayJob})
 expect(messages[3]).toMatchObject({ok:true,result:{kind:'display',diagnostics:{report:{boundaryEdges:0,nonManifoldEdges:0}}}})
 expect(mainSolidResult(mainSolidExpectation(displayJob),(messages[3] as any).result)).toBe(true)
 expect((messages[3] as any).result.diagnostics.section.contours.length).toBeGreaterThan(0)
 expect(JSON.stringify(source)).toBe(snapshot)
})

it('restores compact linked geometry through the authoritative document parser',async()=>{
 const {emptyDirectDocument,serializeDirectDocument,parseDirectDocument}=await import('../src/services/directModeling')
 const {createSolidInstance}=await import('../src/services/solidInstances')
 const {createBrepBox,tessellateNurbsBrep}=await import('../src/services/geometry/brep')
 const {stringifyMeshJson}=await import('../src/services/meshJson')
 const source=emptyDirectDocument(),brep=createBrepBox([0,0,0],[2,3,4])
 source.bodies.push({id:'source',name:'Source',brep,mesh:tessellateNurbsBrep(brep,1)})
 const doc=createSolidInstance(source,'source','linked',[[1,0,0,10],[0,1,0,0],[0,0,1,0],[0,0,0,1]])
 const text=serializeDirectDocument(doc),messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 await handle({version:1,id:1,job:{kind:'restoreDocument',text}})
 const response=messages.at(-1)!
 expect(response.ok).toBe(true)
 if(response.ok)expect(stringifyMeshJson(response.result)).toBe(stringifyMeshJson(parseDirectDocument(text)))
 await handle({version:1,id:2,job:{kind:'restoreDocument',text:'{"version":1,"bodies":[{}]}'}})
 expect(messages.at(-1)).toMatchObject({id:2,kind:'restoreDocument',ok:false})
})

it('prepares transferable display buffers without detaching request geometry',async()=>{
 const mesh={positions:new Float64Array([0,0,0,1,0,0,0,1,0]),indices:new Uint32Array([0,1,2])}
 let received:MainSolidResponse|undefined,transferred=0
 const handle=createMainSolidWorkerHandler((message,buffers=[])=>{transferred=buffers.length;received=structuredClone(message,{transfer:buffers})})
 await handle({version:1,id:1,job:{kind:'displayMesh',mesh,segments:12}})
 expect(received).toMatchObject({kind:'displayMesh',ok:true,result:{map:null,normals:[[0,0,1]]}})
 expect(transferred).toBe(2)
 expect(mesh.positions.byteLength).toBe(72);expect(mesh.indices.byteLength).toBe(12)
})

it('imports RushGraph definitions into a validated document without mutating or detaching its source',async()=>{
 const {readFileSync}=await import('node:fs')
 const {emptyDirectDocument,extrudeDirectSketch}=await import('../src/services/directModeling')
 const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const document=emptyDirectDocument(),text=readFileSync('tests/fixtures/solid-rush-import.json','utf8')
 document.bodies.push(extrudeDirectSketch({id:'profile',name:'Profile',closed:true,points:[[0,0],[2,0],[2,3],[0,3]]},4,'existing'))
 const snapshot=JSON.stringify(document),messages:MainSolidResponse[]=[]
 const handle=createMainSolidWorkerHandler((message,buffers=[])=>messages.push(structuredClone(message,{transfer:buffers})))
 const job={kind:'rushGraphImport' as const,document,text,group:'Imported'}
 await handle({version:1,id:1,job})
 const response=messages[0];expect(response.ok).toBe(true)
 if(response.ok){
  expect(mainSolidResult(mainSolidExpectation(job),response.result)).toBe(true)
  expect(response.result).toMatchObject({curves:[{name:'path',group:'Imported'}],surfaces:[{name:'skin',group:'Imported'}]})
 }
 expect(JSON.stringify(document)).toBe(snapshot)
 expect(document.bodies[0].mesh.positions.byteLength).toBeGreaterThan(0)
 await handle({version:1,id:2,job:{...job,text:'{"language":"rush/nurbs-1"}'}})
 expect(messages[1]).toMatchObject({ok:false,kind:'rushGraphImport'})
})

it('measures vertices and curvature in the worker and validates numeric response shapes',async()=>{
 const {createBrepCylinder,tessellateNurbsBrep}=await import('../src/services/geometry/brep')
 const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const brep=createBrepCylinder(3,5),body={id:'c',name:'Cylinder',brep,mesh:tessellateNurbsBrep(brep,2)}
 const edge=brep.edges.findIndex(e=>e.curve.degree===2),messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(m=>messages.push(m))
 const vertexJob={kind:'measureVertices' as const,a:body,indexA:0,b:body,indexB:0},edgeJob={kind:'measureEdge' as const,body,edge,parameter:.25}
 await handle({version:1,id:1,job:vertexJob});await handle({version:1,id:2,job:edgeJob})
 expect(messages[0]).toMatchObject({ok:true,result:{distanceMm:0}})
 expect(messages[1].ok).toBe(true)
 if(messages[1].ok){
  const r=messages[1].result as import('../src/services/solidMeasurements').CurveMeasurement
  expect(r.radiusMm).toBeCloseTo(3,8)
  expect(mainSolidResult(mainSolidExpectation(edgeJob),r)).toBe(true)
  expect(mainSolidResult(mainSolidExpectation(edgeJob),{...r,radiusMm:-1})).toBe(false)
 }
 expect(mainSolidResult(mainSolidExpectation(vertexJob),{a:[0,0,0],b:[1,0,0],deltaMm:[1,0,0],distanceMm:NaN})).toBe(false)
 await handle({version:1,id:3,job:{...vertexJob,indexA:999}});expect(messages[2]).toMatchObject({ok:false})
})

it('reports sampled surface boundaries through the worker with bounded result validation',async()=>{
 const {readFileSync}=await import('node:fs')
 const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const fixture=JSON.parse(readFileSync('tests/fixtures/solid-surface-boundary.json','utf8'))
 const job={kind:'surfaceBoundary' as const,a:fixture.surfaces[0].surface,b:fixture.surfaces[1].surface,options:{boundaryA:'uMax' as const,boundaryB:'uMin' as const,reverse:false,samples:65,toleranceMm:.01,angleToleranceDeg:1}}
 const before=JSON.stringify(job),messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(m=>messages.push(m))
 await handle({version:1,id:1,job})
 expect(messages[0]).toMatchObject({ok:true,result:{samplingOnly:true,maxGapMm:.25,maxTangentPlaneAngleDeg:0,sampledWithinTolerance:false}})
 if(messages[0].ok){
  const result=messages[0].result as import('../src/services/solidSurfaceDiagnostics').SurfaceBoundaryReport
  expect(mainSolidResult(mainSolidExpectation(job),result)).toBe(true)
  expect(mainSolidResult(mainSolidExpectation(job),{...result,samples:result.samples.slice(1)})).toBe(false)
  expect(mainSolidResult(mainSolidExpectation(job),{...result,maxGapMm:Infinity})).toBe(false)
  expect(mainSolidResult(mainSolidExpectation(job),{...result,worstGapSample:65})).toBe(false)
 }
 expect(JSON.stringify(job)).toBe(before)
 await handle({version:1,id:2,job:{...job,options:{...job.options,samples:1}}})
 expect(messages[1]).toMatchObject({ok:false,kind:'surfaceBoundary'})
})

it('tessellates surface display into owned transferable buffers and rejects malformed meshes',async()=>{
 const {createSolidNurbsSurface,tessellateSolidNurbsSurface}=await import('../src/services/solidNurbs')
 const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const item=createSolidNurbsSurface('surface'),before=JSON.stringify(item),messages:MainSolidResponse[]=[]
 let transferred=0
 const handle=createMainSolidWorkerHandler((message,buffers=[])=>{transferred=buffers.length;messages.push(structuredClone(message,{transfer:buffers}))})
 const job={kind:'surfaceMesh' as const,item};await handle({version:1,id:1,job})
 expect(messages[0]).toMatchObject({ok:true,kind:'surfaceMesh'});expect(transferred).toBe(2)
 if(messages[0].ok){
  expect(messages[0].result).toEqual(tessellateSolidNurbsSurface(item))
  expect(mainSolidResult(mainSolidExpectation(job),messages[0].result)).toBe(true)
  expect(mainSolidResult(mainSolidExpectation(job),{positions:[0,0,0],indices:[0,1,2]})).toBe(false)
  expect(mainSolidResult(mainSolidExpectation(job),{positions:[NaN,0,0],indices:[]})).toBe(false)
 }
 expect(JSON.stringify(item)).toBe(before)
})

it('samples retained profile loops in the worker without changing rational geometry',async()=>{
 const {authorBrepProfile}=await import('../src/services/geometry/brepProfile')
 const {retainedProfileDisplay}=await import('../src/services/retainedSketchProfile')
 const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const {SolidProfileDisplayQueue}=await import('../src/services/solidProfileDisplayQueue')
 const profile=authorBrepProfile({kind:'circle',radius:7}),before=JSON.stringify(profile)
 const job={kind:'profileDisplay' as const,profile},messages:MainSolidResponse[]=[]
 await createMainSolidWorkerHandler(m=>messages.push(m))({version:1,id:1,job})
 expect(messages[0]).toMatchObject({ok:true,kind:'profileDisplay'})
 if(!messages[0].ok)throw Error('Worker failed')
 const result=messages[0].result as [number,number][][]
 expect(result).toEqual(retainedProfileDisplay(profile));expect(JSON.stringify(profile)).toBe(before)
 const expectation=mainSolidExpectation(job)
 expect(mainSolidResult(expectation,result)).toBe(true)
 expect(mainSolidResult(expectation,[result[0].slice(1)])).toBe(false)
 const malformed=structuredClone(result);malformed[0][0][0]=NaN
 expect(mainSolidResult(expectation,malformed)).toBe(false)
 let resolve!:(r:typeof result)=>void,runs=0,cancels=0
 const queue=new SolidProfileDisplayQueue({run:()=>{runs++;return new Promise(r=>resolve=r)},cancel:()=>{cancels++}})
 const item={id:'p',profile},pending=queue.prepare([item]);queue.cancel();resolve(result)
 expect(await pending).toEqual({changed:false,errors:[]});expect(queue.get(item)).toBeUndefined()
 const retry=queue.prepare([item,{...item,id:'copy'}]);resolve(result);expect((await retry).changed).toBe(true)
 expect(runs).toBe(2);expect(cancels).toBeGreaterThan(1);expect(queue.get(item)).toEqual(result)
 await queue.prepare([item]);expect(runs).toBe(2)
 const edited={id:'p',profile:authorBrepProfile({kind:'circle',radius:9})}
 expect(queue.get(edited)).toBeUndefined()
 const changed=queue.prepare([edited]);edited.profile.toleranceMm*=2;resolve(result)
 await changed;expect(queue.get(edited)).toBeUndefined()
})

it('samples rational curve display in a cancellable queue with exact input keys',async()=>{
 const {createSolidNurbsCurve,sampleSolidNurbsCurve}=await import('../src/services/solidNurbs')
 const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const {SolidCurveDisplayQueue}=await import('../src/services/solidCurveDisplayQueue')
 const item=createSolidNurbsCurve('curve'),before=JSON.stringify(item)
 const job={kind:'curveDisplay' as const,curve:item.curve},messages:MainSolidResponse[]=[]
 await createMainSolidWorkerHandler(m=>messages.push(m))({version:1,id:1,job})
 expect(messages[0]).toMatchObject({ok:true,kind:'curveDisplay'})
 if(!messages[0].ok)throw Error('Worker failed')
 const result=messages[0].result as number[][]
 expect(result).toEqual(sampleSolidNurbsCurve(item.curve));expect(JSON.stringify(item)).toBe(before)
 const expectation=mainSolidExpectation(job)
 expect(mainSolidResult(expectation,result)).toBe(true)
 expect(mainSolidResult(expectation,result.slice(1))).toBe(false)
 const malformed=structuredClone(result);malformed[0][0]=Infinity
 expect(mainSolidResult(expectation,malformed)).toBe(false)
 expect(mainSolidResult(expectation,result.map(p=>[...p,0]))).toBe(false)
 let resolve!:(r:typeof result)=>void,runs=0
 const queue=new SolidCurveDisplayQueue({run:()=>{runs++;return new Promise(r=>resolve=r)},cancel:()=>{}})
 const pending=queue.prepare([item]);queue.cancel();resolve(result)
 expect(await pending).toEqual({changed:false,errors:[]});expect(queue.get(item)).toBeUndefined()
 const retry=queue.prepare([item,{...item,id:'copy'}]);resolve(result);expect((await retry).changed).toBe(true)
 await queue.prepare([item]);expect(runs).toBe(2)
 const edited=structuredClone(item);edited.curve.controlPoints[0][0]+=1
 expect(queue.get(edited)).toBeUndefined()
 const changed=queue.prepare([edited]);edited.curve.controlPoints[0][0]+=1;resolve(result)
 await changed;expect(queue.get(edited)).toBeUndefined()
 const tiny=new SolidCurveDisplayQueue({run:async()=>result,cancel:()=>{}},1,1)
 expect((await tiny.prepare([item])).errors[0].id).toBe(item.id)
})

it('prepares mesh topology in the worker and rejects invalid picking indices',async()=>{
 const {solidTopology}=await import('../src/services/directSolidTools')
 const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const {createBrepBox,tessellateNurbsBrep}=await import('../src/services/geometry/brep')
 const mesh=tessellateNurbsBrep(createBrepBox([0,0,0],[10,12,14]),2),before=structuredClone(mesh)
 const job={kind:'topology' as const,mesh},messages:MainSolidResponse[]=[]
 await createMainSolidWorkerHandler(m=>messages.push(m))({version:1,id:1,job})
 expect(messages[0]).toMatchObject({ok:true,kind:'topology'})
 if(!messages[0].ok)throw Error('Topology failed')
 const result=messages[0].result as ReturnType<typeof solidTopology>,expectation=mainSolidExpectation(job)
 expect(result).toEqual(solidTopology(mesh));expect(mesh).toEqual(before)
 expect(mainSolidResult(expectation,result)).toBe(true)
 const bad=structuredClone(result);bad.faces[0].triangles.push(bad.faces[0].triangles[0])
 expect(mainSolidResult(expectation,bad)).toBe(false)
 const edge=structuredClone(result);edge.edges[0].faces[0]=result.faces.length
 expect(mainSolidResult(expectation,edge)).toBe(false)
 const vertex=structuredClone(result);vertex.faces[0].vertices[0]=mesh.positions.length/3
 expect(mainSolidResult(expectation,vertex)).toBe(false)
 const normal=structuredClone(result);normal.faces[0].normal[0]=NaN
 expect(mainSolidResult(expectation,normal)).toBe(false)
 const flat={positions:new Float64Array([0,0,0,1,0,0,2,0,0]),indices:new Uint32Array([0,1,2])}
 await createMainSolidWorkerHandler(m=>messages.push(m))({version:1,id:2,job:{kind:'topology',mesh:flat}})
 expect(messages[1]).toMatchObject({ok:true,result:{faces:[],edges:[]}})
})

it('prepares authored edges in a worker with bounded vertex references',async()=>{
 const {createBrepCylinder,tessellateNurbsBrep}=await import('../src/services/geometry/brep')
 const {solidBodyEdges}=await import('../src/services/solidBodyEdges')
 const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const brep=createBrepCylinder(3,5),body={id:'edges',name:'Edges',brep,mesh:tessellateNurbsBrep(brep,2)}
 const job={kind:'bodyEdges' as const,body},before=structuredClone(body),messages:MainSolidResponse[]=[]
 await createMainSolidWorkerHandler(m=>messages.push(m))({version:1,id:1,job})
 expect(messages[0]).toMatchObject({ok:true,kind:'bodyEdges'})
 if(!messages[0].ok)throw Error('Edge preparation failed')
 expect(messages[0].result).toEqual(solidBodyEdges(body));expect(body).toEqual(before)
 const result=messages[0].result as ReturnType<typeof solidBodyEdges>,expectation=mainSolidExpectation(job)
 expect(mainSolidResult(expectation,result)).toBe(true)
 expect(mainSolidResult(expectation,result.slice(1))).toBe(false)
 const bad=structuredClone(result);bad[0].a=brep.vertices.length
 expect(mainSolidResult(expectation,bad)).toBe(false)
 const nonfinite=structuredClone(result);nonfinite[0].points[0][0]=NaN
 expect(mainSolidResult(expectation,nonfinite)).toBe(false)
})

it('prepares a planar face workplane and rejects a stale face index',async()=>{
 const {createBrepBox,tessellateNurbsBrep}=await import('../src/services/geometry/brep')
 const {solidTopology}=await import('../src/services/directSolidTools')
 const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const brep=createBrepBox([0,0,0],[10,12,14]),body={id:'workplane',name:'Workplane',brep,mesh:tessellateNurbsBrep(brep,2)}
 const face=solidTopology(body.mesh).faces.findIndex(f=>f.normal[2]>.99),before=structuredClone(body)
 const job={kind:'faceSketch' as const,body,face},messages:MainSolidResponse[]=[],handler=createMainSolidWorkerHandler(m=>messages.push(m))
 await handler({version:1,id:1,job})
 expect(messages[0]).toMatchObject({ok:true,kind:'faceSketch'})
 if(!messages[0].ok)throw Error('Plane failed')
 const result=messages[0].result as import('../src/services/mainSolidProtocol').MainSolidResults['faceSketch']
 expect(result.plane.origin[2]).toBeCloseTo(14);expect(result.normal).toEqual([0,0,1]);expect(result.outline).toHaveLength(1)
 expect(mainSolidResult(mainSolidExpectation(job),result)).toBe(true)
 expect(mainSolidResult(mainSolidExpectation(job),{...result,outline:[]})).toBe(false)
 expect(body).toEqual(before)
 await handler({version:1,id:2,job:{...job,face:999}});expect(messages[1]).toMatchObject({ok:false})
})

it('prepares cloneable exact snap geometry in a worker and validates curve intervals',async()=>{
 const {createBrepCylinder,tessellateNurbsBrep}=await import('../src/services/geometry/brep')
 const {bodySnapGeometry}=await import('../src/services/solidSnapGeometry')
 const {resolveModelingSnap}=await import('../src/services/modelingSnaps')
 const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const brep=createBrepCylinder(5,10),body={id:'snap-body',name:'Snaps',brep,mesh:tessellateNurbsBrep(brep,2)},before=structuredClone(body)
 const job={kind:'bodySnaps' as const,body},messages:MainSolidResponse[]=[]
 await createMainSolidWorkerHandler(m=>messages.push(structuredClone(m)))({version:1,id:1,job})
 expect(messages[0]).toMatchObject({ok:true,kind:'bodySnaps'})
 if(!messages[0].ok)throw Error('Snap preparation failed')
 const result=messages[0].result as import('../src/services/modelingSnaps').SnapGeometry,expectation=mainSolidExpectation(job)
 expect(result).toEqual(bodySnapGeometry(body));expect(body).toEqual(before)
 expect(mainSolidResult(expectation,result)).toBe(true)
 const segment=result.segments.find(s=>s.nurbs)!,point=segment.a.map((v,i)=>(v+segment.b[i])/2) as [number,number,number]
 const snap=resolveModelingSnap(point,{points:[],segments:[segment]},{project:p=>[p[0]*100,p[1]*100],grid:0,geometry:true})
 expect(snap.kind).toBe('edge');expect(Math.hypot(snap.point[0],snap.point[1])).toBeCloseTo(5,10)
 const bad=structuredClone(result);bad.segments.find(s=>s.nurbs)!.nurbs!.end=Infinity
 expect(mainSolidResult(expectation,bad)).toBe(false)
 const weights=structuredClone(result);weights.segments.find(s=>s.nurbs)!.nurbs!.curve.weights[0]=0
 expect(mainSolidResult(expectation,weights)).toBe(false)
 expect(mainSolidResult(expectation,{...result,points:[{point:[NaN,0,0],kind:'vertex'}]})).toBe(false)
})

it('prepares planar sketch snap targets in the worker and rejects malformed results',async()=>{
 const {mainSolidExpectation,mainSolidResult}=await import('../src/services/mainSolidProtocol')
 const {sketchSnapGeometry,resolveModelingSnap}=await import('../src/services/modelingSnaps')
 const sketches:import('../src/services/directModeling').DirectSketch[]=[
  {id:'polygon',name:'Polygon',closed:true,points:[[0,0],[20,0],[20,10],[0,10]]},
  {id:'rational',name:'Rational',closed:false,points:[],retainedProfile:{loops:[[{degree:2,knots:[0,0,0,1,1,1],weights:[1,Math.SQRT1_2,1],controlPoints:[[10,0],[10,10],[0,10]]}]]}},
  {id:'circle',name:'Circle',closed:true,points:[],analytic:{kind:'circle',center:[2,3],radius:5,start:0,sweep:360}},
  {id:'retained',name:'Retained',closed:false,points:[],retainedProfile:{loops:[[{degree:1,knots:[0,0,1,1],weights:[1,1],controlPoints:[[0,0],[10,0]]}]]}},
 ]
 const messages:MainSolidResponse[]=[],handler=createMainSolidWorkerHandler(m=>messages.push(structuredClone(m)))
 for(const sketch of sketches){
  const before=structuredClone(sketch),job={kind:'sketchSnaps' as const,sketch}
  await handler({version:1,id:messages.length+1,job})
  const response=messages.at(-1)!;expect(response).toMatchObject({ok:true,kind:'sketchSnaps'})
  if(!response.ok)throw Error('Sketch snaps failed')
  const result=response.result as import('../src/services/modelingSnaps').SnapGeometry,expected=mainSolidExpectation(job)
  expect(result).toEqual(sketchSnapGeometry([sketch]));expect(sketch).toEqual(before)
  if(sketch.id==='rational'){const bad=structuredClone(result);bad.segments[0].nurbs!.curve.controlPoints[1][2]=1;expect(mainSolidResult(expected,bad)).toBe(false)}
  expect(mainSolidResult(expected,result)).toBe(true)
  expect(mainSolidResult(expected,{...result,points:[{point:[0,0,1],kind:'vertex'}]})).toBe(false)
  expect(mainSolidResult(expected,{...result,segments:[{a:[0,0,0],b:[1,0,0],evaluate:()=>[0,0,0]}]})).toBe(false)
  expect(mainSolidResult(expected,{...result,circles:[{center:[0,0,0],radius:-1,start:0,sweep:360}]})).toBe(false)
  expect(mainSolidResult(expected,{...result,points:Array.from({length:100},()=>({point:[0,0,0],kind:'vertex'}))})).toBe(false)
  if(sketch.analytic){
   const snap=resolveModelingSnap([5,7.01,0],result,{project:p=>[p[0]*100,p[1]*100],grid:0,geometry:true})
   expect(snap.kind).toBe('edge');expect(Math.hypot(snap.point[0]-2,snap.point[1]-3)).toBeCloseTo(5,10)
  }
 }
})

it('trims an open rational curve at a picked point in the worker without changing its identity',async()=>{
 const {evaluateNurbsCurve}=await import('../src/services/nurbsCurve')
 const document={version:1 as const,sketches:[],bodies:[],curves:[{id:'trim-target',name:'Trim target',curve:{degree:2,knots:[0,0,0,1,1,1],controlPoints:[[10,0,0],[10,10,0],[0,10,0]],weights:[1,Math.SQRT1_2,1]}}]}
 const before=structuredClone(document),messages:MainSolidResponse[]=[],handler=createMainSolidWorkerHandler(m=>messages.push(structuredClone(m)))
 await handler({version:1,id:1,job:{kind:'nurbsEdit',document,options:{kind:'trim-point',id:'trim-target',point:[8,6,0],keep:'end',maxDistance:.01}}})
 expect(messages[0]).toMatchObject({ok:true,kind:'nurbsEdit'});if(!messages[0].ok)throw Error('Trim failed')
 const result=messages[0].result as import('../src/services/directModeling').DirectDocument,curve=result.curves![0].curve
 expect(result.curves![0].id).toBe('trim-target');expect(result.curves![0].name).toBe('Trim target')
 const p=evaluateNurbsCurve(curve,curve.knots[curve.degree]).point
 expect(p[0]).toBeCloseTo(8,7);expect(p[1]).toBeCloseTo(6,7);expect(document).toEqual(before)
 await handler({version:1,id:2,job:{kind:'nurbsEdit',document,options:{kind:'trim-point',id:'trim-target',point:[10,0,0],keep:'start',maxDistance:.01}}})
 expect(messages[1]).toMatchObject({ok:false});expect(document).toEqual(before)
})

it('trims a screen-picked curve in model coordinates and preserves depth',async()=>{
 const document={version:1 as const,sketches:[],bodies:[],curves:[{id:'line',name:'Depth',curve:{degree:1,knots:[0,0,1,1],controlPoints:[[0,0,10],[10,0,20]],weights:[1,1]}}]}
 const before=structuredClone(document),messages:MainSolidResponse[]=[],handler=createMainSolidWorkerHandler(m=>messages.push(structuredClone(m)))
 await handler({version:1,id:1,job:{kind:'nurbsEdit',document,options:{kind:'trim-screen-point',id:'line',point:[108,201],matrix:[[2,0,0,100],[0,3,0,200]],keep:'end',radius:2}}})
 expect(messages[0]).toMatchObject({ok:true,kind:'nurbsEdit'});if(!messages[0].ok)throw Error('Screen trim failed')
 const result=messages[0].result as import('../src/services/directModeling').DirectDocument
 const p=result.curves![0].curve.controlPoints[0]
 expect(p[0]).toBeCloseTo(4,7);expect(p[1]).toBe(0);expect(p[2]).toBeCloseTo(14,7)
 expect(result.curves![0].id).toBe('line');expect(document).toEqual(before)
})

it('keeps the midpoint projection of a tilted rational arc through the worker',async()=>{
 const curve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[10,0,10],[10,10,15],[0,10,15]],weights:[1,Math.SQRT1_2,1]}
 const document={version:1 as const,sketches:[],bodies:[],curves:[{id:'tilted',name:'Tilted',curve}]}
 const {evaluateNurbsCurve}=await import('../src/services/nurbsCurve')
 const p=evaluateNurbsCurve(curve,.5).point
 const messages:MainSolidResponse[]=[],handler=createMainSolidWorkerHandler(m=>messages.push(structuredClone(m)))
 await handler({version:1,id:1,job:{kind:'nurbsEdit',document,options:{kind:'trim-point',id:'tilted',point:p,keep:'start',maxDistance:.01}}})
 expect(messages[0]).toMatchObject({ok:true});if(!messages[0].ok)throw Error('Tilted trim failed')
 const result=messages[0].result as import('../src/services/directModeling').DirectDocument
 result.curves![0].curve.controlPoints.at(-1)!.forEach((v,i)=>expect(v).toBeCloseTo(p[i],8))
 expect(document.curves[0].curve.controlPoints).toEqual([[10,0,10],[10,10,15],[0,10,15]])
})

it('reports opt-in warmup execution and transfer preparation timings without changing the result',async()=>{
 const messages:MainSolidResponse[]=[],handle=createMainSolidWorkerHandler(message=>messages.push(message))
 const body={id:'points',name:'Points',mesh:{positions:[0,0,0,3,4,0],indices:[]}}
 const job={kind:'measureVertices' as const,a:body,b:body,indexA:0,indexB:1}
 await handle({version:1,id:901,job,traceTiming:true});await handle({version:1,id:902,job})
 expect(messages[0]).toMatchObject({ok:true,result:{distanceMm:5}});expect(messages[1]).toMatchObject({ok:true,result:{distanceMm:5}})
 expect(messages[1]).not.toHaveProperty('timing')
 for(const value of Object.values(messages[0].timing!)){expect(Number.isFinite(value)).toBe(true);expect(value).toBeGreaterThanOrEqual(0)}
 await handle({version:1,id:903,job:{...job,indexA:999},traceTiming:true});expect(messages[2]).toMatchObject({ok:false});expect(messages[2].timing).toBeDefined()
})
