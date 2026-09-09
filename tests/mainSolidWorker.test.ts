import {it,expect,vi,afterEach} from 'vitest'
import {computeMainSolid,cancelMainSolid} from '../src/services/mainSolidWorker'
import type {MainParameters} from '../src/services/mainModeling'
class FakeWorker {
 static instances:FakeWorker[]=[];onmessage:any;onerror:any;terminate=vi.fn();postMessage=vi.fn()
 constructor(){FakeWorker.instances.push(this)}
}
const p:MainParameters={amount:1,x:0,y:0,z:0,axis:'z',edge:0,shape:'circle',width:2,height:2,cut:false}
afterEach(()=>{cancelMainSolid();FakeWorker.instances=[];vi.unstubAllGlobals()})
it('cancels superseded geometry without allowing stale results to resolve',async()=>{
 vi.stubGlobal('Worker',FakeWorker)
 const first=computeMainSolid([],0,null,'shell',p),rejected=expect(first).rejects.toMatchObject({name:'AbortError'})
 const second=computeMainSolid([],0,null,'shell',p)
 await rejected;expect(FakeWorker.instances[0].terminate).toHaveBeenCalledTimes(1)
 const doc={version:1,sketches:[],bodies:[]};FakeWorker.instances[1].onmessage({data:{document:doc}})
 await expect(second).resolves.toEqual(doc)
})
it('propagates geometry failures and terminates the worker',async()=>{
 vi.stubGlobal('Worker',FakeWorker)
 const task=computeMainSolid([],0,null,'fillet',p)
 FakeWorker.instances[0].onmessage({data:{error:'Radius consumes a face'}})
 await expect(task).rejects.toThrow('Radius consumes a face');expect(FakeWorker.instances[0].terminate).toHaveBeenCalledOnce()
})
