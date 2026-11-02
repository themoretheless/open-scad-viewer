import {expect,it} from 'vitest'
import {degreeReductionZeroCases} from '../tools/browser-qualification/degree-reduction-zero-fixture'
for(const testCase of degreeReductionZeroCases) {
 it('executes actual WASM: '+testCase.name,()=>{
  let checks=0
  testCase.run((value,label)=>{checks++;expect(value,label).toBe(true)})
  expect(checks).toBeGreaterThan(0)
 })
}
