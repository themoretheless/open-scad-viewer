import {expect,it,vi} from 'vitest'

it('initializes asynchronously and retries a failed compile',async()=>{
 vi.resetModules()
 const kernel=await import('../src/services/geometry/kernel')
 const instantiate=vi.spyOn(WebAssembly,'instantiate').mockRejectedValueOnce(new Error('injected compile failure'))
 try{
  await expect(kernel.warmGeometryKernel()).rejects.toThrow('injected compile failure')
  await kernel.warmGeometryKernel()
  const runtime=kernel.kernelRuntime()
  await kernel.warmGeometryKernel()
  expect(kernel.kernelRuntime().exports).toBe(runtime.exports)
  expect(instantiate).toHaveBeenCalledTimes(2)
 }finally{instantiate.mockRestore()}
})

it('shares warmup and never replaces a synchronously initialized native owner',async()=>{
 vi.resetModules()
 const kernel=await import('../src/services/geometry/kernel')
 const first=kernel.warmGeometryKernel(),second=kernel.warmGeometryKernel()
 expect(second).toBe(first)
 const created=kernel.callGeometryRust<{handle:string}>('mesh_picking',{action:'create',vertices:[-1,-1,0,1,-1,0,0,1,0],indices:[0,1,2],stride:3,leafSize:1})
 const runtime=kernel.kernelRuntime()
 try{
  await first
  expect(kernel.kernelRuntime().exports).toBe(runtime.exports)
  expect(kernel.callGeometryRust('mesh_picking',{action:'query',handle:created.handle,origin:[0,0,2],direction:[0,0,-1],excludedTriangles:[]})).toMatchObject({t:2})
  await kernel.warmGeometryKernel()
  expect(kernel.kernelRuntime().exports).toBe(runtime.exports)
 }finally{kernel.callGeometryRust('mesh_picking',{action:'dispose',handle:created.handle})}
})
