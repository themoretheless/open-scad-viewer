import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {obliqueSphereRequests} from './fixtures/oblique-sphere-distance'
import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {solidDistanceExpectation,validSolidDistance} from '../src/services/solidDistance'
const cases=JSON.parse(readFileSync(new URL('../docs/qualification/cad-roadmap-2026-09-28/solid-distance-2026-09-30/contract-fixtures.json',import.meta.url),'utf8')).cases
it('accepts native containment, invalid-volume, contact and separated-volume reports',()=>{
 for(const c of cases)expect(validSolidDistance(solidDistanceExpectation(c.request),c.result)).toBe(true)
 expect(cases[0].result.reason).toBe('material-containment')
 expect(cases[1].result.reason).toBe('volume-validity-unproven')
 expect(cases[2].result.reason).toBe('certified-boundary-contact')
 expect(cases[3].result.reason).toBe('separated-volumes')
 const gap=cases[3].result.distanceIntervalMm
 expect(gap[0]).toBeLessThanOrEqual(3);expect(gap[1]).toBeGreaterThanOrEqual(3)
 expect(gap[1]-gap[0]).toBeLessThanOrEqual(cases[3].request.toleranceMm)
})
it('rejects inconsistent proof, budgets, shell ownership and contact coordinates',()=>{
 const {request,result}=cases[0],e=solidDistanceExpectation(request)
 for(const patch of [{scope:'boundary-shells-bounded-joins'},{converged:false},{distanceIntervalMm:[1,2]},{materialOverlap:null},{visitedShellPairs:0},{totalShellPairs:2},{cells:request.maxCells+1},{validity:[]},{reason:'separated-volumes'}])expect(validSolidDistance(e,{...result,...patch})).toBe(false)
 for(const mutate of [
  (r:any)=>r.validity[0].exactAgreement=false,
  (r:any)=>r.validity[0].orientations[0].expectedOutward=false,
  (r:any)=>r.validity[0].orientations[0].outward=null,
  (r:any)=>r.validity[0].nestingRolesConsistent=null,
  (r:any)=>r.limits.validity.exactWork++,
 ]){const r=structuredClone(result);mutate(r);expect(validSolidDistance(e,r)).toBe(false)}
 const contact=cases[2],ce=solidDistanceExpectation(contact.request)
 for(const mutate of [
  (r:any)=>r.contact.faces[0]=999,
  (r:any)=>r.contact.firstUv[0]=[-1,-1],
  (r:any)=>r.contact.contractionUpper=0.5,
  (r:any)=>r.contact.pointIntervalMm[0]=[2,1],
  (r:any)=>r.contactPairsVisited=0,
  (r:any)=>r.contact=null,
 ]){const r=structuredClone(contact.result);mutate(r);expect(validSolidDistance(ce,r)).toBe(false)}
 const separated=cases[3],se=solidDistanceExpectation(separated.request)
 for(const patch of [{materialOverlap:true},{distanceIntervalMm:[0,3]},{visitedShellPairs:0},{converged:false},{reason:'material-containment'}])expect(validSolidDistance(se,{...separated.result,...patch})).toBe(false)
 for(const mutate of [
  (r:any)=>r.separationWitness=null,
  (r:any)=>r.separationWitness.faces[0]=999,
  (r:any)=>r.separationWitness.parameters[0][0]=-1,
  (r:any)=>r.separationWitness.points[0][0]=Infinity,
  (r:any)=>r.separationWitness.pointEnclosures[0][0]=[9,8],
  (r:any)=>{r.separationWitness.points[0][0]=20;r.separationWitness.pointEnclosures[0][0]=[20,20]},
 ]){const r=structuredClone(separated.result);mutate(r);expect(validSolidDistance(se,r)).toBe(false)}
 const invalid=cases[1],ie=solidDistanceExpectation(invalid.request)
 expect(validSolidDistance(ie,{...invalid.result,distanceIntervalMm:[0,0]})).toBe(false)
 expect(validSolidDistance(ie,{...invalid.result,converged:true})).toBe(false)
})
it('terminates an obsolete solid-distance request and rejects incomplete proof in replies',async()=>{
 const {MainSolidWorkerClient}=await import('../src/services/mainSolidWorkerClient')
 const ports:any[]=[]
 const client=new MainSolidWorkerClient(()=>{const p:any={onmessage:null,onerror:null,onmessageerror:null,terminated:false,postMessage(m:any){this.request=m},terminate(){this.terminated=true}};ports.push(p);return p})
 const job={kind:'solidDistance' as const,options:cases[0].request}
 try{
  const first=client.run(job),cancelled=expect(first).rejects.toMatchObject({name:'AbortError'}),stale=ports[0].onmessage,old=ports[0].request
  const second=client.run(job);await cancelled;expect(ports[0].terminated).toBe(true)
  let settled=false;void second.then(()=>{settled=true})
  stale({data:{version:1,id:old.id,kind:job.kind,ok:true,result:cases[0].result}})
  await Promise.resolve();expect(settled).toBe(false)
  ports[1].onmessage({data:{version:1,id:ports[1].request.id,kind:job.kind,ok:true,result:cases[0].result}})
  await expect(second).resolves.toMatchObject({distanceIntervalMm:[0,0]})
  const forged=client.run(job),rejected=expect(forged).rejects.toMatchObject({code:'CAD_PROTOCOL'}),port=ports.at(-1)
  port.onmessage({data:{version:1,id:port.request.id,kind:job.kind,ok:true,result:{...cases[0].result,validity:[]}}})
  await rejected
 }finally{client.dispose()}
})

