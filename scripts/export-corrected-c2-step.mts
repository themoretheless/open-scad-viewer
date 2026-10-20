import assert from 'node:assert/strict'
import {mkdirSync,writeFileSync,readFileSync,appendFileSync,renameSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {correctedC2BodySources} from './corrected-c2-sources.mjs'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
import {sweepStepCapLayout} from './sweep-step-cap-layout.mjs'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
import {readNativeGeometryData} from '../src/services/nativeGeometrySnapshot'
import {readSweepBodyBoundaryViewportEvidence} from '../src/services/sweepViewportEvidence'
import {callGeometryRust} from '../src/services/geometry/kernel'
import {exportDirectStepV5} from '../src/services/cadNurbsStep'
import {evaluateNurbsCurve,reverseNurbsCurve} from '../src/services/nurbsCurve'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {inspectSweepVolume,inspectSweepEmbedding,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'
import {inspectMiterProfileSmoothness} from '../src/services/miterProfileSmoothness'
import {inspectSweepRetainedWallCharts} from '../src/services/nurbsSweepRetainedCharts'
import {inspectSweepCoedgeExact} from '../src/services/nurbsSweepAudit'
import type {NurbsBrep} from '../src/services/geometry/brep'

// Supplemental QA: all source construction and certificates remain Rust-owned.
// Keep the canonical 207-case matrix intact. The retained STEP inspection below
// uses its unchanged manifest fields and independent OCCT/reference contracts.
const root=resolve(process.argv[2]);mkdirSync(root,{recursive:true})
const artifactProvenance=sweepStepArtifactProvenance(),automaticSmoothCaps=true
const automaticBodies=new WeakMap<NurbsBrep,any>()
const fixtures:any[]=correctedC2BodySources.map(name=>{
 const sourceFile='examples/rush/'+name,source=readFileSync(sourceFile,'utf8')
 const graph=compileRushFrontend(source).document
 const built=buildOwnNurbs(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
 assert.ok(built.nativeGeometry,'Actual Rush build must retain a native body')
 const boundaryEvidence=readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)
 assert.ok(boundaryEvidence?.continuousBound&&boundaryEvidence.withinBudget)
 const model=readNativeGeometryData(built.nativeGeometry).geometry as NurbsBrep
 const node=graph.nodes.find(n=>n.op==='brep_progressive_sweep')!
 const construction=built.report.construction![node.id]
 assert.ok('bodyBoundaryEvidence' in construction)
 assert.equal(construction.bodyBoundaryEvidence.capRepresentation,name.includes('-caps-')?'legacy':'radial-annular')
 const reversed=(id:number)=>model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===id))[0]!.reversed
 const caps=name.includes('-caps-')?[model.faces.length-2,model.faces.length-1]:[]
 return {file:name.replace(/\.r$/,'.step'),model,faces:model.faces.length,
  shells:model.shells.length,capHoleFaces:caps.length,capFaces:caps,
  outwardCapContours:caps.map(id=>{const face=model.faces[id]!;return [face.outer,...face.holes].flatMap(wire=>
   (reversed(id)?[...model.loops[wire]!.coedges].reverse():model.loops[wire]!.coedges).map(use=>
    use.reversed!==reversed(id)?reverseNurbsCurve(model.edges[use.edge]!.curve):model.edges[use.edge]!.curve))}),
  expectedVolume:0,volumeReferenceMethod:'rational-boundary-gauss-reference',requireNativeSolid:true,
  volumeBudgets:DEFAULT_SWEEP_VOLUME_BUDGETS,
  sourceFile,sourceSha256:createHash('sha256').update(source).digest('hex'),construction,boundaryEvidence}
})
const cases=fixtures.map(({model,...storedFixture})=>{
 const nativeBody=automaticBodies.get(model)
 const construction=storedFixture.construction as any
 const bodyEvidence=construction?.bodyBoundaryEvidence
  ??('boundaryError' in storedFixture?construction?.[storedFixture.boundaryError?.nodeId]?.bodyBoundaryEvidence:undefined)
 const capLayout=nativeBody?sweepStepCapLayout(nativeBody):bodyEvidence?.capRepresentation
  ?sweepStepCapLayout({...bodyEvidence,model}):construction?.stationReconstruction&&construction.capRepresentation
  ?sweepStepCapLayout({...construction,model}):null
 const fixture={...storedFixture,...capLayout,
  ...(capLayout?.capRepresentation==='radial-annular'?{outwardCapContours:[]}:{}),
 }
 const auditBudgets={
  ...DEFAULT_SWEEP_VOLUME_BUDGETS,maxTrimPairs:10000,maxTrimCells:100000,
  maxTrimDomainCells:1000000,maxPairs:10000,maxCells:100000,maxDomainCells:1000000,
  cellsPerPair:1000,domainCellsPerPair:10000,
  ...('volumeBudgets' in fixture?fixture.volumeBudgets:{}),
 }
 const caps='capFaces' in fixture?fixture.capFaces:fixture.capHoleFaces?[model.faces.length-2,model.faces.length-1]:[]
 const nativeProfileSmoothness=inspectMiterProfileSmoothness(model,capLayout?.physicalCapFaces??caps)
 if('requireStationG2' in fixture&&fixture.requireStationG2&&
   (nativeProfileSmoothness.stationContinuity!=='G2'||('stationSeamCount' in fixture
    ?nativeProfileSmoothness.station.edgeIds.length!==fixture.stationSeamCount:nativeProfileSmoothness.station.edgeIds.length<24))){
  throw Error('Nonuniform native station G2/complete seam set refused: '+fixture.file)
 }
 const genericNativeRetainedWallCharts=inspectSweepRetainedWallCharts(model,caps,100000)
 const genericNativeVolume=inspectSweepVolume(model,caps,auditBudgets)
 const replay='replayRouting' in fixture&&fixture.replayRouting
  ?callGeometryRust<any>('brep_miter_owned_place',{...fixture.replayRouting,expectedResultModel:model,requireSolid:true,wallCells:100000,volumeBudgets:auditBudgets}):null
 if(replay&&(replay.resultModelBound!==true||replay.solidGeometryCertified!==true))throw Error('Native replay final-model/material binding refused: '+fixture.file)
 const nativeRetainedWallCharts=replay?.retainedWallCharts??genericNativeRetainedWallCharts
 const nativeVolume=replay?.volume??genericNativeVolume
 if('requireNativeSolid' in fixture&&fixture.requireNativeSolid&&!nativeVolume.solidGeometryCertified)throw Error('Required native Solid certificate refused: '+fixture.file)
 const genericNativeBoundary=genericNativeVolume.boundary??(caps.length?inspectSweepEmbedding(model,caps,auditBudgets):null)
 if(replay&&!replay.volume?.boundary)throw Error('Fresh replay boundary report missing: '+fixture.file)
 const nativeBoundary=replay?.volume.boundary??genericNativeBoundary
 let exactWork=0
 const nativeExactUses=model.faces.flatMap((face,faceId)=>[face.outer,...face.holes].flatMap(wire=>
  model.loops[wire]!.coedges.map((coedge,index)=>{
   const report=inspectSweepCoedgeExact(face.surface,model.edges[coedge.edge]!.curve,coedge.pcurve,{reversed:coedge.reversed,maxWork:1000000-exactWork})
   exactWork+=report.work
   // Independent retries diagnose shared-budget exhaustion. They never replace
   // the bounded combined embedding/volume certificate above.
   const independent=report.status==='unresolved'
    ?inspectSweepCoedgeExact(face.surface,model.edges[coedge.edge]!.curve,coedge.pcurve,{reversed:coedge.reversed,maxWork:1000000})
    :null
   return {face:faceId,wire,coedge:index,edge:coedge.edge,...report,independent}
  })))
 const text=exportDirectStepV5(model).text
 writeFileSync(resolve(root,fixture.file),text)
 const walls=model.faces.filter((_,index)=>!caps.includes(index))
 const wallFaceReversed=walls.map(face=>{
  const faceId=model.faces.indexOf(face)
  const uses=model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===faceId))
  if(uses.length!==1)throw Error('STEP oracle requires unique retained wall face ownership')
  return uses[0]!.reversed
 })
 const wallSamples=walls.map(face=>{
  const surface=face.surface
  return [0,.17,.5,.83,1].flatMap(u=>[0,.13,.5,.87,1].map(v=>{const e=evaluateNurbsSurface(surface,u,v);return {u,v,point:e.point,du:e.du,dv:e.dv}}))
 })
 if(wallSamples.length!==fixture.faces-caps.length)throw Error('Unexpected wall sample coverage')
 return {...fixture,capFaces:caps,nativeAuditBudgets:auditBudgets,nativeProfileSmoothness,nativeRetainedWallCharts,nativeVolume,genericNativeRetainedWallCharts,genericNativeVolume,nativeReplayFinalModelBound:replay?.resultModelBound??null,nativeBoundary,genericNativeBoundary,nativeExactUses,exactWork,edges:model.edges.length,faceLoops:model.faces.map(face=>({outer:model.loops[face.outer]!.coedges.map(c=>c.edge),holes:face.holes.map(loop=>model.loops[loop]!.coedges.map(c=>c.edge))})),edgeCurves:model.edges.map(edge=>({curve:edge.curve,samples:[0,.25,.5,.75,1].map(u=>({u,point:evaluateNurbsCurve(edge.curve,u).point}))})),wallCoedges:walls.map(face=>model.loops[face.outer]!.coedges.map(c=>({edge:c.edge,reversed:c.reversed,pcurve:c.pcurve}))),wallSurfaces:walls.map(face=>face.surface),wallFaceReversed,wallSamples,surfaceToleranceMm:1e-8,solids:1,sha256:createHash('sha256').update(text).digest('hex'),relativeVolumeTolerance:1e-7}
})
// Complete reports can exceed V8's maximum single string length. Preserve
// every field while writing one case at a time; publish only the finished file.
const stagedManifest=resolve(root,`manifest-${process.pid}.json.writing`)
writeFileSync(stagedManifest,JSON.stringify({schema:'sweep-external-step/2',units:'mm',artifactProvenance,automaticSmoothCaps}).slice(0,-1)+',"cases":[\n')
for(let index=0;index<cases.length;index++)appendFileSync(stagedManifest,(index?',\n':'')+JSON.stringify(cases[index]))
appendFileSync(stagedManifest,'\n]}\n')
renameSync(stagedManifest,resolve(root,'manifest.json'))
console.log(root)
