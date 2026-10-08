import {expect,it} from 'vitest'
import * as native from '../src/services/openScadStablePrimitiveSemantics'
import * as reference from '../benchmarks/rush/openScadStablePrimitive-reference'

it('preserves box plans and ordered warnings against the frozen TS implementation',()=>{
 const sizes:unknown[]=[undefined,null,true,'NaN',1,0,-1,-0,NaN,Infinity,-Infinity,[],[2,3],[2,3,4],[2,'bad',4],['bad',3,4],[NaN,'bad',4],[2,3,'bad'],[2,3,4,5],[Infinity,3,4]]
 for(const name of ['resolveOpenScadCube','resolveOpenScadSquare'] as const)
 for(const size of sizes)for(const center of [undefined,true,false,1])for(const checkParameterRanges of [false,true]){
  const actualWarnings:unknown[]=[],expectedWarnings:unknown[]=[]
  const input={size,center}
  const actual=native[name](input,{checkParameterRanges,warn:w=>actualWarnings.push(w)})
  const expected=reference[name](input,{checkParameterRanges,warn:w=>expectedWarnings.push(w)})
  expect(actual).toEqual(expected)
  expect(actualWarnings).toEqual(expectedWarnings)
  expect(Object.isFrozen(actual)).toBe(true)
  expect(Object.isFrozen(actual.dimensions)).toBe(true)
 }
})

it('preserves radial and cylinder plans and ordered warnings for permissive authored values',()=>{
 const values:unknown[]=[undefined,null,'NaN',true,0,-0,-2,2,NaN,Infinity,-Infinity,[],[1]]
 const compare=(name:'resolveOpenScadSphere'|'resolveOpenScadCircle'|'resolveOpenScadCylinder',input:native.OpenScadCylinderPrimitiveInput)=>{
  for(const checkParameterRanges of [false,true]){
   const actualWarnings:unknown[]=[],expectedWarnings:unknown[]=[]
   expect(native[name](input,{checkParameterRanges,warn:w=>actualWarnings.push(w)})).toEqual(reference[name](input,{checkParameterRanges,warn:w=>expectedWarnings.push(w)}))
   expect(actualWarnings).toEqual(expectedWarnings)
  }
 }
 for(const r of values)for(const d of values){
  compare('resolveOpenScadSphere',{r,d});compare('resolveOpenScadCircle',{r,d})
 }
 for(let i=0;i<values.length**3;i++){
  const a=i%values.length,b=Math.floor(i/values.length)%values.length,c=Math.floor(i/values.length**2)
  compare('resolveOpenScadCylinder',{r:values[a],d:values[b],r1:values[c],d1:values[(a+3)%values.length],r2:values[(b+5)%values.length],d2:values[(c+7)%values.length],h:values[(a+b+c)%values.length],center:i%2===0})
 }
})
