import {chromium} from 'playwright'
import {spawn} from 'node:child_process'
import {writeFile} from 'node:fs/promises'

const port=4178,url=`http://127.0.0.1:${port}`
const server=spawn('npm',['run','dev','--','--host','127.0.0.1','--port',String(port)],{
  cwd:process.cwd(),stdio:['ignore','pipe','pipe'],
})
let serverError=''
server.stderr.on('data',chunk=>{serverError+=chunk})
const wait=async()=>{
  for(let attempt=0;attempt<480;attempt++){
    try{if((await fetch(url)).ok)return}catch{}
    await new Promise(resolve=>setTimeout(resolve,250))
  }
  throw Error(`Vite did not start${server.exitCode===null?'':` (exit ${server.exitCode})`}: ${serverError}`)
}
let browser
try{
  await wait()
  browser=await chromium.launch({headless:true,args:['--enable-features=WebAssemblyUnlimitedSyncCompilation']})
  const page=await browser.newPage()
  await page.goto(url,{waitUntil:'networkidle'})
  const first=await page.evaluate(async()=>{
    localStorage.removeItem('open-scad-viewer.retained-step-v1')
    const geometry=await import('/src/services/geometry/brep.ts')
    const step=await import('/src/services/cadNurbsStep.ts')
    const route=await import('/src/services/cadStepRouting.ts')
    const cad=await import('/src/services/cadWorkbench.ts')
    const source=geometry.createBrepBox([0,0,0],[2,3,4])
    const text=step.exportDirectStepV6(source).text
    const routed=route.importStepForWorkbench(text)
    route.retainedStepSession.set(routed.retainedModel,routed.report)
    const body=routed.bodies[0]
    const edited=cad.cadOperation({version:1,sketches:[],bodies:[body]},{
      action:'resize',ids:[body.id],sketches:[],axis:[0,0,1],origin:[0,0,0],
      amount:0,count:1,width:4,height:6,depth:8,pitch:1,secondary:1,mode:'min',pathId:'',profileIds:[],
    }).bodies[0]
    route.retainedStepSession.set(edited.brep,routed.report)
    const exported=route.exportRetainedStepForWorkbench(edited.brep)
    const reimported=step.importDirectStepV6(exported.text)
    return {capability:exported.certificate.capability,bodies:reimported.model.bodies.length,
      vertices:reimported.model.vertices.length,stored:localStorage.getItem('open-scad-viewer.retained-step-v1')?.length??0}
  })
  await page.reload({waitUntil:'networkidle'})
  const afterReload=await page.evaluate(async()=>{
    const route=await import('/src/services/cadStepRouting.ts')
    const step=await import('/src/services/cadNurbsStep.ts')
    const model=route.retainedStepSession.reload()
    if(!model)throw Error('retained model did not reload')
    const exported=route.exportRetainedStepForWorkbench(model)
    return {capability:exported.certificate.capability,bodies:step.importDirectStepV6(exported.text).model.bodies.length}
  })
  if(first.capability!=='step-interchange/6'||first.bodies!==1||first.stored<=0
    ||afterReload.capability!=='step-interchange/6'||afterReload.bodies!==1)throw Error('browser lifecycle assertion failed')
  const result={first,afterReload,userAgent:await page.evaluate(()=>navigator.userAgent)}
  await writeFile('output/qualification/step-interchange-6-v1/browser-retained.json',JSON.stringify(result,null,2))
  console.log(JSON.stringify(result))
}finally{
  await browser?.close()
  server.kill('SIGTERM')
}
