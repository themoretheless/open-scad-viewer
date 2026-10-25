// Build a disposable production-runtime copy without changing the live package.
import assert from 'node:assert/strict'
import {cpSync,existsSync,mkdirSync,readFileSync,readdirSync,realpathSync,symlinkSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {basename,dirname,isAbsolute,relative,resolve,sep} from 'node:path'
import {fileURLToPath} from 'node:url'
import {packGeometryKernel} from './pack-geometry-kernel.mjs'
import {verifyPackedWasmChunk} from './verify-packed-wasm.mjs'

const [input,output]=process.argv.slice(2)
assert(input&&output,'Provide a WASM file and a new external qualification directory')
const root=resolve(dirname(fileURLToPath(import.meta.url)),'..')
const stage=resolve(output)
const rel=relative(realpathSync(root),resolve(realpathSync(dirname(stage)),basename(stage)))
assert(isAbsolute(output),'Qualification directory must be absolute')
assert(rel.startsWith('..'+sep)||rel==='..','Use a directory outside the live checkout')
assert(!existsSync(stage),'Qualification directory must be new')
const wasm=readFileSync(resolve(input))
const hash=bytes=>createHash('sha256').update(bytes).digest('hex')
const live=resolve(root,'public/wasm/geometry-kernel.wasm')
const liveBefore=hash(readFileSync(live))
mkdirSync(stage)
for(const name of ['src','scripts'])cpSync(resolve(root,name),resolve(stage,name),{recursive:true,dereference:true})
for(const name of ['package.json','tsconfig.json'])if(existsSync(resolve(root,name)))cpSync(resolve(root,name),resolve(stage,name))
symlinkSync(resolve(root,'node_modules'),resolve(stage,'node_modules'),'dir')
const publicDir=resolve(stage,'public/wasm'),generated=resolve(stage,'src/generated/geometry-kernels')
mkdirSync(publicDir,{recursive:true})
// An exact copied stamp may skip compression only after all outputs exist.
writeFileSync(resolve(publicDir,'geometry-kernel.wasm'),wasm)
packGeometryKernel(wasm,generated,publicDir)
verifyPackedWasmChunk(readFileSync(resolve(generated,'bytes.ts'),'utf8'),wasm,'staged expression worker package')
assert.equal(hash(readFileSync(live)),liveBefore,'Live WASM changed during staging')
const runtimeFiles={}
function inventory(directory){
 for(const entry of readdirSync(directory,{withFileTypes:true})){
  const path=resolve(directory,entry.name)
  assert(!entry.isSymbolicLink(),'Staged runtime must own its source files')
  if(entry.isDirectory())inventory(path)
  else if(entry.isFile())runtimeFiles[relative(stage,path)]=hash(readFileSync(path))
 }
}
for(const name of ['src','scripts'])inventory(resolve(stage,name))
for(const name of ['package.json','tsconfig.json'])if(existsSync(resolve(stage,name)))runtimeFiles[name]=hash(readFileSync(resolve(stage,name)))
const manifest=Buffer.from(JSON.stringify({schema:'expression-worker-runtime-inputs/1',files:runtimeFiles},null,2)+'\n')
writeFileSync(resolve(stage,'runtime-inputs.json'),manifest,{flag:'wx'})
const report={schema:'expression-worker-staging/1',stage,input:resolve(input),wasmSha256:hash(wasm),byteLength:wasm.length,
 liveWasmSha256:liveBefore,liveWasmUnchanged:true,workerQualified:false,
 runtimeManifestSha256:hash(manifest),runtimeFileCount:Object.keys(runtimeFiles).length,
 scope:'Disposable runtime/package copy only. Run checker from this directory with absolute saved request and native replay paths. No live package publication.'}
writeFileSync(resolve(stage,'staging.json'),JSON.stringify(report,null,2)+'\n',{flag:'wx'})
console.log(JSON.stringify(report,null,2))
