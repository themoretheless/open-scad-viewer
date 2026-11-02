import {test} from 'node:test'
import assert from 'node:assert/strict'
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,existsSync,rmSync} from 'node:fs'
import {tmpdir} from 'node:os'
import {resolve,dirname} from 'node:path'
import {execFileSync} from 'node:child_process'
import {createHash} from 'node:crypto'
import {snapshot,recordCandidate} from '../scripts/record-geometry-build-candidate.mjs'
function setup(){
 const root=mkdtempSync(resolve(tmpdir(),'geometry-candidate-'));const put=(p,b)=>{mkdirSync(dirname(resolve(root,p)),{recursive:true});writeFileSync(resolve(root,p),b)}
 execFileSync('git',['init','--quiet'],{cwd:root});put('crates/Cargo.toml','[workspace]\n');put('rust-toolchain.toml','[toolchain]\nchannel="test"\n');execFileSync('git',['add','.'],{cwd:root});execFileSync('git',['-c','user.name=Fixture','-c','user.email=fixture@example.invalid','commit','--quiet','-m','Fixture'],{cwd:root})
 const wasm=Buffer.from([0,97,115,109,1,0,0,0]);put('public/wasm/geometry-kernel.wasm',wasm);put('src/generated/geometry-kernels/kernel_bg.wasm',wasm);put('src/generated/geometry-kernels/identity.ts','// Generated\nexport default Object.freeze('+JSON.stringify({sha256:createHash('sha256').update(wasm).digest('hex'),byteLength:8})+')\n');put('src/core/currentRuntimeGeometryEvidence.ts','frozen runtime policy\n')
 return {root,put,wasm,close:()=>rmSync(root,{recursive:true,force:true})}
}
test('captures exact candidate bytes and stable authored source without changing admission policy',()=>{
 const f=setup();try{const before=snapshot(f.root);const output=resolve(f.root,'capture');const r=recordCandidate(f.root,output,before,'fixture compiler');assert.equal(r.qualificationClaim,'none');assert.equal(r.sourceFiles,2);assert.equal(r.sourceSha256,before.sha256);assert.deepEqual(readFileSync(resolve(output,'geometry-kernel.wasm')),f.wasm);assert.equal(readFileSync(resolve(f.root,'src/core/currentRuntimeGeometryEvidence.ts'),'utf8'),'frozen runtime policy\n');assert.throws(()=>recordCandidate(f.root,output,before,'fixture compiler'));}finally{f.close()}
})
test('refuses a source change across compilation before creating any output',()=>{
 const f=setup();try{const before=snapshot(f.root);f.put('crates/Cargo.toml','changed\n');const out=resolve(f.root,'capture');assert.throws(()=>recordCandidate(f.root,out,before,'fixture compiler'),/inputs changed/);assert(!existsSync(out));}finally{f.close()}
})
test('refuses stale identity and unequal generated/public artifacts',()=>{
 for(const mode of ['identity','bytes']){const f=setup();try{const before=snapshot(f.root);if(mode==='identity')f.put('src/generated/geometry-kernels/identity.ts','export default Object.freeze({"sha256":"bad","byteLength":8})\n');else f.put('src/generated/geometry-kernels/kernel_bg.wasm',Buffer.from([1,2]));const out=resolve(f.root,'capture');assert.throws(()=>recordCandidate(f.root,out,before,'fixture compiler'));assert(!existsSync(out));}finally{f.close()}}
})
test('includes untracked authored Rust inputs instead of silently binding an older tree',()=>{
 const f=setup();try{const before=snapshot(f.root);f.put('crates/new.rs','pub fn new_source() {}\n');const after=snapshot(f.root);assert.equal(after.files.length,before.files.length+1);assert.notEqual(after.sha256,before.sha256);assert.throws(()=>recordCandidate(f.root,resolve(f.root,'capture'),before,'fixture compiler'),/inputs changed/);}finally{f.close()}
})
