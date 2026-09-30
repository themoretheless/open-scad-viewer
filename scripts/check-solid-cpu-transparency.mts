import assert from 'node:assert/strict'
import {mkdir,writeFile} from 'node:fs/promises'
import {TransparentBsp,type TransparentFragment} from '../src/services/transparentBsp'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const directory=process.argv[2]??'/tmp/cpu-transparency'
await mkdir(directory,{recursive:true})
const red:TransparentFragment={owner:'red',triangle:[[-1,-1,-.6],[1,-1,.6],[0,1,0]]}
const blue:TransparentFragment={owner:'blue',triangle:[[-1,-1,.6],[1,-1,-.6],[0,1,0]]}
const reference=[red,blue].flatMap(f=>[
 {owner:f.owner,triangle:[f.triangle[0],[0,-1,0],[0,1,0]]},
 {owner:f.owner,triangle:[[0,-1,0],f.triangle[1],[0,1,0]]},
]) as TransparentFragment[]
// Explicit far-to-near order, derived from z=+/-0.6*x on each half-plane.
const referenceOrder=[reference[0],reference[3],reference[2],reference[1]]
const {playwright}=await loadQualificationPlaywrightPackage()
const browser=await playwright.chromium.launch({headless:true})
try{
 const page=await browser.newPage({viewport:{width:256,height:256}}),errors:string[]=[]
 page.on('pageerror',e=>errors.push(String(e)))
 const frames=[]
 for(const [name,fragments] of [['forward',new TransparentBsp([red,blue]).ordered([0,0,1])],['reversed',new TransparentBsp([blue,red]).ordered([0,0,1])],['reference',referenceOrder]] as const){
  const svg=`<svg xmlns="http://www.w3.org/2000/svg" width="256" height="256" viewBox="-1.5 -1.5 3 3"><rect x="-1.5" y="-1.5" width="3" height="3" fill="white"/>${fragments.map(f=>`<polygon points="${f.triangle.map(v=>`${v[0]},${v[1]}`).join(' ')}" fill="${f.owner}" opacity="0.5" stroke="none"/>`).join('')}</svg>`
  await page.setContent(`<style>body{margin:0}</style>${svg}`)
  await page.screenshot({path:`${directory}/${name}.png`})
  const samples=await page.evaluate(async svg=>{
   const image=new Image();image.src='data:image/svg+xml;charset=utf-8,'+encodeURIComponent(svg);await image.decode()
   const canvas=document.createElement('canvas');canvas.width=canvas.height=256;const ctx=canvas.getContext('2d')!;ctx.drawImage(image,0,0)
   return Array.from({length:100},(_,i)=>{const x=Math.floor((1.5-.5+(i%10)*.11)/3*256),y=Math.floor((1.5-.75+Math.floor(i/10)*.05)/3*256);return Array.from(ctx.getImageData(x,y,1,1).data)})
  },svg)
  frames.push({name,samples})
 }
 assert.deepEqual(frames[0].samples,frames[2].samples);assert.deepEqual(frames[1].samples,frames[2].samples)
 const expected=[[128,64,192,255],[192,64,128,255]]
 for(let sample=0;sample<100;sample++)for(let channel=0;channel<4;channel++)assert.ok(Math.abs(frames[0].samples[sample][channel]-expected[sample%10<5?0:1][channel])<=1,'source-over color must match within one 8-bit rounding unit')
 assert.deepEqual(errors,[])
 await writeFile(`${directory}/report.json`,JSON.stringify({frames,expected,errors,scope:'SVG body style and production BSP; 100 interior samples, not whole-app or edge-pixel acceptance.'},null,2))
 console.log(JSON.stringify({frames:frames.length,samplesPerFrame:100,matchesReference:true,matchesAlphaWithinOneUnit:true,errors}))
}finally{await browser.close()}