it('validates actual WASM volume-distance responses without changing either input',async()=>{
 const {measureSolidDistance}=await import('../src/services/solidDistance')
 for(const [index,c] of cases.entries()){
  const before=JSON.stringify(c.request),r=measureSolidDistance(c.request)
  expect(validSolidDistance(solidDistanceExpectation(c.request),r)).toBe(true)
  expect(r.reason).toBe(c.result.reason)
  if(c.result.distanceIntervalMm===null)expect(r.distanceIntervalMm).toBeNull()
  if(c.result.separationWitness===null)expect(r.separationWitness).toBeNull()
  else {
   const witness=r.separationWitness!
   expect(witness).not.toBeNull()
   for(let side=0;side<2;side++){
    const surface=[c.request.a,c.request.b][side].faces[witness.faces[side]].surface
    const uv=witness.parameters[side],point=evaluateNurbsSurface(surface,uv[0],uv[1]).point
    for(let axis=0;axis<3;axis++){
     expect(witness.points[side][axis]).toBeCloseTo(point[axis],10)
     expect(witness.pointEnclosures[side][axis][0]).toBeLessThanOrEqual(witness.points[side][axis])
     expect(witness.pointEnclosures[side][axis][1]).toBeGreaterThanOrEqual(witness.points[side][axis])
    }
   }
  }
  if(r.reason==='separated-volumes'){const gap=c.expectedDistanceMm??(index===3?3:1);expect(r.distanceIntervalMm![0]).toBeLessThanOrEqual(gap);expect(r.distanceIntervalMm![1]).toBeGreaterThanOrEqual(gap);expect(r.distanceIntervalMm![1]-r.distanceIntervalMm![0]).toBeLessThanOrEqual(c.request.toleranceMm)}
  expect(JSON.stringify(c.request)).toBe(before)
 }
})

it('keeps the cavity witness on the inner authored shell',()=>{
 const {request,result}=cases[4]
 expect(result.reason).toBe('separated-volumes')
 expect(validSolidDistance(solidDistanceExpectation(request),result)).toBe(true)
 const face=result.separationWitness.faces[0]
 expect(request.a.bodies[0].innerShells.some((s:number)=>request.a.shells[s].faces.some((f:any)=>f.face===face))).toBe(true)
 expect(result.distanceIntervalMm[0]).toBeLessThanOrEqual(1)
 expect(result.distanceIntervalMm[1]).toBeGreaterThanOrEqual(1)
})

it('validates native curved-cylinder volume and original-face witness reports',()=>{
 const {request,result,displayMeshes}=cases[5]
 expect(validSolidDistance(solidDistanceExpectation(request),result)).toBe(true)
 expect(result.validity.every((v:any)=>v.proven)).toBe(true)
 expect(result.reason).toBe('separated-volumes')
 expect(result.distanceIntervalMm[0]).toBeLessThanOrEqual(3)
 expect(result.distanceIntervalMm[1]).toBeGreaterThanOrEqual(3)
 expect(displayMeshes.every((mesh:any)=>mesh.positions.length>100)).toBe(true)
})

it('validates native exact-sphere volume and separation without accepting incomplete validity',()=>{
 const {request,result,displayMeshes,expectedDistanceMm}=cases[6]
 const e=solidDistanceExpectation(request)
 expect(validSolidDistance(e,result)).toBe(true);expect(expectedDistanceMm).toBe(2)
 expect(result.validity.every((v:any)=>v.proven&&v.boundaryProven&&v.exactAgreement&&v.selfIntersectionAbsent)).toBe(true)
 expect(result.reason).toBe('separated-volumes');expect(result.materialOverlap).toBe(false)
 expect(result.distanceIntervalMm[0]).toBeLessThanOrEqual(2);expect(result.distanceIntervalMm[1]).toBeGreaterThanOrEqual(2)
 expect(result.separationWitness.points).toHaveLength(2)
 expect(displayMeshes.every((mesh:any)=>mesh.positions.length>100)).toBe(true)
 for(const side of [0,1]){
  const incomplete=structuredClone(result);incomplete.validity[side].selfIntersectionAbsent=false
  expect(validSolidDistance(e,incomplete)).toBe(false)
 }
})

