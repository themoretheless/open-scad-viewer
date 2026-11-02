import {afterEach,beforeEach,describe,it,expect,vi} from 'vitest'
import {SurfaceSelfIntersectionBrowser} from '../src/services/surfaceSelfIntersectionBrowser'
import type {SurfaceIsoTargetQuery} from '../src/services/surfaceIsoTargetProtocol'
class FakeWorker {
 static all:FakeWorker[]=[]
 onmessage:any;onerror:any;onmessageerror:any;sent:any;terminated=0
 constructor(){FakeWorker.all.push(this)}
 postMessage(v:any){this.sent=v}
 terminate(){this.terminated++}
 reply(report:any){this.onmessage({data:{type:'surface-self-result/1',id:this.sent.id,ok:true,report}})}
}
const query=():SurfaceIsoTargetQuery=>({source:{} as any,target:{} as any,axis:0,parameter:.5,window:[[0,1],[-2,2],[10,14]],maxCells:100})
const report=():any=>({schema:'nurbs-surface-iso-target-window/1',status:'absent-in-window',axis:0,parameter:.5,sourceIsoParameterCoordinates:'normalized-original-transverse-domain',parameterCoordinates:['source-natural-t','target-natural-u','target-natural-v'],window:[[0,1],[-2,2],[10,14]],maxCells:100,cells:1,roots:[],presenceProven:false,windowAbsenceProven:true,windowCoverageComplete:true,unresolved:[],allSurfaceBranchesEnumerated:false,trimAdmission:false,bodyAdmission:false})
const lane=()=>new SurfaceSelfIntersectionBrowser(new WebAssembly.Module(new Uint8Array([0,97,115,109,1,0,0,0])))
beforeEach(()=>{FakeWorker.all=[];vi.stubGlobal('Worker',FakeWorker)})
afterEach(()=>vi.unstubAllGlobals())
describe('iso target browser ownership',()=>{
 it('routes the selected query and terminates the worker after validated success',async()=>{const l=lane(),q=query(),before=JSON.stringify(q),pending=l.inspectIsoTarget(q);const w=FakeWorker.all[0]!;expect(w.sent.type).toBe('surface-iso-target-window/1');expect(w.sent.window).toEqual(q.window);w.reply(report());expect((await pending).status).toBe('absent-in-window');expect(JSON.stringify(q)).toBe(before);expect(l.snapshot()).toEqual({active:false,workersStarted:1,workersTerminated:1});expect(w.terminated).toBe(1)})
 it('rejects a valid report belonging to a different requested window',async()=>{const l=lane(),p=l.inspectIsoTarget(query()),r=report();r.window[1]=[-3,3];FakeWorker.all[0]!.reply(r);await expect(p).rejects.toThrow('Invalid surface audit report');expect(l.snapshot().active).toBe(false)})
 it('binds the original query even if the caller mutates its window later',async()=>{const l=lane(),q=query(),p=l.inspectIsoTarget(q);q.window[1]=[-3,3];FakeWorker.all[0]!.reply(report());await expect(p).resolves.toMatchObject({window:[ [0,1],[-2,2],[10,14] ]})})
 it('shares the busy guard with whole-surface audits and releases on abort',async()=>{const l=lane(),c=new AbortController(),p=l.inspectIsoTarget(query(),{signal:c.signal});await expect(l.inspect({} as any,100)).rejects.toThrow('busy');c.abort();await expect(p).rejects.toThrow('cancelled');expect(l.snapshot()).toEqual({active:false,workersStarted:1,workersTerminated:1})})
 it('refuses pre-aborted and malformed queries without starting workers',async()=>{const l=lane(),c=new AbortController();c.abort();await expect(l.inspectIsoTarget(query(),{signal:c.signal})).rejects.toThrow('cancelled');await expect(l.inspectIsoTarget({...query(),maxCells:0})).rejects.toThrow('Invalid iso');expect(FakeWorker.all).toHaveLength(0)})
 it('terminates a native refusal and can service the next query',async()=>{const l=lane(),p=l.inspectIsoTarget(query()),w=FakeWorker.all[0]!;w.onmessage({data:{type:'surface-self-result/1',id:w.sent.id,ok:false,error:'native refused'}});await expect(p).rejects.toThrow('native refused');const next=l.inspectIsoTarget(query());FakeWorker.all[1]!.reply(report());await expect(next).resolves.toHaveProperty('windowCoverageComplete',true);expect(l.snapshot()).toEqual({active:false,workersStarted:2,workersTerminated:2})})
})
