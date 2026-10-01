import {expect,it,vi} from 'vitest'
import {previewSolidCurveOffset} from '../src/services/previewSolidCurveOffset'
import {MainSolidWorkerError,type MainSolidWorkerClient} from '../src/services/mainSolidWorkerClient'
const document={version:1 as const,bodies:[],sketches:[],curves:[{id:'source',name:'Контур корпуса',curve:{degree:1,knots:[0,0,1,1],weights:[1,1],controlPoints:[[0,0],[1,1]]}}]}
const options={id:'source',createdId:'offset',offsetJoin:'trim-evenodd',distance:1,maxError:.01,locale:'ru'}
it('localizes a native refusal with the source curve and preserves the failed input',async()=>{
 const original=new Error('Intersection order is unresolved; coincident or close cuts need graph reconciliation.')
 const run=vi.fn().mockRejectedValue(original),client={run} as unknown as MainSolidWorkerClient
 const before=structuredClone(document)
 const error=await previewSolidCurveOffset(client,document,options).catch(error=>error)
 expect(error.message).toContain('Контур корпуса:');expect(error.message).toContain('Разделите профиль либо выберите Bevel')
 expect(error.cause).toBe(original);expect(document).toEqual(before)
 expect(run.mock.calls[0]![0]).toMatchObject({kind:'trimmedCurveOffset',options:{fillRule:'evenodd'}})
})
it('retains typed worker failure identity for retry and cancellation handling',async()=>{
 for(const code of ['CAD_CRASH','CAD_CANCELLED','CAD_PROTOCOL']){
  const failure=new MainSolidWorkerError(code,'Worker failure',code==='CAD_CANCELLED'?'AbortError':undefined)
  const client={run:vi.fn().mockRejectedValue(failure)} as unknown as MainSolidWorkerClient
  await expect(previewSolidCurveOffset(client,document,options)).rejects.toBe(failure)
 }
})