it('qualifies actual WASM exact sphere distances across the tested radius scales',async()=>{
 const {callGeometryRust}=await import('../src/services/geometry/kernel')
 const {measureSolidDistance}=await import('../src/services/solidDistance')
 for(const radius of [0.000011444091796875,0.375,1.5,6,12,786432]){
  const a=callGeometryRust<any>('brep_nurbs_sphere',{radius}),contained=radius===786432
  const b=contained?callGeometryRust<any>('brep_nurbs_sphere',{radius:3}):structuredClone(a),offset=2*radius+2
  if(!contained){
   for(const v of b.vertices)v.point[0]+=offset
   for(const edge of b.edges)for(const p of edge.curve.controlPoints)p[0]+=offset
   for(const face of b.faces)for(const row of face.surface.controlPoints)for(const p of row)p[0]+=offset
  }
  const options={...structuredClone(cases[6].request),a,b},before=JSON.stringify(options)
  const r=measureSolidDistance(options)
  expect(validSolidDistance(solidDistanceExpectation(options),r)).toBe(true)
  expect(r.validity.every(v=>v.proven),`radius=${radius}`).toBe(true)
  expect(r.converged,`radius=${radius}, reason=${r.reason}`).toBe(true)
  expect(r.materialOverlap).toBe(contained)
  const gap=contained?0:2
  expect(r.distanceIntervalMm![0]).toBeLessThanOrEqual(gap);expect(r.distanceIntervalMm![1]).toBeGreaterThanOrEqual(gap)
  expect(r.distanceIntervalMm![1]-r.distanceIntervalMm![0]).toBeLessThanOrEqual(options.toleranceMm)
  expect(JSON.stringify(options)).toBe(before)
  if(contained){
   const invalid=structuredClone(a);for(const v of invalid.vertices)v.point[0]+=offset
   expect(()=>measureSolidDistance({...options,b:invalid})).toThrow('Invalid vertex coordinates')
  }
 }
})

it('audits multispan NURBS trim boundaries in WASM and retains unknown when trim work runs out',async()=>{
 const {warmGeometryKernel}=await import('../src/services/geometry/kernel')
 const {measureSolidDistance}=await import('../src/services/solidDistance')
 await warmGeometryKernel()
 const request=structuredClone(cases[3].request)
 const coedge=request.a.loops[request.a.faces[0].outer].coedges[0],curve=coedge.pcurve
 const [a,b]=curve.controlPoints
 coedge.pcurve={degree:2,knots:[0,0,0,.5,1,1,1],controlPoints:[0,.25,.75,1].map(t=>a.map((x:number,i:number)=>x*(1-t)+b[i]*t)),weights:[1,1,1,1],periodic:false}
 const before=structuredClone(request)
 const result=measureSolidDistance(request)
 expect(result.validity[0].trimValid).toBe(true)
 expect(validSolidDistance(solidDistanceExpectation(request),result)).toBe(true)
 const limited={...request,validityLimits:{...request.validityLimits,trimCells:1}}
 const incomplete=measureSolidDistance(limited)
 expect(incomplete.validity[0].trimValid).toBe(false)
 expect(incomplete.converged).toBe(false)
 expect(incomplete.reason).toBe('volume-validity-unproven')
 expect(validSolidDistance(solidDistanceExpectation(limited),incomplete)).toBe(true)
 expect(request).toEqual(before)
})

it('converges oblique sphere clearances with original-face witnesses in actual WASM',async()=>{
 const {measureSolidDistance}=await import('../src/services/solidDistance')
 for(const {options,expected} of await obliqueSphereRequests()){
  const before=structuredClone(options),r=measureSolidDistance(options)
  expect(validSolidDistance(solidDistanceExpectation(options),r)).toBe(true)
  expect(r.converged,JSON.stringify(r)).toBe(true)
  expect(r.reason).toBe('separated-volumes')
  expect(r.distanceIntervalMm![0]).toBeLessThanOrEqual(expected)
  expect(r.distanceIntervalMm![1]).toBeGreaterThanOrEqual(expected)
  expect(r.distanceIntervalMm![1]-r.distanceIntervalMm![0]).toBeLessThanOrEqual(options.toleranceMm)
  expect(r.separationWitness).not.toBeNull()
  expect(options).toEqual(before)
 }
})
