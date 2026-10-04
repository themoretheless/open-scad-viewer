import assert from 'node:assert/strict'
import {createServer} from 'node:http'
import {mkdir,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const directory=path.resolve(process.argv[2]??'/tmp/two-webgpu-devices')
await mkdir(directory,{recursive:true})
const server=createServer((_request,response)=>response.writeHead(200,{'Content-Type':'text/html'}).end('<canvas id="first" width="80" height="80"></canvas><canvas id="second" width="80" height="80"></canvas>'))
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve))
let browser
try{
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true,args:['--enable-unsafe-webgpu'],...(process.env.CHROMIUM_EXECUTABLE?{executablePath:process.env.CHROMIUM_EXECUTABLE}:{})})
 const page=await browser.newPage();await page.goto(`http://127.0.0.1:${server.address().port}`)
 const result=await page.evaluate(async single=>{
  const errors=[],lost=[],devices=[],adapters=[],contexts=[]
  if(!navigator.gpu)return {supported:false,errors,lost}
  async function initialize(id,delay){
   const adapter=await navigator.gpu.requestAdapter();if(!adapter)throw Error('No GPU adapter');adapters.push(adapter)
   if(delay)await new Promise(resolve=>setTimeout(resolve,delay))
   const device=await adapter.requestDevice();devices.push(device)
   device.addEventListener('uncapturederror',event=>errors.push({id,message:event.error.message}))
   void device.lost.then(info=>lost.push({id,reason:info.reason,message:info.message}))
   const context=document.getElementById(id).getContext('webgpu');context.configure({device,format:navigator.gpu.getPreferredCanvasFormat()})
   contexts.push({id,context,device})
  }
  await Promise.all(single?[initialize('first',100)]:[initialize('first',100),initialize('second',0)])
  for(let frame=0;frame<24;frame++){
   await new Promise(resolve=>requestAnimationFrame(resolve))
   for(const {id,context,device} of contexts){
    device.pushErrorScope('validation')
    try{
     const encoder=device.createCommandEncoder(),pass=encoder.beginRenderPass({colorAttachments:[{view:context.getCurrentTexture().createView(),clearValue:{r:0,g:0.5,b:1,a:1},loadOp:'clear',storeOp:'store'}]})
     pass.end();device.queue.submit([encoder.finish()])
    }catch(error){errors.push({id,frame,message:String(error)})}
    try{const error=await device.popErrorScope();if(error)errors.push({id,frame,message:error.message})}catch(error){errors.push({id,frame,message:String(error)})}
   }
  }
  const result={supported:true,frames:24,adapters:adapters.map(adapter=>({vendor:adapter.info?.vendor,architecture:adapter.info?.architecture,device:adapter.info?.device,description:adapter.info?.description})),devices:devices.length,errors,lost:lost.slice()}
  contexts.forEach(({context})=>context.unconfigure());devices.forEach(device=>device.destroy())
  return result
 },process.argv.includes('--single-device'))
 const report={browser:browser.version(),...result}
 await writeFile(path.join(directory,'two-devices.json'),JSON.stringify(report,null,2)+'\n');console.log({browser:report.browser,devices:report.devices,errors:report.errors.length,lost:report.lost.length})
 assert.equal(result.supported,true);assert.deepEqual(result.errors,[]);assert.deepEqual(result.lost,[])
}finally{await browser?.close();await new Promise(resolve=>server.close(resolve))}
