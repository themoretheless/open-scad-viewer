import { expect, it } from 'vitest'
import { flattenGroupGeometry } from '../src/services/meshFlatten'

function mesh() {
 return {vertices:new Float32Array([0,0,0,0,0,1,1,0,0,0,0,1,0,1,0,0,0,1]),indices:new Uint32Array([0,1,2]),
  transform:new Float32Array([1,0,0,0,0,1,0,0,0,0,1,0,0,0,0,1])}
}
it('welds exact shared coordinates and preserves reflected winding in Rust',()=>{
 const a=mesh(),b=mesh();b.transform[0]=-1
 const flat=flattenGroupGeometry([a,b]);expect(Array.from(flat.positions)).toEqual([0,0,0,1,0,0,0,1,0,-1,0,0]);expect(Array.from(flat.indices)).toEqual([0,1,2,0,2,3])
 expect(Array.from(b.indices)).toEqual([0,1,2])
})
it('preserves binary64 placement and distinct nearby coordinates without tolerance welding',()=>{
 const a=mesh(),b=mesh();b.transform[3]=1e-7
 const result=flattenGroupGeometry([a,b])
 expect(result.positions).toHaveLength(18)
 expect(result.positions[12]).toBe(1+b.transform[3])
 expect(result.positions[12]).not.toBe(Math.fround(result.positions[12]))
 const singular=mesh();singular.transform[0]=0
 expect(Array.from(flattenGroupGeometry([singular]).indices)).toEqual([0,0,1])
})
it('rejects malformed input and cumulative triangle overflow before producing a result',()=>{
 const invalid=mesh();invalid.indices[2]=99
 expect(()=>flattenGroupGeometry([mesh(),invalid])).toThrow('Malformed')
 const huge=mesh();huge.indices=new Uint32Array(100000*3)
 expect(()=>flattenGroupGeometry([huge,mesh()])).toThrow('100000 triangles')
 const empty=flattenGroupGeometry([]);expect(empty.positions.length).toBe(0);expect(empty.indices.length).toBe(0)
})

it('transfers and flattens the full 100000-triangle expanded scene through WASM',()=>{
 const triangles=100000,large=mesh()
 large.vertices=new Float32Array(triangles*18)
 large.indices=new Uint32Array(triangles*3)
 large.transform[3]=.1
 for(let i=0;i<triangles;i++){
  large.vertices.set([i*2,0,0,0,0,1,i*2+1,0,0,0,0,1,i*2,1,0,0,0,1],i*18)
  large.indices.set([i*3,i*3+1,i*3+2],i*3)
 }
 const result=flattenGroupGeometry([large])
 expect(result.positions).toHaveLength(triangles*9)
 expect(result.indices).toHaveLength(triangles*3)
 expect(Array.from(result.positions.slice(-9))).toEqual([199998+large.transform[3],0,0,199999+large.transform[3],0,0,199998+large.transform[3],1,0])
 expect(Array.from(result.indices.slice(-3))).toEqual([299997,299998,299999])
 expect(large.vertices[large.vertices.length-6]).toBe(199998)
 const oversized={...large,vertices:new Float32Array(large.vertices.length+6)}
 oversized.vertices.set(large.vertices)
 oversized.vertices.set([300000,2,3,0,0,1],large.vertices.length)
 expect(()=>flattenGroupGeometry([oversized])).toThrow('300000 vertices')
 expect(Array.from(flattenGroupGeometry([mesh()]).indices)).toEqual([0,1,2])
 expect(Array.from(result.indices.slice(-3))).toEqual([299997,299998,299999])
},30000)
