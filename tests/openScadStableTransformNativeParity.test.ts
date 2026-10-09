import {expect,it} from 'vitest'
import * as native from '../src/services/openScadStableTransformSemantics'
import * as reference from '../benchmarks/rush/openScadStableTransform-reference'

it('preserves native matrix classification and ordered warnings for all transform plans',()=>{
 const compare=(name:'resolveOpenScadTranslate'|'resolveOpenScadScale'|'resolveOpenScadMirror'|'resolveOpenScadMultmatrix',value:unknown)=>{
  const actualWarnings:unknown[]=[],expectedWarnings:unknown[]=[]
  expect(native[name](value,{warn:w=>actualWarnings.push(w)})).toEqual(reference[name](value,{warn:w=>expectedWarnings.push(w)}))
  expect(actualWarnings).toEqual(expectedWarnings)
 }
 for(const value of [undefined,null,1,0,-0,NaN,Infinity,[],[1,2],[1,2,3],[0,0,0],[NaN,2,3],[1,Infinity,3],[1e-250,1e250,2]]){
  for(const name of ['resolveOpenScadTranslate','resolveOpenScadScale','resolveOpenScadMirror'] as const)compare(name,value)
 }
 for(let seed=0;seed<200;seed++){
  const rows=Array.from({length:4},(_,r)=>Array.from({length:4},(_,c)=>((seed*(r+1)*17+c*31)%23)-11))
  compare('resolveOpenScadMultmatrix',rows)
 }
 for(const value of [NaN,Infinity,-Infinity,0,-0,1e-300,1e300]){
  const rows=[[1,0,0,0],[0,1,0,0],[0,0,value,0],[value,0,0,1]]
  compare('resolveOpenScadMultmatrix',rows)
 }
 for(const a of [0,90,-90,360,45,NaN,Infinity,[20,30,40],[0,0,0],[NaN,2,3]]){
  const actualWarnings:unknown[]=[],expectedWarnings:unknown[]=[]
  expect(native.resolveOpenScadRotate({a,v:[1,2,3]},{warn:w=>actualWarnings.push(w)})).toEqual(reference.resolveOpenScadRotate({a,v:[1,2,3]},{warn:w=>expectedWarnings.push(w)}))
  expect(actualWarnings).toEqual(expectedWarnings)
 }
})

it('keeps Euler quadrant rotations exact through packed language WASM',()=>{
 for(const x of [-360,-90,0,90,180,360])for(const y of [-90,0,90])for(const z of [-90,0,90]){
  const input={a:[x,y,z]}
  expect(native.resolveOpenScadRotate(input)).toEqual(reference.resolveOpenScadRotate(input))
 }
})

it('preserves stable reflection arithmetic for tiny, overflowing and nonfinite normals',()=>{
 for(const x of [0,-0,1,-2,1e-250,1e250,NaN,Infinity])for(const y of [0,2,1e-250,1e250])for(const z of [0,-3]){
  const value=[x,y,z],actualWarnings:unknown[]=[],expectedWarnings:unknown[]=[]
  expect(native.resolveOpenScadMirror(value,{warn:w=>actualWarnings.push(w)})).toEqual(reference.resolveOpenScadMirror(value,{warn:w=>expectedWarnings.push(w)}))
  expect(actualWarnings).toEqual(expectedWarnings)
 }
})

it('preserves axis-angle plans for zero, scaled and nonfinite axes',()=>{
 for(const a of [0,90,-90,45,NaN,Infinity])for(const v of [[0,0,0],[1,2,3],[1e-250,2e-250,3e-250],[1e250,2e250,3e250],[NaN,0,0],[Infinity,NaN,0],[Infinity,0,0]]){
  const actualWarnings:unknown[]=[],expectedWarnings:unknown[]=[]
  expect(native.resolveOpenScadRotate({a,v},{warn:w=>actualWarnings.push(w)})).toEqual(reference.resolveOpenScadRotate({a,v},{warn:w=>expectedWarnings.push(w)}))
  expect(actualWarnings).toEqual(expectedWarnings)
 }
})

