import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
import {mkdirSync,writeFileSync,readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {deepStrictEqual} from 'node:assert'
import {createHash} from 'node:crypto'
import {circleNurbsCurve,miterNurbsProfileSections} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve,evaluateNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'
import {createMiterBrepProfileBody,createProgressiveMiterBrepProfileBody,createRationalBrepSectionLoft,transformNurbsBrep} from '../src/services/geometry/brep'
import {projectSweepSections} from '../src/services/nurbsSectionProjection'
import {inspectSweepRetainedCorrespondence,inspectSweepRetainedCaps} from '../src/services/sweepRetainedCorrespondence'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {exportDirectStepV5} from '../src/services/cadNurbsStep'
import {inspectSweepVolume,inspectSweepEmbedding,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'
import {inspectMiterProfileSmoothness} from '../src/services/miterProfileSmoothness'
import {inspectSweepRetainedWallCharts} from '../src/services/nurbsSweepRetainedCharts'
import {inspectSweepCoedgeExact} from '../src/services/nurbsSweepAudit'
import {compileModelGraphText} from '../src/services/modelGraphText'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
import type {NurbsBrep} from '../src/services/geometry/brep'
const root=resolve(process.argv[2]??'docs/qualification/sweep-coverage-2026-10-01/external-step')
mkdirSync(root,{recursive:true})
const artifactProvenance=sweepStepArtifactProvenance()
const loops=[[circleNurbsCurve([0,0,0],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))]]
const points:[number,number,number][]=[[0,0,0],[0,0,10],[10,0,10],[10,10,15]]
const body=createMiterBrepProfileBody(loops,points,[1,0,0],2)
const spatialSections=miterNurbsProfileSections(loops.flat(),points,[1,0,0],2)
const correction=projectSweepSections(spatialSections,[{section:spatialSections.length-1,
 plane:{axis:1,coefficients:[0,-.5],offset:17.5},quantum:2**-40,tolerance:1e-9}],1000000)
if(!correction.sections||correction.wallDisplacementUpper===null)throw Error('Spatial section interpolation correction unproved')
const correctedSections=correction.sections.map(row=>row.map(curve=>[curve]))
const correctedSpatial=createRationalBrepSectionLoft(correctedSections)
const correctionCorrespondence=inspectSweepRetainedCorrespondence(correctedSpatial,correctedSections,false)
if(!correctionCorrespondence.exact)throw Error('Corrected retained wall correspondence unproved')
const constructorCorrected=createMiterBrepProfileBody(loops,points,[1,0,0],2,false,
 {quantum:2**-40,tolerance:1e-9,maxWork:1000000})
const straight=createMiterBrepProfileBody(loops,[[0,0,0],[0,0,10]],[1,0,0],2)
const obliqueTransform=[[1,0,0,0],[0,1,0,0],[1,1,1,0],[0,0,0,1]]
const oblique=transformNurbsBrep(straight.model,obliqueTransform)
const obliqueSections=miterNurbsProfileSections(loops.flat(),[[0,0,0],[0,0,10]],[1,0,0],2)
for(const row of obliqueSections)for(const curve of row)for(const pole of curve.controlPoints)pole[2]=pole[0]!+pole[1]!+pole[2]!
const obliqueEndpoints=obliqueSections.map(row=>row.map(curve=>[curve]))
const obliqueCaps=inspectSweepRetainedCaps(oblique,[obliqueEndpoints[0]!,obliqueEndpoints.at(-1)!],DEFAULT_SWEEP_VOLUME_BUDGETS.capBudgets)
const dyadicRing=(radius:number,z:number)=>[
 [[radius,0],[radius,radius],[0,radius]],[[0,radius],[-radius,radius],[-radius,0]],
 [[-radius,0],[-radius,-radius],[0,-radius]],[[0,-radius],[radius,-radius],[radius,0]],
].map(poles=>({degree:2,knots:[0,0,0,1,1,1],controlPoints:poles.map(([x,y])=>[x!,y!,z]),weights:[1,Math.SQRT1_2,1],periodic:false}))
const dyadicSections=[0,10].map(z=>[dyadicRing(.5,z),dyadicRing(.25,z).reverse().map(reverseNurbsCurve)])
const dyadicOblique=transformNurbsBrep(createRationalBrepSectionLoft(dyadicSections),obliqueTransform)
for(const rings of dyadicSections)for(const ring of rings)for(const curve of ring)for(const pole of curve.controlPoints)pole[2]=pole[0]!+pole[1]!+pole[2]!
const dyadicCaps=inspectSweepRetainedCaps(dyadicOblique,[dyadicSections[0]!,dyadicSections[1]!],DEFAULT_SWEEP_VOLUME_BUDGETS.capBudgets)
if(!dyadicCaps.exact)throw Error('Dyadic oblique retained cap correspondence unproved: '+JSON.stringify(dyadicCaps))
const closedLoops=[[circleNurbsCurve([0,0,0],[1,0,0],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[1,0,0],.2))]]
const closed=createMiterBrepProfileBody(closedLoops,[[0,0,0],[10,0,0],[10,10,0],[0,10,0]],[0,0,1],2,true)
const law=(a:number,b:number)=>({degree:1,knots:[0,0,1,1],values:[a,b],weights:[1,1]})
const closedAffine=createProgressiveMiterBrepProfileBody(closedLoops,[[0,0,0],[10,0,0],[10,10,0],[0,10,0]],law(1,1),law(0,0),{
 normal:[0,0,1],closed:true,miterLimit:2,maxDeviation:.001,maxSteps:1,
 axisScale:{degree:1,knots:[2,2,5,5],values:[[2,1,1],[2,1,1]],weights:[1,1]},
 centerLaw:{degree:1,knots:[7,7,9,9],values:[[.125,0,0],[.125,0,0]],weights:[1,1]},
})
// Closed joint frame/guide/affine fixture. The volume oracle integrates the
// quadratic determinant of each retained linear ellipse family analytically.
const closedJoint=createProgressiveMiterBrepProfileBody(
 [[circleNurbsCurve([0,0,0],[1,0,0],.25)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[1,0,0],.1))]],
 [[0,0,0],[10,0,0],[10,10,0],[0,10,0]],law(1,1),law(0,0),{
 normal:[0,0,1],closed:true,maxDeviation:10,maxSteps:8,retainedWallMaxInjectivityCells:10000,
 frameAxis:{degree:1,knots:[0,0,.25,.5,.75,1,1],values:[[1,-1,0],[1,1,0],[-1,1,0],[-1,-1,0],[1,-1,0]],weights:[1,1,1,1,1]},
 frameNormal:{degree:1,knots:[2,2,5,5],values:[[0,0,1],[0,0,1]],weights:[1,1]},
 orientationGuide:{degree:1,knots:[0,0,.25,.5,.75,1,1],controlPoints:[[0,0,100],[10,0,100],[10,10,100],[0,10,100],[0,0,100]],weights:[1,1,1,1,1],periodic:false},
 axisScale:{degree:1,knots:[7,7,9,9],values:[[2,1,1],[2,1,1]],weights:[1,1]},
})
const determinant=(d:number[],a:number[],b:number[])=>d[0]!*(a[1]!*b[2]!-a[2]!*b[1]!)-d[1]!*(a[0]!*b[2]!-a[2]!*b[0]!)+d[2]!*(a[0]!*b[1]!-a[1]!*b[0]!)
const ellipse=(curve:NurbsCurve)=>{
 const p=curve.controlPoints,c=p[0]!.map((x,i)=>(x+p[4]![i]!)/2)
 return {c,a:p[0]!.map((x,i)=>x-c[i]!),b:p[2]!.map((x,i)=>x-c[i]!)}
}
const jointSections=closedJoint.approximation.sections!
const jointLoopVolume=(loop:number)=>{
 let volume=0
 for(let i=0;i<jointSections.length-1;i++){
  const a=ellipse(jointSections[i]![loop]!),b=ellipse(jointSections[i+1]![loop]!),d=b.c.map((x,j)=>x-a.c[j]!)
  volume+=Math.PI*(determinant(d,a.a,a.b)/3+(determinant(d,a.a,b.b)+determinant(d,b.a,a.b))/6+determinant(d,b.a,b.b)/3)
 }
 return Math.abs(volume)
}
const closedJointVolume=jointLoopVolume(0)-jointLoopVolume(1)
const progressive=createProgressiveMiterBrepProfileBody(loops,[[0,0,0],[0,0,10]],law(1,1.5),law(0,30),{normal:[1,0,0],maxDeviation:.02,maxSteps:16})
const steps=progressive.approximation.report.steps
// Exact real-arithmetic integral for the retained straight circular family:
// interpolated rotated/scaled circles have quadratic transverse area.
let retainedIntegral=0
for(let i=0;i<steps;i++){
 const a=1+.5*i/steps,b=1+.5*(i+1)/steps
 retainedIntegral+=(a*a+a*b*Math.cos(Math.PI/6/steps)+b*b)/(3*steps)
}
const progressiveVolume=Math.PI*(.5**2-.2**2)*10*retainedIntegral
const affineRushSource=readFileSync('examples/rush/miter-affine-hollow.r','utf8')
const affineRushGraph=compileModelGraphText(affineRushSource)
const affineRushNode=affineRushGraph.document.nodes.find(node=>node.op==='brep_progressive_miter_sweep')!
const affineRushBuilt=buildOwnNurbs(affineRushGraph.document,{action:'build'})
const {kind:affineRushKind,...affineRushDefinition}=affineRushBuilt.report.definitions[affineRushNode.id] as unknown as NurbsBrep & {kind:string}
if(affineRushKind!=='brep')throw Error('Affine Rush source did not retain a B-rep')
const affineRushModel=affineRushDefinition as NurbsBrep
const affineRushConstruction=affineRushBuilt.report.construction![affineRushNode.id]
const authoredRushFixture=(sourcePath:string,file:string,areaMultiplier:number,requireNativeSolid=true,parallel:[boolean,boolean]=[true,true])=>{
 const source=readFileSync(sourcePath,'utf8')
 const graph=compileModelGraphText(source)
 const node=graph.document.nodes.find(node=>node.op==='brep_progressive_miter_sweep')!
 const built=buildOwnNurbs(graph.document,{action:'build'})
 const {kind,...geometry}=built.report.definitions[node.id] as unknown as NurbsBrep & {kind:string}
 if(kind!=='brep')throw Error('Authored Rush fixture did not retain a B-rep: '+sourcePath)
 const model=geometry as NurbsBrep
 const construction=built.report.construction![node.id]
 if(sourcePath.endsWith('-corrected.r')){
  const evidence=construction as {sectionCorrection?:{reason:string;wallDisplacementUpper:number|null;exactPlanarSections:number[]};retainedCorrespondence?:{exact:boolean};boundaryErrorWithinBudget?:boolean|null;boundaryErrorUpper?:number|null;filledCapErrorUpper?:[number,number]|null;capParallelism?:{parallel:[boolean,boolean]|null};idealCapDomains?:{idealCapDomainsCertified:boolean}}
  if(evidence.sectionCorrection?.reason!=='bounded-section-interpolation'||evidence.sectionCorrection.wallDisplacementUpper===null||!evidence.retainedCorrespondence?.exact||evidence.sectionCorrection.exactPlanarSections.length!==2)throw Error('Corrected fixture lacks retained correction proof: '+sourcePath)
  if(evidence.boundaryErrorWithinBudget!==true||evidence.boundaryErrorUpper==null||!Number.isFinite(evidence.boundaryErrorUpper)||!evidence.filledCapErrorUpper||!evidence.capParallelism?.parallel||evidence.capParallelism.parallel.some((p,i)=>p!==parallel[i])||evidence.idealCapDomains?.idealCapDomainsCertified!==true)throw Error('Corrected fixture lacks complete boundary error budget proof: '+sourcePath)
 }
 return {file,model,faces:model.faces.length,shells:1,capHoleFaces:2,requireNativeSolid,
  sourceSha256:createHash('sha256').update(source).digest('hex'),construction:built.report.construction![node.id],
  expectedVolume:Math.PI*(.5**2-.2**2)*10*areaMultiplier}
}
// Each quadratic quarter has signed area 5/6, hence source area 10/3.
// The hole is a reversed copy scaled by 1/4; the length is 10.
const unsegmentedRing=(radius:number):NurbsCurve=>({degree:2,knots:[0,0,0,.25,.5,.75,1,1,1],
 controlPoints:[[1,0,0],[1,1,0],[-1,1,0],[-1,-1,0],[1,-1,0],[1,0,0]].map(p=>p.map(v=>v*radius)),
 weights:[1,1,1,1,1,1],periodic:false})
