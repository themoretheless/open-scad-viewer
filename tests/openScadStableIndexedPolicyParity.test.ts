import {expect,it} from 'vitest'
import * as native from '../src/services/openScadStablePrimitiveSemantics'
import * as reference from '../benchmarks/rush/openScadStablePrimitive-reference'
it('preserves indexed limit admission, error priority and convexity plans',()=>{
 const values:unknown[]=[undefined,null,'Infinity',true,-1,-0,0,1,2.9,NaN,Infinity,Number.MAX_SAFE_INTEGER,Number.MAX_SAFE_INTEGER+1]
 for(const name of ['resolveOpenScadPolygon','resolveOpenScadPolyhedron'] as const){
  for(const convexity of values){
   expect(native[name]({convexity})).toMatchObject(reference[name]({convexity}))
  }
  for(const value of values)for(const field of ['maximumPoints','maximumFacesOrPaths','maximumIndices']){
   const input={limits:{[field]:value}} as Parameters<typeof native[typeof name]>[0]
   let actual:unknown,expected:unknown
   try{actual=native[name](input)}catch(error){actual=(error as Error).constructor.name+':'+(error as Error).message}
   try{expected=reference[name](input)}catch(error){expected=(error as Error).constructor.name+':'+(error as Error).message}
   if(typeof actual==='object'&&actual!==null)expect(actual).toMatchObject(expected as object)
   else expect(actual).toEqual(expected)
  }
  expect(()=>native[name]({limits:{maximumPoints:-1,maximumFacesOrPaths:-2,maximumIndices:-3}})).toThrow('maximumPoints must be')
 }
})
