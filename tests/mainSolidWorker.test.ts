import {it,expect,vi,afterEach,beforeEach} from 'vitest'
import type {MainParameters} from '../src/services/mainModeling'
// The warm worker is module state; reset modules so each test starts fresh.
let computeMainSolid:typeof import('../src/services/mainSolidWorker').computeMainSolid
let cancelMainSolid:typeof import('../src/services/mainSolidWorker').cancelMainSolid
beforeEach(async()=>{vi.resetModules();({computeMainSolid,cancelMainSolid}=await import('../src/services/mainSolidWorker'))})
class FakeWorker {
 static instances:FakeWorker[]=[];onmessage:any;onerror:any;onmessageerror:any;terminate=vi.fn();postMessage=vi.fn()
 constructor(){FakeWorker.instances.push(this)}
 reply(result:unknown,error?:string){const {version,id,job}=this.postMessage.mock.lastCall![0];this.onmessage({data:{version,id,kind:job.kind,...(error?{ok:false,error:{name:'Error',message:error}}:{ok:true,result})}})}
}
const p:MainParameters={amount:1,x:0,y:0,z:0,axis:'z',edge:0,shape:'circle',width:2,height:2,cut:false}
afterEach(()=>{cancelMainSolid();FakeWorker.instances=[];vi.unstubAllGlobals()})
it('cancels superseded geometry without allowing stale results to resolve',async()=>{
 vi.stubGlobal('Worker',FakeWorker)
 const first=computeMainSolid([],0,null,'shell',p),rejected=expect(first).rejects.toMatchObject({name:'AbortError'})
 const second=computeMainSolid([],0,null,'shell',p)
 await rejected;expect(FakeWorker.instances[0].terminate).toHaveBeenCalledTimes(1)
 const doc={version:1,sketches:[],bodies:[]};FakeWorker.instances[1].reply(doc)
 await expect(second).resolves.toEqual(doc)
})
it('propagates geometry failures without killing the warm worker',async()=>{
 vi.stubGlobal('Worker',FakeWorker)
 const task=computeMainSolid([],0,null,'fillet',p)
 FakeWorker.instances[0].reply(null,'Radius consumes a face')
 await expect(task).rejects.toThrow('Radius consumes a face')
 expect(FakeWorker.instances[0].terminate).not.toHaveBeenCalled()
 // The next operation reuses the surviving worker instead of recompiling WASM.
 const doc={version:1,sketches:[],bodies:[]}
 const second=computeMainSolid([],0,null,'shell',p)
 expect(FakeWorker.instances).toHaveLength(1)
 FakeWorker.instances[0].reply(doc)
 await expect(second).resolves.toEqual(doc)
})