const unsegmentedLoops=[[unsegmentedRing(1)],[reverseNurbsCurve(unsegmentedRing(.25))]]
const vectorLaw=(values:[number,number,number])=>({degree:1,knots:[0,0,1,1],values:[values,values],weights:[1,1]})
const unsegmentedFixture=(mode:'plain'|'affine'|'frame-affine'|'guide-affine'|'frame-guide-affine')=>{
 const affine=mode==='plain'?{}:{axisScale:vectorLaw([2,1,1]),centerLaw:vectorLaw([.125,0,0])}
 const frames=(mode==='frame-affine'||mode==='frame-guide-affine')?{frameAxis:vectorLaw([0,0,2]),frameNormal:vectorLaw([0,3,0])}:{}
 const guide=(mode==='guide-affine'||mode==='frame-guide-affine')?{orientationGuide:{degree:1,knots:[0,0,1,1],controlPoints:[[0,1,0],[0,1,10]],weights:[1,1],periodic:false}}:{}
 console.log('Unsegmented STEP mode:',mode)
 const body=createProgressiveMiterBrepProfileBody(unsegmentedLoops,[[0,0,0],[0,0,10]],law(1,1),law(0,0),{
  normal:[1,0,0],initialSteps:1,maxSteps:16,maxDeviation:.001,...affine,...frames,...guide,
 })
 if(body.approximation.report.authoredFramesApplied!==((mode==='frame-affine'||mode==='frame-guide-affine'))||body.approximation.report.orientationGuideApplied!==((mode==='guide-affine'||mode==='frame-guide-affine'))||body.approximation.report.affineLawsApplied!==(mode!=='plain'))throw Error('Unsegmented fixture mode was not applied: '+mode)
 if(body.retainedCorrespondence.exact||!body.retainedDecomposition?.certified||!body.retainedCapDecomposition?.certified||body.boundaryErrorWithinBudget!==true)throw Error('Unsegmented fixture lacks complete decomposition boundary proof: '+mode)
 const source=readFileSync('examples/rush/miter-unsegmented-'+mode+'-hollow.r','utf8')
 const compiled=compileModelGraphText(source)
 const node=compiled.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const built=buildOwnNurbs(compiled.document,{action:'build'})
 const {kind,...rushModel}=built.report.definitions[node.id] as unknown as NurbsBrep & {kind:string}
 if(kind!=='brep')throw Error('Unsegmented Rush fixture did not retain B-rep: '+mode)
 deepStrictEqual(rushModel,body.model,'Rush and public constructors must retain identical unsegmented-mode geometry: '+mode)
 const construction=built.report.construction![node.id] as {boundaryErrorWithinBudget:boolean|null;retainedDecomposition:typeof body.retainedDecomposition;retainedCapDecomposition:typeof body.retainedCapDecomposition}
 if(construction.boundaryErrorWithinBudget!==true||!construction.retainedDecomposition?.certified||!construction.retainedCapDecomposition?.certified)throw Error('Rush dropped decomposition boundary evidence: '+mode)
 return {file:'unsegmented-'+mode+'-hollow-miter.step',model:body.model,faces:body.model.faces.length,shells:1,capHoleFaces:2,requireNativeSolid:true,
  sourceSha256:createHash('sha256').update(source).digest('hex'),construction,
  admission:body.approximation.report,retainedDecomposition:body.retainedDecomposition,retainedCapDecomposition:body.retainedCapDecomposition,
  idealCapDomains:body.idealCapDomains,boundaryErrorUpper:body.boundaryErrorUpper,boundaryErrorWithinBudget:body.boundaryErrorWithinBudget,
  expectedVolume:(10/3)*(1-1/16)*10*(mode==='plain'?1:2)}
}
const circleSource=readFileSync('examples/rush/progressive-miter-circle-corrected-hollow.r','utf8')
const circleGraph=compileModelGraphText(circleSource)
const circleNode=circleGraph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
const circleBuilt=buildOwnNurbs(circleGraph.document,{action:'build'})
const {kind:circleKind,...circleModel}=circleBuilt.report.definitions[circleNode.id] as unknown as NurbsBrep & {kind:string}
if(circleKind!=='brep')throw Error('Circle correction did not retain B-rep')
const circleConstruction=circleBuilt.report.construction![circleNode.id] as any
if(!circleConstruction.profileSmoothness.profile.exactG1G2Certified||circleConstruction.boundaryErrorWithinBudget!==true||!circleConstruction.volume.solidGeometryCertified)throw Error('Circle corrected Rush fixture lacked required proofs')
const spatialCircleSource=readFileSync('examples/rush/progressive-miter-spatial-circle-corrected-hollow.r','utf8')
const spatialCircleGraph=compileModelGraphText(spatialCircleSource)
const spatialCircleNode=spatialCircleGraph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
const spatialCircleBuilt=buildOwnNurbs(spatialCircleGraph.document,{action:'build'})
const {kind:spatialCircleKind,...spatialCircleModel}=spatialCircleBuilt.report.definitions[spatialCircleNode.id] as unknown as NurbsBrep & {kind:string}
const spatialCircleConstruction=spatialCircleBuilt.report.construction![spatialCircleNode.id] as any
if(spatialCircleKind!=='brep'||!spatialCircleConstruction.profileSmoothness.profile.exactG1G2Certified||spatialCircleConstruction.boundaryErrorWithinBudget!==true||!spatialCircleConstruction.volume.solidGeometryCertified)throw Error('Spatial circle corrected Rush fixture lacked required proofs')
const affineCircleSource=readFileSync('examples/rush/progressive-miter-affine-circle-corrected-hollow.r','utf8')
const affineCircleGraph=compileModelGraphText(affineCircleSource)
const affineCircleNode=affineCircleGraph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
const affineCircleBuilt=buildOwnNurbs(affineCircleGraph.document,{action:'build'})
const {kind:affineCircleKind,...affineCircleModel}=affineCircleBuilt.report.definitions[affineCircleNode.id] as unknown as NurbsBrep & {kind:string}
const affineCircleConstruction=affineCircleBuilt.report.construction![affineCircleNode.id] as any
if(affineCircleKind!=='brep'||!affineCircleConstruction.affineLawsApplied||!affineCircleConstruction.profileSmoothness.profile.exactG1G2Certified||affineCircleConstruction.boundaryErrorWithinBudget!==true||!affineCircleConstruction.volume.solidGeometryCertified)throw Error('Affine circle corrected Rush fixture lacked required proofs')
const obliqueCircleSource=readFileSync('examples/rush/progressive-miter-oblique-circle-corrected-hollow.r','utf8')
const obliqueCircleGraph=compileModelGraphText(obliqueCircleSource)
const obliqueCircleNode=obliqueCircleGraph.document.nodes.find(n=>n.op==='transform')!
const obliqueCircleBuilt=buildOwnNurbs(obliqueCircleGraph.document,{action:'build'})
const {kind:obliqueCircleKind,...obliqueCircleModel}=obliqueCircleBuilt.report.definitions[obliqueCircleNode.id] as unknown as NurbsBrep & {kind:string}
const obliqueCircleConstruction=obliqueCircleBuilt.report.construction![obliqueCircleNode.id] as any
if(obliqueCircleKind!=='brep'||!obliqueCircleConstruction.profileSmoothness.profile.exactG1G2Certified||obliqueCircleConstruction.boundaryErrorWithinBudget!==true||!obliqueCircleConstruction.volume.solidGeometryCertified||obliqueCircleConstruction.placement.arithmeticErrorUpper!==0)throw Error('Oblique circle corrected Rush fixture lacked required proofs')
const curvedGuideSource=readFileSync('examples/rush/miter-rational-curved-guide-frame-affine-hollow.r','utf8')
const curvedGuideGraph=compileModelGraphText(curvedGuideSource)
const curvedGuideNode=curvedGuideGraph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
const curvedGuideBuilt=buildOwnNurbs(curvedGuideGraph.document,{action:'build'})
const {kind:curvedGuideKind,...curvedGuideModel}=curvedGuideBuilt.report.definitions[curvedGuideNode.id] as unknown as NurbsBrep & {kind:string}
const curvedGuideConstruction=curvedGuideBuilt.report.construction![curvedGuideNode.id] as any
if(curvedGuideKind!=='brep'||!curvedGuideConstruction.profileSmoothness.profile.exactG1G2Certified||curvedGuideConstruction.boundaryErrorWithinBudget!==true||!curvedGuideConstruction.volume.solidGeometryCertified)throw Error('Rational curved guide Rush fixture lacked required proofs')
// Independent source-law integral: interpolated rotations R_i diag(2,1)
// have area integral 2*(2+cos(theta_i-theta_j))/3 on each station interval.
const guideX=(t:number)=>.16*t*(1-t)/((1-t)**2+1.6*t*(1-t)+t*t)
let curvedGuideIntegral=0
for(let i=0;i<curvedGuideConstruction.steps;i++){
 const x=guideX(i/curvedGuideConstruction.steps),y=guideX((i+1)/curvedGuideConstruction.steps)
 const cosine=(1+x*y)/Math.sqrt((1+x*x)*(1+y*y))
 curvedGuideIntegral+=2*(2+cosine)/(3*curvedGuideConstruction.steps)
}
const curvedGuideVolume=Math.PI*(.5**2-.2**2)*10*curvedGuideIntegral
const stationSource=readFileSync('examples/rush/progressive-miter-station-g2-hollow.r','utf8')
const stationGraph=compileModelGraphText(stationSource)
const stationNode=stationGraph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
const stationBuilt=buildOwnNurbs(stationGraph.document,{action:'build'})
const {kind:stationKind,...stationModel}=stationBuilt.report.definitions[stationNode.id] as unknown as NurbsBrep & {kind:string}
const stationConstruction=stationBuilt.report.construction![stationNode.id] as any
if(stationKind!=='brep'||!stationConstruction.profileSmoothness.station.stationG2Certified||!stationConstruction.volume.solidGeometryCertified)throw Error('Station G2 Rush fixture lacked required proofs')
const g1Source=readFileSync('examples/rush/miter-g1-profile-frame-guide-affine.r','utf8')
const g1Graph=compileModelGraphText(g1Source)
const g1Node=g1Graph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
const g1Built=buildOwnNurbs(g1Graph.document,{action:'build'})
const {kind:g1Kind,...g1Model}=g1Built.report.definitions[g1Node.id] as unknown as NurbsBrep & {kind:string}
if(g1Kind!=='brep')throw Error('G1 Rush profile did not retain a body')
const g1Construction=g1Built.report.construction![g1Node.id] as {profileSmoothness:{profileG1Certified:boolean;profile:{exactG1G2Certified:boolean}}}
if(!g1Construction.profileSmoothness.profileG1Certified||g1Construction.profileSmoothness.profile.exactG1G2Certified)throw Error('G1/G2 Rush distinction lost')
const fixtures=[
 (()=>{
  const sourceFile='examples/rush/miter-periodic-moving-axis-guide-affine-hollow-authored-caps.r'
  const source=readFileSync(sourceFile,'utf8'),graph=compileModelGraphText(source).document
  const node=graph.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
  const built=buildOwnNurbs(graph,{action:'build'})
  const {kind,...model}=built.report.definitions[node.id] as unknown as NurbsBrep & {kind:string}
  const construction=built.report.construction![node.id] as {continuousBound:boolean;boundaryCertificate:{withinBudget:boolean}}
  if(kind!=='brep'||!construction.continuousBound||!construction.boundaryCertificate.withinBudget)throw Error('Authored-plane moving-axis body unproved')
  const reversed=(face:number)=>{
   const uses=model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===face))
   if(uses.length!==1)throw Error('Polynomial volume reference needs single face ownership')
   return uses[0]!.reversed
  }
  const caps=[model.faces.length-2,model.faces.length-1]
  const outwardCapContours=caps.map(id=>{
   const face=model.faces[id]!
   return [face.outer,...face.holes].flatMap(wire=>(reversed(id)?[...model.loops[wire]!.coedges].reverse():model.loops[wire]!.coedges).map(use=>{
    const curve=model.edges[use.edge]!.curve
    return use.reversed!==reversed(id)?reverseNurbsCurve(curve):curve
   }))
  })
  return {file:'rush-periodic-moving-axis-authored-caps.step',model:model as NurbsBrep,faces:model.faces.length,
   shells:1,capHoleFaces:2,requireNativeSolid:true,construction,sourceFile,
   sourceSha256:createHash('sha256').update(source).digest('hex'),
   volumeReferenceMethod:'polynomial-boundary-divergence-integral',
   wallFaceReversed:model.faces.flatMap((_,id)=>caps.includes(id)?[]:[reversed(id)]),outwardCapContours,
   // The independent Python reference must fill this before OCCT verification.
   expectedVolume:0}
 })(),
 ...(['unclamped','periodic','periodic-frame-guide-affine'] as const).map(mode=>{
  const source=readFileSync(`examples/rush/miter-${mode}-hollow.r`,'utf8')
  const graph=compileModelGraphText(source).document
  const node=graph.nodes.find(node=>node.op==='brep_progressive_miter_sweep')!
  const built=buildOwnNurbs(graph,{action:'build'})
  const {kind,...model}=built.report.definitions[node.id] as unknown as NurbsBrep & {kind:string}
  if(kind!=='brep')throw Error(`${mode} Rush profile did not retain a body`)
  if(mode.startsWith('periodic')&&graph.nodes.filter(n=>n.op==='curve'&&n.periodic).length!==2)throw Error('Periodic Rush storage lost')
  const construction=built.report.construction![node.id] as {authoredFramesApplied?:boolean;orientationGuideApplied?:boolean;affineLawsApplied?:boolean;boundaryCertificate:{continuousBound:boolean;withinBudget:boolean}}
  if(!construction.boundaryCertificate.continuousBound||!construction.boundaryCertificate.withinBudget)throw Error(`${mode} complete boundary certificate unproved`)
  if(mode==='periodic-frame-guide-affine'&&(!construction.authoredFramesApplied||!construction.orientationGuideApplied||!construction.affineLawsApplied))throw Error('Periodic joint-mode evidence missing')
  // Each quadratic span has x=1/2-t, y=1/2+t-t² in a rotated
  // quadrant: 1/2 integral(x y'-y x')=5/12. Four spans give 5/3.
  // Remove the 1/4-scale hole and extrude 10 mm: (5/3)(1-1/16)10=125/8.
  return {file:`rush-${mode}-hollow.step`,model:model as NurbsBrep,faces:model.faces.length,
   shells:1,capHoleFaces:2,requireNativeSolid:true,construction,
   // Constant transverse affine scale doubles area; frame/guide are constant
   // and the center offset is a translation, so volume is independently 125/4.
   sourceSha256:createHash('sha256').update(source).digest('hex'),expectedVolume:(125/8)*(mode==='periodic-frame-guide-affine'?2:1)}
 }),
 (()=>{
  const fixture=authoredRushFixture('examples/rush/miter-moving-frame-guide-affine-hollow-corrected.r','rush-moving-frame-guide-affine-hollow-corrected.step',2,true,[true,false])
  const steps=(fixture.construction as {steps:number}).steps
  // F(x,y,f)=[2x,A(f)y,10f+B(f)y]. The determinant's y term
  // integrates to zero over each centered annulus; A is the retained linear
  // interpolation of 1/sqrt(1+f^2). Endpoint Z correction changes B only.
  let integral=0
  for(let i=0;i<steps;i++)integral+=(1/Math.hypot(1,i/steps)+1/Math.hypot(1,(i+1)/steps))/(2*steps)
  return {...fixture,expectedVolume:fixture.expectedVolume*integral}
 })(),
 authoredRushFixture('examples/rush/miter-combined-frame-guide-affine-hollow-corrected.r','rush-combined-frame-guide-affine-hollow-corrected.step',2),
 authoredRushFixture('examples/rush/miter-combined-frame-guide-affine-hollow.r','rush-combined-frame-guide-affine-hollow-miter.step',2),
 ...(['plain','affine','frame-affine','guide-affine','frame-guide-affine'] as const).map(unsegmentedFixture),
 authoredRushFixture('examples/rush/miter-guide-hollow-corrected.r','rush-miter-guide-hollow-corrected.step',1),
 authoredRushFixture('examples/rush/miter-guide-affine-hollow-corrected.r','rush-miter-guide-affine-hollow-corrected.step',2),
 authoredRushFixture('examples/rush/miter-authored-frame-hollow-corrected.r','rush-miter-authored-frame-hollow-corrected.step',1),
 authoredRushFixture('examples/rush/miter-combined-frame-affine-hollow-corrected.r','rush-miter-combined-frame-affine-hollow-corrected.step',2),

 authoredRushFixture('examples/rush/miter-guide-hollow.r','rush-guide-hollow-miter.step',1),
 authoredRushFixture('examples/rush/miter-guide-affine-hollow.r','rush-guide-affine-hollow-miter.step',2),
 authoredRushFixture('examples/rush/miter-authored-frame-hollow.r','rush-authored-frame-hollow-miter.step',1),
 authoredRushFixture('examples/rush/miter-combined-frame-affine-hollow.r','rush-combined-frame-affine-hollow-miter.step',2),
 {file:'rush-circle-corrected-scale-twist.step',model:circleModel as NurbsBrep,faces:circleModel.faces.length,shells:1,capHoleFaces:2,requireNativeSolid:true,construction:circleConstruction,sourceSha256:createHash('sha256').update(circleSource).digest('hex'),expectedVolume:progressiveVolume},
 {file:'rush-circle-corrected-spatial.step',model:spatialCircleModel as NurbsBrep,faces:spatialCircleModel.faces.length,shells:1,capHoleFaces:2,requireNativeSolid:true,construction:spatialCircleConstruction,sourceSha256:createHash('sha256').update(spatialCircleSource).digest('hex'),expectedVolume:Math.PI*(.5**2-.2**2)*(20+Math.hypot(10,5))},
 {file:'rush-circle-corrected-affine.step',model:affineCircleModel as NurbsBrep,faces:affineCircleModel.faces.length,shells:1,capHoleFaces:2,requireNativeSolid:true,construction:affineCircleConstruction,sourceSha256:createHash('sha256').update(affineCircleSource).digest('hex'),expectedVolume:Math.PI*(.5**2-.2**2)*10*1.5},
 {file:'rush-circle-corrected-oblique.step',model:obliqueCircleModel as NurbsBrep,faces:obliqueCircleModel.faces.length,shells:1,capHoleFaces:2,requireNativeSolid:true,construction:obliqueCircleConstruction,sourceSha256:createHash('sha256').update(obliqueCircleSource).digest('hex'),expectedVolume:Math.PI*(.5**2-.2**2)*10},
 {file:'rush-rational-curved-guide-frame-affine.step',model:curvedGuideModel as NurbsBrep,faces:curvedGuideModel.faces.length,shells:1,capHoleFaces:2,requireNativeSolid:true,construction:curvedGuideConstruction,sourceSha256:createHash('sha256').update(curvedGuideSource).digest('hex'),expectedVolume:curvedGuideVolume},
 {file:'rush-station-g2-hollow.step',model:stationModel as NurbsBrep,faces:stationModel.faces.length,shells:1,capHoleFaces:2,requireNativeSolid:true,construction:stationConstruction,sourceSha256:createHash('sha256').update(stationSource).digest('hex'),expectedVolume:Math.PI*(.5**2-.25**2)*10*1.5},
 {file:'rush-g1-profile-frame-guide-affine.step',model:g1Model as NurbsBrep,faces:g1Model.faces.length,shells:1,capHoleFaces:0,capFaces:[g1Model.faces.length-2,g1Model.faces.length-1],requireNativeSolid:true,construction:g1Construction,sourceSha256:createHash('sha256').update(g1Source).digest('hex'),expectedVolume:512/7},
 {file:'closed-frame-guide-affine-hollow-miter.step',model:closedJoint.model,faces:closedJoint.model.faces.length,shells:2,capHoleFaces:0,requireNativeSolid:true,construction:closedJoint.approximation.report,boundaryCertificate:closedJoint.boundaryCertificate,expectedVolume:closedJointVolume},
 {file:'closed-authored-affine-hollow-miter.step',model:closedAffine.model,faces:32,shells:2,capHoleFaces:0,
  requireNativeSolid:true,admission:closedAffine.approximation.report,
  retainedCorrespondence:closedAffine.retainedCorrespondence,
  expectedVolume:Math.PI*(.5**2-.2**2)*40*2},
 {file:'open-straight-hollow-miter.step',model:straight.model,faces:10,shells:1,capHoleFaces:2,
  construction:straight.report,expectedVolume:Math.PI*(.5**2-.2**2)*10},
 {file:'progressive-scale-twist-hollow-miter.step',model:progressive.model,faces:progressive.model.faces.length,shells:1,capHoleFaces:2,expectedVolume:progressiveVolume,admission:progressive.approximation.report},
 {file:'open-spatial-hollow-miter.step',model:body.model,faces:26,shells:1,capHoleFaces:2,expectedVolume:Math.PI*(.5**2-.2**2)*(20+Math.hypot(10,5))},
 {file:'open-spatial-bounded-correction.step',model:correctedSpatial,faces:26,shells:1,capHoleFaces:2,
  correction:{...correction,sections:undefined},correctionCorrespondence,retainedWallCorrectionUpper:correction.wallDisplacementUpper,
  expectedVolume:Math.PI*(.5**2-.2**2)*(20+Math.hypot(10,5))},
 {file:'open-spatial-constructor-correction.step',model:constructorCorrected.model,faces:26,shells:1,capHoleFaces:2,
  construction:constructorCorrected.report,expectedVolume:Math.PI*(.5**2-.2**2)*(20+Math.hypot(10,5))},
 {file:'closed-planar-hollow-miter.step',model:closed.model,faces:32,shells:2,capHoleFaces:0,construction:closed.report,expectedVolume:Math.PI*(.5**2-.2**2)*40},
 {file:'affine-oblique-hollow-miter.step',model:oblique,faces:10,shells:1,capHoleFaces:2,
  affineTransform:obliqueTransform,retainedCaps:obliqueCaps,expectedVolume:Math.PI*(.5**2-.2**2)*10},
 {file:'affine-oblique-dyadic-hollow.step',model:dyadicOblique,faces:10,shells:1,capHoleFaces:2,
  affineTransform:obliqueTransform,retainedCaps:dyadicCaps,expectedVolume:Math.PI*(.5**2-.25**2)*10},
 {file:'rush-authored-affine-hollow-miter.step',model:affineRushModel,faces:affineRushModel.faces.length,shells:1,capHoleFaces:2,
  requireNativeSolid:true,
  sourceSha256:createHash('sha256').update(affineRushSource).digest('hex'),construction:affineRushConstruction,
  expectedVolume:Math.PI*(.5**2-.2**2)*10*1.5},
]
const cases=fixtures.map(({model,...fixture})=>{
 const auditBudgets={
  ...DEFAULT_SWEEP_VOLUME_BUDGETS,maxTrimPairs:10000,maxTrimCells:100000,
  maxTrimDomainCells:1000000,maxPairs:10000,maxCells:100000,maxDomainCells:1000000,
  cellsPerPair:1000,domainCellsPerPair:10000,
 }
 const caps='capFaces' in fixture?fixture.capFaces:fixture.capHoleFaces?[model.faces.length-2,model.faces.length-1]:[]
 const nativeProfileSmoothness=inspectMiterProfileSmoothness(model,caps)
 const nativeRetainedWallCharts=inspectSweepRetainedWallCharts(model,caps,100000)
 const nativeVolume=inspectSweepVolume(model,caps,auditBudgets)
 if('requireNativeSolid' in fixture&&fixture.requireNativeSolid&&!nativeVolume.solidGeometryCertified)throw Error('Required native Solid certificate refused: '+fixture.file)
 const nativeBoundary=caps.length?inspectSweepEmbedding(model,caps,auditBudgets):null
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
 const wallSamples=walls.map(face=>{
  const surface=face.surface
  return [0,.17,.5,.83,1].flatMap(u=>[0,.13,.5,.87,1].map(v=>{const e=evaluateNurbsSurface(surface,u,v);return {u,v,point:e.point,du:e.du,dv:e.dv}}))
 })
 if(wallSamples.length!==fixture.faces-caps.length)throw Error('Unexpected wall sample coverage')
 return {...fixture,capFaces:caps,nativeProfileSmoothness,nativeRetainedWallCharts,nativeVolume,nativeBoundary,nativeExactUses,exactWork,edges:model.edges.length,faceLoops:model.faces.map(face=>({outer:model.loops[face.outer]!.coedges.map(c=>c.edge),holes:face.holes.map(loop=>model.loops[loop]!.coedges.map(c=>c.edge))})),edgeCurves:model.edges.map(edge=>({curve:edge.curve,samples:[0,.25,.5,.75,1].map(u=>({u,point:evaluateNurbsCurve(edge.curve,u).point}))})),wallCoedges:walls.map(face=>model.loops[face.outer]!.coedges.map(c=>({edge:c.edge,reversed:c.reversed,pcurve:c.pcurve}))),wallSurfaces:walls.map(face=>face.surface),wallSamples,surfaceToleranceMm:1e-8,solids:1,sha256:createHash('sha256').update(text).digest('hex'),relativeVolumeTolerance:1e-7}
})
writeFileSync(resolve(root,'manifest.json'),JSON.stringify({schema:'sweep-external-step/2',units:'mm',artifactProvenance,cases},null,2)+'\n')
console.log(root)
