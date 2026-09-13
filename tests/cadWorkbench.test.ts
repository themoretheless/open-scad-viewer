import {describe,it,expect} from 'vitest'
import {cadOperation,bounds,type CadOptions} from '../src/services/cadWorkbench'
import {extrudeDirectSketch,type DirectDocument} from '../src/services/directModeling'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
import {inspectCadPairs} from '../src/services/cadInspection'
import {cadDrawing} from '../src/services/cadDrawing'
import {createBrepBox,createBrepSphere,transformNurbsBrep,tessellateNurbsBrep,analyzeNurbsBrep,inspectNurbsBrep} from '../src/services/geometry/brep'
const box=(id='0',x=0)=>extrudeDirectSketch({id:'s',name:'Box',closed:true,points:[[x,0],[x+10,0],[x+10,10],[x,10]]},10,id)
const options:CadOptions={action:'union',ids:['0'],sketches:[],axis:[0,0,1],origin:[0,0,0],amount:5,count:3,width:2,height:1,depth:12,pitch:1,secondary:4,mode:'plain',pathId:'',profileIds:[]}
const doc=(...bodies:ReturnType<typeof box>[]):DirectDocument=>({version:1,sketches:[],bodies})
const volume=(d:DirectDocument)=>d.bodies.reduce((s,b)=>s+inspectPolygonMesh(b.mesh).signedVolumeMm3,0)
describe('CAD workbench geometry',()=>{
 it('mirrors retained topology across an offset oblique plane and is an involution',()=>{
  const brep=createBrepBox([2,3,4],[5,7,9]),input=doc({id:'0',name:'Retained',brep,mesh:tessellateNurbsBrep(brep,1)}),before=JSON.stringify(input)
  const operation={...options,action:'mirror' as const,axis:[1,1,0] as [number,number,number],origin:[3,2,0] as [number,number,number]}
  const result=cadOperation(input,operation),body=result.bodies[0]
  // The plane is x+y=5, so reflection sends (x,y,z) to (5-y,5-x,z).
  body.brep!.vertices.forEach((v,i)=>{const p=brep.vertices[i].point;[5-p[1],5-p[0],p[2]].forEach((x,k)=>expect(v.point[k]).toBeCloseTo(x,12))})
  expect(body.brep!.loops).toEqual(brep.loops)
  expect(body.brep!.topologyIds).toEqual(brep.topologyIds)
  expect(body.id).toBe('0');expect(body.name).toBe('Retained')
  expect(analyzeNurbsBrep(body.brep!).signedVolumeMm3).toBeCloseTo(60,9)
  expect(inspectPolygonMesh(body.mesh).signedVolumeMm3).toBeCloseTo(60,9)
  const meshBounds=bounds([body]),brepBounds=bounds([{...body,mesh:{positions:body.brep!.vertices.flatMap(v=>v.point),indices:[]}}])
  expect(brepBounds).toEqual(meshBounds)
  const restored=cadOperation(result,operation).bodies[0]
  restored.mesh.positions.forEach((x,i)=>expect(x).toBeCloseTo(input.bodies[0].mesh.positions[i],12))
  expect(restored.mesh.indices).toEqual(input.bodies[0].mesh.indices)
  expect(JSON.stringify(input)).toBe(before)
 })
 it('copies reflected B-rep bodies and rejects an invalid later body atomically',()=>{
  const brep=createBrepBox([1,2,3],[3,5,7]),input=doc({id:'0',name:'Original',brep,mesh:tessellateNurbsBrep(brep,1)})
  const operation={...options,action:'mirror' as const,mode:'copy',axis:[1,0,0] as [number,number,number],origin:[5,0,0] as [number,number,number]}
  const result=cadOperation(input,operation)
  expect(result.bodies).toHaveLength(2);expect(result.bodies[0]).toEqual(input.bodies[0])
  expect(result.bodies[1].id).not.toBe('0')
  expect(bounds([result.bodies[1]])).toEqual({min:[7,2,3],max:[9,5,7]})
  expect(analyzeNurbsBrep(result.bodies[1].brep!).signedVolumeMm3).toBeCloseTo(24,9)
  const bad=structuredClone(input.bodies[0]);bad.id='bad';bad.brep!.vertices[0].point[0]+=1
  const invalid=doc(input.bodies[0],bad),before=JSON.stringify(invalid)
  expect(()=>cadOperation(invalid,{...operation,ids:['0','bad']})).toThrow()
  expect(JSON.stringify(invalid)).toBe(before)
  expect(()=>cadOperation(input,{...operation,axis:[0,0,0]})).toThrow(/nonzero/)
  expect(cadOperation(input,operation).bodies).toHaveLength(2)
 })
 it('resizes a retained rational sphere into an ellipsoid without replacing its surfaces',()=>{
  const brep=createBrepSphere(2),mesh=tessellateNurbsBrep(brep,4)
  const result=cadOperation(doc({id:'0',name:'Sphere',brep,mesh}),{...options,action:'resize',width:4,height:6,depth:8}).bodies[0]
  expect(result.brep!.faces.map(f=>f.surface.weights)).toEqual(brep.faces.map(f=>f.surface.weights))
  expect(result.brep!.loops).toEqual(brep.loops)
  expect(bounds([result])).toEqual({min:[-2,-2,-2],max:[2,4,6]})
  expect(analyzeNurbsBrep(result.brep!).signedVolumeMm3).toBeCloseTo(32*Math.PI,6)
  expect(inspectPolygonMesh(result.mesh).signedVolumeMm3).toBeCloseTo(inspectPolygonMesh(mesh).signedVolumeMm3*3,8)
 })
 it('resizes mesh and retained B-rep together around one group minimum',()=>{
  const bodies=[10,30,60].map((x,i)=>{
   const brep=createBrepBox([x,20,30],[x+10,30,40])
   return {id:String(i),name:`Part ${i}`,brep,mesh:tessellateNurbsBrep(brep,1)}
  })
  const input=doc(...bodies),before=JSON.stringify(input)
  const result=cadOperation(input,{...options,action:'resize',ids:['0','1'],width:6,height:3,depth:2})
  expect(bounds(result.bodies.slice(0,2))).toEqual({min:[10,20,30],max:[16,23,32]})
  for(let i=0;i<2;i++){
   const body=result.bodies[i],brep=body.brep!
   expect(body.name).toBe(bodies[i].name)
   expect(brep.edges.map(e=>e.vertices)).toEqual(bodies[i].brep.edges.map(e=>e.vertices))
   expect(brep.topologyIds).toEqual(bodies[i].brep.topologyIds)
   expect(()=>inspectNurbsBrep(brep)).not.toThrow()
   expect(analyzeNurbsBrep(brep).signedVolumeMm3).toBeCloseTo(12,8)
   expect(inspectPolygonMesh(body.mesh).signedVolumeMm3).toBeCloseTo(12,8)
   expect(bounds([{...body,mesh:{positions:brep.vertices.flatMap(v=>v.point),indices:[]}}])).toEqual(bounds([body]))
  }
  expect(result.bodies[2]).toEqual(input.bodies[2])
  expect(JSON.stringify(input)).toBe(before)
 })
 it('rejects a late invalid B-rep without changing the source selection',()=>{
  const bodies=[0,20].map((x,i)=>{const brep=createBrepBox([x,0,0],[x+10,10,10]);return {id:String(i),name:'Part',brep,mesh:tessellateNurbsBrep(brep,1)}})
  bodies[1].brep.vertices[0].point[0]+=1
  const input=doc(...bodies),before=JSON.stringify(input)
  expect(()=>cadOperation(input,{...options,action:'resize',ids:['0','1'],width:6,height:3,depth:2})).toThrow()
  expect(JSON.stringify(input)).toBe(before)
 })
 it('computes native group bounds in binary64 including unreferenced vertices',()=>{
  const b=box(),empty={...b,id:'empty',mesh:{positions:[],indices:[]}}
  b.mesh.positions.push(-20.0000000001,30.0000000001,40.0000000001)
  const before=[...b.mesh.positions]
  expect(bounds([empty,b])).toEqual({min:[-20.0000000001,0,0],max:[10,30.0000000001,40.0000000001]})
  expect(b.mesh.positions).toEqual(before)
 })
 it('refuses empty, malformed and nonfinite bounds inputs',()=>{
  expect(()=>bounds([])).toThrow('Select bodies')
  const b=box();b.mesh.positions.push(1)
  expect(()=>bounds([b])).toThrow('complete coordinate triples')
  const nonfinite=box();nonfinite.mesh.positions.push(Infinity,0,0)
  expect(()=>bounds([nonfinite])).toThrow(/finite/i)
  expect(bounds([box()])).toEqual({min:[0,0,0],max:[10,10,10]})
 })
 it('booleans preserve selection order and source bodies',()=>{const d=doc(box(),box('1',5));expect(volume(cadOperation(d,{...options,action:'union',ids:['0','1']}))).toBeCloseTo(1500);expect(volume(cadOperation(d,{...options,action:'difference',ids:['1','0']}))).toBeCloseTo(500);expect(volume(d)).toBe(2000)})
 it('mirrors winding, resizes and patterns groups',()=>{expect(volume(cadOperation(doc(box()),{...options,action:'mirror',mode:'replace',axis:[1,0,0]}))).toBeCloseTo(1000);expect(volume(cadOperation(doc(box()),{...options,action:'resize',width:2,height:3,depth:4}))).toBeCloseTo(24);expect(cadOperation(doc(box()),{...options,action:'pattern'}).bodies).toHaveLength(3)})
 it('aligns to first selected body and distributes centers',()=>{const d=doc(box(),box('1',20),box('2',50));const a=cadOperation(d,{...options,action:'align',ids:['1','0'],axis:[1,0,0],mode:'min'});expect(bounds([a.bodies[0]]).min[0]).toBe(20);const b=cadOperation(d,{...options,action:'distribute',ids:['0','1','2'],axis:[1,0,0],mode:'center'});expect(bounds([b.bodies[1]]).min[0]).toBe(25)})
 it('cuts plain/counterbored/countersunk holes and threaded holes',()=>{for(const mode of ['plain','counterbore','countersink']){const d=cadOperation(doc(box()),{...options,action:'hole',origin:[5,5,-1],mode});expect(volume(d)).toBeLessThan(1000);expect(inspectPolygonMesh(d.bodies[0].mesh).closed).toBe(true)}const d=cadOperation(doc(box()),{...options,action:'thread',origin:[5,5,-1],mode:'internal'});expect(volume(d)).toBeLessThan(1000)})
 it('reports intersecting volume and exact separated box gap',()=>{expect(inspectCadPairs([box(),box('1',13)])[0].gapMm).toBeCloseTo(3);expect(inspectCadPairs([box(),box('1',5)])[0].overlapMm3).toBeCloseTo(500)})
 it('produces vector drawings with dimensions and section',()=>{const d=cadDrawing([box()],5);expect(d.svg).toContain('10.00 mm');expect(d.svg).toContain('SECTION Z=5.00');expect(d.pdf).toContain('xref\n0 6');expect(d.pdf).toContain('/Type /Page')})
})

