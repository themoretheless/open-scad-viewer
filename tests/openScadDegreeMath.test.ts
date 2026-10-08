import {expect,it,vi} from 'vitest'
import * as kernel from '../src/services/languages/kernel'
import {languageRequest} from '../src/services/languages/kernel'

it('preserves exact degree values, nonfinite results and signed zero in a batch',()=>{
 const response=languageRequest(35,{calls:[
  {function:'sin',args:[30]},{function:'cos',args:[60]},{function:'tan',args:[45]},
  {function:'asin',args:[.5]},{function:'acos',args:[.5]},{function:'atan',args:[1]},
  {function:'atan2',args:[1,1]},{function:'sin',args:['-0']},
  {function:'tan',args:[180]},{function:'tan',args:[90]},{function:'tan',args:[270]},
  {function:'asin',args:[2]},{function:'cos',args:['Infinity']},
 ]}) as {ok:boolean;value:(number|string)[]}
 expect(response.ok).toBe(true)
 const values=response.value.map(Number)
 expect(values.slice(0,7)).toEqual([.5,.5,1,30,60,45,45])
 expect(Object.is(values[7],-0)).toBe(true)
 expect(Object.is(values[8],-0)).toBe(true)
 expect(values.slice(9,11)).toEqual([Infinity,-Infinity])
 expect(values[11]).toBeNaN();expect(values[12]).toBeNaN()
})

it('bounds the batch and validates function names and argument counts',()=>{
 for(const calls of [Array.from({length:1025},()=>({function:'sin',args:[0]})),
  [{function:'missing',args:[0]}],[{function:'atan2',args:[0]}],
  [{function:'sin',args:['bad']}],
 ])expect(languageRequest(35,{calls})).toMatchObject({ok:false})
 expect(languageRequest(35,{calls:[]})).toEqual({ok:true,value:[]})
})

import {evaluateDegreeBatch,evaluateDegreeScalar} from '../src/services/openScadDegreeMath'
it('chunks large batches in order and preserves scalar pair edge cases',()=>{
 const calls=Array.from({length:2051},(_,i)=>({function:'sin' as const,args:[i%2?30:90]}))
 expect(evaluateDegreeBatch(calls)).toEqual(calls.map((_,i)=>i%2?.5:1))
 for(let i=0;i<300;i++){evaluateDegreeScalar('sin',[i]);evaluateDegreeScalar('cos',[i])}
 expect(Object.is(evaluateDegreeScalar('sin',[-0]),-0)).toBe(true)
 expect(Object.is(evaluateDegreeScalar('sin',[0]),0)).toBe(true)
 expect(evaluateDegreeScalar('cos',[-0])).toBe(1)
 expect(evaluateDegreeScalar('sin',[Infinity])).toBeNaN()
})

it('reduces ABI crossings for adjacent sin/cos pairs',()=>{
 const request=vi.spyOn(kernel,'languageRequest')
 try {
  evaluateDegreeScalar('tan',[0])
  request.mockClear()
  evaluateDegreeScalar('sin',[12345.125]);evaluateDegreeScalar('cos',[12345.125])
  request.mockClear()
  evaluateDegreeScalar('sin',[12346.125]);evaluateDegreeScalar('cos',[12346.125])
  expect(request).toHaveBeenCalledTimes(1)
  expect(request.mock.calls[0][1]).toMatchObject({calls:[{function:'sin'},{function:'cos'}]})
 } finally {request.mockRestore()}
})
