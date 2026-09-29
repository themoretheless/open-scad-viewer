import {afterEach,expect,it,vi} from 'vitest'
import {SolidGpuLayer} from '../src/services/solidGpuView'
afterEach(()=>vi.unstubAllGlobals())
function fixture(){
 let lose!:()=>void,frame!:FrameRequestCallback
 const lost=new Promise<void>(resolve=>{lose=resolve})
 const pass={end:vi.fn(),setPipeline:vi.fn(),setBindGroup:vi.fn(),setVertexBuffer:vi.fn(),draw:vi.fn()}
 const device={lost,destroy:vi.fn(()=>lose()),addEventListener:vi.fn(),createShaderModule:()=>({}),createRenderPipeline:()=>({getBindGroupLayout:()=>({})}),createBuffer:()=>({destroy:vi.fn()}),createBindGroup:()=>({}),createTexture:()=>({createView:()=>({}),destroy:vi.fn()}),createCommandEncoder:()=>({beginRenderPass:()=>pass,finish:()=>({})}),queue:{writeBuffer:vi.fn(),submit:vi.fn(),onSubmittedWorkDone:()=>Promise.resolve()}}
 const context={configure:vi.fn(),unconfigure:vi.fn(),getCurrentTexture:()=>({createView:()=>({})})}
 vi.stubGlobal('navigator',{gpu:{requestAdapter:async()=>({requestDevice:async()=>device}),getPreferredCanvasFormat:()=> 'bgra8unorm'}})
 vi.stubGlobal('GPUBufferUsage',{UNIFORM:1,COPY_DST:2,VERTEX:4});vi.stubGlobal('GPUTextureUsage',{RENDER_ATTACHMENT:1})
 vi.stubGlobal('requestAnimationFrame',(callback:FrameRequestCallback)=>{frame=callback;return 1});vi.stubGlobal('cancelAnimationFrame',vi.fn())
 const unavailable=vi.fn(),layer=new SolidGpuLayer({width:100,height:100,getContext:()=>context} as unknown as HTMLCanvasElement,unavailable)
 return {layer,device,lose,unavailable,draw:()=>frame(0)}
}
it('notifies the view exactly once after device loss',async()=>{
 const f=fixture();expect(await f.layer.init()).toBe(true);f.lose();await Promise.resolve();await Promise.resolve()
 expect(f.layer.ready).toBe(false);expect(f.unavailable).toHaveBeenCalledTimes(1)
 f.layer.destroy();expect(f.unavailable).toHaveBeenCalledTimes(1)
})
it('switches to fallback when queue completion rejects',async()=>{
 const f=fixture();await f.layer.init();f.device.queue.onSubmittedWorkDone=()=>Promise.reject(new Error('device unavailable'))
 f.layer.setBodies([]);f.draw();await Promise.resolve();await Promise.resolve();await Promise.resolve()
 expect(f.unavailable).toHaveBeenCalledTimes(1);expect(f.layer.ready).toBe(false)
})
it('does not notify an unmounted view after normal disposal',async()=>{
 const f=fixture();await f.layer.init();f.layer.destroy();await Promise.resolve();await Promise.resolve()
 expect(f.unavailable).not.toHaveBeenCalled()
})
