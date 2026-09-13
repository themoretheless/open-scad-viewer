import {it,expect} from 'vitest'
import {pushPullFace,shellSolid,solidTopology} from '../src/services/directSolidTools'
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
it('builds a retained shell through selected openings and preserves source geometry',()=>{
 const input=stock(),before=JSON.stringify(input),opening=top(input)
 check(shellSolid(input,[opening],1),424)
 check(shellSolid(input,[opening,opening],1),424)
 expect(JSON.stringify(input)).toBe(before)
 expect(()=>shellSolid(input,[],1)).toThrow()
 expect(()=>shellSolid(input,solidTopology(input.mesh).faces.map((_,i)=>i),1)).toThrow('closed face')
 expect(()=>shellSolid(input,[opening],6)).toThrow()
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
it('retains B-rep for a chamfer and explicitly faceted fillet',async()=>{
 const {bevelSolidEdge}=await import('../src/services/directSolidTools')
 const input=stock(),before=JSON.stringify(input)
 const chamfer=bevelSolidEdge(input,0,2,'chamfer');check(chamfer,980)
 const fillet=bevelSolidEdge(input,0,2,'fillet'),report=inspectPolygonMesh(fillet.mesh)
 expect(fillet.brep!.faces.length).toBeGreaterThan(chamfer.brep!.faces.length)
 expect(report.closed).toBe(true)
 expect(report.signedVolumeMm3).toBeCloseTo(1000-10*(4-Math.PI),0)
 expect(analyzeNurbsBrep(fillet.brep!).signedVolumeMm3).toBeCloseTo(report.signedVolumeMm3,7)
 expect(chamfer.id).toBe(input.id);expect(fillet.name).toBe(input.name)
 expect(JSON.stringify(input)).toBe(before)
})
it('edits connected edge chains atomically and refuses invalid chains or parameters',async()=>{
 const {bevelBrepBody}=await import('../src/services/directSolidTools')
 const input=stock(),edges=solidTopology(input.mesh).edges,first=edges[0]
 const connected=edges.findIndex((e,i)=>i>0&&[e.a,e.b].some(v=>v===first.a||v===first.b))
 expect(connected).toBeGreaterThan(0)
 for(const kind of ['chamfer','fillet'] as const){
  const edited=bevelBrepBody(input,[0,connected],1,kind,4)
  expect(edited.brep).toBeDefined();expect(inspectPolygonMesh(edited.mesh).closed).toBe(true)
  expect(analyzeNurbsBrep(edited.brep!).signedVolumeMm3).toBeCloseTo(inspectPolygonMesh(edited.mesh).signedVolumeMm3,7)
 }
 const before=JSON.stringify(input)
 expect(()=>bevelBrepBody(input,[0,999],1,'chamfer')).toThrow()
 expect(()=>bevelBrepBody(input,[],1,'chamfer')).toThrow()
 expect(()=>bevelBrepBody(input,[0],50,'fillet')).toThrow()
 expect(()=>bevelBrepBody(input,[0],1,'fillet',33)).toThrow()
 expect(JSON.stringify(input)).toBe(before)
})
