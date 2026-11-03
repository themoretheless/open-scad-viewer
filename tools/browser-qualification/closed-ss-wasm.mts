import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {resolve} from 'node:path'
import {createRequire} from 'node:module'
const [wasmPath,fixturePath,out,sourceCommit]=process.argv.slice(2)
assert(wasmPath&&fixturePath&&out&&/^[a-f0-9]{40}$/.test(sourceCommit??''))
const repo=resolve(process.env.OSV_BROWSER_ROOT??process.cwd())
const require=createRequire(resolve(repo,'tools/browser-qualification/package.json'))
const {chromium}=require('playwright')
const rootRequire=createRequire(resolve(repo,'package.json'))
const {createServer}=await import(rootRequire.resolve('vite'))
const bytes=readFileSync(wasmPath),fixture=JSON.parse(readFileSync(fixturePath,'utf8'))
const identity={sha256:createHash('sha256').update(bytes).digest('hex'),bytes:bytes.length,sourceCommit}
const common=`
function request(v){
 const data=codec.encodeBinary(v),ptr=w.abi_alloc(data.length);if(!ptr)throw Error('allocation');
 new Uint8Array(w.memory.buffer,ptr,data.length).set(data);
 try{const packed=w.abi_request(0,ptr,data.length),p=Number(packed&0xffffffffn),n=Number(packed>>32n);
  try{const r=codec.decodeBinary(new Uint8Array(w.memory.buffer,p,n));
   const retained=new Uint8Array(w.memory.buffer,ptr,data.length);if(!retained.every((b,i)=>b===data[i]))throw Error('input mutated');return r;
  }finally{w.abi_free(p,n);}
 }finally{w.abi_free(ptr,data.length);}
}
const before=JSON.stringify(fixture),start=performance.now(),r=request(fixture);
if(!r.ok)throw Error(JSON.stringify(r));const v=r.value;
if(v.originalSourceClosedCoverage!==true||v.materialClassified!==false||v.a.faces.length!==2||v.b.faces.length!==2||v.worldErrorUpper>1e-6||v.uvErrorUpper.some(x=>x>1e-6))throw Error('bad closed partition');
for(const m of [v.a,v.b]){if(m.faces[0].holes.length!==1)throw Error('missing hole');}
const failures=[];
for(const [key,value] of [['faceA',99],['toleranceMm',0],['materialClassified',true]]){
 const q=structuredClone(fixture);q[key]=value;const bad=request(q);if(bad.ok||typeof bad.error.code!=='string')throw Error('accepted '+key);failures.push({key,code:bad.error.code});
}
if(JSON.stringify(fixture)!==before)throw Error('fixture mutated');
return {passed:true,segments:v.segments,worldErrorUpper:v.worldErrorUpper,uvErrorUpper:v.uvErrorUpper,failures,elapsedMs:performance.now()-start,inputIntact:true};`
let browser:any,server:any
try{
 server=await createServer({root:repo,configFile:false,server:{host:'127.0.0.1',port:0}})
 server.middlewares.use((req:any,res:any,next:any)=>{
  if(req.url==='/__closed_ss.wasm'){res.setHeader('Content-Type','application/wasm');res.end(bytes)}
  else if(req.url==='/__closed_ss'){res.setHeader('Content-Type','text/html');res.end('<!doctype html><title>Closed SS ABI qualification</title>')}
  else next()
 })
 await server.listen();browser=await chromium.launch({headless:true})
 const page=await browser.newPage();const errors:string[]=[];page.on('pageerror',(e:any)=>errors.push(e.message))
 await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/__closed_ss`)
 const result=await page.evaluate(async({fixture,common,identity}:any)=>{
  const raw=await(await fetch('/__closed_ss.wasm')).arrayBuffer()
  const sha=[...new Uint8Array(await crypto.subtle.digest('SHA-256',raw))].map(x=>x.toString(16).padStart(2,'0')).join('')
  if(sha!==identity.sha256)throw Error('candidate mismatch')
  const module=await WebAssembly.compile(raw);if(WebAssembly.Module.imports(module).length)throw Error('unexpected imports')
  const codec=await import('/src/services/valueBinaryCodec.ts')
  const instance=await WebAssembly.instantiate(module,{})
  const main=new Function('w','codec','fixture',common)(instance.exports,codec,fixture)
  const workerBody=`import * as codec from ${JSON.stringify(location.origin+'/src/services/valueBinaryCodec.ts')};onmessage=async e=>{try{const w=(await WebAssembly.instantiate(e.data.module,{})).exports;const fixture=e.data.fixture;const result=(function(){${common}})();postMessage({ok:true,result});}catch(e){postMessage({ok:false,error:String(e)})}};`
  const url=URL.createObjectURL(new Blob([workerBody],{type:'text/javascript'})),worker=new Worker(url,{type:'module'})
  try{const thread=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('worker timeout')),120000);worker.onmessage=e=>{clearTimeout(timer);resolve(e.data)};worker.onerror=e=>{clearTimeout(timer);reject(Error(e.message))};worker.postMessage({module,fixture})});return {main,thread}}
  finally{worker.terminate();URL.revokeObjectURL(url)}
 },{fixture,common,identity})
 assert.equal(result.main.passed,true);assert.equal(result.thread.ok,true,JSON.stringify(result.thread));assert.equal(result.thread.result.passed,true);assert.deepEqual(errors,[])
 const report={schema:'closed-ss-wasm-qualification/1',identity,browser:browser.version(),result,pageErrors:errors,passed:true,scope:'One regular nonperiodic original closed component, two finite-face subdivisions; no material/Boolean/general-singularity qualification.'}
 mkdirSync(out,{recursive:true});writeFileSync(resolve(out,'browser.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(report))
}finally{await browser?.close();await server?.close()}
