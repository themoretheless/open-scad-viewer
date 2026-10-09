

import {writeFileSync,readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {deepStrictEqual} from 'node:assert'
import {createHash} from 'node:crypto'
import {circleNurbsCurve,miterNurbsProfileSections} from '../src/services/nurbsConstructors'

import {reverseNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'
import {createMiterBrepProfileBody,createProgressiveMiterBrepProfileBody,createRationalBrepSectionLoft,transformNurbsBrep} from '../src/services/geometry/brep'

import {projectSweepSections} from '../src/services/nurbsSectionProjection'
import {inspectSweepRetainedCorrespondence,inspectSweepRetainedCaps} from '../src/services/sweepRetainedCorrespondence'

import {DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'

import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
import {readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
import {miterLawMatrixSource,miterLawMatrixName,type MiterLawMatrixMode} from './miter-law-matrix-sources'
import type {NurbsBrep} from '../src/services/geometry/brep'

export function buildMiterStepFixtures(root:string){
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
const affineRushGraph=compileRushFrontend(affineRushSource)
const affineRushNode=affineRushGraph.document.nodes.find(node=>node.op==='brep_progressive_miter_sweep')!
const affineRushBuilt=buildOwnNurbs(affineRushGraph.document,{action:'build'})
const {kind:affineRushKind,...affineRushDefinition}=affineRushBuilt.report.definitions[affineRushNode.id] as unknown as NurbsBrep & {kind:string}
if(affineRushKind!=='brep')throw Error('Affine Rush source did not retain a B-rep')
const affineRushModel=affineRushDefinition as NurbsBrep
const affineRushConstruction=affineRushBuilt.report.construction![affineRushNode.id]
const authoredRushFixture=(sourcePath:string,file:string,areaMultiplier:number,requireNativeSolid=true,parallel:[boolean,boolean]=[true,true])=>{
 const source=readFileSync(sourcePath,'utf8')
 const graph=compileRushFrontend(source)
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
// Exact same source requests as the public law matrix; independent reference
// stages inspect exported retained geometry and never reuse a source volume.
const miterLawMatrixFixture=(mode:MiterLawMatrixMode)=>{
 const source=miterLawMatrixSource(mode),name=miterLawMatrixName(mode)
 writeFileSync(resolve(root,name+'.r'),source)
 const document=compileRushFrontend(source).document
 const node=document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const built=buildOwnNurbs(document,{action:'build',display:{segments:4,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing native law matrix boundary: '+name)
 const evidence=readSweepViewportEvidence(built.nativeGeometry)
 if(!evidence?.continuousBound||!evidence.boundaryErrorWithinBudget||!evidence.solidGeometryCertified)throw Error('Incomplete law matrix boundary/Solid: '+name)
 const construction=built.report.construction![node.id]
 deepStrictEqual([construction.affineLawsApplied,construction.authoredFramesApplied,construction.orientationGuideApplied],[mode.affine,mode.authored,mode.guided])
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry as NurbsBrep
 const capFaces=mode.closed?[]:[model.faces.length-2,model.faces.length-1]
 const reversed=(id:number)=>model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===id))[0]!.reversed
 return {file:name+'.step',model,faces:model.faces.length,shells:mode.closed?2:1,capHoleFaces:mode.closed?0:2,capFaces,
  requireNativeSolid:true,expectedVolume:0,volumeReferenceMethod:'rational-boundary-gauss-reference',lawMatrixMode:mode,
  sourceFile:name+'.r',sourceSha256:createHash('sha256').update(source).digest('hex'),construction,boundaryEvidence:evidence,
  outwardCapContours:capFaces.map(id=>{const face=model.faces[id]!;return [face.outer,...face.holes].flatMap(wire=>
   (reversed(id)?[...model.loops[wire]!.coedges].reverse():model.loops[wire]!.coedges).map(use=>
    use.reversed!==reversed(id)?reverseNurbsCurve(model.edges[use.edge]!.curve):model.edges[use.edge]!.curve))})}
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
 const compiled=compileRushFrontend(source)
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
const circleGraph=compileRushFrontend(circleSource)
const circleNode=circleGraph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
const circleBuilt=buildOwnNurbs(circleGraph.document,{action:'build'})
const {kind:circleKind,...circleModel}=circleBuilt.report.definitions[circleNode.id] as unknown as NurbsBrep & {kind:string}
if(circleKind!=='brep')throw Error('Circle correction did not retain B-rep')
const circleConstruction=circleBuilt.report.construction![circleNode.id] as any
if(!circleConstruction.profileSmoothness.profile.exactG1G2Certified||circleConstruction.boundaryErrorWithinBudget!==true||!circleConstruction.volume.solidGeometryCertified)throw Error('Circle corrected Rush fixture lacked required proofs')
const spatialCircleSource=readFileSync('examples/rush/progressive-miter-spatial-circle-corrected-hollow.r','utf8')
const spatialCircleGraph=compileRushFrontend(spatialCircleSource)
const spatialCircleNode=spatialCircleGraph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
const spatialCircleBuilt=buildOwnNurbs(spatialCircleGraph.document,{action:'build'})
const {kind:spatialCircleKind,...spatialCircleModel}=spatialCircleBuilt.report.definitions[spatialCircleNode.id] as unknown as NurbsBrep & {kind:string}
const spatialCircleConstruction=spatialCircleBuilt.report.construction![spatialCircleNode.id] as any
if(spatialCircleKind!=='brep'||!spatialCircleConstruction.profileSmoothness.profile.exactG1G2Certified||spatialCircleConstruction.boundaryErrorWithinBudget!==true||!spatialCircleConstruction.volume.solidGeometryCertified)throw Error('Spatial circle corrected Rush fixture lacked required proofs')
const affineCircleSource=readFileSync('examples/rush/progressive-miter-affine-circle-corrected-hollow.r','utf8')
const affineCircleGraph=compileRushFrontend(affineCircleSource)
const affineCircleNode=affineCircleGraph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
const affineCircleBuilt=buildOwnNurbs(affineCircleGraph.document,{action:'build'})
const {kind:affineCircleKind,...affineCircleModel}=affineCircleBuilt.report.definitions[affineCircleNode.id] as unknown as NurbsBrep & {kind:string}
const affineCircleConstruction=affineCircleBuilt.report.construction![affineCircleNode.id] as any
if(affineCircleKind!=='brep'||!affineCircleConstruction.affineLawsApplied||!affineCircleConstruction.profileSmoothness.profile.exactG1G2Certified||affineCircleConstruction.boundaryErrorWithinBudget!==true||!affineCircleConstruction.volume.solidGeometryCertified)throw Error('Affine circle corrected Rush fixture lacked required proofs')
const obliqueCircleSource=readFileSync('examples/rush/progressive-miter-oblique-circle-corrected-hollow.r','utf8')
const obliqueCircleGraph=compileRushFrontend(obliqueCircleSource)
const obliqueCircleNode=obliqueCircleGraph.document.nodes.find(n=>n.op==='transform')!
const obliqueCircleBuilt=buildOwnNurbs(obliqueCircleGraph.document,{action:'build'})
const {kind:obliqueCircleKind,...obliqueCircleModel}=obliqueCircleBuilt.report.definitions[obliqueCircleNode.id] as unknown as NurbsBrep & {kind:string}
const obliqueCircleConstruction=obliqueCircleBuilt.report.construction![obliqueCircleNode.id] as any
if(obliqueCircleKind!=='brep'||!obliqueCircleConstruction.profileSmoothness.profile.exactG1G2Certified||obliqueCircleConstruction.boundaryErrorWithinBudget!==true||!obliqueCircleConstruction.volume.solidGeometryCertified||obliqueCircleConstruction.placement.arithmeticErrorUpper!==0)throw Error('Oblique circle corrected Rush fixture lacked required proofs')
const curvedGuideSource=readFileSync('examples/rush/miter-rational-curved-guide-frame-affine-hollow.r','utf8')
const curvedGuideGraph=compileRushFrontend(curvedGuideSource)
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
const stationGraph=compileRushFrontend(stationSource)
const stationNode=stationGraph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
const stationBuilt=buildOwnNurbs(stationGraph.document,{action:'build'})
const {kind:stationKind,...stationModel}=stationBuilt.report.definitions[stationNode.id] as unknown as NurbsBrep & {kind:string}
const stationConstruction=stationBuilt.report.construction![stationNode.id] as any
if(stationKind!=='brep'||!stationConstruction.profileSmoothness.station.stationG2Certified||!stationConstruction.volume.solidGeometryCertified)throw Error('Station G2 Rush fixture lacked required proofs')
const g1Source=readFileSync('examples/rush/miter-g1-profile-frame-guide-affine.r','utf8')
const g1Graph=compileRushFrontend(g1Source)
const g1Node=g1Graph.document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
const g1Built=buildOwnNurbs(g1Graph.document,{action:'build'})
const {kind:g1Kind,...g1Model}=g1Built.report.definitions[g1Node.id] as unknown as NurbsBrep & {kind:string}
if(g1Kind!=='brep')throw Error('G1 Rush profile did not retain a body')
const g1Construction=g1Built.report.construction![g1Node.id] as {profileSmoothness:{profileG1Certified:boolean;profile:{exactG1G2Certified:boolean}}}
if(!g1Construction.profileSmoothness.profileG1Certified||g1Construction.profileSmoothness.profile.exactG1G2Certified)throw Error('G1/G2 Rush distinction lost')

return {law,miterLawMatrixFixture,authoredRushFixture,unsegmentedFixture,circleModel,circleConstruction,circleSource,progressiveVolume,spatialCircleModel,spatialCircleConstruction,spatialCircleSource,affineCircleModel,affineCircleConstruction,affineCircleSource,obliqueCircleModel,obliqueCircleConstruction,obliqueCircleSource,curvedGuideModel,curvedGuideConstruction,curvedGuideSource,curvedGuideVolume,stationModel,stationConstruction,stationSource,g1Model,g1Construction,g1Source,closedJoint,closedJointVolume,closedAffine,straight,progressive,body,correctedSpatial,correction,correctionCorrespondence,constructorCorrected,closed,oblique,obliqueTransform,obliqueCaps,dyadicOblique,dyadicCaps,affineRushModel,affineRushSource,affineRushConstruction}
}
