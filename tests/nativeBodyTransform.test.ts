import {it,expect} from 'vitest'
import {transformBodies} from '../src/services/directSolidTools'
import {createBrepBox,tessellateNurbsBrep,analyzeNurbsBrep} from '../src/services/geometry/brep'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
const bodies=()=>[0,8].map((x,i)=>{const brep=createBrepBox([x,0,0],[x+2,4,6]);return {id:String(i),name:`Part ${i}`,brep,mesh:tessellateNurbsBrep(brep,1)}})
it('rotates and scales mesh and B-rep about the shared selection pivot in Rust',()=>{
 const input=bodies(),before=JSON.stringify(input)
 const result=transformBodies(input,[10,-20,30],[0,0,1],90,2)
 // Shared center (5,2,3); quarter turn maps q to (-q.y,q.x,q.z).
 result.forEach((body,i)=>{
  body.brep!.vertices.forEach((v,j)=>{const p=input[i].brep.vertices[j].point;const expected=[19-2*p[1],2*p[0]-28,2*p[2]+27];expected.forEach((x,k)=>expect(v.point[k]).toBeCloseTo(x,11))})
  body.mesh.positions.forEach((x,j)=>{const p=input[i].mesh.positions,k=j%3,base=j-k;const expected=[19-2*p[base+1],2*p[base]-28,2*p[base+2]+27];expect(x).toBeCloseTo(expected[k],11)})
  expect(body.id).toBe(input[i].id);expect(body.name).toBe(input[i].name)
  expect(body.brep!.loops).toEqual(input[i].brep.loops)
  expect(analyzeNurbsBrep(body.brep!).signedVolumeMm3).toBeCloseTo(384,8)
  expect(inspectPolygonMesh(body.mesh).signedVolumeMm3).toBeCloseTo(384,8)
 })
 expect(JSON.stringify(input)).toBe(before)
})
it('normalizes extreme axes and rejects invalid batches without changing the source',()=>{
 const input=bodies(),reference=transformBodies(input,[0,0,0],[0,0,1],33,1)
 for(const z of [Number.MIN_VALUE,Number.MAX_VALUE])expect(transformBodies(input,[0,0,0],[0,0,z],33,1)).toEqual(reference)
 input[1].brep.vertices[0].point[0]+=1
 const before=JSON.stringify(input)
 expect(()=>transformBodies(input,[0,0,0],[0,0,1],33,1)).toThrow()
 expect(JSON.stringify(input)).toBe(before)
 expect(()=>transformBodies(bodies(),[0,0,0],[0,0,0],0,1)).toThrow('Zero direction')
 expect(()=>transformBodies(bodies(),[0,0,0],[0,0,1],0,0)).toThrow('Invalid transform')
 expect(transformBodies([], [0,0,0],[0,0,1],0,1)).toEqual([])
 expect(transformBodies(bodies(),[0,0,0],[0,0,1],0,1)).toHaveLength(2)
})

it('transforms mixed selections about one pivot and preserves analytic sketches',async()=>{
 const {transformSelection}=await import('../src/services/directSolidTools')
 const body=bodies()[0]
 const sketch={id:'s',name:'Arc',closed:false,points:[[8,0],[10,2]] as [number,number][],analytic:{kind:'arc' as const,center:[8,2] as [number,number],radius:2,start:-90,sweep:90}}
 const input={version:1 as const,bodies:[body],sketches:[sketch]},before=JSON.stringify(input)
 const result=transformSelection(input,['0','s'],[1,2,3],[1,0,0],90,2)
 // Joint bounds center is (5,2,3); rotated world point is (2x-4,10-2z,2y+2).
 const moved=result.sketches[0]
 expect(moved.points).toEqual([[16,0],[20,4]])
 expect(moved.analytic).toEqual({...sketch.analytic,center:[16,4],radius:4})
 const {worldPoint}=await import('../src/services/directSketchGeometry')
 moved.points.forEach((p,i)=>{const expected=[2*sketch.points[i][0]-4,10,2*sketch.points[i][1]+2];worldPoint(p,moved.plane).forEach((x,k)=>expect(x).toBeCloseTo(expected[k],10))})
 result.bodies[0].brep!.vertices.forEach((v,i)=>{const p=body.brep.vertices[i].point;[2*p[0]-4,10-2*p[2],2*p[1]+2].forEach((x,k)=>expect(v.point[k]).toBeCloseTo(x,10))})
 expect(JSON.stringify(input)).toBe(before)
})
it('keeps coplanar analytic edits in local coordinates and refuses malformed late sketches',async()=>{
 const {transformSelection}=await import('../src/services/directSolidTools')
 const sketch={id:'s',name:'Arc',closed:false,points:[[0,0],[2,2]] as [number,number][],analytic:{kind:'arc' as const,center:[1,1] as [number,number],radius:1,start:0,sweep:90}}
 const input={version:1 as const,bodies:[],sketches:[sketch]}
 const moved=transformSelection(input,['s'],[3,4,0],[0,0,1],90,2).sketches[0]
 expect(moved.plane).toBeUndefined()
 expect(moved.analytic!.center).toEqual([4,5]);expect(moved.analytic!.radius).toBe(2);expect(moved.analytic!.start).toBeCloseTo(90)
 const invalid={version:1 as const,bodies:bodies(),sketches:[{...sketch,points:[[1]] as unknown as [number,number][]}]},before=JSON.stringify(invalid)
 expect(()=>transformSelection(invalid,['0','s'],[1,0,0],[0,0,1],0,1)).toThrow()
 expect(JSON.stringify(invalid)).toBe(before)
})
it('rejects analytic overflow instead of returning null geometry',async()=>{
 const {transformSelection}=await import('../src/services/directSolidTools')
 const sketch={id:'s',name:'Overflow',closed:false,points:[[0,0],[1,1]] as [number,number][],analytic:{kind:'arc' as const,center:[0,0] as [number,number],radius:Number.MAX_VALUE,start:0,sweep:90}}
 const input={version:1 as const,bodies:[],sketches:[sketch]},before=JSON.stringify(input)
 expect(()=>transformSelection(input,['s'],[0,0,0],[0,0,1],0,2)).toThrow(/finite numeric range/)
 expect(JSON.stringify(input)).toBe(before)
})
