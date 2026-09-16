import {it,expect} from 'vitest'
import {DirectSolidCapabilityError,pushPullFace,shellSolid,solidTopology} from '../src/services/directSolidTools'
import {createBrepBox,createBrepSphere,tessellateNurbsBrep,analyzeNurbsBrep} from '../src/services/geometry/brep'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
const stock=()=>{const brep=createBrepBox([0,0,0],[10,10,10]);return {id:'b',name:'Stock',brep,mesh:tessellateNurbsBrep(brep,1)}}
const top=(b:ReturnType<typeof stock>)=>solidTopology(b.mesh).faces.findIndex(f=>f.normal[2]>.99)
const check=(b:ReturnType<typeof pushPullFace>,volume:number)=>{
 expect(b.brep).toBeDefined();expect(analyzeNurbsBrep(b.brep!).signedVolumeMm3).toBeCloseTo(volume,7)
 const report=inspectPolygonMesh(b.mesh);expect(report.closed).toBe(true);expect(report.signedVolumeMm3).toBeCloseTo(volume,7)
}
it('pushes and pulls retained planar bodies with synchronized meshes and supports repeated edits',()=>{
 const input=stock(),before=JSON.stringify(input)
 const raised=pushPullFace(input,top(input),3);check(raised,1300)
 expect(raised.id).toBe(input.id);expect(raised.name).toBe(input.name)
 check(pushPullFace(raised,solidTopology(raised.mesh).faces.findIndex(f=>f.normal[2]>.99),-3),1000)
 check(pushPullFace(input,top(input),-3),700)
 expect(JSON.stringify(input)).toBe(before)
})
it('refuses retained shells with a typed capability error and preserves source geometry',()=>{
 const input=stock(),before=JSON.stringify(input),opening=top(input)
 let refusal:unknown
 try{shellSolid(input,[opening],1)}catch(error){refusal=error}
 expect(refusal).toBeInstanceOf(DirectSolidCapabilityError)
 expect((refusal as DirectSolidCapabilityError).code).toBe('BREP_ANALYTIC_SHELL_REFUSED')
 expect((refusal as Error).message).toContain('refuse faceted fallback')
 expect(JSON.stringify(input)).toBe(before)
})
it('refuses invalid and curved edits without falling back to mesh geometry',()=>{
 const input=stock(),before=JSON.stringify(input)
 expect(()=>pushPullFace(input,-1,1)).toThrow()
 expect(()=>pushPullFace(input,top(input),-11)).toThrow()
 expect(()=>shellSolid(input,[999],1)).toThrow()
 expect(JSON.stringify(input)).toBe(before)
 const brep=createBrepSphere(2),sphere={id:'s',name:'Sphere',brep,mesh:tessellateNurbsBrep(brep,4)}
 expect(()=>pushPullFace(sphere,0,1)).toThrow()
 expect(()=>shellSolid(sphere,[0],1)).toThrow()
})
it('refuses retained chamfer and fillet faceting with typed errors',async()=>{
 const {bevelSolidEdge}=await import('../src/services/directSolidTools')
 const input=stock(),before=JSON.stringify(input)
 for(const [kind,code] of [['chamfer','BREP_ANALYTIC_CHAMFER_REFUSED'],['fillet','BREP_ANALYTIC_FILLET_REFUSED']] as const){
  let refusal:unknown
  try{bevelSolidEdge(input,0,2,kind)}catch(error){refusal=error}
  expect(refusal).toBeInstanceOf(DirectSolidCapabilityError)
  expect((refusal as DirectSolidCapabilityError).code).toBe(code)
  expect((refusal as Error).message).toContain('refuse faceted fallback')
 }
 expect(JSON.stringify(input)).toBe(before)
})
it('refuses retained edge chains atomically without faceted fallback',async()=>{
 const {bevelBrepBody}=await import('../src/services/directSolidTools')
 const input=stock(),edges=solidTopology(input.mesh).edges,first=edges[0]
 const connected=edges.findIndex((e,i)=>i>0&&[e.a,e.b].some(v=>v===first.a||v===first.b))
 expect(connected).toBeGreaterThan(0)
 const before=JSON.stringify(input)
 for(const kind of ['chamfer','fillet'] as const){
  let refusal:unknown
  try{bevelBrepBody(input,[0,connected],1,kind,4)}catch(error){refusal=error}
  expect(refusal).toBeInstanceOf(DirectSolidCapabilityError)
  expect((refusal as DirectSolidCapabilityError).code).toBe(kind==='chamfer'?'BREP_ANALYTIC_CHAMFER_REFUSED':'BREP_ANALYTIC_FILLET_REFUSED')
 }
 expect(JSON.stringify(input)).toBe(before)
})