it('round-trips faceted STEP as closed solids and rejects analytic files',async()=>{const {exportFacetedStep,importFacetedStep}=await import('../src/services/cadStep');const text=exportFacetedStep([box(),box('1',20)]),b=importFacetedStep(text);expect(b).toHaveLength(2);expect(volume(doc(...b))).toBeCloseTo(2000);expect(()=>importFacetedStep(text.replace('FACE_SURFACE','ADVANCED_FACE'))).toThrow('supports planar')})
it('lofts sections and sweeps an open path',()=>{const sketches=[{id:'a',name:'a',closed:true,points:[[0,0],[4,0],[4,4],[0,4]] as [number,number][]},{id:'b',name:'b',closed:true,points:[[0,0],[2,0],[2,2],[0,2]] as [number,number][],plane:{origin:[0,0,10] as [number,number,number],u:[1,0,0] as [number,number,number],v:[0,1,0] as [number,number,number]}},{id:'path',name:'path',closed:false,points:[[0,0],[0,10],[10,10]] as [number,number][]}];expect(volume(cadOperation(doc(),{...options,action:'loft',profileIds:['a','b'],sketches}))).toBeGreaterThan(0);expect(volume(cadOperation(doc(),{...options,action:'sweep',profileIds:['a'],pathId:'path',sketches}))).toBeGreaterThan(0)})
it('persists bounded joint positions and applies absolute slider travel after source rebuild',async()=>{
 const {jointOptions,saveCadJoint}=await import('../src/services/cadJoints'),{patchMainSource}=await import('../src/services/mainSourceEditing'),{parseOpenSCAD}=await import('../src/services/openscadParser'),{sceneBody}=await import('../src/services/mainModeling')
 const source='cube(10);translate([20,0,0]) cube(10);',meshes=(await parseOpenSCAD(source)).meshes,before=doc(...meshes.map(sceneBody)),o={...options,action:'joint' as const,ids:['0','1'],mode:'slider',amount:5},d=cadOperation(before,jointOptions(source,before.bodies,o,-10,10)),saved=saveCadJoint(patchMainSource(source,meshes,d),d.bodies,o,-10,10),rebuilt=(await parseOpenSCAD(saved)).meshes.map(sceneBody),next=jointOptions(saved,rebuilt,{...o,amount:8},-10,10)
 expect(next.amount).toBe(3);expect(()=>jointOptions(saved,rebuilt,{...o,amount:11},-10,10)).toThrow('limits');expect(bounds(cadOperation(doc(...rebuilt),next).bodies).max[2]).toBe(18)
})
it('drafts mesh vertices natively about the neutral plane with unchanged connectivity',()=>{
 const input=doc(box()),before=JSON.stringify(input),angle=20
 const result=cadOperation(input,{...options,action:'draft',amount:angle}).bodies[0]
 const t=Math.tan(angle*Math.PI/180)
 for(let i=0;i<result.mesh.positions.length;i+=3){
  const [x,y,z]=input.bodies[0].mesh.positions.slice(i,i+3),f=1+z*t/5
  expect(result.mesh.positions[i]).toBeCloseTo(5+(x-5)*f,10)
  expect(result.mesh.positions[i+1]).toBeCloseTo(5+(y-5)*f,10)
  expect(result.mesh.positions[i+2]).toBeCloseTo(z,10)
 }
 expect(result.mesh.indices).toEqual(input.bodies[0].mesh.indices)
 expect(inspectPolygonMesh(result.mesh).closed).toBe(true)
 expect(result.id).toBe('0');expect(JSON.stringify(input)).toBe(before)
})
it('refuses unsupported curved B-rep draft atomically even after a mesh operand',()=>{
 const brep=createBrepSphere(2),input=doc(box(),{id:'b',name:'B',brep,mesh:tessellateNurbsBrep(brep,1)}),before=JSON.stringify(input)
 expect(()=>cadOperation(input,{...options,action:'draft',ids:['0','b']})).toThrow()
 expect(JSON.stringify(input)).toBe(before)
 expect(cadOperation(doc(box()),{...options,action:'draft',amount:0}).bodies[0].mesh.positions).toEqual(input.bodies[0].mesh.positions)
})
it('refuses collapsed and invalid draft requests without publishing partial results',()=>{
 const input=doc(box()),before=JSON.stringify(input)
 for(const patch of [{amount:-60},{amount:61},{axis:[0,0,0] as [number,number,number]}])expect(()=>cadOperation(input,{...options,action:'draft',...patch})).toThrow()
 expect(JSON.stringify(input)).toBe(before)
})

