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
const verifyHost=process.env.OSV_VERIFY_INSTALLED_HOST==='1'
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
const branches=fixture.op==='brep_partition_all_closed_ss'?2:1;
if(v.originalSourceClosedCoverage!==true||v.materialClassified!==false||v.a.faces.length!==branches+1||v.b.faces.length!==branches+1||v.worldErrorUpper>fixture.toleranceMm||v.uvErrorUpper.some(x=>x>fixture.toleranceUv))throw Error('bad closed partition');
for(const m of [v.a,v.b]){
 if(m.faces[0].holes.length!==branches)throw Error('missing hole');
 for(const f of m.faces)for(const id of [f.outer,...f.holes]){
  const c=m.loops[id].coedges;
  for(let i=0;i<c.length;i++){
   const a=c[i],b=c[(i+1)%c.length],ea=m.edges[a.edge],eb=m.edges[b.edge];
   if(ea.vertices[a.reversed?0:1]!==eb.vertices[b.reversed?1:0])throw Error('open topology');
   const p=a.pcurve.controlPoints.at(-1),q=b.pcurve.controlPoints[0];
   if(Math.hypot(...p.map((x,k)=>x-q[k]))>fixture.toleranceUv)throw Error('open UV loop');
   for(const end of [0,1]){
    const p=ea.curve.controlPoints[end?ea.curve.controlPoints.length-1:0],q=m.vertices[ea.vertices[end]].point;
    if(Math.hypot(...p.map((x,k)=>x-q[k]))>m.toleranceMm)throw Error('unbound world endpoint');
   }
  }
 }
}
const failures=[];
for(const [key,value] of [['faceA',99],['toleranceMm',0],['materialClassified',true]]){
 const q=structuredClone(fixture);q[key]=value;const bad=request(q);if(bad.ok||typeof bad.error.code!=='string')throw Error('accepted '+key);failures.push({key,code:bad.error.code});
}
if(JSON.stringify(fixture)!==before)throw Error('fixture mutated');
return {passed:true,branches,segments:v.segments,worldErrorUpper:v.worldErrorUpper,uvErrorUpper:v.uvErrorUpper,failures,elapsedMs:performance.now()-start,inputIntact:true};`
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
 await page.route('**/__closed_ss.wasm',(route:any)=>route.fulfill({contentType:'application/wasm',body:bytes}))
 await page.route('**/__closed_ss',(route:any)=>route.fulfill({contentType:'text/html',body:'<!doctype html><title>Closed SS ABI qualification</title>'}))
 await page.goto(`http://127.0.0.1:${server.httpServer.address().port}/__closed_ss`)
 const result=await page.evaluate(async({fixture,common,identity,verifyHost}:any)=>{
  const raw=await(await fetch('/__closed_ss.wasm')).arrayBuffer()
  const sha=[...new Uint8Array(await crypto.subtle.digest('SHA-256',raw))].map(x=>x.toString(16).padStart(2,'0')).join('')
  if(sha!==identity.sha256)throw Error('candidate mismatch')
  const module=await WebAssembly.compile(raw);if(WebAssembly.Module.imports(module).length)throw Error('unexpected imports')
  const codec=await import('/src/services/valueBinaryCodec.ts')
  const instance=await WebAssembly.instantiate(module,{})
  const main=new Function('w','codec','fixture',common)(instance.exports,codec,fixture)
  if(verifyHost){
   const kernel=await import('/src/services/geometry/kernel.ts');await kernel.warmGeometryKernel()
   const {op,...args}=fixture;const host:any=kernel.callGeometryRust(op,args)
   if(host.segments!==main.segments||host.materialClassified!==false)throw Error('host dispatch mismatch')
   const packed=(await import('/src/generated/geometry-kernels/bytes.ts')).default
   const {unpackBrotliWasmBase64}=await import('/src/services/wasmBrotliPacking.ts')
   const decoded=unpackBrotliWasmBase64(packed)
   const hash=[...new Uint8Array(await crypto.subtle.digest('SHA-256',decoded))].map(x=>x.toString(16).padStart(2,'0')).join('')
   if(hash!==identity.sha256||decoded.length!==identity.bytes)throw Error('embedded package mismatch')
   main.hostCall=true;main.embeddedHashVerified=true
  }
  const workerBody=`import * as codec from ${JSON.stringify(location.origin+'/src/services/valueBinaryCodec.ts')};onmessage=async e=>{try{const w=(await WebAssembly.instantiate(e.data.module,{})).exports;const fixture=e.data.fixture;const result=(function(){${common}})();if(${verifyHost}){globalThis.__G1_SHARED_GEOMETRY_MODULE__=true;const kernel=await import(${JSON.stringify(location.origin+'/src/services/geometry/kernel.ts')});await kernel.installGeometryKernelFromTrustedHost(e.data.module,${JSON.stringify({sha256:identity.sha256,byteLength:identity.bytes})});const {op,...args}=fixture;const host=kernel.callGeometryRust(op,args);if(host.segments!==result.segments||host.materialClassified!==false)throw Error('worker host mismatch');result.hostCall=true;}postMessage({ok:true,result});}catch(e){postMessage({ok:false,error:String(e)})}};`
  const url=URL.createObjectURL(new Blob([workerBody],{type:'text/javascript'})),worker=new Worker(url,{type:'module'})
  try{const thread=await new Promise((resolve,reject)=>{const timer=setTimeout(()=>reject(Error('worker timeout')),120000);worker.onmessage=e=>{clearTimeout(timer);resolve(e.data)};worker.onerror=e=>{clearTimeout(timer);reject(Error(e.message))};worker.postMessage({module,fixture})});return {main,thread}}
  finally{worker.terminate();URL.revokeObjectURL(url)}
 },{fixture,common,identity,verifyHost})
 assert.equal(result.main.passed,true);assert.equal(result.thread.ok,true,JSON.stringify(result.thread));assert.equal(result.thread.result.passed,true);assert.deepEqual(errors,[])
 const report={schema:'closed-ss-wasm-qualification/1',identity,browser:browser.version(),result,pageErrors:errors,passed:true,scope:'Finite original regular nonperiodic closed-component fixtures and paired finite-face subdivisions; no material/Boolean/general-singularity qualification.'}
 mkdirSync(out,{recursive:true});writeFileSync(resolve(out,'browser.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(report))
}finally{await browser?.close();await server?.close()}
