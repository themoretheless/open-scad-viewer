import {expect,it} from 'vitest'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {inspectSolidDisplay} from '../src/services/solidDiagnostics'
import {solidExactEdgeFeature} from '../src/services/solidExactEdgeFeature'
const fixture=()=>tessellateNurbsBrep(createBrepBox([0,0,0],[10,8,6]))
it.each([[0,0,1],[1,0,0],[0,1,0],[1,1,1]])('places a closed section on the requested world-space plane %j',(...normal)=>{
 const mesh=fixture(),before=structuredClone(mesh),offset=3
 const result=inspectSolidDisplay(mesh,offset,normal)
 expect(result.sectionError).toBe('')
 expect(result.section.contours).toHaveLength(1)
 const points=result.section.contours[0].points,length=Math.hypot(...normal)
 expect(points[0]).toEqual(points.at(-1))
 for(const p of points){
  expect(p).toHaveLength(3)
  expect(p.reduce((sum,x,i)=>sum+x*normal[i]/length,0)).toBeCloseTo(offset,9)
  for(let i=0;i<3;i++){expect(p[i]).toBeGreaterThanOrEqual(-1e-8);expect(p[i]).toBeLessThanOrEqual([10,8,6][i]+1e-8)}
 }
 expect(mesh).toEqual(before)
})
it('normalizes plane normal without changing the distance in mm',()=>{
 expect(inspectSolidDisplay(fixture(),2,[0,0,10]).section).toEqual(inspectSolidDisplay(fixture(),2,[0,0,1]).section)
})
it('keeps topology diagnostics visible if the plane is invalid or misses the model',()=>{
 const invalid=inspectSolidDisplay(fixture(),2,[0,0,0])
 expect(invalid.sectionError).toContain('normal')
 expect(invalid.report.closed).toBe(true)
 const outside=inspectSolidDisplay(fixture(),20)
 expect(outside.sectionError).toBe('');expect(outside.section.contours).toEqual([])
})
it('keeps the open-boundary evidence when a defective mesh cannot form a closed section',()=>{
 const mesh=fixture(),indices=Array.from(mesh.indices)
 const triangle=Array.from({length:indices.length/3},(_,i)=>i).find(i=>{
  const z=indices.slice(i*3,i*3+3).map(v=>mesh.positions[v*3+2])
  return Math.min(...z)<3&&Math.max(...z)>3
 })!
 indices.splice(triangle*3,3);mesh.indices=Uint32Array.from(indices)
 const result=inspectSolidDisplay(mesh,3)
 expect(result.report.boundaryEdges).toBe(3)
 expect(result.boundaries.length).toBeGreaterThan(0)
 expect(result.sectionError).not.toBe('')
})
it('shows the real variable-fillet section without welding collapsed floating-point nodes',()=>{
 const brep=createBrepBox([-10,-10,0],[10,10,20])
 const source={id:'part',name:'Part',brep,mesh:tessellateNurbsBrep(brep)}
 const body=solidExactEdgeFeature(source,[8],2,'fillet','brep',{mode:'variable',endRadius:3}).body
 const result=inspectSolidDisplay(body.mesh,5)
 expect(result.sectionError).toBe('')
 expect(result.section.contours).toHaveLength(1)
 expect(result.section.collapsedSegmentTriangles.length).toBeGreaterThan(0)
 expect(result.section.contours[0].points.every(point=>point[2]===5)).toBe(true)
})
it('localizes branching boundaries even when ordered boundary loops are impossible',()=>{
 const mesh={positions:Float64Array.from([0,0,0, 1,0,0, 0,1,0, 0,-1,0, 0,0,1]),indices:Uint32Array.from([0,1,2,0,1,3,1,0,4])}
 const before=structuredClone(mesh),result=inspectSolidDisplay(mesh,.2)
 expect(result.locations.nonManifoldEdges).toEqual([[0,1]])
 expect(result.boundaryError).not.toBe('')
 expect(result.defectLines.filter(d=>d.kind==='boundary')).toHaveLength(result.report.boundaryEdges)
 expect(result.defectLines.find(d=>d.kind==='non-manifold')?.points).toEqual([[0,0,0],[1,0,0]])
 expect(mesh).toEqual(before)
})
it('localizes orientation conflicts and degenerate triangles with source indices',()=>{
 const mesh={positions:Float64Array.from([0,0,0, 1,0,0, 0,1,0, 0,-1,0]),indices:Uint32Array.from([0,1,2,0,1,3])}
 const orientation=inspectSolidDisplay(mesh,.2)
 expect(orientation.locations.orientationEdges).toEqual([[0,1]])
 expect(orientation.defectLines.filter(d=>d.kind==='orientation')).toHaveLength(orientation.report.orientationConflicts)
 mesh.indices=Uint32Array.from([0,1,1])
 const degenerate=inspectSolidDisplay(mesh,.2)
 expect(degenerate.locations.degenerateTriangles).toEqual([0])
 expect(degenerate.defectLines.find(d=>d.kind==='degenerate')?.points).toEqual([[0,0,0],[1,0,0],[1,0,0],[0,0,0]])
})

it('locates an overlapping pair through WASM with original triangle outlines',async()=>{
 const {inspectSolidIntersections}=await import('../src/services/solidDiagnostics')
 const mesh={positions:new Float64Array([0,0,0,2,0,0,0,2,0]),indices:new Uint32Array([0,1,2,0,1,2])}
 const result=inspectSolidIntersections(mesh)
 expect(result.contact?.triangles).toEqual([0,1])
 expect(result.lines).toHaveLength(2)
 expect(result.lines[0]).toEqual([[0,0,0],[2,0,0],[0,2,0],[0,0,0]])
 expect(result.contact?.toleranceMm).toBeCloseTo(2e-9,15)
 expect(inspectSolidIntersections(fixture()).contact).toBeNull()
})

it('enumerates every overlap and distinguishes an empty partial scan from completion',async()=>{
 const {inspectSolidIntersections}=await import('../src/services/solidDiagnostics')
 const mesh={positions:new Float64Array([0,0,0,2,0,0,0,2,0]),indices:new Uint32Array([0,1,2,0,1,2,0,1,2])}
 const all=inspectSolidIntersections(mesh)
 expect(all.complete).toBe(true);expect(all.contacts.map(c=>c.triangles)).toEqual([[0,1],[0,2],[1,2]])
 expect(all.triangleIds).toEqual([0,1,2]);expect(all.lines).toHaveLength(3)
 const partial=inspectSolidIntersections(mesh,{maxWork:1})
 expect(partial).toMatchObject({complete:false,stopReason:'work-limit',contacts:[],lines:[]})
 const capped=inspectSolidIntersections(mesh,{maxContacts:1})
 expect(capped.complete).toBe(false);expect(capped.stopReason).toBe('contact-limit');expect(capped.contacts).toHaveLength(1)
})
