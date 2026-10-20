// Timing and receipt transport only. Geometry and all admission predicates run in Rust WASM.
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {pathToFileURL} from 'node:url'
const [root,sourceName,output]=process.argv.slice(2)
const sha=(v:string|Uint8Array)=>createHash('sha256').update(v).digest('hex')
const module=(name:string)=>import(pathToFileURL(resolve(root,'src/services',name+'.ts')).href)
const {compileRushFrontend}=await module('rushFrontend')
const {buildOwnNurbsAsync}=await module('rushGraphNurbsKernel')
const {readNativeGeometryData}=await module('nativeGeometrySnapshot')
const {inspectProgressiveSweepSolidAdmission}=await module('sweepSolidAdmission')
const source=readFileSync(resolve(root,'examples/rush',sourceName),'utf8')
const document=compileRushFrontend(source).document,before=JSON.stringify(document)
const start=performance.now()
const built=await buildOwnNurbsAsync(document,{action:'build',display:{segments:2,subdivisionLevels:0}})
const buildMs=performance.now()-start
assert(built.nativeGeometry)
const data=readNativeGeometryData(built.nativeGeometry)
const admission=inspectProgressiveSweepSolidAdmission(built.nativeGeometry,data.geometry)
assert(admission.solidGeometryCertified&&admission.allFacesInjective&&admission.allPairsClassified)
assert.equal(JSON.stringify(document),before)
writeFileSync(output,JSON.stringify({passed:true,sourceName,sourceSha256:sha(source),
 wasmSha256:sha(readFileSync(resolve(root,'public/wasm/geometry-kernel.wasm'))),nodeVersion:process.version,
 buildMs,modelSha256:sha(JSON.stringify(data.geometry)),snapshotSha256:sha(JSON.stringify(data)),
 constructionSha256:sha(JSON.stringify(built.report.construction)),admissionSha256:sha(JSON.stringify(admission)),
 scope:'One unchanged original source; fresh-process paired performance diagnostic, not the deadline or full qualification gate.'},null,2)+'\n')
