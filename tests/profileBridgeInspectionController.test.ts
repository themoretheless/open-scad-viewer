import {readFileSync} from 'node:fs'
import {effectScope,ref,shallowRef,toRaw} from 'vue'
import {expect,it,vi} from 'vitest'
import {useProfileBridgeInspection} from '../src/services/useProfileBridgeInspection'
import type {ProfileBridgeChordOptions,ProfileBridgeChordReport} from '../src/services/nurbsProfileBridgeChords'
const fixture=JSON.parse(readFileSync('docs/qualification/mapped-retained-carrier-2026-10-08/profile-bridge-native-transport-2026-10-10.json','utf8')).wholeCoverage
const {op:_,...options}=fixture.request as ProfileBridgeChordOptions&{op:string}
it('invalidates whole-product coverage after edits, cancel and late failures; allows Retry',async()=>{
 const tasks:{resolve:(r:ProfileBridgeChordReport)=>void;reject:(e:Error)=>void}[]=[]
 const worker={cancel:vi.fn(),dispose:vi.fn(),run:vi.fn(()=>new Promise<ProfileBridgeChordReport>((resolve,reject)=>tasks.push({resolve,reject})))}
 const source=shallowRef<ProfileBridgeChordOptions|undefined>(structuredClone(options)),active=ref(true),scope=effectScope()
 const controller=scope.run(()=>useProfileBridgeInspection(source,active,worker as never))!
 try{
  const initial=controller.run();tasks[0].resolve(fixture.report);await initial
  expect(controller.result.value?.coverage?.completeExclusion).toBe(true)
  controller.cancel();expect(controller.result.value).toBeNull()
  const late=controller.run();toRaw(source.value!).coverage!.cells=0
  tasks[1].reject(new Error('Late error for obsolete coverage'));await late
  expect(controller.result.value).toBeNull();expect(controller.error.value).toBe('')
  source.value=structuredClone(options)
  const retry=controller.retry();tasks[2].resolve(fixture.report);await retry
  expect(controller.result.value?.coverage?.completeExclusion).toBe(true)
  const stale=controller.run();source.value={...options,scales:[4,4]}
  tasks[3].resolve(fixture.report);await stale
  expect(controller.result.value).toBeNull();expect(controller.pending.value).toBe(false)
  const hidden=controller.run();active.value=false;tasks[4].resolve(fixture.report);await hidden
  expect(controller.result.value).toBeNull()
 }finally{scope.stop()}
 expect(worker.dispose).toHaveBeenCalledOnce()
})

it.each(['success','failure'] as const)('preserves the current coverage when an older generation delivers a late %s',async(kind)=>{
 const tasks:{resolve:(r:ProfileBridgeChordReport)=>void;reject:(e:Error)=>void}[]=[]
 const worker={cancel:vi.fn(),dispose:vi.fn(),run:vi.fn(()=>new Promise<ProfileBridgeChordReport>((resolve,reject)=>tasks.push({resolve,reject})))}
 const source=shallowRef<ProfileBridgeChordOptions|undefined>(structuredClone(options)),active=ref(true),scope=effectScope()
 const controller=scope.run(()=>useProfileBridgeInspection(source,active,worker as never))!
 try{
  const obsolete=controller.run()
  controller.cancel()
  const current=controller.retry()
  // The new generation completes before the cancelled request settles.
  tasks[1].resolve(structuredClone(fixture.report));await current
  const accepted=controller.result.value
  expect(accepted?.coverage?.completeExclusion).toBe(true)
  if(kind==='failure')tasks[0].reject(Object.assign(new Error('Obsolete worker timeout'),{code:'CAD_TIMEOUT'}))
  else tasks[0].resolve(structuredClone(fixture.report))
  await obsolete
  expect(controller.result.value).toBe(accepted)
  expect(controller.pending.value).toBe(false)
  expect(controller.error.value).toBe('')
  expect(controller.errorCode.value).toBe('')
 }finally{scope.stop()}
 expect(worker.dispose).toHaveBeenCalledOnce()
})
