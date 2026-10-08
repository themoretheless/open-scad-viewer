import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
import {mkdirSync,writeFileSync,readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {createProgressiveMiterBrepProfileBody,smoothCertifiedMiterBody} from '../src/services/geometry/brep'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
import type {NurbsBrep} from '../src/services/geometry/brep'
import {exportDirectStepV5} from '../src/services/cadNurbsStep'
import {inspectMiterProfileSmoothness} from '../src/services/miterProfileSmoothness'
import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
const root=resolve(process.argv[2]??'/tmp/sweep-smooth-station-step')
mkdirSync(root,{recursive:true})
const artifactProvenance=sweepStepArtifactProvenance()
const signs=[[1,0],[1,1],[0,1],[-1,1],[-1,0],[-1,-1],[0,-1],[1,-1],[1,0]]
const profile={degree:2,knots:[0,0,0,.25,.25,.5,.5,.75,.75,1,1,1],
 controlPoints:signs.map(([x,y])=>[x!,y!,0]),weights:signs.map((_,i)=>i%2===0?1:Math.SQRT1_2),periodic:false}
const law=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
const qualificationCatalog=JSON.parse(readFileSync(new URL('../docs/design/sweep-qualification-catalog.json',import.meta.url),'utf8'))
const cases=(qualificationCatalog.step.smoothModes as (boolean|string)[]).map(mode=>{
 const curved=mode!==false
 const weightMode=typeof mode==='string'&&mode.startsWith('weight-')
 const middleWeight=mode==='weight-half'?.5:mode==='weight-one'?1:mode==='weight-two'?2:Math.SQRT1_2
 let smoothed:ReturnType<typeof smoothCertifiedMiterBody>
 if(typeof mode==='string'){
  const sourceFile=weightMode?`progressive-miter-reconstructed-conic-${mode.slice(7)}.r`:mode==='rush-closed'?'closed-progressive-miter-reconstructed-frame-guide-affine-hollow.r':mode==='rush-sharp'?'progressive-miter-reconstructed-sharp.r':mode==='rush-guide'?'miter-rational-curved-guide-frame-affine-hollow.r':mode==='rush-moving'?'miter-moving-frame-guide-affine-hollow-corrected.r':'progressive-miter-reconstructed-stations.r'
  let source=readFileSync(`examples/rush/${sourceFile}`,'utf8')
  if(mode==='rush-moving')source=source.replace('cap_correction_tolerance:','circle_correction_tolerance:1e-9mm,circle_correction_max_work:100000,cap_correction_tolerance:')
  if(mode==='rush-guide'||mode==='rush-moving')source=source.replace(/\.brep_tessellate\(\d+\)/,'.brep_smooth_miter_stations(wall_tolerance:1mm,quantum:0.0000000000004547473508864641mm,max_work:1000000,max_deviation:2mm).brep_tessellate(4)')
  const graph=compileRushFrontend(source).document
  const node=graph.nodes.find(n=>n.op==='brep_smooth_miter_stations')!
  const built=buildOwnNurbs(graph,{action:'build'})
  const {kind,...model}=built.report.definitions[node.id] as unknown as NurbsBrep & {kind:string}
  if(kind!=='brep')throw Error('Rush reconstruction did not retain a B-rep')
  smoothed={...(built.report.construction![node.id] as ReturnType<typeof smoothCertifiedMiterBody>),model}
 }else{
 const weightedProfile={...profile,weights:signs.map((_,i)=>i%2===0?1:middleWeight)}
 const source=createProgressiveMiterBrepProfileBody([[weightedProfile]],[[0,0,0],[0,0,10]],law(1),law(0),{
  normal:[1,0,0],initialSteps:2,maxSteps:2,maxDeviation:curved?1:.01,
  ...(curved?{centerLaw:{degree:1,knots:[0,0,.5,1,1],values:[[0,0,0],[1,0,0],[0,0,0]],weights:[1,1,1]}}:{}),
 })
 if(!source.boundaryCertificate.continuousBound||!source.boundaryCertificate.withinBudget)throw Error(JSON.stringify({mode,middleWeight,boundaryCertificate:source.boundaryCertificate,retainedCorrespondence:source.retainedCorrespondence,retainedDecomposition:source.retainedDecomposition}))
 const sections=source.approximation.sections.map(station=>station.map(curve=>[curve]))
 smoothed=smoothCertifiedMiterBody(source,sections,[],{quantum:.125,maxWork:10000,wallTolerance:.5,maxDeviation:curved?2:.01})
 }
 const model=smoothed.model,capFaces=smoothed.boundaryCertificate.closed?[]:[model.faces.length-2,model.faces.length-1]
 const text=exportDirectStepV5(model).text,file=weightMode?`quintic-conic-${middleWeight}-owned-solid.step`:mode==='rush-closed'?'quintic-rush-closed-frame-guide-affine-hollow.step':mode==='rush-guide'?'quintic-rush-guide-frame-affine-hollow.step':mode==='rush-moving'?'quintic-rush-moving-frame-guide-affine-hollow.step':mode==='rush-sharp'?'quintic-rush-sharp-solid.step':mode==='rush'?'quintic-rush-reconstructed-solid.step':curved?'quintic-curved-owned-solid.step':'quintic-straight-owned-solid.step'
 writeFileSync(resolve(root,file),text)
 const walls=model.faces.filter((_,i)=>!capFaces.includes(i))
 return {file,faces:model.faces.length,edges:model.edges.length,solids:1,shells:smoothed.boundaryCertificate.closed?2:1,capHoleFaces:mode==='rush-guide'||mode==='rush-moving'?2:0,capFaces,
  requireNativeSolid:true,nativeVolume:smoothed.volume,boundaryCertificate:smoothed.boundaryCertificate,
  candidate:{wallDisplacementUpper:smoothed.candidate.wallDisplacementUpper,work:smoothed.candidate.work},
  nativeProfileSmoothness:inspectMiterProfileSmoothness(model,capFaces),
  faceLoops:model.faces.map(f=>({outer:model.loops[f.outer]!.coedges.map(c=>c.edge),holes:f.holes.map(l=>model.loops[l]!.coedges.map(c=>c.edge))})),
  edgeCurves:model.edges.map(e=>({curve:e.curve,samples:[0,.25,.5,.75,1].map(u=>({u,point:evaluateNurbsCurve(e.curve,u).point}))})),
  wallSurfaces:walls.map(f=>f.surface),
  wallSamples:walls.map(f=>[0,.17,.5,.83,1].flatMap(u=>[0,.13,.5,.87,1].map(v=>{const e=evaluateNurbsSurface(f.surface,u,v);return {u,v,point:e.point,du:e.du,dv:e.dv}}))),
  expectedVolume:weightMode||mode==='rush-guide'||mode==='rush-moving'||mode==='rush-closed'?null:Math.PI*(mode==='rush-sharp'?20:10),...(weightMode?{volumeReferenceMethod:'conic-generator-polynomial-integral'}:mode==='rush-guide'||mode==='rush-moving'||mode==='rush-closed'?{volumeReferenceMethod:'canonical-generator-polynomial-integral'}:{}),relativeVolumeTolerance:1e-7,surfaceToleranceMm:1e-8,
  sha256:createHash('sha256').update(text).digest('hex')}
})
if(new Set(cases.map(c=>c.file)).size!==cases.length||cases.length!==qualificationCatalog.step.smooth.length||qualificationCatalog.step.smooth.some((c:{file:string})=>!cases.some(item=>item.file===c.file)))throw Error('Smooth STEP cases differ from the shared catalog')

writeFileSync(resolve(root,'manifest.json'),JSON.stringify({schema:'sweep-external-step/1',units:'mm',selectionCatalogSha256:createHash('sha256').update(readFileSync(new URL('../docs/design/sweep-qualification-catalog.json',import.meta.url))).digest('hex'),artifactProvenance,cases},null,2)+'\n')
console.log(root)