it('drafts retained planar prism supports and regenerates a matching closed display mesh',()=>{
 const brep=createBrepBox([0,0,0],[10,10,10]),input=doc({id:'0',name:'Prism',brep,mesh:tessellateNurbsBrep(brep,1)}),before=JSON.stringify(input)
 for(const axis of [[0,0,1],[1,0,0]] as [number,number,number][]){
  for(const angle of [10,-10]){
   const result=cadOperation(input,{...options,action:'draft',axis,amount:angle}).bodies[0]
   const t=Math.tan(angle*Math.PI/180),expected=1000+2000*t+4000*t*t/3
   expect(analyzeNurbsBrep(result.brep!).signedVolumeMm3).toBeCloseTo(expected,6)
   expect(inspectPolygonMesh(result.mesh).signedVolumeMm3).toBeCloseTo(expected,6)
   expect(inspectPolygonMesh(result.mesh).closed).toBe(true)
   expect(result.brep!.faces).toHaveLength(6);expect(result.id).toBe('0');expect(result.name).toBe('Prism')
  }
 }
 expect(cadOperation(input,{...options,action:'draft',amount:0}).bodies[0].brep).toEqual(brep)
 const middle=cadOperation(input,{...options,action:'draft',origin:[0,0,5],amount:10}).bodies[0]
 expect(analyzeNurbsBrep(middle.brep!).signedVolumeMm3).toBeCloseTo(1000+1000*Math.tan(Math.PI/18)**2/3,6)
 expect(()=>cadOperation(input,{...options,action:'draft',amount:-60})).toThrow()
 expect(JSON.stringify(input)).toBe(before)
})

