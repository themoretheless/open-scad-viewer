import {expect,it} from 'vitest'
import {resolveOpenScadPolygon as native} from '../src/services/openScadStablePrimitiveSemantics'
import {resolveOpenScadPolygon as reference} from '../benchmarks/rush/openScadStablePrimitive-reference'
it('preserves polygon conversion, limits and ordered warnings through packed WASM',()=>{
 const pointSets:unknown[]=[undefined,[],[[0,0]],[[0,0],[1,0],[0,1]],[[NaN,0],[1,0],[0,1]],[[Infinity,0],[1,0]],[[0,0],['bad',1],[0,1]],[[0,0,0],[1,0]]]
 const paths:unknown[]=[undefined,[],[[0,1,2]],[[0,99,1,2]],[[0,1],[2,0,1]],[[0,'bad',2]],[[0,-1,NaN,Infinity,1.9,-.5]],['bad',[0,1,2]],[[0,1,2,0],[],[1,2,0]]]
 for(const points of pointSets)for(const path of paths)for(const maximumIndices of [0,1,3,undefined])for(const maximumFacesOrPaths of [0,1,undefined]){
  const input={points,paths:path,limits:{maximumIndices,maximumFacesOrPaths}}
  const actualWarnings:unknown[]=[],expectedWarnings:unknown[]=[]
  expect(native(input,{warn:w=>actualWarnings.push(w)})).toEqual(reference(input,{warn:w=>expectedWarnings.push(w)}))
  expect(actualWarnings).toEqual(expectedWarnings)
 }
})
