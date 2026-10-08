import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {dirname,resolve} from 'node:path'

// Independent reconciliation of native global certificates with OCCT results.
// Finite fixture coverage never promotes an unresolved body or a general theorem.
const [manifestPath,oraclePath,outputPath]=process.argv.slice(2)
assert.ok(manifestPath&&oraclePath&&outputPath,'Usage: manifest.json occt.json output.json')
const manifestBytes=readFileSync(manifestPath)
const manifestSha256=createHash('sha256').update(manifestBytes).digest('hex')
const manifest=JSON.parse(manifestBytes.toString('utf8'))
const oracle=JSON.parse(readFileSync(oraclePath,'utf8'))
assert.equal(oracle.passed,true)
assert.equal(oracle.manifestSha256,manifestSha256,'OCCT report belongs to another manifest')
const artifact=manifest.artifactProvenance
assert.equal(artifact?.schema,'sweep-step-artifact/1','Missing captured WASM provenance')
assert.match(artifact.geometryWasmSha256,/^[a-f0-9]{64}$/)
assert.ok(Number.isSafeInteger(artifact.geometryWasmByteLength)&&artifact.geometryWasmByteLength>0)
assert.equal(artifact.publicAndPackedVerified,true)
assert.deepEqual(oracle.artifactProvenance,artifact,'OCCT/native WASM provenance differs')
const expectedRefusals=new Set(['open-spatial-hollow-miter.step','affine-oblique-hollow-miter.step'])
// These raw exact-boundary negatives must carry actual Different identities
// and a separately fully certified corrected companion, never an unresolved retry.
const pairedCapGaps=new Map([
 ['rmf-oblique-linear-scale-hollow.step','rmf-oblique-linear-scale-hollow-corrected.step'],
 ['frenet-quadratic-hollow.step','frenet-quadratic-hollow-corrected.step'],
 ['rmf-planar-quadratic-hollow.step','rmf-quadratic-hollow-corrected.step'],
])
const external=new Map(oracle.cases.map(c=>[c.file,c]))
assert.equal(external.size,oracle.cases.length,'Duplicate external fixture')
assert.equal(manifest.cases.length,external.size,'Incomplete external coverage')
// Full qualification must retain the negative controls and their corrected
// companions; an entirely omitted refusal cannot count as verified coverage.
const requiredControls=[...expectedRefusals,...pairedCapGaps.keys(),...pairedCapGaps.values()]
const capturedFiles=new Set(manifest.cases.map(item=>item.file))
const controlPlaneOnly=process.argv.includes('--control-plane-only')
if(controlPlaneOnly)assert.ok(manifest.cases.every(item=>item.nativeVolume?.solidGeometryCertified===false),
 'Partial control-plane verification cannot admit a Solid')
