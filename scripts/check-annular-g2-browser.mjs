import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {createServer} from 'node:http'
import {readFile,writeFile,mkdir} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'

const root=path.resolve(process.env.SOLID_QUALIFICATION_DIST??'dist')
const out=path.resolve(process.env.G2_QUALIFICATION_OUTPUT??'output/qualification/annular-g2-2026-10-15/browser')
const size=process.env.G2_TRANSITION_SIZE??'0.1'
const individual=process.env.G2_TRANSITION_SIZES?.split(',').map(Number)
if(individual)assert.ok(individual.length===4&&individual.every(x=>Number.isFinite(x)&&x>0))
await mkdir(out,{recursive:true})
const server=createServer(async(req,res)=>{
 try{
  const pathname=decodeURIComponent(new URL(req.url,'http://localhost').pathname)
  const file=path.resolve(root,'.'+(pathname==='/'?'/index.html':pathname))
  if(!file.startsWith(root+'/'))throw Error('Outside distribution')
  res.setHeader('Content-Type',file.endsWith('.js')?'text/javascript':file.endsWith('.css')?'text/css':file.endsWith('.html')?'text/html':file.endsWith('.wasm')?'application/wasm':'application/octet-stream')
  res.end(await readFile(file))
 }catch{res.writeHead(404).end()}
})
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve))
let browser
try{
 const {playwright}=await loadQualificationPlaywrightPackage()
 browser=await playwright.chromium.launch({headless:true,args:['--enable-unsafe-webgpu','--use-angle=metal','--enable-features=WebGPU']})
 const page=await browser.newPage({viewport:{width:1440,height:1000},acceptDownloads:true})
 const errors=[];page.on('pageerror',e=>errors.push(String(e)))
 await page.goto(`http://127.0.0.1:${server.address().port}`)
 await page.getByRole('button',{name:'Solid',exact:true}).click()
 const solid=page.getByRole('region',{name:'Solid — CAD-лепка',exact:true})
 await solid.getByRole('button',{name:'Инструменты',exact:true}).click()
 const create=solid.getByRole('button',{name:'Создать',exact:true})
 if(await create.getAttribute('aria-expanded')!=='true')await create.click()
 await solid.getByText('Параметры круглых тел',{exact:true}).click()
 await solid.getByLabel('G2-переходы торцов трубы',{exact:true}).check()
 await solid.getByLabel('Размер G2-перехода, мм',{exact:true}).fill(size)
 if(individual){
  await solid.getByLabel('Разные переходы торцов',{exact:true}).check()
  for(const [i,name] of ['Наружный снизу','Наружный сверху','Внутренний сверху','Внутренний снизу'].entries())await solid.getByLabel(name+' · мм',{exact:true}).fill(String(individual[i]))
 }
 if(individual&&process.env.G2_TEST_RETRY==='1'){
  const first=solid.getByLabel('Наружный снизу · мм',{exact:true})
  await first.fill('100')
  await solid.getByRole('button',{name:'Труба',exact:true}).click()
  await solid.getByText(/^Проверьте внутренний радиус и уменьшите G2-переходы:/).waitFor()
  assert.ok(await solid.locator('input[aria-invalid="true"]').count()>=4)
  assert.equal(await solid.getByText('Труба · 3D · точный B-rep · G2',{exact:true}).count(),0)
  await first.fill(String(individual[0]))
  await solid.getByRole('button',{name:'Повторить создание',exact:true}).click()
 }else await solid.getByRole('button',{name:'Труба',exact:true}).click()
 await solid.getByText('Труба · 3D · точный B-rep · G2',{exact:true}).first().waitFor({timeout:30000})
 for(const name of ['snap-preparation','topology-preparation','display-refinement'])await solid.getByRole('status',{name,exact:true}).waitFor({state:'hidden',timeout:30000})
 await page.screenshot({path:path.join(out,'g2-tube.png')})
 const menu=solid.locator('summary[title="Файл"]')
 async function downloadProject(file){
  if(!await menu.evaluate(e=>e.parentElement.open))await menu.click()
  const pending=page.waitForEvent('download')
  await solid.getByRole('button',{name:'Скачать проект JSON',exact:true}).click()
  await(await pending).saveAs(path.join(out,file))
  return JSON.parse(await readFile(path.join(out,file),'utf8'))
 }
 const doc=await downloadProject('g2-tube.json')
 assert.equal(doc.bodies.at(-1)?.brep?.faces.length,32)
 if(process.env.G2_STEP_ORACLE){
  const oracle=JSON.parse(await readFile(process.env.G2_STEP_ORACLE,'utf8'))
  const part=oracle.parts.find(p=>p.name===(process.env.G2_STEP_PART??'closed-small-transition'))
  assert.ok(part,'Independent design-volume oracle required')
  const pending=page.waitForEvent('download')
  await solid.getByRole('button',{name:'STEP выбранного тела · текущая геометрия',exact:true}).click()
  const file='g2-tube.step';await(await pending).saveAs(path.join(out,file))
  const sha256=createHash('sha256').update(await readFile(path.join(out,file))).digest('hex')
  await writeFile(path.join(out,'manifest.json'),JSON.stringify({...oracle,parts:[{...part,file,sha256}]},null,2))
 }
 if(await menu.evaluate(e=>e.parentElement.open))await menu.click()
 await solid.getByRole('button',{name:'↶',exact:true}).click()
 await solid.getByRole('button',{name:'↷',exact:true}).click()
 await page.waitForTimeout(1500)
 await page.reload();await page.getByRole('button',{name:'Solid',exact:true}).click()
 await solid.getByRole('status',{name:'history-restore',exact:true}).waitFor({state:'hidden'})
 const restored=await downloadProject('restored.json')
 assert.deepEqual(restored.bodies,doc.bodies)
 assert.deepEqual(errors,[])
 const kernelSha256=createHash('sha256').update(await readFile(path.join(root,'wasm/geometry-kernel.wasm'))).digest('hex')
 await writeFile(path.join(out,'receipt.json'),JSON.stringify({faces:32,errors,createdThroughUi:true,undoRedoReload:true,nativeLimitRetry:process.env.G2_TEST_RETRY==='1',transitionSize:Number(size),transitionSizes:individual??Array(4).fill(Number(size)),kernelSha256},null,2))
}finally{await browser?.close();server.close()}
