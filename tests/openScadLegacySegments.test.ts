import {expect,it} from 'vitest'
import {resolveLegacyOpenScadSegments,roundLegacyOpenScadSegments} from '../src/services/openScadLegacySegments'
import {referenceLegacySegments} from '../benchmarks/rush/openScadLegacySegments-reference'
it('preserves rounding, caps and preview reduction from the frozen viewer-subset rule',()=>{
 for(const requested of [null,-1.5,-.5,-.49999999999999994,-0,0,.49999999999999994,.5,1.5,47.5,48.5,255.5,256.5,1000,1e308,Number.MAX_SAFE_INTEGER])
 for(const fallback of [12,32,48])for(const minimum of [3,4])for(const quality of ['preview','full'] as const){
  expect(resolveLegacyOpenScadSegments(requested,fallback,minimum,quality)).toEqual(referenceLegacySegments(requested,fallback,minimum,quality))
 }
})
it('rounds source intents without applying renderer caps',()=>{
 expect(Object.is(roundLegacyOpenScadSegments(-.5),-0)).toBe(true)
 expect(roundLegacyOpenScadSegments(.49999999999999994)).toBe(0)
 expect(roundLegacyOpenScadSegments(1000.5)).toBe(1001)
})
