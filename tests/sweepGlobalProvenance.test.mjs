import assert from 'node:assert/strict'
import {test} from 'node:test'
import {mkdtempSync,writeFileSync,readFileSync,existsSync,rmSync} from 'node:fs'
import {tmpdir} from 'node:os'
import {join,resolve} from 'node:path'
import {spawnSync} from 'node:child_process'
import {createHash} from 'node:crypto'

const sha=bytes=>createHash('sha256').update(bytes).digest('hex')
const script=resolve('scripts/verify-sweep-global-matrix.mjs')
// Control-plane fixtures model already externally checked, deliberately
// unproved native cases. They never assert a geometric or Solid certificate.
function fixture(directory){
 const artifactProvenance={schema:'sweep-step-artifact/1',geometryWasmSha256:'a'.repeat(64),geometryWasmByteLength:123,publicAndPackedVerified:true}
 const files=['open-spatial-hollow-miter.step','affine-oblique-hollow-miter.step']
 const cases=files.map(file=>{
  const bytes='control-plane fixture '+file;writeFileSync(join(directory,file),bytes)
  return {file,sha256:sha(bytes),faces:1,shells:1,capHoleFaces:0,nativeVolume:{solidGeometryCertified:false,boundaryEmbeddingCertified:false,nesting:null,orientations:[{outward:null}]}}
 })
 const manifest={artifactProvenance,cases}
 const oracle={passed:true,artifactProvenance:{...artifactProvenance},cases:cases.map(c=>({file:c.file,passed:true,stepSha256:c.sha256,faces:c.faces,shells:c.shells,cap_hole_faces:0,...Object.fromEntries(['valid','manifold_edges','opposite_edge_uses','shell_orientation','face_loop_ownership_preserved','full_domain_edge_distance_within_tolerance','full_domain_wall_distance_within_tolerance'].map(key=>[key,true]))}))}
 return {manifest,oracle}
}
for(const mutation of ['none','changed-manifest','changed-kernel','missing-provenance'])test(`global reconciliation refuses stale control-plane evidence: ${mutation}`,()=>{
 const directory=mkdtempSync(join(tmpdir(),'sweep-global-provenance-'))
 try{
  const {manifest,oracle}=fixture(directory)
  const original=JSON.stringify(manifest)
  oracle.manifestSha256=sha(original)
  if(mutation==='changed-manifest')manifest.recordedAt='changed after OCCT'
  if(mutation==='changed-kernel')oracle.artifactProvenance.geometryWasmSha256='b'.repeat(64)
  if(mutation==='missing-provenance'){delete manifest.artifactProvenance;delete oracle.artifactProvenance;oracle.manifestSha256=sha(JSON.stringify(manifest))}
  const manifestPath=join(directory,'manifest.json'),oraclePath=join(directory,'oracle.json'),output=join(directory,'result.json')
  writeFileSync(manifestPath,JSON.stringify(manifest));writeFileSync(oraclePath,JSON.stringify(oracle))
  const result=spawnSync(process.execPath,[script,manifestPath,oraclePath,output,'--control-plane-only'],{encoding:'utf8'})
  if(mutation==='none'){
   assert.equal(result.status,0,result.stderr)
   const report=JSON.parse(readFileSync(output,'utf8'))
   assert.equal(report.schema,'sweep-global-control-plane/1');assert.equal(report.controlPlaneOnly,true)
   assert.equal(report.certified,0);assert.equal(report.refused,2)
   assert.equal(report.manifestSha256,oracle.manifestSha256)
   assert.deepEqual(report.artifactProvenance,manifest.artifactProvenance)
  }else{assert.notEqual(result.status,0);assert.equal(existsSync(output),false)}
 }finally{rmSync(directory,{recursive:true,force:true})}
})

