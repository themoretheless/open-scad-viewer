import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {gunzipSync} from 'node:zlib'
import {sourceBodyExpectation,validSeamEndpointLimits,type SourceSeamResult,type SourceBodyOptions} from '../src/services/sourceBody'
const request=JSON.parse(gunzipSync(readFileSync(new URL('../docs/qualification/rolling-ball-offset-foundation-2026-10-05/source-body-request.json.gz',import.meta.url))).toString())
const options=():SourceBodyOptions=>{const {op:_op,...value}=structuredClone(request);return value}
it('snapshots source seam limits before the request object changes',()=>{
 const o={...options(),seamQualification:{edge:0,limits:{maxSineSquared:1e-6,cells:128,curveSpans:256,normalSpans:256}}}
 const expected=sourceBodyExpectation(o)
 o.seamQualification.limits.maxSineSquared=0.5
 o.seamQualification.limits.cells=100000
 o.seamQualification.edge=1
 expect(expected.seamLimits).toEqual({maxSineSquared:1e-6,cells:128,curveSpans:256,normalSpans:256})
 expect(expected.seamQualification).not.toBe(sourceBodyExpectation(o).seamQualification)
})
it('binds endpoint diagnostics to their canonical endpoint, budgets and separated original limits',()=>{
 const r:SourceSeamResult={request:{edge:0,endpointLimitsExactWork:100,limits:{maxSineSquared:1e-6,cells:1,curveSpans:2,normalSpans:2}},
  qualified:false,sineSquaredBounds:null,reason:'source-seam-endpoint-normal-unresolved',cells:1,curveSpans:2,normalSpans:2,acceptedCells:0,uncertainCanonical:[0,0],
  endpointLimits:{canonical:0,exactWork:99,reason:'source-seam-endpoint-distinct-tangent-limits',sides:[{side:0,boundaryEnd:false,collapsedPoint:[0,0,0],components:[[[0,0],[0,0],[1,1]],[[0,0],[-2,-2],[1,1]]],sineSquaredBounds:[0.79,0.81],distinctTangentLimits:true}]}}
 expect(validSeamEndpointLimits(r,100,1e-6)).toBe(true)
 for(const mutate of [
  (x:SourceSeamResult)=>{x.endpointLimits=null},
  (x:SourceSeamResult)=>{x.endpointLimits!.canonical=1},
  (x:SourceSeamResult)=>{x.endpointLimits!.exactWork=101},
  (x:SourceSeamResult)=>{x.qualified=true},
  (x:SourceSeamResult)=>{x.endpointLimits!.sides.push(structuredClone(x.endpointLimits!.sides[0]!))},
  (x:SourceSeamResult)=>{x.endpointLimits!.sides[0]!.collapsedPoint=null},
  (x:SourceSeamResult)=>{x.endpointLimits!.sides[0]!.sineSquaredBounds=[0,1]},
  (x:SourceSeamResult)=>{x.endpointLimits!.sides[0]!.components[0][0]=[1,-1]},
 ]){const x=structuredClone(r);mutate(x);expect(validSeamEndpointLimits(x,100,1e-6)).toBe(false)}
 expect(validSeamEndpointLimits(r,undefined,1e-6)).toBe(false)
 for(const budget of [0,100000001,NaN])expect(()=>sourceBodyExpectation({...options(),seamQualification:{...r.request,endpointLimitsExactWork:budget}})).toThrow()
 const stopped=structuredClone(r);stopped.endpointLimits={canonical:0,exactWork:1,reason:'source-seam-endpoint-limits-work-limit',sides:[]}
 expect(validSeamEndpointLimits(stopped,1,1e-6)).toBe(true)
})
