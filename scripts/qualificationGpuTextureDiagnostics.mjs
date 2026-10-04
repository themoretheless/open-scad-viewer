/** Diagnostic interception only: captured validation failures remain a failing gate. */
export async function installGpuTextureDiagnostics(page) {
 const events=[]
 const prefix='__cad_gpu_texture__ '
 page.on('console',message=>{if(message.type()==='debug'&&message.text().startsWith(prefix))events.push(JSON.parse(message.text().slice(prefix.length)))})
 await page.addInitScript(()=>{
  if(!navigator.gpu||!window.GPUCanvasContext||!window.GPUTexture)return
  const documentId=crypto.randomUUID(),devices=new WeakMap(),textures=new WeakMap(),contexts=new WeakMap(),adapters=new WeakSet()
  let sequence=0
  const record=event=>console.debug('__cad_gpu_texture__ '+JSON.stringify({documentId,time:performance.now(),...event}))
  const configure=GPUCanvasContext.prototype.configure,get=GPUCanvasContext.prototype.getCurrentTexture,view=GPUTexture.prototype.createView
  GPUCanvasContext.prototype.configure=function(descriptor){contexts.set(this,descriptor.device);record({kind:'configure',device:devices.get(descriptor.device),canvas:this.canvas.className});return configure.call(this,descriptor)}
  GPUCanvasContext.prototype.getCurrentTexture=function(...args){const texture=get.apply(this,args),device=contexts.get(this);if(device)textures.set(texture,device);return texture}
  GPUTexture.prototype.createView=function(...args){
   const device=textures.get(this)
   if(!device)return view.apply(this,args)
   const stack=new Error().stack,format=this.format,width=this.width,height=this.height
   device.pushErrorScope('validation')
   let result
   try{result=view.apply(this,args)}finally{
    void device.popErrorScope().then(error=>{if(error)record({kind:'view-error',device:devices.get(device),message:error.message,format,width,height,stack})},error=>record({kind:'scope-cancelled',message:String(error)}))
   }
   return result
  }
  const request=navigator.gpu.requestAdapter.bind(navigator.gpu)
  navigator.gpu.requestAdapter=async(...args)=>{
   const adapter=await request(...args)
   if(!adapter||adapters.has(adapter))return adapter
   adapters.add(adapter)
   const acquire=adapter.requestDevice.bind(adapter)
   adapter.requestDevice=async(...args)=>{
    const device=await acquire(...args);devices.set(device,++sequence)
    const make=device.createTexture.bind(device),destroy=device.destroy.bind(device)
    device.createTexture=descriptor=>{const texture=make(descriptor);textures.set(texture,device);return texture}
    device.destroy=()=>{record({kind:'destroy-device',device:devices.get(device),stack:new Error().stack});return destroy()}
    device.addEventListener('uncapturederror',event=>record({kind:'uncaptured-error',device:devices.get(device),message:event.error.message}))
    void device.lost.then(info=>record({kind:'device-lost',device:devices.get(device),reason:info.reason,message:info.message}))
    return device
   }
   return adapter
  }
 })
 return ()=>events.slice()
}
