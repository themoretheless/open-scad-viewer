import {mkdir,readFile,writeFile} from 'node:fs/promises'
import path from 'node:path'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const {playwright}=await loadQualificationPlaywrightPackage()
const directory=path.resolve(process.argv[2]??'docs/qualification/advanced-loft-2026-10-02')
await mkdir(directory,{recursive:true})
const browser=await playwright.chromium.launch({headless:true})
try{
 for(const file of ['auto-guided-loft.r','g2-loft-surface.r','natural-loft-solid.r']){
  const page=await browser.newPage({viewport:{width:1440,height:1000}}),errors=[]
  page.on('pageerror',e=>errors.push(e.message))
  await page.goto(process.argv[3]??'http://127.0.0.1:5181/')
  await page.getByRole('button',{name:'Исходный код',exact:true}).click()
  const source=await readFile(path.resolve('examples/rush',file))
  await page.locator('input[accept=".scad,.r,.mg,text/plain"]').setInputFiles({name:file,mimeType:'text/plain',buffer:source})
  await page.getByRole('button',{name:'Собрать',exact:true}).click()
  await page.waitForTimeout(1500)
  await page.locator('.more-menu > summary').click()
  await page.getByRole('button',{name:'Перенести сцену в Mesh',exact:true}).click()
  await page.getByRole('button',{name:'Исходный код',exact:true}).click()
  await page.locator('.mesh-workspace').focus()
  await page.keyboard.press('f')
  // Pan the CPU mesh into the visible viewport before taking qualification images.
  const svg=page.locator('.mesh-view'),box=await svg.boundingBox()
  const bounds=await svg.locator('polygon').evaluateAll(nodes=>{
   const boxes=nodes.map(node=>node.getBoundingClientRect())
   return boxes.length?{left:Math.min(...boxes.map(b=>b.left)),right:Math.max(...boxes.map(b=>b.right)),top:Math.min(...boxes.map(b=>b.top)),bottom:Math.max(...boxes.map(b=>b.bottom))}:null
  })
  if(box&&bounds){
   const viewport=page.viewportSize(),x=box.x+30,y=box.y+30
   const targetX=(Math.max(box.x,0)+Math.min(box.x+box.width,viewport.width))/2
   const targetY=(Math.max(box.y,0)+Math.min(box.y+box.height,viewport.height))/2
   await page.mouse.move(x,y);await page.keyboard.down('Shift');await page.mouse.down()
   await page.mouse.move(x+targetX-(bounds.left+bounds.right)/2,y+targetY-(bounds.top+bounds.bottom)/2,{steps:8})
   await page.mouse.up();await page.keyboard.up('Shift')
  }
  await page.waitForTimeout(1000)
  const name=file.replace('.r',''),text=await page.locator('body').innerText()
  await page.screenshot({path:path.join(directory,name+'.png'),fullPage:true})
  await writeFile(path.join(directory,name+'.browser.json'),JSON.stringify({file,pageErrors:errors,text,render:'headless CPU fallback; GPU qualification excluded'},null,2)+'\n')
  if(errors.length)throw new Error(JSON.stringify(errors))
  if(file==='natural-loft-solid.r'&&!text.includes('замкнут'))throw new Error('Capped loft Mesh preview is not closed')
  console.log(name+': rendered without pageerror')
  await page.close()
 }
}finally{await browser.close()}