it('preserves partial vec2 and vec3 conversion and range warnings',()=>{
 const values:unknown[]=[0,2,NaN,Infinity,'bad',null,undefined]
 for(const x of values)for(const y of values)for(const z of values)for(const vector of [[x,y],[x,y,z]]){
  for(const name of ['resolveOpenScadTranslate','resolveOpenScadScale','resolveOpenScadMirror'] as const){
   const actualWarnings:unknown[]=[],expectedWarnings:unknown[]=[]
   expect(native[name](vector,{checkParameterRanges:true,warn:w=>actualWarnings.push(w)})).toEqual(reference[name](vector,{checkParameterRanges:true,warn:w=>expectedWarnings.push(w)}))
   expect(actualWarnings).toEqual(expectedWarnings)
  }
 }
})

it('preserves reverse Euler fallback, empty vectors and ignored extra components',()=>{
 const values:unknown[]=[0,2,NaN,Infinity,'bad',null,undefined]
 const compare=(a:unknown[])=>{
  const actualWarnings:unknown[]=[],expectedWarnings:unknown[]=[]
  expect(native.resolveOpenScadRotate({a,v:[1,2,3]},{warn:w=>actualWarnings.push(w)})).toEqual(reference.resolveOpenScadRotate({a,v:[1,2,3]},{warn:w=>expectedWarnings.push(w)}))
  expect(actualWarnings).toEqual(expectedWarnings)
 }
 compare([])
 for(const x of values){compare([x]);for(const y of values){compare([x,y]);for(const z of values){compare([x,y,z]);compare([x,y,z,4])}}}
})

it('preserves authored matrix defaults, homogeneous division and partial rows',()=>{
 const inputs:unknown[]=[undefined,null,2,[],[null],[[2]],[[2,'bad',3]],[1,[2,3],null,[4]],[[1,2,3,4,99],[5,6],[],[7,8,9,2],[99]]]
 for(const w of [0,-0,2,-2,NaN,Infinity,-Infinity])inputs.push([[1,2,'bad',4],[0,1,0,0],[0,0,1,0],[2,3,4,w]])
 for(const value of inputs){
  const actualWarnings:unknown[]=[],expectedWarnings:unknown[]=[]
  expect(native.resolveOpenScadMultmatrix(value,{warn:w=>actualWarnings.push(w)})).toEqual(reference.resolveOpenScadMultmatrix(value,{warn:w=>expectedWarnings.push(w)}))
  expect(actualWarnings).toEqual(expectedWarnings)
 }
})

it('preserves scalar rotation defaults and warning field priority',()=>{
 for(const a of [undefined,null,'bad',true,0,-0,90,-45,NaN,Infinity])for(const v of [undefined,null,'bad',[],[1,2],[1,'bad'],[1,2,3],[1,'bad',3],[NaN,0,0],[Infinity,0,0],[0,0,0],[1,2,3,4]]){
  const actualWarnings:unknown[]=[],expectedWarnings:unknown[]=[]
  expect(native.resolveOpenScadRotate({a,v},{warn:w=>actualWarnings.push(w)})).toEqual(reference.resolveOpenScadRotate({a,v},{warn:w=>expectedWarnings.push(w)}))
  expect(actualWarnings).toEqual(expectedWarnings)
 }
})

it('preserves viewer axis-angle matrices against the previous TS formula',()=>{
 for(const degrees of [0,30,90,180,-90,359.75,720])for(const axis of [[1,0,0],[1,2,3],[1e-200,2e-200,3e-200],[1e200,2e200,3e200]]){
  const length=Math.hypot(...axis),[x,y,z]=axis.map(value=>value/length)
  const a=degrees*Math.PI/180,c=Math.cos(a),s=Math.sin(a),t=1-c
  const expected=[t*x*x+c,t*x*y+s*z,t*x*z-s*y,0,t*x*y-s*z,t*y*y+c,t*y*z+s*x,0,t*x*z+s*y,t*y*z-s*x,t*z*z+c,0,0,0,0,1]
  const actual=native.resolveOpenScadViewerAxisAngle(axis,degrees)!
  expect(actual).toHaveLength(16)
  for(let index=0;index<16;index++)expect(actual[index]).toBeCloseTo(expected[index],14)
 }
 expect(native.resolveOpenScadViewerAxisAngle([0,0,0],90)).toBeNull()
})