for(const mutation of ['missing-companion','unresolved-gap','outside-cap'])test(`paired cap refusal requires exact negative and positive companion: ${mutation}`,()=>{
 const directory=mkdtempSync(join(tmpdir(),'sweep-cap-pair-control-'))
 try{
  const {manifest,oracle}=fixture(directory)
  const file='rmf-oblique-linear-scale-hollow.step',bytes='synthetic refusal routing only'
  writeFileSync(join(directory,file),bytes)
  const item={...structuredClone(manifest.cases[0]),file,sha256:sha(bytes),requireNativeSolid:false,capFaces:[0],
   nativeVolume:structuredClone(manifest.cases[0].nativeVolume),nativeBoundary:{exactBoundaryCertified:false},
   nativeExactUses:[{face:mutation==='outside-cap'?1:0,status:mutation==='unresolved-gap'?'unresolved':'different',
    exactIdentityCertified:false,globalEmbeddingCertified:false,work:1,independent:null}]}
  manifest.cases.push(item)
  oracle.cases.push({...structuredClone(oracle.cases[0]),file,stepSha256:item.sha256})
  oracle.manifestSha256=sha(JSON.stringify(manifest))
  const manifestPath=join(directory,'manifest.json'),oraclePath=join(directory,'oracle.json'),output=join(directory,'result.json')
  writeFileSync(manifestPath,JSON.stringify(manifest));writeFileSync(oraclePath,JSON.stringify(oracle))
  const result=spawnSync(process.execPath,[script,manifestPath,oraclePath,output,'--control-plane-only'],{encoding:'utf8'})
  assert.notEqual(result.status,0)
  assert.equal(existsSync(output),false)
  assert.match(result.stderr,mutation==='missing-companion'?/Missing corrected cap companion/:/Cap negative is unresolved or outside cap scope/)
 }finally{rmSync(directory,{recursive:true,force:true})}
})

for(const mutation of ['unbound-model','missing-boundary','different-boundary'])test(`owned replay boundary is bound to actual volume audit: ${mutation}`,()=>{
 const directory=mkdtempSync(join(tmpdir(),'sweep-owned-boundary-control-'))
 try{
  const {manifest,oracle}=fixture(directory),item=manifest.cases[0]
  item.replayRouting={source:{}};item.nativeReplayFinalModelBound=mutation!=='unbound-model'
  item.genericNativeBoundary={boundaryEmbeddingCertified:false}
  item.nativeBoundary={boundaryEmbeddingCertified:false}
  if(mutation!=='missing-boundary')item.nativeVolume.boundary={boundaryEmbeddingCertified:mutation==='different-boundary'}
  oracle.manifestSha256=sha(JSON.stringify(manifest))
  const manifestPath=join(directory,'manifest.json'),oraclePath=join(directory,'oracle.json'),output=join(directory,'result.json')
  writeFileSync(manifestPath,JSON.stringify(manifest));writeFileSync(oraclePath,JSON.stringify(oracle))
  const result=spawnSync(process.execPath,[script,manifestPath,oraclePath,output,'--control-plane-only'],{encoding:'utf8'})
  assert.notEqual(result.status,0);assert.equal(existsSync(output),false)
  assert.match(result.stderr,mutation==='unbound-model'?/Replay final model is not bound/:mutation==='missing-boundary'?/Fresh replay boundary report missing/:/Replay boundary differs from actual volume audit/)
 }finally{rmSync(directory,{recursive:true,force:true})}
})

for(const omitted of ['open-spatial-hollow-miter.step','rmf-oblique-linear-scale-hollow-corrected.step'])test(`complete qualification rejects omitted mandatory control: ${omitted}`,()=>{
 const directory=mkdtempSync(join(tmpdir(),'sweep-required-controls-'))
 try{
  const {manifest,oracle}=fixture(directory)
  for(const file of ['rmf-oblique-linear-scale-hollow.step','rmf-oblique-linear-scale-hollow-corrected.step','frenet-quadratic-hollow.step','frenet-quadratic-hollow-corrected.step','rmf-planar-quadratic-hollow.step','rmf-quadratic-hollow-corrected.step']){
   manifest.cases.push({...structuredClone(manifest.cases[0]),file})
   oracle.cases.push({...structuredClone(oracle.cases[0]),file})
  }
  manifest.cases=manifest.cases.filter(c=>c.file!==omitted)
  oracle.cases=oracle.cases.filter(c=>c.file!==omitted)
  oracle.manifestSha256=sha(JSON.stringify(manifest))
  const manifestPath=join(directory,'manifest.json'),oraclePath=join(directory,'oracle.json'),output=join(directory,'result.json')
  writeFileSync(manifestPath,JSON.stringify(manifest));writeFileSync(oraclePath,JSON.stringify(oracle))
  const result=spawnSync(process.execPath,[script,manifestPath,oraclePath,output],{encoding:'utf8'})
  assert.notEqual(result.status,0);assert.equal(existsSync(output),false)
  assert.ok(result.stderr.includes(`Missing mandatory native control: ${omitted}`),result.stderr)
 }finally{rmSync(directory,{recursive:true,force:true})}
})
