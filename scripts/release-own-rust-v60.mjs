#!/usr/bin/env node
// Dependency-only successor; retain the observed v59 binary and its provenance.
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync,existsSync} from 'node:fs'
import {resolve} from 'node:path'
const root=resolve(import.meta.dirname,'..'),read=p=>readFileSync(resolve(root,p))
const sha=b=>createHash('sha256').update(b).digest('hex')
const previousPath='docs/qualification/own-rust-cad-v59.json'
const output='docs/qualification/own-rust-cad-v60.json'
if(existsSync(resolve(root,output)))throw new Error('Historical OwnRust60 already exists; publish a new version')
const previous=JSON.parse(read(previousPath)),next=structuredClone(previous)
if(sha(read(previous.wasm.path))!==previous.wasm.sha256||sha(read(previous.wasm.publishedPath))!==previous.wasm.sha256)throw new Error('Retained binary differs from observed OwnRust59 artifact')
next.id='own-rust-cad-v60'
next.successorOf=previousPath
next.previousSha256=sha(read(previousPath))
next.recordedAt='2026-10-10'
next.purpose='Dependency-only refresh: remove unused GPU backend dependencies from CPU-only osv-math; retain backends in osv-math-compute. Bind the changed Cargo.lock without claiming a new WASM build, browser/STEP run or clean semantic qualification.'
next.dependencyFingerprints.rustLockfileSha256=sha(read('crates/Cargo.lock'))
const runtimePath='src/core/ownRustCadEvidence.ts'
const oldRuntime=read(runtimePath).toString()
if(!oldRuntime.includes(previous.dependencyFingerprints.rustLockfileSha256))throw new Error('Runtime predecessor identity differs')
writeFileSync(resolve(root,runtimePath),oldRuntime.replace(previous.dependencyFingerprints.rustLockfileSha256,next.dependencyFingerprints.rustLockfileSha256))
next.sourceBundle.paths=[...new Set([...previous.sourceBundle.paths,'crates/math-core/Cargo.toml','crates/Cargo.lock',runtimePath,'scripts/release-own-rust-v60.mjs'])].sort()
next.sourceBundle.sha256=sha(Buffer.concat(next.sourceBundle.paths.flatMap(p=>[Buffer.from(p+'\0'),read(p),Buffer.from('\n')])))
writeFileSync(resolve(root,output),JSON.stringify(next,null,2)+'\n',{flag:'wx'})
console.log('Published dependency-only OwnRust60; observed OwnRust59 WASM retained')
