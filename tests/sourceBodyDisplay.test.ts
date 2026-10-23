import {describe,it,expect,vi} from 'vitest'
import {readFileSync} from 'node:fs'
import {gunzipSync} from 'node:zlib'
import {SourceBodyDisplay} from '../src/services/sourceBodyDisplay'
import type {MainSolidWorkerClient} from '../src/services/mainSolidWorkerClient'
import type {SourceBodyRecord} from '../src/services/sourceBodyArchive'
import type {SourceBodyResult} from '../src/services/sourceBody'
const fixture=JSON.parse(gunzipSync(readFileSync(new URL('../docs/qualification/rolling-ball-offset-foundation-2026-10-05/source-body-document.json.gz',import.meta.url))).toString()).sourceBodies[0] as SourceBodyRecord
const admitted={admitted:true} as SourceBodyResult
function deferred(){let resolve!:(v:SourceBodyResult)=>void;const promise=new Promise<SourceBodyResult>(r=>resolve=r);return {promise,resolve}}
describe('source display preparation',()=>{
 it('ignores a late result after cancellation',async()=>{
  const task=deferred(),publish=vi.fn(),fail=vi.fn(),cancel=vi.fn()
  const display=new SourceBodyDisplay({run:vi.fn(()=>task.promise),cancel} as unknown as MainSolidWorkerClient)
  const pending=display.prepare([fixture],publish,fail);display.cancel();task.resolve(admitted);await pending
  expect(publish).not.toHaveBeenCalled();expect(fail).not.toHaveBeenCalled();expect(cancel).toHaveBeenCalledTimes(2)
 })
 it('retries with source inputs and preserves errors per body',async()=>{
  const run=vi.fn().mockRejectedValueOnce(new Error('native failure')).mockResolvedValueOnce(admitted)
  const display=new SourceBodyDisplay({run,cancel:vi.fn()} as unknown as MainSolidWorkerClient),publish=vi.fn(),fail=vi.fn()
  await display.prepare([fixture],publish,fail);expect(fail).toHaveBeenCalledWith(fixture.id,expect.any(Error))
  await display.prepare([fixture],publish,fail);expect(publish).toHaveBeenCalledWith(fixture.id,admitted)
  expect(run.mock.calls[1]![0].options.displaySegments).toBe(32)
 })
 it('a replacement request owns publication even if the old worker responds',async()=>{
  const old=deferred(),next=deferred(),run=vi.fn().mockReturnValueOnce(old.promise).mockReturnValueOnce(next.promise)
  const display=new SourceBodyDisplay({run,cancel:vi.fn()} as unknown as MainSolidWorkerClient),publish=vi.fn(),fail=vi.fn()
  const a=display.prepare([fixture],publish,fail),b=display.prepare([{...fixture,id:'next'}],publish,fail)
  old.resolve(admitted);next.resolve(admitted);await Promise.all([a,b])
  expect(publish).toHaveBeenCalledTimes(1);expect(publish).toHaveBeenCalledWith('next',admitted)
 })
 it('retains native diagnostics on refusal and continues with the next body',async()=>{
  const diagnostics={reason:'source-volume-initial-work-limit',incidence:{uncertainPair:[1,2]},embedding:null,volume:{uncertainFace:3}}
  const run=vi.fn().mockResolvedValueOnce({admitted:false,sourceBody:null,edges:[],diagnostics}).mockResolvedValueOnce(admitted)
  const display=new SourceBodyDisplay({run,cancel:vi.fn()} as unknown as MainSolidWorkerClient),publish=vi.fn(),fail=vi.fn()
  await display.prepare([fixture,{...fixture,id:'next'}],publish,fail)
  expect(fail).toHaveBeenCalledWith(fixture.id,expect.objectContaining({code:'CAD_SOURCE_BODY_RESTORE',sourceBodyId:fixture.id,diagnostics}))
  expect(publish).toHaveBeenCalledTimes(1);expect(publish).toHaveBeenCalledWith('next',admitted)
 })

})
