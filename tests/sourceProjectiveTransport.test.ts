import {createSourceBodyRecord,sourceBodyRecordOptions} from '../src/services/sourceBodyArchive'
import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {gunzipSync} from 'node:zlib'
import {sourceBodyExpectation,validSeamEndpointLimits,type SourceBodyOptions,type SourceSeamResult} from '../src/services/sourceBody'
const request=JSON.parse(gunzipSync(readFileSync(new URL('../docs/qualification/rolling-ball-offset-foundation-2026-10-05/source-body-request.json.gz',import.meta.url))).toString())
const options=():SourceBodyOptions=>{const {op:_op,...value}=structuredClone(request);return value}
it('binds transport identity to the native projective discriminator',()=>{
 const shared=options(),projective=options()
 projective.definition.kind='projective-body'
 expect(sourceBodyExpectation(projective).definition).not.toBe(sourceBodyExpectation(shared).definition)
 expect(()=>sourceBodyExpectation({...projective,definition:{...projective.definition,inverseShear:{axes:[0,1],coefficient:1}}})).toThrow()
 expect(()=>sourceBodyExpectation({...shared,definition:{...shared.definition,kind:'unknown'} as never})).toThrow()
})
it('preserves an explicit unproven projective endpoint result without fabricated work or sides',()=>{
 const seam:SourceSeamResult={request:{edge:0,limits:{maxSineSquared:1e-6,cells:10,curveSpans:10,normalSpans:10}},qualified:false,sineSquaredBounds:null,reason:'source-seam-endpoint-normal-unresolved',cells:1,curveSpans:1,normalSpans:1,acceptedCells:0,uncertainCanonical:[0,0],endpointLimits:{canonical:0,exactWork:0,reason:'projective-endpoint-limits-unproven',sides:[]}}
 expect(validSeamEndpointLimits(seam,100,1e-6)).toBe(true)
 expect(validSeamEndpointLimits({...seam,endpointLimits:{...seam.endpointLimits!,exactWork:1}},100,1e-6)).toBe(false)
 expect(validSeamEndpointLimits({...seam,qualified:true},100,1e-6)).toBe(false)
 expect(validSeamEndpointLimits(seam,undefined,1e-6)).toBe(false)
})

it('preserves projective recipes and signed zero through the opaque reload archive',()=>{
 const original=options();original.definition.kind='projective-body';original.limits.volume.origin=-0
 const before=sourceBodyExpectation(original).definition
 const record=createSourceBodyRecord('projective-source','Projective source',original)
 const restored=sourceBodyRecordOptions(JSON.parse(JSON.stringify(record)))
 expect(restored.definition.kind).toBe('projective-body')
 expect(Object.is(restored.limits.volume.origin,-0)).toBe(true)
 expect(sourceBodyExpectation(restored).definition).toBe(before)
 expect(original.definition.kind).toBe('projective-body')
})