for(const file of controlPlaneOnly?[]:requiredControls){
 assert.ok(capturedFiles.has(file),`Missing mandatory native control: ${file}`)
 assert.ok(external.has(file),`Missing mandatory external control: ${file}`)
}
const seen=new Set(),cases=[]
for(const item of manifest.cases){
 assert.ok(!seen.has(item.file),'Duplicate source fixture');seen.add(item.file)
 const imported=external.get(item.file);assert.ok(imported,'Missing external fixture')
 const hash=createHash('sha256').update(readFileSync(resolve(dirname(manifestPath),item.file))).digest('hex')
 assert.equal(hash,item.sha256,'STEP source hash changed')
 assert.equal(imported.stepSha256,hash,'External oracle was not run against this STEP artifact')
 assert.equal(imported.passed,true)
 assert.equal(imported.faces,item.faces);assert.equal(imported.shells,item.shells)
 assert.equal(imported.cap_hole_faces,item.capHoleFaces)
 for(const key of ['valid','manifold_edges','opposite_edge_uses','shell_orientation','face_loop_ownership_preserved','full_domain_edge_distance_within_tolerance','full_domain_wall_distance_within_tolerance'])assert.equal(imported[key],true,`${item.file}: external ${key}`)
 const native=item.nativeVolume;assert.ok(native)
 if(item.replayRouting){
  assert.equal(item.nativeReplayFinalModelBound,true,'Replay final model is not bound')
  assert.ok(native.boundary,'Fresh replay boundary report missing')
  assert.ok(item.genericNativeBoundary,'Generic boundary diagnostic missing')
  assert.deepEqual(item.nativeBoundary,native.boundary,'Replay boundary differs from actual volume audit')
 }

 const companionFile=pairedCapGaps.get(item.file)
 const refused=expectedRefusals.has(item.file)||companionFile!==undefined
 assert.equal(native.solidGeometryCertified,!refused,`${item.file}: unexpected native admission`)
 if(companionFile){
  assert.equal(item.requireNativeSolid,false,'Raw exact-cap negative cannot be a required Solid')
  assert.equal(item.nativeBoundary?.exactBoundaryCertified,false,'Exact cap gap was promoted')
  const uses=item.nativeExactUses;assert.ok(Array.isArray(uses)&&uses.length>0,'Missing exact coedge evidence')
  const gaps=uses.filter(use=>use.status!=='equal');assert.ok(gaps.length>0,'Missing proven cap gap')
  assert.ok(gaps.every(use=>use.status==='different'&&use.exactIdentityCertified===false&&
   use.globalEmbeddingCertified===false&&item.capFaces.includes(use.face)&&use.independent===null&&
   Number.isSafeInteger(use.work)&&use.work>0&&use.work<=1000000),'Cap negative is unresolved or outside cap scope')
  const companion=manifest.cases.find(c=>c.file===companionFile);assert.ok(companion,'Missing corrected cap companion')
  assert.equal(companion.requireNativeSolid,true)
  assert.equal(companion.nativeVolume?.solidGeometryCertified,true,'Corrected companion is unproved')
  assert.equal(companion.faces,item.faces);assert.equal(companion.shells,item.shells);assert.equal(companion.capHoleFaces,item.capHoleFaces)
  assert.ok(companion.nativeExactUses?.length>0&&companion.nativeExactUses.every(use=>use.status==='equal'&&use.exactIdentityCertified===true),'Corrected coedges lack exact identity')
 }
 if(refused){
  assert.equal(native.boundaryEmbeddingCertified,false)
  assert.equal(native.nesting,null)
  assert.ok(native.orientations.every(o=>o.outward===null),'Unproved orientation promoted')
 }else{
  for(const key of ['allFacesInjective','allPairsClassified','boundaryEmbeddingCertified'])assert.equal(native[key],true,`${item.file}: native ${key}`)
  assert.equal(native.nextPair,null)
  const charts=item.nativeRetainedWallCharts;assert.equal(charts?.allChartsCertified,true,'Unproved retained wall charts')
  assert.deepEqual(charts.unresolvedFaces,[])
  assert.equal(charts.charts.length,item.faces-item.capFaces.length)
  assert.deepEqual(charts.charts.map(chart=>chart.face).sort((a,b)=>a-b),Array.from({length:item.faces},(_,face)=>face).filter(face=>!item.capFaces.includes(face)))
  assert.ok(Number.isSafeInteger(charts.cells)&&charts.cells>=0&&charts.cells<=100000,'Retained chart budget exceeded')
  assert.equal(charts.cells,charts.charts.reduce((sum,chart)=>sum+chart.audit.cells,0))
  assert.ok(charts.charts.every(chart=>chart.audit.certified===true&&!item.capFaces.includes(chart.face)))
  if(item.construction?.wallRegularityCertified!==undefined)assert.equal(item.construction.wallRegularityCertified,true,'Unproved retained-wall regularity')
  if(item.construction?.profileRegularityCertified!==undefined)assert.equal(item.construction.profileRegularityCertified,true,'Unproved profile regularity')
  const nesting=native.nesting;assert.equal(nesting?.rolesConsistent,true)
  assert.equal(nesting.parents.length,item.shells)
  assert.equal(nesting.visitedPairs,nesting.totalPairs)
  assert.ok(nesting.pairs.every(pair=>pair.boundarySeparationCertified===true))
  assert.equal(native.orientations.length,item.shells)
  assert.equal(new Set(native.orientations.map(o=>o.shell)).size,item.shells)
  for(const orientation of native.orientations){
   assert.ok(Number.isInteger(orientation.shell)&&orientation.shell>=0&&orientation.shell<item.shells)
   let depth=0,parent=nesting.parents[orientation.shell],ancestors=new Set([orientation.shell])
   while(parent!==null){
    assert.ok(Number.isInteger(parent)&&parent>=0&&parent<item.shells&&!ancestors.has(parent),'Invalid nesting forest')
    ancestors.add(parent);depth++;parent=nesting.parents[parent]
   }
   assert.equal(orientation.expectedOutward,depth%2===0)
   assert.equal(orientation.outward,orientation.expectedOutward)
  }
  assert.equal(imported.native_external_material_agreement,true)
  {
   const boundary=item.nativeBoundary;assert.ok(boundary,`${item.file}: missing complete native boundary report`)
   for(const key of ['boundaryEmbeddingCertified','exactBoundaryCertified','trimCertified','allFacesInjective','allPairsClassified'])assert.equal(boundary[key],true,`${item.file}: boundary ${key}`)
   assert.deepEqual(boundary.unresolvedFaces,[])
   assert.equal(boundary.caps.length,item.capFaces.length,'Incomplete cap certificate coverage')
   assert.deepEqual(boundary.caps.map(cap=>cap.face).sort((a,b)=>a-b),[...item.capFaces].sort((a,b)=>a-b))
   assert.ok(boundary.caps.every(cap=>cap.capCertified===true&&cap.unresolvedWalls.length===0))
  }
 }
 cases.push({file:item.file,stepSha256:hash,status:companionFile?'correctly-refused-exact-cap-gap':refused?'correctly-unproved':'certified',shells:item.shells,capHoleFaces:item.capHoleFaces,wallRegularityCertified:item.nativeRetainedWallCharts?.allChartsCertified??null,profileRegularityCertified:item.construction?.profileRegularityCertified??null,parents:native.nesting?.parents??null,orientations:native.orientations})
}
assert.ok([...expectedRefusals].every(file=>seen.has(file)),'Missing deliberate refusal')
const report={schema:controlPlaneOnly?'sweep-global-control-plane/1':'sweep-global-matrix/1',manifestSha256,artifactProvenance:artifact,oracleSha256:createHash('sha256').update(readFileSync(oraclePath)).digest('hex'),controlPlaneOnly,scope:controlPlaneOnly?'Synthetic refused control-plane evidence only; not a complete geometric qualification.':'Declared finite STEP fixtures; native global certificates reconciled with independent OCCT topology/material evidence. Unresolved cases remain refused; no universal or whole-boundary smoothness claim.',passed:true,certified:cases.filter(c=>c.status==='certified').length,refused:cases.filter(c=>c.status!=='certified').length,cases}
writeFileSync(outputPath,JSON.stringify(report,null,2)+'\n')
console.log(`${report.certified} certified, ${report.refused} correctly unproved`)
