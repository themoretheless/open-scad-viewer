import assert from 'node:assert/strict'
import {readFileSync,writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {dirname,resolve} from 'node:path'

// Independent reconciliation of native global certificates with OCCT results.
// Finite fixture coverage never promotes an unresolved body or a general theorem.
const [manifestPath,oraclePath,outputPath]=process.argv.slice(2)
assert.ok(manifestPath&&oraclePath&&outputPath,'Usage: manifest.json occt.json output.json')
const manifest=JSON.parse(readFileSync(manifestPath,'utf8'))
const oracle=JSON.parse(readFileSync(oraclePath,'utf8'))
assert.equal(oracle.passed,true)
const expectedRefusals=new Set(['open-spatial-hollow-miter.step','affine-oblique-hollow-miter.step'])
const external=new Map(oracle.cases.map(c=>[c.file,c]))
assert.equal(external.size,oracle.cases.length,'Duplicate external fixture')
assert.equal(manifest.cases.length,external.size,'Incomplete external coverage')
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
 const refused=expectedRefusals.has(item.file)
 assert.equal(native.solidGeometryCertified,!refused,`${item.file}: unexpected native admission`)
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
  if(item.capFaces.length){
   const boundary=item.nativeBoundary;assert.ok(boundary)
   for(const key of ['boundaryEmbeddingCertified','exactBoundaryCertified','trimCertified','allFacesInjective','allPairsClassified'])assert.equal(boundary[key],true,`${item.file}: boundary ${key}`)
   assert.deepEqual(boundary.unresolvedFaces,[])
   assert.equal(boundary.caps.length,item.capFaces.length,'Incomplete cap certificate coverage')
   assert.deepEqual(boundary.caps.map(cap=>cap.face).sort((a,b)=>a-b),[...item.capFaces].sort((a,b)=>a-b))
   assert.ok(boundary.caps.every(cap=>cap.capCertified===true&&cap.unresolvedWalls.length===0))
  }
 }
 cases.push({file:item.file,stepSha256:hash,status:refused?'correctly-unproved':'certified',shells:item.shells,capHoleFaces:item.capHoleFaces,wallRegularityCertified:item.nativeRetainedWallCharts?.allChartsCertified??null,profileRegularityCertified:item.construction?.profileRegularityCertified??null,parents:native.nesting?.parents??null,orientations:native.orientations})
}
assert.ok([...expectedRefusals].every(file=>seen.has(file)),'Missing deliberate refusal')
const report={schema:'sweep-global-matrix/1',scope:'Declared finite STEP fixtures; native global certificates reconciled with independent OCCT topology/material evidence. Unresolved cases remain refused; no universal or whole-boundary smoothness claim.',passed:true,certified:cases.filter(c=>c.status==='certified').length,refused:cases.filter(c=>c.status==='correctly-unproved').length,cases}
writeFileSync(outputPath,JSON.stringify(report,null,2)+'\n')
console.log(`${report.certified} certified, ${report.refused} correctly unproved`)
