import {expect,it} from 'vitest'
import {languageRequest} from '../src/services/languages/kernel'
import {automaticTwistSlices,automaticNonuniformSlices} from '../benchmarks/modelgraph/openScadExtrudeSlices-reference'

it('matches the frozen TS subdivision rules across twist, taper and fragment policies',()=>{
  for(const twist of [-720,-135,0,90,360])for(const scale of [[1,1],[0,0],[.25,.25],[2,2],[.25,2]] as [number,number][])for(const fn of [0,5.9,48,Infinity])for(const height of [.01,20]) {
    const points:[number,number][]=[[3,4],[-2,1],[.001,-.001]]
    const fa=12,fs=2
    const result=languageRequest(12,{points,height,twist,scale,fn:Number.isFinite(fn)?fn:null,fa,fs}) as {ok:boolean;value:{slices:number;source:string}}
    expect(result.ok).toBe(true)
    const expected=twist!==0?automaticTwistSlices(points,height,twist,scale,fn,fa,fs):scale[0]!==scale[1]?automaticNonuniformSlices(points,height,scale,fn,fs):{slices:1,source:'single'}
    if(!Number.isFinite(expected.slices)||expected.slices<1)expected.slices=1
    expect(result.value).toEqual(expected)
  }
})

it('keeps nonfinite fragment special values distinct from missing defaults',()=>{
  for(const fa of [NaN,Infinity,-Infinity])for(const fs of [NaN,Infinity,-Infinity]) {
    const points:[number,number][]=[[3,4]]
    const expected=automaticTwistSlices(points,20,135,[.25,.25],0,fa,fs)
    if(!Number.isFinite(expected.slices)||expected.slices<1)expected.slices=1
    expect(languageRequest(12,{points,height:20,twist:135,scale:[.25,.25],fn:0,fa:String(fa),fs:String(fs)})).toEqual({ok:true,value:expected})
  }
})
