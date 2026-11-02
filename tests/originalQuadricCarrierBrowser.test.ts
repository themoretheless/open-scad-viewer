import {beforeEach,afterEach,describe,it,expect,vi} from 'vitest'
import {SurfaceSelfIntersectionBrowser} from '../src/services/surfaceSelfIntersectionBrowser'
import {quadricQuery,quadricReport} from './fixtures/quadricCarrier'
class FakeWorker {
 static all:FakeWorker[]=[]
 onmessage:any;onerror:any;onmessageerror:any;sent:any;terminated=0
 constructor(){FakeWorker.all.push(this)}
 postMessage(v:any){this.sent=v}
 terminate(){this.terminated++}
 reply(report:any){this.onmessage({data:{type:'surface-self-result/1',id:this.sent.id,ok:true,report}})}
}
const lane=()=>new SurfaceSelfIntersectionBrowser(new WebAssembly.Module(new Uint8Array([0,97,115,109,1,0,0,0])))
beforeEach(()=>{FakeWorker.all=[];vi.stubGlobal('Worker',FakeWorker)})
afterEach(()=>vi.unstubAllGlobals())
describe('quadric carrier browser ownership',()=>{
 it('routes the original quadric query and terminates on success',async()=>{const l=lane(),q=quadricQuery(),before=JSON.stringify(q),p=l.inspectOriginalQuadricCarrierRoots(q),w=FakeWorker.all[0]!;expect(w.sent.type).toBe('original-quadric-carrier-roots/1');expect(w.sent.quadric).toEqual(q.quadric);w.reply(quadricReport());await expect(p).resolves.toHaveProperty('finiteSurfaceUvAdmission',false);expect(JSON.stringify(q)).toBe(before);expect(l.snapshot()).toEqual({active:false,workersStarted:1,workersTerminated:1})})
 it('rejects finite-surface promotion and releases its worker',async()=>{const l=lane(),p=l.inspectOriginalQuadricCarrierRoots(quadricQuery()),r=quadricReport();r.finiteSurfaceUvAdmission=true;FakeWorker.all[0]!.reply(r);await expect(p).rejects.toThrow('Invalid surface audit report');expect(l.snapshot().workersTerminated).toBe(1)})
 it('binds the requested budget even if caller mutates later',async()=>{const l=lane(),q=quadricQuery(),p=l.inspectOriginalQuadricCarrierRoots(q);q.maxExactWork=1;FakeWorker.all[0]!.reply(quadricReport());await expect(p).resolves.toHaveProperty('maxExactWork',100000)})
 it('rejects a valid report for a different budget',async()=>{const l=lane(),p=l.inspectOriginalQuadricCarrierRoots(quadricQuery()),r=quadricReport();r.maxExactWork=100001;FakeWorker.all[0]!.reply(r);await expect(p).rejects.toThrow('Invalid surface audit report')})
 it('shares the busy guard and releases on abort',async()=>{const l=lane(),c=new AbortController(),p=l.inspectOriginalQuadricCarrierRoots(quadricQuery(),{signal:c.signal});await expect(l.inspect({} as any,10)).rejects.toThrow('busy');c.abort();await expect(p).rejects.toThrow('cancelled');expect(l.snapshot()).toEqual({active:false,workersStarted:1,workersTerminated:1})})
 it('rejects invalid matrices/products and budgets before starting a worker',async()=>{for(const modify of [(q:any)=>q.quadric=Array(4),(q:any)=>q.quadric[0][0]=NaN,(q:any)=>q.constantProducts=Array(1),(q:any)=>q.maxExactWork=1000001]){const q=quadricQuery();modify(q);await expect(lane().inspectOriginalQuadricCarrierRoots(q)).rejects.toThrow('Invalid original quadric query')}expect(FakeWorker.all).toHaveLength(0)})
 it('handles native refusal and successfully recovers on the next request',async()=>{const l=lane(),p=l.inspectOriginalQuadricCarrierRoots(quadricQuery()),w=FakeWorker.all[0]!;w.onmessage({data:{type:'surface-self-result/1',id:w.sent.id,ok:false,error:'native refused'}});await expect(p).rejects.toThrow('native refused');const next=l.inspectOriginalQuadricCarrierRoots(quadricQuery());FakeWorker.all[1]!.reply(quadricReport());await expect(next).resolves.toHaveProperty('status','present-on-carrier');expect(l.snapshot().workersTerminated).toBe(2)})
})
