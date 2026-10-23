import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {gunzipSync} from 'node:zlib'
import {sourceBodyExpectation,type SourceBodyOptions} from '../src/services/sourceBody'
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
