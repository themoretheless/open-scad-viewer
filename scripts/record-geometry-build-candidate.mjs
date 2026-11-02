// Capture actual build inputs/bytes for review; never grants runtime admission.
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {execFileSync} from 'node:child_process'
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs'
import {resolve,dirname} from 'node:path'
import {pathToFileURL} from 'node:url'
const sha=bytes=>createHash('sha256').update(bytes).digest('hex')
const inputPaths=['crates','rust-toolchain.toml','package-lock.json','scripts/build-geometry-kernels.mjs','scripts/build-wasm-brotli.mjs','scripts/wasm-optimize.mjs','scripts/reproducible-cargo.mjs','scripts/pack-geometry-kernel.mjs']
export function snapshot(root){
 const paths=[...new Set(execFileSync('git',['ls-files','--cached','--others','--exclude-standard','-z','--',...inputPaths],{cwd:root,encoding:'utf8'}).split('\0').filter(Boolean))].sort()
 assert(paths.length,'No authored build inputs found')
 const h=createHash('sha256');const files=paths.map(path=>{const bytes=readFileSync(resolve(root,path));h.update(path+'\0').update(bytes).update('\n');return {path,sha256:sha(bytes),byteLength:bytes.length}})
 return {schema:'geometry-source-inputs/1',revision:execFileSync('git',['rev-parse','HEAD'],{cwd:root,encoding:'utf8'}).trim(),canonicalization:'UTF-8 sorted path + NUL + exact bytes + LF',sha256:h.digest('hex'),files}
}
export function recordCandidate(root,output,before,compiler){
 const after=snapshot(root);assert.equal(after.revision,before.revision,'Revision changed during source build');assert.deepEqual(after,before,'Authored build inputs changed during source build')
 const wasm=readFileSync(resolve(root,'public/wasm/geometry-kernel.wasm'))
 assert.deepEqual(readFileSync(resolve(root,'src/generated/geometry-kernels/kernel_bg.wasm')),wasm,'Generated/public candidate bytes differ')
 const text=readFileSync(resolve(root,'src/generated/geometry-kernels/identity.ts'),'utf8')
 const match=text.match(/^export default Object\.freeze\((\{[^\n]+\})\)\s*$/m);assert(match,'Unrecognized generated identity')
 const identity={sha256:sha(wasm),byteLength:wasm.length};assert.deepEqual(JSON.parse(match[1]),identity,'Candidate identity differs from actual bytes')
 const module=new WebAssembly.Module(wasm);assert.deepEqual(WebAssembly.Module.imports(module),[],'Candidate imports external functions')
 const receipt={schema:'geometry-build-candidate/1',identity,sourceRevision:after.revision,sourceSha256:after.sha256,sourceFiles:after.files.length,compiler,scope:'Observed authored inputs stable across a completed source build and actual packaged bytes. Not a formal compilation attestation, runtime admission, cross-host equivalence, complete frontend or global CAD qualification.',qualificationClaim:'none',platform:process.platform,arch:process.arch,node:process.version,exports:WebAssembly.Module.exports(module)}
 mkdirSync(dirname(output),{recursive:true});mkdirSync(output)
 writeFileSync(resolve(output,'source-before.json'),JSON.stringify(before,null,2)+'\n',{flag:'wx'})
 writeFileSync(resolve(output,'source-after.json'),JSON.stringify(after,null,2)+'\n',{flag:'wx'})
 writeFileSync(resolve(output,'geometry-kernel.wasm'),wasm,{flag:'wx'})
 writeFileSync(resolve(output,'identity.ts'),text,{flag:'wx'})
 writeFileSync(resolve(output,'receipt.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'})
 return receipt
}
if(process.argv[1]&&import.meta.url===pathToFileURL(resolve(process.argv[1])).href){
 const [mode,output,beforePath]=process.argv.slice(2),root=process.cwd();assert(output,'Output path required')
 if(mode==='before'){
  const value=snapshot(root);mkdirSync(dirname(resolve(output)),{recursive:true});writeFileSync(output,JSON.stringify(value,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify({revision:value.revision,sourceSha256:value.sha256,sourceFiles:value.files.length}))
 }else{
  assert.equal(mode,'after');assert(beforePath,'Before-source manifest required');const before=JSON.parse(readFileSync(beforePath,'utf8'))
  const result=recordCandidate(root,resolve(output),before,execFileSync('rustc',['--version','--verbose'],{encoding:'utf8'}).trim());console.log(JSON.stringify(result))
 }
}
