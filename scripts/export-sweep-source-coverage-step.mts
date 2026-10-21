/** Exact-original-source STEP supplement. All geometry and audits call Rust. */
import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {readFileSync,writeFileSync,mkdirSync,existsSync,appendFileSync,renameSync} from 'node:fs'
import {resolve,join} from 'node:path'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbsAsync} from '../src/services/rushGraphNurbsKernel'
import {readNativeGeometryData} from '../src/services/nativeGeometrySnapshot'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'
import {readSweepBodyBoundaryViewportEvidence,readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
import {exportDirectStepV5} from '../src/services/cadNurbsStep'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {evaluateNurbsCurve,reverseNurbsCurve} from '../src/services/nurbsCurve'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'

const [inventoryPath,destination]=process.argv.slice(2)
assert(inventoryPath&&destination,'Usage: export-sweep-source-coverage-step.mts INVENTORY.json NEW_OUTPUT_DIRECTORY')
const sha=(data:string|Buffer)=>createHash('sha256').update(data).digest('hex')
const inventory=JSON.parse(readFileSync(inventoryPath,'utf8'))
const out=resolve(destination)
assert(!existsSync(out),'Preserve all earlier runs; use a new output directory')
mkdirSync(out,{recursive:true})
const provenance=sweepStepArtifactProvenance()
assert.equal(provenance.geometryWasmSha256,inventory.geometryWasmSha256)
for(const [file,digest]of Object.entries(inventory.inputsSha256))assert.equal(sha(readFileSync(file)),digest,file)
const selected=inventory.rows.filter((row:any)=>row.positiveSolid&&row.nativeStepWitnesses.length===0)
assert.equal(selected.length,37,'The captured uncovered-original-source inventory must not silently change')
const records:any[]=[]
function manifest(name:string) {
 writeFileSync(join(out,name),JSON.stringify({schema:'sweep-external-step/2',units:'mm',artifactProvenance:provenance,
  scope:'Fresh original Rush sources and exact previously qualified wide/narrow UI model hashes; no changed geometry or budgets.',
  inventorySha256:sha(readFileSync(inventoryPath)),cases:records})+'\n')
}
for(const [index,row]of selected.entries()) {
 console.log(`Original-source STEP ${index+1}/${selected.length}: ${row.file}`)
 const source=readFileSync(row.sourcePath,'utf8');assert.equal(sha(source),row.sourceSha256)
 const graph=compileRushFrontend(source).document
 const mesh=await buildOwnNurbsAsync(graph,{action:'build',display:{segments:2,subdivisionLevels:0}})
 assert(mesh.nativeGeometry,'Missing original-source BRep')
 const data=readNativeGeometryData(mesh.nativeGeometry),model=data.geometry
 const modelSha=sha(JSON.stringify(model))
 for(const name of row.groups) {
  const parent=Object.keys(inventory.inputsSha256).find((path:string)=>path.endsWith(`/ui478/${name}/matrix.json`))
  assert(parent,'Missing bound UI matrix')
  const matrix=JSON.parse(readFileSync(parent,'utf8'))
  for(const c of matrix.cases.filter((c:any)=>c.file===row.file))assert.equal(modelSha,c.solid.geometrySha256,'Original UI model differs')
 }
 const volume=inspectProgressiveSweepSolidAdmission(mesh.nativeGeometry,model)
 assert(volume?.solidGeometryCertified&&volume.boundaryEmbeddingCertified&&volume.allFacesInjective&&volume.allPairsClassified)
 assert(volume.boundary&&volume.nextPair===null)
 const caps=volume.boundary.caps.map((cap:any)=>cap.face)
 assert.equal(new Set(caps).size,caps.length)
 const walls=model.faces.filter((_:any,id:number)=>!caps.includes(id))
 const reversed=(face:any)=>{
  const id=model.faces.indexOf(face)
  const uses=model.shells.flatMap((shell:any)=>shell.faces.filter((use:any)=>use.face===id))
  assert.equal(uses.length,1,'Missing unique face ownership')
  return uses[0].reversed
 }
 const outwardCapContours=caps.map((id:number)=>{
  const face=model.faces[id],reverse=reversed(face)
  return [face.outer,...face.holes].flatMap((wire:number)=>
   (reverse?[...model.loops[wire].coedges].reverse():model.loops[wire].coedges).map((use:any)=>{
    const curve=model.edges[use.edge].curve
    return use.reversed!==reverse?reverseNurbsCurve(curve):curve
   }))
 })
 const file=row.file.replace(/\.r$/,'.step')
 const step=exportDirectStepV5(model).text
 writeFileSync(join(out,file),step)
 records.push({file,sourceFile:row.sourcePath,sourceSha256:row.sourceSha256,uiModelSha256:modelSha,
  faces:model.faces.length,shells:model.shells.length,solids:model.bodies.length,
  capFaces:caps,capHoleFaces:model.faces.filter((face:any)=>face.holes.length>0).length,
  requireNativeSolid:true,nativeVolume:volume,nativeBoundary:volume.boundary,
  boundaryError:readSweepBodyBoundaryViewportEvidence(mesh.nativeGeometry)??readSweepViewportEvidence(mesh.nativeGeometry),
  construction:data.sweepBodyBoundaryEvidence??data.sweepStationReconstruction??null,
  edges:model.edges.length,faceLoops:model.faces.map((face:any)=>({
   outer:model.loops[face.outer].coedges.map((c:any)=>c.edge),
   holes:face.holes.map((loop:number)=>model.loops[loop].coedges.map((c:any)=>c.edge))})),
  edgeCurves:model.edges.map((edge:any)=>({curve:edge.curve,
   samples:[0,.25,.5,.75,1].map(u=>({u,point:evaluateNurbsCurve(edge.curve,u).point}))})),
  wallCoedges:walls.map((face:any)=>model.loops[face.outer].coedges.map((c:any)=>({edge:c.edge,reversed:c.reversed,pcurve:c.pcurve}))),
  wallSurfaces:walls.map((face:any)=>face.surface),wallFaceReversed:walls.map(reversed),
  wallSamples:walls.map((face:any)=>[0,.17,.5,.83,1].flatMap(u=>[0,.13,.5,.87,1].map(v=>{
   const e=evaluateNurbsSurface(face.surface,u,v);return {u,v,point:e.point,du:e.du,dv:e.dv}
  }))),outwardCapContours,expectedVolume:0,volumeReferenceMethod:'rational-boundary-gauss-reference',
  surfaceToleranceMm:1e-8,relativeVolumeTolerance:1e-7,sha256:sha(step)})
 manifest('partial-manifest.json')
}
for(const [file,digest]of Object.entries(inventory.inputsSha256))assert.equal(sha(readFileSync(file)),digest,file)
manifest('manifest.json.writing');renameSync(join(out,'manifest.json.writing'),join(out,'manifest.json'))
console.log(`All ${records.length} original-source STEP exports bound to both previously qualified UI models.`)
