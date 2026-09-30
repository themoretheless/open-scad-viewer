import assert from 'node:assert/strict'
import {readFile,mkdir,writeFile} from 'node:fs/promises'
import {compactWgslLiterals} from './compact-wgsl.mjs'
import {instancedObjectShader,immediateObjectShader} from '../src/services/shaders/variants'
import {loadQualificationPlaywrightPackage} from './qualificationPlaywrightPackage.mjs'
const output=process.argv[2]??'/tmp/compact-wgsl'
await mkdir(output,{recursive:true})
const normalize=(s:string)=>s.replace(/\/\/[^\r\n]*/g,'').replace(/\s+/g,' ').trim()
const sources=[] as {name:string;code:string}[]
let saved=0
for(const file of ['src/services/shaders/generated/sources.ts','src/services/solidGpuView.ts']){
 const original=await readFile(file,'utf8'),compact=compactWgslLiterals(original)
 saved+=Buffer.byteLength(original)-Buffer.byteLength(compact)
 const literals=(s:string)=>Array.from(s.matchAll(/const (\w+) = \/\* wgsl \*\/\s*`([^`]*)`/g),m=>({name:m[1],code:m[2]}))
 const before=literals(original),after=literals(compact);assert.equal(before.length,after.length)
 for(let i=0;i<before.length;i++){
  const a=before[i],b=after[i];assert.equal(normalize(b.code),normalize(a.code));if(b.code.includes('@vertex'))sources.push(b)
  if(a.code.includes('var<uniform> ob: Obj;')){
   const kind=a.code.includes('struct EdgeV {')?'EdgeV':'V'
   if(a.code.includes('@fragment fn fs(v: '+kind+') -> @location(0) vec4f {')){
    const variant=instancedObjectShader(b.code,kind);assert.equal(normalize(variant),normalize(instancedObjectShader(a.code,kind)));sources.push({name:b.name+'-instanced',code:variant})
   }
   assert.equal(normalize(immediateObjectShader(b.code)),normalize(immediateObjectShader(a.code)))
  }
 }
}
assert.ok(saved>3000)
for(const body of ['let a = ${expression};','/* nested /* comment */ */ let a=1;','let a = "quoted";','let a=1;\\n']){
 const source='const s=/* wgsl */`'+body+'`';assert.equal(compactWgslLiterals(source),source)
}
const {playwright}=await loadQualificationPlaywrightPackage()
const browser=await playwright.chromium.launch({headless:false,args:['--enable-unsafe-webgpu']})
try{
 const page=await browser.newPage()
 // A local secure origin is required for WebGPU.
 await page.route('http://localhost:5319/**',route=>route.fulfill({contentType:'text/html',body:'<!doctype html><title>WGSL qualification</title>'}))
 await page.goto('http://localhost:5319/')
 const results=await page.evaluate(async sources=>{
  const adapter=await navigator.gpu.requestAdapter();if(!adapter)throw Error('No GPU adapter')
  const device=await adapter.requestDevice(),results=[]
  try{for(const source of sources){const info=await device.createShaderModule({code:source.code}).getCompilationInfo();results.push({name:source.name,errors:info.messages.filter(m=>m.type==='error').map(m=>m.message)})}}finally{device.destroy()}
  return results
 },sources)
 for(const result of results)assert.deepEqual(result.errors,[],result.name)
 const report={savedSourceBytes:saved,modules:results.length,tokenAndVariantParity:true,results,browser:browser.version()}
 await writeFile(output+'/report.json',JSON.stringify(report,null,2));console.log(report)
}finally{await browser.close()}
