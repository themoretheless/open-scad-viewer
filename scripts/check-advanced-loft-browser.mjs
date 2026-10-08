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
  await page.locator('input[accept=".scad,.r,text/plain"]').setInputFiles({name:file,mimeType:'text/plain',buffer:source})
  await page.getByRole('button',{name:'Собрать',exact:true}).click()
  await page.waitForTimeout(1500)
  await page.locator('.more-menu > summary').click()
  await page.getByRole('button',{name:'Перенести сцену в Mesh',exact:true}).click()
  await page.getByRole('button',{name:'Исходный код',exact:true}).click()
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
