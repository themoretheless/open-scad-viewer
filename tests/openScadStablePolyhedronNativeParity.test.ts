import {expect,it} from 'vitest'
import {resolveOpenScadPolyhedron as native} from '../src/services/openScadStablePrimitiveSemantics'
import {resolveOpenScadPolyhedron as reference} from '../benchmarks/rush/openScadStablePrimitive-reference'

it('preserves polyhedron faces, stopping points and ordered warnings through packed WASM',()=>{
 const pointSets:unknown[]=[undefined,[],[[0,0],[1,0],[0,1]],[[0,0,0],[1,0,0],[0,1,0]],[[0,0],['bad',1],[0,1]],[[NaN,0],[0,0],[1,1]],[[Infinity,0],[0,0]],[[0,0,0,0],[1,0]]]
 const faceSets:unknown[]=[undefined,[],[[0,1,2]],[[0,99,1,2]],[[0,1],[2,0,1]],[[0,'bad',2]],[[0,-1,NaN,Infinity,1.9,-.5]],['bad',[0,1,2]],[[0,1,2,0],[],[1,2,0]]]
 for(const points of pointSets)for(const entries of faceSets)for(const alias of [false,true])for(const maximumIndices of [0,1,3,10,undefined]){
  const input={points,...(alias?{triangles:entries}:{faces:entries}),limits:{maximumIndices}}
  const actualWarnings:unknown[]=[],expectedWarnings:unknown[]=[]
  const {vertices,indices,usable,...actual}=native(input,{warn:w=>actualWarnings.push(w)})
  expect(actual).toEqual(reference(input,{warn:w=>expectedWarnings.push(w)}))
  expect(vertices.length % 3).toBe(0)
  expect(indices.length % 3).toBe(0)
  expect(usable).toBe(indices.length>0&&vertices.every(value=>Number.isFinite(Math.fround(value))))
  expect(actualWarnings).toEqual(expectedWarnings)
 }
})

it('packs face order and reversed winding and rejects float32 overflow',()=>{
 const plan=native({points:[[0,0,0],[1,0,0],[1,1,0],[0,1,0]],faces:[[0,1,2,3]]})
 expect(plan.vertices).toEqual([0,0,0,1,0,0,1,1,0,0,1,0])
 expect(plan.indices).toEqual([0,2,1,0,3,2])
 expect(plan.usable).toBe(true)
 const overflow=native({points:[[Number.MAX_VALUE,0,0],[0,1,0],[0,0,1]],faces:[[0,1,2]]})
 expect(overflow.empty).toBe(false)
 expect(overflow.usable).toBe(false)
})
