import {afterEach,expect,it,vi} from 'vitest'
import * as transparencyKernel from '../src/services/geometry/transparentBspKernel'
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
 const diagnostic=vi.fn(),unavailable=vi.fn(),limited=vi.fn(),layer=new SolidGpuLayer({width:100,height:100,getContext:()=>context,setAttribute:diagnostic} as unknown as HTMLCanvasElement,unavailable,limited)
 return {layer,device,context,lose,unavailable,limited,diagnostic,pass,draw:()=>frame(0)}
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

it('preserves the renderer failure reason before releasing resources',async()=>{
 const f=fixture();await f.layer.init()
 f.layer.setBodies([{id:'invalid',positions:new Float32Array([0,0,0,1,0,0,0,1,0]),normals:new Float32Array(9),hues:new Float32Array(1),opacity:NaN}])
 expect(f.diagnostic).toHaveBeenCalledWith('data-gpu-error','Invalid material opacity')
 expect(f.unavailable).toHaveBeenCalledTimes(1);expect(f.layer.ready).toBe(false)
})


it('releases the acquired device and context if pipeline initialization throws',async()=>{
 const f=fixture();f.device.createRenderPipeline=()=>{throw Error('pipeline unavailable')}
 expect(await f.layer.init()).toBe(false)
 expect(f.device.destroy).toHaveBeenCalledTimes(1)
 expect(f.context.unconfigure).toHaveBeenCalledTimes(1)
 expect(f.layer.ready).toBe(false)
 expect(f.unavailable).not.toHaveBeenCalled()
})

it('keeps WebGPU available when a temporary transparent preview exhausts native sorting limits',async()=>{
 const f=fixture();await f.layer.init()
 const build=vi.spyOn(transparencyKernel,'buildTransparentBsp').mockImplementation(()=>{throw Error('Transparency operation limit exceeded')})
 try{
  f.layer.setBodies([{id:'preview',positions:new Float32Array([0,0,0,1,0,0,0,1,0]),normals:new Float32Array(9),hues:new Float32Array(1),opacity:.28}])
  expect(f.layer.ready).toBe(true);expect(f.unavailable).not.toHaveBeenCalled();expect(f.limited).toHaveBeenLastCalledWith(true)
  f.draw();expect(f.pass.draw).toHaveBeenLastCalledWith(3)
  f.layer.setDragOffset(['preview'],[1,0,0]);expect(build).toHaveBeenCalledTimes(1)
  f.layer.setBodies([]);expect(f.limited).toHaveBeenLastCalledWith(false);expect(f.layer.ready).toBe(true)
 }finally{build.mockRestore();f.layer.destroy()}
})

it('uploads opaque colors and all current drag positions when sorting reaches its budget during a drag',async()=>{
 const f=fixture();await f.layer.init()
 const build=vi.spyOn(transparencyKernel,'buildTransparentBsp').mockImplementationOnce(()=>({planes:new Float64Array(),links:new Uint32Array(),owners:new Uint32Array(),vertices:new Float64Array(),root:0,width:13,count:0,operations:0})).mockImplementation(()=>{throw Error('Transparency fragment limit exceeded')})
 try{
  f.layer.setBodies([{id:'preview',positions:new Float32Array([0,0,0,1,0,0,0,1,0]),normals:new Float32Array(9),hues:new Float32Array(1),opacity:.28}])
  f.layer.setDragOffset(['preview'],[2,3,4])
  const uploaded=f.device.queue.writeBuffer.mock.calls.at(-1)![2] as Float32Array
  expect(Array.from(uploaded.slice(0,3))).toEqual([2,3,4]);expect(uploaded[12]).toBe(1);expect(uploaded[25]).toBe(1)
  expect(f.layer.ready).toBe(true);expect(f.unavailable).not.toHaveBeenCalled();expect(f.limited).toHaveBeenLastCalledWith(true)
 }finally{build.mockRestore();f.layer.destroy()}
})
