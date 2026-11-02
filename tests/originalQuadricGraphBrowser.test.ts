import {beforeEach,afterEach,describe,it,expect,vi} from 'vitest'
import {SurfaceSelfIntersectionBrowser} from '../src/services/surfaceSelfIntersectionBrowser'
import {graphQuery,graphReport,graphUnprovedReport} from './fixtures/quadricGraph'
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
describe('finite quadric graph browser transport',()=>{
 it('sends the finite graph operation and terminates after validated success',async()=>{const l=lane(),q=graphQuery(),before=JSON.stringify(q),p=l.inspectOriginalQuadricGraphContacts(q),w=FakeWorker.all[0]!;expect(w.sent.type).toBe('original-quadric-graph-contacts/1');expect(w.sent.patch).toEqual(q.patch);w.reply(graphReport());await expect(p).resolves.toHaveProperty('finiteSurfaceUvAdmission',true);expect(JSON.stringify(q)).toBe(before);expect(l.snapshot()).toEqual({active:false,workersStarted:1,workersTerminated:1})})
 it('rejects an unbound finite admission and releases ownership',async()=>{const l=lane(),p=l.inspectOriginalQuadricGraphContacts(graphQuery()),r=graphReport();delete r.carrier.roots[0].normalizedOriginalSpanParameters;FakeWorker.all[0]!.reply(r);await expect(p).rejects.toThrow('Invalid surface audit report');expect(l.snapshot().workersTerminated).toBe(1)})
 it('binds the requested maximum budget through later caller mutation',async()=>{const l=lane(),q=graphQuery(),p=l.inspectOriginalQuadricGraphContacts(q);q.maxExactWork=1;FakeWorker.all[0]!.reply(graphReport());await expect(p).resolves.toHaveProperty('maxExactWork',1000000)})
 it('rejects a structurally valid report for another requested span limit',async()=>{const l=lane(),p=l.inspectOriginalQuadricGraphContacts(graphQuery()),r=graphReport();r.maxSpans=11;r.carrier.maxSpans=11;FakeWorker.all[0]!.reply(r);await expect(p).rejects.toThrow('Invalid surface audit report')})
 it('shares the carrier busy guard and terminates on abort',async()=>{const l=lane(),c=new AbortController(),p=l.inspectOriginalQuadricGraphContacts(graphQuery(),{signal:c.signal});await expect(l.inspectOriginalQuadricCarrierRoots(graphQuery())).rejects.toThrow('busy');c.abort();await expect(p).rejects.toThrow('cancelled');expect(l.snapshot()).toEqual({active:false,workersStarted:1,workersTerminated:1})})
 it('accepts explicit target-proof exhaustion at zero budget',async()=>{const l=lane(),p=l.inspectOriginalQuadricGraphContacts({...graphQuery(),maxExactWork:0}),r=graphUnprovedReport();r.maxExactWork=0;r.exactWork=0;r.targetMembership={status:'indeterminate',reason:'ResourceLimit',workUsed:0};FakeWorker.all[0]!.reply(r);await expect(p).resolves.toHaveProperty('targetGraphProven',false)})
 it('refuses malformed scalar options before starting a worker',async()=>{await expect(lane().inspectOriginalQuadricGraphContacts({...graphQuery(),maxExactWork:1000001})).rejects.toThrow('Invalid original quadric graph query');expect(FakeWorker.all).toHaveLength(0)})
})