it('drafts a rotated translated prism in its own axis frame',()=>{
 const c=Math.SQRT1_2,brep=transformNurbsBrep(createBrepBox([0,0,0],[10,10,10]),[[c,0,c,5],[0,1,0,-3],[-c,0,c,7],[0,0,0,1]])
 const input=doc({id:'0',name:'Placed',brep,mesh:tessellateNurbsBrep(brep,1)})
 const result=cadOperation(input,{...options,action:'draft',axis:[c,0,c],origin:[5,-3,7],amount:10}).bodies[0]
 const t=Math.tan(Math.PI/18)
 expect(analyzeNurbsBrep(result.brep!).signedVolumeMm3).toBeCloseTo(1000+2000*t+4000*t*t/3,6)
 expect(inspectPolygonMesh(result.mesh).closed).toBe(true)
 expect(()=>cadOperation(input,{...options,action:'draft',axis:[0,0,1]})).toThrow('prism')
})
it('samples polyline arclength in native batches with clamping and repeated vertices',async()=>{
 const {pathPoint,pathPoints}=await import('../src/services/cadWorkbench')
 const path=[[0,0,0],[0,0,0],[3,0,0],[3,4,0]],before=JSON.stringify(path)
 const fractions=[-1,0,3/7,.5,1,2]
 const expected=[[0,0,0],[0,0,0],[3,0,0],[3,.5,0],[3,4,0],[3,4,0]]
 expect(pathPoints(path,fractions)).toEqual(expected)
 expect(pathPoint(path,.5)).toEqual([3,.5,0])
 expect(pathPoints([[0,0],[3,0],[3,4]],[.5])).toEqual([[3,.5]])
 expect(JSON.stringify(path)).toBe(before)
})
it('rejects invalid path sampling atomically and recovers on a valid path',async()=>{
 const {pathPoints}=await import('../src/services/cadWorkbench')
 for(const path of [[],[[0,0]],[[0,0],[0,0]],[[0,0],[1,2,3]],[[0,0],[Infinity,1]]])expect(()=>pathPoints(path,[.5])).toThrow()
 expect(()=>pathPoints([[0,0],[1,0]],[0,NaN])).toThrow()
 expect(()=>pathPoints([[0,0],[1,0]],Array(4097).fill(0))).toThrow('4096')
 expect(pathPoints([[0,0],[1,0]],[.5])).toEqual([[.5,0]])
})
it('matches unequal loft sections natively and refuses an invalid later section atomically',()=>{
 const sketches=[{id:'a',name:'A',closed:true,points:[[0,0],[4,0],[4,4],[0,4]] as [number,number][]},{id:'b',name:'B',closed:true,points:[[0,0],[2,0],[4,0],[4,2],[4,4],[2,4],[0,4],[0,2]] as [number,number][],plane:{origin:[0,0,10] as [number,number,number],u:[1,0,0] as [number,number,number],v:[0,1,0] as [number,number,number]}}]
 const input=doc(box()),before=JSON.stringify(input),operation={...options,action:'loft' as const,profileIds:['a','b'],sketches}
 const result=cadOperation(input,operation)
 expect(result.bodies).toHaveLength(2)
 expect(inspectPolygonMesh(result.bodies[1].mesh).signedVolumeMm3).toBeCloseTo(160,8)
 expect(result.bodies[0]).toEqual(input.bodies[0]);expect(JSON.stringify(input)).toBe(before)
 expect(()=>cadOperation(input,{...operation,sketches:[sketches[0],{...sketches[1],closed:false}]})).toThrow('closed')
 expect(()=>cadOperation(input,{...operation,profileIds:['missing']})).toThrow()
 expect(JSON.stringify(input)).toBe(before)
})
it('does not preserve a stale B-rep after a mesh-only threading request',()=>{
 const brep=createBrepBox([0,0,0],[10,10,10]),input=doc({id:'0',name:'Stock',brep,mesh:tessellateNurbsBrep(brep,1)}),before=JSON.stringify(input)
 expect(()=>cadOperation(input,{...options,action:'thread'})).toThrow('retained B-rep')
 expect(JSON.stringify(input)).toBe(before)
})
it('applies native mesh threading in either axis frame and preserves body metadata',async()=>{
 const {buildModelGraphThread}=await import('../src/services/modelGraphThreads')
 const cutter=buildModelGraphThread({diameter:4,pitch:1,length:2,internal:false,wall:1,clearance:0,starts:1,left_handed:false,segments_per_turn:16})
 const removed=inspectPolygonMesh(cutter.mesh).signedVolumeMm3
 const input=doc(box()),before=JSON.stringify(input)
 for(const [axis,origin] of [[[0,0,1],[5,5,0]],[[1,0,0],[0,5,5]]] as [[number,number,number],[number,number,number]][]){
  const result=cadOperation(input,{...options,action:'thread',axis,origin,width:4,depth:2,pitch:1}).bodies[0]
  expect(inspectPolygonMesh(result.mesh).closed).toBe(true)
  expect(inspectPolygonMesh(result.mesh).signedVolumeMm3).toBeCloseTo(1000-removed,5)
  expect(result.id).toBe('0');expect(result.name).toBe(input.bodies[0].name)
 }
 expect(JSON.stringify(input)).toBe(before)
 const external=cadOperation(input,{...options,action:'thread',mode:'external',axis:[0,0,1],origin:[5,5,9],width:4,depth:2,pitch:1}).bodies[0]
 // Independent triangle clipping above local z=1. Translate that plane to z=0:
 // its closing cap then contributes zero to the signed tetrahedral volume.
 let exposed=0
 for(let i=0;i<cutter.mesh.indices.length;i+=3){
  const triangle=cutter.mesh.indices.slice(i,i+3).map(id=>{const p=cutter.mesh.positions.slice(3*id,3*id+3);p[2]-=1;return p}),polygon:number[][]=[]
  for(let j=0;j<3;j++){
   const a=triangle[j],b=triangle[(j+1)%3],inside=a[2]>=0,next=b[2]>=0
   if(inside)polygon.push(a)
   if(inside!==next){const t=-a[2]/(b[2]-a[2]);polygon.push(a.map((x,k)=>x+t*(b[k]-x)))}
  }
  for(let j=1;j+1<polygon.length;j++){
   const [a,b,c]=[polygon[0],polygon[j],polygon[j+1]]
   exposed+=(a[0]*(b[1]*c[2]-b[2]*c[1])+a[1]*(b[2]*c[0]-b[0]*c[2])+a[2]*(b[0]*c[1]-b[1]*c[0]))/6
  }
 }
 expect(inspectPolygonMesh(external.mesh).signedVolumeMm3).toBeCloseTo(1000+exposed,5)
 expect(()=>cadOperation(input,{...options,action:'thread',axis:[0,0,0]})).toThrow('direction')
})
it('builds native mesh holes, counterbores and countersinks with polygonal volume checks',()=>{
 const input=doc(box()),before=JSON.stringify(input),area=24*Math.sin(Math.PI/24)
 for(const [mode,removed] of [['plain',6*area],['counterbore',12*area],['countersink',26*area/3]] as [string,number][]){
  for(const [axis,origin] of [[[0,0,1],[5,5,0]],[[1,0,0],[0,5,5]]] as [[number,number,number],[number,number,number]][]){
   const result=cadOperation(input,{...options,action:'hole',mode,width:2,secondary:4,depth:6,height:2,axis,origin}).bodies[0]
   const report=inspectPolygonMesh(result.mesh)
   expect(report.closed).toBe(true);expect(report.signedVolumeMm3).toBeCloseTo(1000-removed,5)
   expect(result.name).toBe(input.bodies[0].name);expect(result.id).toBe('0')
  }
 }
 expect(()=>cadOperation(input,{...options,action:'hole',mode:'counterbore',width:4,secondary:2})).toThrow('diameter')
 expect(()=>cadOperation(input,{...options,action:'hole',axis:[0,0,0]})).toThrow('direction')
 expect(JSON.stringify(input)).toBe(before)
})
it('reduces mesh Boolean operands natively and removes selected bodies for an empty result',()=>{
 const input=doc(box('a'),box('b',5),box('c',10),box('keep',50)),before=JSON.stringify(input)
 const union=cadOperation(input,{...options,action:'union',ids:['b','a','c']})
 expect(union.bodies.map(b=>b.id)).toEqual(['b','keep'])
 expect(inspectPolygonMesh(union.bodies[0].mesh).signedVolumeMm3).toBeCloseTo(2000,7)
 const empty=cadOperation(input,{...options,action:'difference',ids:['a','a']})
 expect(empty.bodies.map(b=>b.id)).toEqual(['b','c','keep'])
 const intersection=cadOperation(input,{...options,action:'intersection',ids:['a','b','c']})
 expect(intersection.bodies.map(b=>b.id)).toEqual(['keep'])
 expect(JSON.stringify(input)).toBe(before)
})
it('admits later mesh operands even after an empty Boolean intermediate',()=>{
 const bad=box('bad');bad.mesh.indices[0]=999999
 const input=doc(box('a'),box('b',20),bad),before=JSON.stringify(input)
 expect(()=>cadOperation(input,{...options,action:'intersection',ids:['a','b','bad']})).toThrow()
 expect(JSON.stringify(input)).toBe(before)
 expect(cadOperation(doc(box('a'),box('b',20)),{...options,action:'intersection',ids:['a','b']}).bodies).toEqual([])
})
