import {callGeometryRust} from '../src/services/geometry/kernel'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
import {mkdirSync,writeFileSync,readFileSync,appendFileSync,renameSync} from 'node:fs'
import {resolve} from 'node:path'
import {deepStrictEqual} from 'node:assert'
import {createHash} from 'node:crypto'
import {bezierNurbsCurve,circleNurbsCurve,miterNurbsProfileSections} from '../src/services/nurbsConstructors'
import type {ProgressiveGuidedSurfaceSweepOptions} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve,evaluateNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'
import {createProgressiveBrepProfileBody,createMiterBrepProfileBody,createProgressiveMiterBrepProfileBody,createRationalBrepSectionLoft,transformNurbsBrep} from '../src/services/geometry/brep'
import type {ProgressiveBrepSweepOptions} from '../src/services/geometry/brep'
import {projectSweepSections} from '../src/services/nurbsSectionProjection'
import {inspectSweepRetainedCorrespondence,inspectSweepRetainedCaps} from '../src/services/sweepRetainedCorrespondence'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {exportDirectStepV5} from '../src/services/cadNurbsStep'
import {inspectSweepVolume,inspectSweepEmbedding,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'
import {inspectMiterProfileSmoothness} from '../src/services/miterProfileSmoothness'
import {inspectSweepRetainedWallCharts} from '../src/services/nurbsSweepRetainedCharts'
import {inspectSweepCoedgeExact} from '../src/services/nurbsSweepAudit'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
import {readSweepBodyBoundaryViewportEvidence,readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'
import {miterLawMatrixModes,miterLawMatrixSource,miterLawMatrixName,type MiterLawMatrixMode} from './miter-law-matrix-sources'
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
// Independent fixed-frame conical annulus: linear radius 1 -> 2 over10mm.
// Its analytic volume is pi*(3^2-1^2)*10*(1+2+4)/3.
const fixedFrameFixture=(orientation:'fixed'|'fixed_normal'|'rmf'='fixed')=>{
 const rings=[[circleNurbsCurve([0,0,0],[0,0,1],3)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],1))]]
 const scalar=(values:number[])=>({degree:1,knots:[0,0,1,1],values,weights:[1,1]})
 const body=createProgressiveBrepProfileBody(rings,bezierNurbsCurve([[0,0,0],[0,0,10]]),scalar([1,2]),scalar([0,0]),{
  normal:[1,0,0],orientation,initialSections:2,maxSections:2,maxDeviation:.01,
 })
 const report=body.approximation.report
 if(!report.accepted||!report.continuousBound||report.continuousErrorScope!=='retained-patches-relative-to-original-profile-transport')throw Error('Fixed source retained-patch certificate is required')
 if(body.globalEmbeddingCertified!==false)throw Error('Fixed construction must keep global admission separate')
 return {file:orientation==='rmf'?'rmf-straight-linear-scale-hollow.step':orientation==='fixed'?'fixed-frame-linear-scale-hollow.step':'fixed-normal-linear-scale-hollow.step',model:body.model,faces:body.model.faces.length,shells:1,capHoleFaces:2,
  expectedVolume:Math.PI*560/3,volumeReferenceMethod:'canonical-generator-polynomial-integral',
  requireNativeSolid:false,construction:report}
}

const obliqueRmfHollowFixture=()=>{
 const axis:[number,number,number]=[3,4,0]
 const rings=[[circleNurbsCurve([0,0,0],axis,.1)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],axis,.05))]]
 const scalar=(values:number[])=>({degree:1,knots:[0,0,1,1],values,weights:[1,1]})
 const body=createProgressiveBrepProfileBody(rings,bezierNurbsCurve([[0,0,0],axis]),scalar([1,2]),scalar([0,0]),{
  normal:[0,0,1],orientation:'rmf',initialSections:2,maxSections:2,maxDeviation:.01,
 })
 const report=body.approximation.report
 if(!report.accepted||!report.continuousBound||report.continuousErrorScope!=='retained-patches-relative-to-original-profile-transport')throw Error('Oblique original RMF retained-patch certificate required')
 if(body.globalEmbeddingCertified!==false)throw Error('RMF retained error must preserve separate global admission')
 return {file:'rmf-oblique-linear-scale-hollow.step',model:body.model,faces:body.model.faces.length,shells:1,capHoleFaces:2,
  expectedVolume:Math.PI*(.1**2-.05**2)*5*7/3,
  volumeReferenceMethod:'rational-boundary-gauss-reference',requireNativeSolid:false,construction:report,
  wallFaceReversed:body.model.faces.slice(0,-2).map((_,id)=>body.model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===id))[0]!.reversed),
  outwardCapContours:body.model.faces.slice(-2).map((face,j)=>{
   const id=body.model.faces.length-2+j
   const uses=body.model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===id))
   if(uses.length!==1)throw Error('Rational volume reference requires unique cap ownership')
   const reversed=uses[0]!.reversed
   return [face.outer,...face.holes].flatMap(wire=>(reversed?[...body.model.loops[wire]!.coedges].reverse():body.model.loops[wire]!.coedges).map(use=>{
    const curve=body.model.edges[use.edge]!.curve
    return use.reversed!==reversed?reverseNurbsCurve(curve):curve
   }))
  })}
}

const closedPlanarRmfBodyFixture=()=>{
 const corners=[[4.9,0,-.1],[5.1,0,-.1],[5.1,0,.1],[4.9,0,.1]]
 const loops=[corners.map((p,i)=>bezierNurbsCurve([p,corners[(i+1)%4]!]))]
 const scalar=(values:number[])=>({degree:1,knots:[0,0,1,1],values,weights:[1,1]})
 const body=createProgressiveBrepProfileBody(loops,circleNurbsCurve([0,0,0],[0,0,1],5),
  scalar([1,1]),scalar([0,360]),{normal:[0,0,1],orientation:'rmf',
   initialSections:5,maxSections:129,maxDeviation:.01})
 const report=body.approximation.report
 if(!report.closedPath||!report.accepted||!report.continuousBound
  ||!body.boundaryContinuousBound||!body.boundaryErrorWithinBudget)
  throw Error('Closed planar RMF requires complete original/retained boundary error')
 return {file:'rmf-closed-planar-full-turn-rectangular.step',model:body.model,
  faces:body.model.faces.length,shells:1,capHoleFaces:0,capFaces:[],
  expectedVolume:0,volumeReferenceMethod:'rational-boundary-gauss-reference',requireNativeSolid:true,
  construction:{...report,boundaryErrorUpper:body.boundaryErrorUpper,boundaryContinuousBound:body.boundaryContinuousBound},
  wallSurfaces:body.model.faces.map(f=>f.surface),
  wallFaceReversed:body.model.faces.map((_,id)=>{
   const uses=body.model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===id))
   if(uses.length!==1)throw Error('Closed rational volume requires unique wall ownership')
   return uses[0]!.reversed
  }),outwardCapContours:[]}
}

const frenetHollowFixture=(orientation:'frenet'|'rmf'='frenet',rectangular=false)=>{
 const corners=[[0,-.1,-.1],[0,.1,-.1],[0,.1,.1],[0,-.1,.1]]
 const rings=rectangular?[corners.map((p,i)=>bezierNurbsCurve([p,corners[(i+1)%4]!]))]:[[circleNurbsCurve([0,0,0],[1,0,0],.1)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],[1,0,0],.05))]]
 const scalar=(x:number)=>({degree:1,knots:[0,0,1,1],values:[x,x],weights:[1,1]})
 const body=createProgressiveBrepProfileBody(rings,bezierNurbsCurve([[0,0,0],[.5,0,0],[1,1,0]]),scalar(1),scalar(0),{
  normal:[0,0,1],orientation,initialSections:3,maxSections:129,maxDeviation:.01,
 })
 const report=body.approximation.report
 if(!report.accepted||!report.continuousBound||report.continuousErrorScope!=='retained-patches-relative-to-original-profile-transport')throw Error('Frenet original retained-patch certificate required')
 if(body.globalEmbeddingCertified!==false)throw Error('Frenet frame/error proof must keep global body admission separate')
 if(rectangular&&(!body.boundaryContinuousBound||!body.boundaryErrorWithinBudget))throw Error('Curved planar RMF full boundary proof required')
 // Nominal original tube volume. The independent Fraction generator integral
 // replaces this with the exact retained section volume before OCCT comparison.
 return {file:rectangular?'rmf-planar-quadratic-rectangular.step':orientation==='rmf'?'rmf-planar-quadratic-hollow.step':'frenet-quadratic-hollow.step',model:body.model,faces:body.model.faces.length,shells:1,capHoleFaces:rectangular?0:2,
  capFaces:[body.model.faces.length-2,body.model.faces.length-1],
  expectedVolume:Math.PI*(.1**2-.05**2)*(.5*Math.sqrt(5)+.25*Math.asinh(2)),
  volumeReferenceMethod:'rational-boundary-gauss-reference',requireNativeSolid:rectangular,construction:{...report,boundaryErrorUpper:body.boundaryErrorUpper,boundaryContinuousBound:body.boundaryContinuousBound},
  wallFaceReversed:body.model.faces.slice(0,-2).map((_,id)=>body.model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===id))[0]!.reversed),
  outwardCapContours:body.model.faces.slice(-2).map((face,j)=>{
   const id=body.model.faces.length-2+j
   const uses=body.model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===id))
   if(uses.length!==1)throw Error('Rational volume reference requires unique cap ownership')
   const reversed=uses[0]!.reversed
   return [face.outer,...face.holes].flatMap(wire=>(reversed?[...body.model.loops[wire]!.coedges].reverse():body.model.loops[wire]!.coedges).map(use=>{
    const curve=body.model.edges[use.edge]!.curve
    return use.reversed!==reversed?reverseNurbsCurve(curve):curve
   }))
  })}
}

// Dyadic hollow rectangular prisms have independently authored exact volumes.
// Their native full-boundary proof is distinct from the native Solid audit below.
const certifiedRectangularBodyFixture=(mode:'plain'|'affine'|'authored'|'guide'|'arc-rmf'|'arc-fixed'|'arc-guide'|'arc-authored'|'arc-curved-authored'|'arc-curved-fixed'|'arc-curved-fixed-normal'|'arc-curved-frenet'|'arc-curved-guided'|'arc-curved-contact'|'arc-curved-planar-rmf'|'arc-curved-nonaxial-planar-rmf')=>{
 const wire=(points:number[][])=>points.map((p,i)=>bezierNurbsCurve([p,points[(i+1)%4]!]))
 const rings=mode==='arc-curved-nonaxial-planar-rmf'
  ?[wire([[0,0,0],[.1,.1,0],[.1,.1,.1],[0,0,.1]]),wire([[.025,.025,.025],[.025,.025,.075],[.075,.075,.075],[.075,.075,.025]])]
  :mode==='arc-curved-contact'
  ?[wire([[1,0,0],[1,.1,0],[.9,.1,0],[.9,0,0]]),wire([[.925,.025,0],[.925,.075,0],[.975,.075,0],[.975,.025,0]])]
  :mode==='arc-curved-frenet'
  ?[wire([[0,0,0],[0,.1,0],[0,.1,.1],[0,0,.1]]),wire([[0,.025,.025],[0,.025,.075],[0,.075,.075],[0,.075,.025]])]
  :mode==='arc-curved-fixed-normal'||(mode==='arc-curved-planar-rmf'||mode==='arc-curved-nonaxial-planar-rmf')||(mode==='arc-curved-guided'||mode==='arc-curved-contact')
  ?[wire([[0,0,0],[.1,0,0],[.1,.1,0],[0,.1,0]]),wire([[.025,.025,0],[.025,.075,0],[.075,.075,0],[.075,.025,0]])]
  :[wire([[0,0,0],[2,0,0],[2,2,0],[0,2,0]]),wire([[.5,.5,0],[.5,1.5,0],[1.5,1.5,0],[1.5,.5,0]])]
 const vector=(v:[number,number,number])=>({degree:1,knots:[0,0,1,1],values:[v,v],weights:[1,1]})
 const options:ProgressiveBrepSweepOptions={orientation:'rmf',normal:[1,0,0],initialSections:3,maxSections:3,maxDeviation:.01}
 if(mode!=='plain'){options.axisScale=vector([2,3,1]);options.centerLaw=vector([0,0,0])}
 if(mode==='authored'||mode==='arc-authored'||mode==='arc-curved-authored'){options.orientation='authored';options.frameAxis=vector([0,0,1]);options.frameNormal=vector([1,0,0])}
 if(mode==='guide'||mode==='arc-guide')options.orientationGuide=bezierNurbsCurve([[1,0,0],[1,0,10]],mode==='arc-guide'?[1,4]:undefined)
 if(mode.startsWith('arc-')){options.spacing='arc_length';options.lengthTolerance=.001;options.lengthMaxCells=100000}
 if(mode==='arc-fixed'||mode==='arc-curved-fixed')options.orientation='fixed'
 const curved=(mode==='arc-curved-planar-rmf'||mode==='arc-curved-nonaxial-planar-rmf')||mode==='arc-curved-authored'||mode==='arc-curved-fixed'||mode==='arc-curved-fixed-normal'||mode==='arc-curved-frenet'||(mode==='arc-curved-guided'||mode==='arc-curved-contact')
 if(curved){options.maxSections=9;options.maxDeviation=.2}
 if((mode==='arc-curved-planar-rmf'||mode==='arc-curved-nonaxial-planar-rmf')||mode==='arc-curved-fixed-normal'||mode==='arc-curved-frenet'||(mode==='arc-curved-guided'||mode==='arc-curved-contact')){options.orientation=mode==='arc-curved-frenet'?'frenet':((mode==='arc-curved-planar-rmf'||mode==='arc-curved-nonaxial-planar-rmf')||mode==='arc-curved-guided'||mode==='arc-curved-contact')?'rmf':'fixed_normal';options.maxSections=17;options.maxDeviation=2;options.capCorrection={quantum:2**-40,tolerance:1e-9,maxWork:1000000}}
 if((mode==='arc-curved-guided'||mode==='arc-curved-contact'))options.orientationGuide=bezierNurbsCurve([[1,0,0],[1,0,.5],[1,1,1]])
 if(mode==='arc-curved-contact')options.contactAnchor={profileIndex:0,parameter:0}
 if(mode==='arc-curved-nonaxial-planar-rmf')options.normal=[1,1,0]
 const sourcePath=mode==='arc-curved-nonaxial-planar-rmf'?bezierNurbsCurve([[0,0,0],[.5,-.5,0],[1,-1,1]]):mode==='arc-curved-frenet'?bezierNurbsCurve([[0,0,0],[.5,0,0],[1,1,0]]):curved?bezierNurbsCurve([[0,0,0],[0,0,.5],[0,1,1]]):bezierNurbsCurve([[0,0,0],[0,0,10]],mode.startsWith('arc-')?[1,2]:undefined)
 const body=createProgressiveBrepProfileBody(rings,sourcePath,law(1,1),law(0,0),options)
 if(!body.boundaryContinuousBound||!body.boundaryErrorWithinBudget||!body.retainedCaps?.exact||!body.retainedWalls?.certified)throw Error('Complete rectangular body boundary proof required: '+mode)
 const reversed=(id:number)=>body.model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===id))[0]!.reversed
 return {file:`progressive-certified-${mode}-rectangular-hollow.step`,model:body.model,faces:body.model.faces.length,shells:1,capHoleFaces:2,
  expectedVolume:curved?0:mode==='plain'?30:180,volumeReferenceMethod:curved?'rational-boundary-gauss-reference':'analytic-hollow-rectangular-prism',requireNativeSolid:true,
  ...(curved?{outwardCapContours:body.model.faces.slice(-2).map((face,j)=>[face.outer,...face.holes].flatMap(wire=>
   (reversed(body.model.faces.length-2+j)?[...body.model.loops[wire]!.coedges].reverse():body.model.loops[wire]!.coedges).map(use=>
    use.reversed!==reversed(body.model.faces.length-2+j)?reverseNurbsCurve(body.model.edges[use.edge]!.curve):body.model.edges[use.edge]!.curve)))}:{}),
  ...(!curved?{analyticPrism:{outer:[2,2],hole:[1,1],height:10,axisScale:mode==='plain'?[1,1]:[2,3]}}:{}),
  construction:{approximation:body.approximation.report,retainedCaps:body.retainedCaps,retainedWalls:body.retainedWalls,capProjection:body.capProjection,capCorrectionErrorUpper:body.capCorrectionErrorUpper,
   filledCapErrorUpper:body.filledCapErrorUpper,boundaryErrorUpper:body.boundaryErrorUpper,boundaryContinuousBound:body.boundaryContinuousBound,
   boundaryErrorWithinBudget:body.boundaryErrorWithinBudget,globalEmbeddingCertified:body.globalEmbeddingCertified}}
}
const multispanAuthoredBodyFixture=()=>{
 const axis={degree:3,knots:[0,0,0,0,.25,.5,.75,1,1,1,1],
  values:[[0,0,1],[.125,0,1],[.375,.125,1],[0,.25,1],[-.375,.125,1],[-.125,0,1],[0,0,1]] as [number,number,number][],
  weights:[1,1,1,2,1,1,1]}
 const normal={degree:1,knots:[0,0,1,1],values:[[1,0,0],[1,0,0]] as [number,number,number][],weights:[1,1]}
 const body=createProgressiveBrepProfileBody([[circleNurbsCurve([0,0,0],[0,0,1],.5)]],
  bezierNurbsCurve([[0,0,0],[0,0,10]]),law(1,1),law(0,0),
  {orientation:'authored',normal:[1,0,0],frameAxis:axis,frameNormal:normal,
   initialSections:5,maxSections:17,maxDeviation:2})
 if(!body.boundaryContinuousBound||!body.boundaryErrorWithinBudget)throw Error('Multispan authored boundary proof required')
 const reversed=(id:number)=>body.model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===id))[0]!.reversed
 return {file:'original-rational-multispan-authored-frame.step',model:body.model,
  faces:body.model.faces.length,shells:1,capHoleFaces:0,capFaces:[body.model.faces.length-2,body.model.faces.length-1],
  expectedVolume:0,volumeReferenceMethod:'rational-boundary-gauss-reference',requireNativeSolid:true,
  sourceFrameAxis:axis,sourceFrameNormal:normal,
  construction:{approximation:body.approximation.report,boundaryContinuousBound:body.boundaryContinuousBound,
   boundaryErrorUpper:body.boundaryErrorUpper,globalEmbeddingCertified:body.globalEmbeddingCertified},
  outwardCapContours:body.model.faces.slice(-2).map((face,j)=>[face.outer,...face.holes].flatMap(wire=>
   (reversed(body.model.faces.length-2+j)?[...body.model.loops[wire]!.coedges].reverse():body.model.loops[wire]!.coedges).map(use=>
    use.reversed!==reversed(body.model.faces.length-2+j)?reverseNurbsCurve(body.model.edges[use.edge]!.curve):body.model.edges[use.edge]!.curve)))}
}
const closedAuthoredTubeFixture=(spacing:'parameter'|'arc_length'='parameter')=>{
 const path=circleNurbsCurve([0,0,0],[0,0,1],4)
 const axis={degree:path.degree,knots:path.knots,weights:path.weights,
  values:path.controlPoints.map(p=>[-p[1]!/4,p[0]!/4,0] as [number,number,number])}
 const normal={degree:1,knots:[0,0,1,1],weights:[1,1],values:[[0,0,1],[0,0,1]] as [number,number,number][]}
 const body=createProgressiveBrepProfileBody([[circleNurbsCurve([4,0,0],[0,1,0],.2)]],path,law(1,1),law(0,0),
  {orientation:'authored',normal:[0,0,1],frameAxis:axis,frameNormal:normal,initialSections:17,maxSections:65,maxDeviation:2,spacing})
 if(!body.boundaryContinuousBound||!body.boundaryErrorWithinBudget)throw Error('Closed authored tube boundary proof required')
 return {file:spacing==='arc_length'?'closed-original-arc-length-authored-tube.step':'closed-original-rational-authored-tube.step',model:body.model,faces:body.model.faces.length,
  shells:1,capHoleFaces:0,capFaces:[],expectedVolume:0,volumeReferenceMethod:'rational-boundary-gauss-reference',
  requireNativeSolid:true,outwardCapContours:[],sourceFrameAxis:axis,sourceFrameNormal:normal,
  construction:{approximation:body.approximation.report,boundaryContinuousBound:body.boundaryContinuousBound,
   boundaryErrorUpper:body.boundaryErrorUpper,globalEmbeddingCertified:body.globalEmbeddingCertified}}
}
const closedGuidedTubeFixture=(spacing:'parameter'|'arc_length'='parameter')=>{
 const path=circleNurbsCurve([0,0,0],[0,0,1],4)
 const rail=circleNurbsCurve([0,0,1],[0,0,1],4)
 const body=createProgressiveBrepProfileBody([[circleNurbsCurve([4,0,0],[0,1,0],.2)]],path,law(1,1),law(0,0),
  {orientation:'rmf',normal:[0,0,1],orientationGuide:rail,initialSections:17,maxSections:65,maxDeviation:2,spacing})
 if(!body.boundaryContinuousBound||!body.boundaryErrorWithinBudget)throw Error('Closed guided tube boundary proof required')
 return {file:spacing==='arc_length'?'closed-original-arc-length-guided-tube.step':'closed-original-rational-guided-tube.step',model:body.model,faces:body.model.faces.length,
  shells:1,capHoleFaces:0,capFaces:[],expectedVolume:0,volumeReferenceMethod:'rational-boundary-gauss-reference',
  requireNativeSolid:true,outwardCapContours:[],sourceOrientationGuide:rail,
  construction:{approximation:body.approximation.report,boundaryContinuousBound:body.boundaryContinuousBound,
   boundaryErrorUpper:body.boundaryErrorUpper,globalEmbeddingCertified:body.globalEmbeddingCertified}}
}
const closedArcRmfTubeFixture=(conic=false,orientation:'rmf'|'fixed_normal'|'corrected_frenet'='rmf',nonaxial=false)=>{
 const spacing='arc_length' as const
 const path=circleNurbsCurve([0,0,0],[0,0,1],4)
 if(conic)path.weights=path.weights.map(w=>w===1?1:.5)
 if(nonaxial)path.controlPoints=path.controlPoints.map(([x,y])=>[x,-x,y])
 const profile=nonaxial?circleNurbsCurve([4,-4,0],[0,0,1],.2):circleNurbsCurve([4,0,0],[0,1,0],.2)
 const body=createProgressiveBrepProfileBody([[profile]],path,law(1,1),law(0,0),
  {orientation,normal:nonaxial?[1,1,0]:[0,0,1],initialSections:17,maxSections:65,maxDeviation:2,spacing})
 if(!body.boundaryContinuousBound||!body.boundaryErrorWithinBudget)throw Error('Closed arc RMF tube boundary proof required')
 return {file:`closed-original-${nonaxial?'nonaxial-':''}arc-length-${orientation==='rmf'?'rmf':orientation==='fixed_normal'?'fixed-normal':'corrected-planar'}-${conic?'conic':'circle'}-tube.step`,model:body.model,faces:body.model.faces.length,
  shells:1,capHoleFaces:0,capFaces:[],expectedVolume:0,volumeReferenceMethod:'rational-boundary-gauss-reference',
  requireNativeSolid:true,outwardCapContours:[],
  construction:{approximation:body.approximation.report,boundaryContinuousBound:body.boundaryContinuousBound,
   boundaryErrorUpper:body.boundaryErrorUpper,globalEmbeddingCertified:body.globalEmbeddingCertified}}
}
const closedSpatialRmfTubeFixture=(hollow=false,varying=false,periodic=false,parameter=false,multipleHoles=false,rationalNonuniform=false,antipodal=false)=>{
 const original=periodic?(varying?'examples/rush/closed-periodic-spatial-rmf-varying-affine-hollow-body.r':'examples/rush/closed-periodic-spatial-rmf-affine-body-certified.r'):varying?'examples/rush/closed-spatial-rmf-varying-affine-hollow-body.r':`examples/rush/closed-spatial-rmf-affine-${hollow?'hollow-':''}body-certified.r`
 const sourceFile=antipodal?'examples/rush/closed-antipodal-spatial-rmf-affine-hollow-body.r':rationalNonuniform?'examples/rush/closed-rational-nonuniform-spatial-rmf-affine-hollow-body.r':multipleHoles?`examples/rush/closed-periodic-spatial-rmf-${parameter?'parameter-':''}multiple-holes-body.r`:parameter?original.replace('spatial-rmf-','spatial-rmf-parameter-'):original
 const source=readFileSync(sourceFile,'utf8')
 const graph=compileRushFrontend(source).document
 const built=buildOwnNurbs(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
 if(!('nativeGeometry' in built))throw Error('Closed spatial RMF retained body missing')
 const evidence=readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)
 if(!evidence?.continuousBound||!evidence.withinBudget)throw Error('Closed spatial RMF boundary error unproved')
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry as NurbsBrep
 if(multipleHoles){
  deepStrictEqual(model.bodies[0]!.innerShells,[1,2,3])
  deepStrictEqual(model.shells.length,4)
 }
 return {file:`closed-original-spatial-rmf-holonomy-${antipodal?'antipodal-':''}${rationalNonuniform?'rational-nonuniform-':''}${parameter?'parameter-':''}${periodic?'periodic-':''}${varying?'varying-':''}affine-${multipleHoles?'three-holes-':hollow?'hollow-':''}tube.step`,model,faces:model.faces.length,
  ...(hollow?{volumeBudgets:{maxSpans:1024,maxLinearCells:100000,maxPairs:20000,maxCells:200000}}:{}),
  shells:multipleHoles?4:hollow?2:1,capHoleFaces:0,capFaces:[],expectedVolume:0,volumeReferenceMethod:'rational-boundary-gauss-reference',
  requireNativeSolid:true,outwardCapContours:[],sourceFile,sourceSha256:createHash('sha256').update(source).digest('hex'),
  construction:built.report.construction,boundaryError:evidence}
}
const correctedFrenetBodyFixture=(sourceFile:string,capHoleFaces=2)=>{
 const source=readFileSync(`examples/rush/${sourceFile}`,'utf8')
 const graph=compileRushFrontend(source).document
 const built=buildOwnNurbs(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
 const native=built.nativeGeometry
 if(!native)throw Error('Missing corrected Frenet retained body: '+sourceFile)
 const evidence=readSweepBodyBoundaryViewportEvidence(native)
 if(!evidence?.continuousBound||!evidence.withinBudget)throw Error('Complete corrected Frenet wall/cap bound required: '+sourceFile)
 const model=JSON.parse(native.geometryJson).geometry as NurbsBrep
 const reversed=(id:number)=>model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===id))[0]!.reversed
 if(/affine-(inflection|zero-curvature)/.test(sourceFile)){
  deepStrictEqual(graph.nodes.find(n=>n.op==='brep_progressive_sweep')?.axis_scale?.values,[[1.25,.75,1],[1.25,.75,1]])
  deepStrictEqual(graph.nodes.find(n=>n.op==='brep_progressive_sweep')?.center_law?.values,[[.002,-.003,0],[.002,-.003,0]])
 }
 const straight=sourceFile.includes('straight')
 if(straight){
  deepStrictEqual(graph.nodes.find(n=>n.op==='brep_progressive_sweep')?.axis_scale?.values,[[2,3,1],[2,3,1]])
  deepStrictEqual(graph.nodes.find(n=>n.op==='brep_progressive_sweep')?.center_law?.values,[[.125,-.25,0],[.125,-.25,0]])
  const poles=model.faces.slice(0,-2).flatMap(face=>face.surface.controlPoints.flat())
  deepStrictEqual([0,1,2].map(k=>[Math.min(...poles.map(p=>p[k]!)),Math.max(...poles.map(p=>p[k]!))]),[[.125,4.125],[-.25,5.75],[0,10]])
 }
 return {file:sourceFile.replace(/\.r$/,'.step'),model,faces:model.faces.length,shells:1,capHoleFaces,
  capFaces:[model.faces.length-2,model.faces.length-1],requireNativeSolid:true,
  sourceSha256:createHash('sha256').update(source).digest('hex'),sourceFile:`examples/rush/${sourceFile}`,
  construction:built.report.construction![native.nodeId],boundaryEvidence:evidence,
  expectedVolume:straight?180:0,volumeReferenceMethod:straight?'analytic-hollow-rectangular-prism':'rational-boundary-gauss-reference',
  ...(straight?{analyticPrism:{outer:[2,2],hole:[1,1],height:10,axisScale:[2,3]}}:{}),
  outwardCapContours:model.faces.slice(-2).map((face,j)=>[face.outer,...face.holes].flatMap(wire=>
   (reversed(model.faces.length-2+j)?[...model.loops[wire]!.coedges].reverse():model.loops[wire]!.coedges).map(use=>
    use.reversed!==reversed(model.faces.length-2+j)?reverseNurbsCurve(model.edges[use.edge]!.curve):model.edges[use.edge]!.curve)))}
}
const sharpProfileStationG2Fixture=()=>{
 const sourcePath='examples/rush/sharp-profile-nonuniform-station-g2-miter.r'
 const source=readFileSync(sourcePath,'utf8'),document=compileRushFrontend(source).document
 const node=document.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
 const built=buildOwnNurbs(document,{action:'build',display:{segments:2,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing retained sharp-profile station G2 body')
 const evidence=readSweepViewportEvidence(built.nativeGeometry)
 if(!evidence?.continuousBound||!evidence.boundaryErrorWithinBudget||!evidence.solidGeometryCertified
  ||evidence.profileG1Certified!==false||evidence.profileG2Certified!==false||evidence.stationG2Certified!==true)
  throw Error('Incomplete sharp-profile/nonuniform-station qualification')
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry as NurbsBrep
 const caps=[model.faces.length-2,model.faces.length-1]
 const reversed=(id:number)=>model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===id))[0]!.reversed
 return {file:'rush-sharp-profile-nonuniform-station-g2-miter.step',model,faces:model.faces.length,
  shells:1,capHoleFaces:0,capFaces:caps,requireNativeSolid:true,requireStationG2:true,
  expectedVolume:0,authoredAnalyticVolume:4*72,volumeReferenceMethod:'rational-boundary-gauss-reference',
  outwardCapContours:caps.map(id=>{const face=model.faces[id]!;return [face.outer,...face.holes].flatMap(wire=>
   (reversed(id)?[...model.loops[wire]!.coedges].reverse():model.loops[wire]!.coedges).map(use=>
    use.reversed!==reversed(id)?reverseNurbsCurve(model.edges[use.edge]!.curve):model.edges[use.edge]!.curve))}),
  sourceFile:sourcePath,sourceSha256:createHash('sha256').update(source).digest('hex'),
  construction:built.report.construction![node.id],boundaryEvidence:evidence}
}
const closedContactBodyFixture=(arc=false,rationalPhase=false)=>{
 const sourceFile=`examples/rush/closed-${arc?'arc-length-':''}${rationalPhase?'rational-phase-':''}contact-hollow-body-boundary.r`
 const source=readFileSync(sourceFile,'utf8')
 const graph=compileRushFrontend(source).document
 const built=buildOwnNurbs(graph,{action:'build',display:{segments:2,subdivisionLevels:0}})
 if(!built.nativeGeometry)throw Error('Missing closed contact retained body')
 const evidence=readSweepBodyBoundaryViewportEvidence(built.nativeGeometry)
 if(!evidence?.continuousBound||!evidence.withinBudget)throw Error('Original complete closed contact bound required')
 const model=JSON.parse(built.nativeGeometry.geometryJson).geometry as NurbsBrep
 if(model.shells.length!==2)throw Error('Closed contact outer/inner shells required')
 return {file:sourceFile.split('/').at(-1)!.replace(/\.r$/,'.step'),model,faces:model.faces.length,
  shells:2,capHoleFaces:0,capFaces:[],outwardCapContours:[],requireNativeSolid:true,
  expectedVolume:0,volumeReferenceMethod:'rational-boundary-gauss-reference',
  sourceFile,sourceSha256:createHash('sha256').update(source).digest('hex'),boundaryEvidence:evidence,
  construction:built.report.construction![built.nativeGeometry.nodeId]}
}
const fixtures=process.argv.includes('--rational-phase-contact-only')?[closedContactBodyFixture(true,true)]
 :process.argv.includes('--closed-contact-only')?[closedContactBodyFixture(),closedContactBodyFixture(true)]
 :process.argv.includes('--sharp-profile-station-only')?[sharpProfileStationG2Fixture()]
 :process.argv.includes('--antipodal-only')?[closedSpatialRmfTubeFixture(true,false,false,false,false,false,true)]:[
 sharpProfileStationG2Fixture(),closedContactBodyFixture(),closedContactBodyFixture(true),closedContactBodyFixture(true,true),
 ...['corrected-frenet-affine-zero-curvature-hollow-body-boundary.r','arc-length-corrected-frenet-affine-zero-curvature-hollow-body-boundary.r','corrected-frenet-affine-inflection-hollow-body-boundary.r','arc-length-corrected-frenet-affine-inflection-hollow-body-boundary.r','corrected-frenet-rational-straight-affine-hollow-body-boundary.r','arc-length-corrected-frenet-rational-straight-affine-hollow-body-boundary.r','rmf-rational-straight-affine-hollow-body-boundary.r','arc-length-rmf-rational-straight-affine-hollow-body-boundary.r','corrected-frenet-straight-affine-hollow-body-boundary.r','corrected-frenet-inflection-hollow-body-boundary.r','arc-length-corrected-frenet-inflection-hollow-body-boundary.r'].map(source=>correctedFrenetBodyFixture(source)),
 closedAuthoredTubeFixture('arc_length'),
 closedAuthoredTubeFixture(),
 multispanAuthoredBodyFixture(),
 ...(['plain','affine','authored','guide'] as const).map(mode=>{
  const rings=[[{degree:2,knots:[0,0,0,.5,1,1,1],controlPoints:[[0,0,0],[.5,-.25,0],[1.5,-.25,0],[2,0,0]],weights:[1,.75,1.25,1],periodic:false},
   bezierNurbsCurve([[2,0,0],[2,2,0]]),bezierNurbsCurve([[2,2,0],[0,2,0]]),bezierNurbsCurve([[0,2,0],[0,0,0]])]]
  const vector=(v:[number,number,number])=>({degree:1,knots:[0,0,1,1],values:[v,v],weights:[1,1]})
  const options:ProgressiveGuidedSurfaceSweepOptions={orientation:'rmf',normal:[1,0,0],initialSections:3,maxSections:3,maxDeviation:.01}
  if(mode!=='plain'){options.axisScale=vector([2,3,1]);options.centerLaw=vector([0,0,0])}
  if(mode==='authored'){options.orientation='authored';options.frameAxis=vector([0,0,1]);options.frameNormal=vector([1,0,0])}
  if(mode==='guide')options.orientationGuide=bezierNurbsCurve([[1,0,0],[1,0,10]])
  const body=createProgressiveBrepProfileBody(rings,bezierNurbsCurve([[0,0,0],[0,0,10]]),law(1,1),law(0,0),options)
  if(!body.boundaryContinuousBound||!body.boundaryErrorWithinBudget||body.bodyDecompositionProducts!==54)throw Error('Constructor extraction boundary proof required: '+mode)
  const reversed=(id:number)=>body.model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===id))[0]!.reversed
  return {file:`progressive-certified-${mode}-unsegmented-rational.step`,model:body.model,faces:body.model.faces.length,shells:1,capHoleFaces:0,
   capFaces:[body.model.faces.length-2,body.model.faces.length-1],expectedVolume:0,volumeReferenceMethod:'rational-boundary-gauss-reference',requireNativeSolid:true,
   construction:{boundaryErrorUpper:body.boundaryErrorUpper,bodyDecompositionErrorUpper:body.bodyDecompositionErrorUpper,bodyDecompositionProducts:body.bodyDecompositionProducts},
   wallFaceReversed:body.model.faces.slice(0,-2).map((_,id)=>reversed(id)),
   outwardCapContours:body.model.faces.slice(-2).map((face,j)=>[face.outer,...face.holes].flatMap(wire=>
    (reversed(body.model.faces.length-2+j)?[...body.model.loops[wire]!.coedges].reverse():body.model.loops[wire]!.coedges).map(use=>
     use.reversed!==reversed(body.model.faces.length-2+j)?reverseNurbsCurve(body.model.edges[use.edge]!.curve):body.model.edges[use.edge]!.curve)))}
 }),
 ...(['plain','affine','authored','guide','arc-rmf','arc-fixed','arc-guide','arc-authored','arc-curved-authored','arc-curved-fixed','arc-curved-fixed-normal','arc-curved-frenet','arc-curved-guided','arc-curved-contact','arc-curved-planar-rmf','arc-curved-nonaxial-planar-rmf'] as const).map(certifiedRectangularBodyFixture),
 obliqueRmfHollowFixture(),
 frenetHollowFixture(),
 frenetHollowFixture('rmf'),
 frenetHollowFixture('rmf',true),
 closedPlanarRmfBodyFixture(),
 closedGuidedTubeFixture(),
 closedGuidedTubeFixture('arc_length'),
 closedArcRmfTubeFixture(),
 closedArcRmfTubeFixture(true),
 closedArcRmfTubeFixture(false,'fixed_normal'),
 closedArcRmfTubeFixture(true,'fixed_normal'),
 closedArcRmfTubeFixture(false,'corrected_frenet'),
 closedArcRmfTubeFixture(true,'corrected_frenet'),
 closedArcRmfTubeFixture(false,'corrected_frenet',true),
 closedArcRmfTubeFixture(true,'corrected_frenet',true),
 closedSpatialRmfTubeFixture(),
 closedSpatialRmfTubeFixture(true),
 closedSpatialRmfTubeFixture(true,true),
 closedSpatialRmfTubeFixture(false,false,true),
 closedSpatialRmfTubeFixture(true,true,true),
 closedSpatialRmfTubeFixture(true,true,false,true),
 closedSpatialRmfTubeFixture(true,true,true,true),
 closedSpatialRmfTubeFixture(true,true,true,false,true),
 closedSpatialRmfTubeFixture(true,true,true,true,true),
 closedSpatialRmfTubeFixture(true,false,false,false,false,true),
 closedSpatialRmfTubeFixture(true,false,false,false,false,false,true),
 correctedFrenetBodyFixture('open-spatial-rmf-affine-corrected-body-certified.r',0),
 {...correctedFrenetBodyFixture('open-spatial-rmf-varying-affine-hollow-corrected-body.r'),
  volumeBudgets:{maxSpans:1024,maxLinearCells:100000,maxPairs:20000,maxCells:200000}},
 {...correctedFrenetBodyFixture('open-spatial-rmf-parameter-varying-affine-hollow-corrected-body.r'),
  volumeBudgets:{maxSpans:1024,maxLinearCells:100000,maxPairs:20000,maxCells:200000}},
 ...miterLawMatrixModes.map(mode=>miterLawMatrixFixture(mode)),
 closedArcRmfTubeFixture(false,'rmf',true),
 closedArcRmfTubeFixture(true,'rmf',true),
 fixedFrameFixture(),
 fixedFrameFixture('fixed_normal'),
 fixedFrameFixture('rmf'),
 (()=>{
  const sourceFile='examples/rush/progressive-miter-reconstructed-rational-profile.r'
  const source=readFileSync(sourceFile,'utf8'),graph=compileRushFrontend(source).document
  const built=buildOwnNurbs(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
  const {kind,...model}=built.report.definitions[built.nativeGeometry!.nodeId] as unknown as NurbsBrep & {kind:string}
  const construction=built.report.construction![built.nativeGeometry!.nodeId] as {
   continuousBound:boolean;boundaryCertificate:{withinBudget:boolean};
   profileSmoothness:{profileG1Certified:boolean;station:{stationG2Certified:boolean}}
  }
  if(kind!=='brep'||!construction.continuousBound||!construction.boundaryCertificate.withinBudget
   ||!construction.profileSmoothness.profileG1Certified||!construction.profileSmoothness.station.stationG2Certified)
   throw Error('General rational reconstruction complete bound/profile G1/station G2 unproved')
  const capFaces=[model.faces.length-2,model.faces.length-1]
  const outwardCapContours=capFaces.map(id=>{
   const face=model.faces[id]!
   const uses=model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===id))
   if(uses.length!==1)throw Error('Rational reference needs one owner per cap')
   const reversed=uses[0]!.reversed
   return [face.outer,...face.holes].flatMap(wire=>(reversed?[...model.loops[wire]!.coedges].reverse():model.loops[wire]!.coedges).map(use=>{
    const curve=model.edges[use.edge]!.curve
    return use.reversed!==reversed?reverseNurbsCurve(curve):curve
   }))
  })
  return {file:'rush-reconstructed-rational-profile.step',model:model as NurbsBrep,
   faces:model.faces.length,shells:1,capHoleFaces:0,capFaces:[model.faces.length-2,model.faces.length-1],
   requireNativeSolid:true,construction,sourceFile,sourceSha256:createHash('sha256').update(source).digest('hex'),
   replayRouting:JSON.parse(built.nativeGeometry!.geometryJson).sweepMiterReplay,
   outwardCapContours,expectedVolume:0,volumeReferenceMethod:'rational-boundary-gauss-reference'}
 })(),
 (()=>{
  const sourceFile='examples/rush/progressive-miter-reconstructed-periodic-profile.r'
  const source=readFileSync(sourceFile,'utf8'),graph=compileRushFrontend(source).document
  const built=buildOwnNurbs(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
  const {kind,...model}=built.report.definitions[built.nativeGeometry!.nodeId] as unknown as NurbsBrep & {kind:string}
  const construction=built.report.construction![built.nativeGeometry!.nodeId] as {
   continuousBound:boolean;boundaryCertificate:{withinBudget:boolean};
   profileSmoothness:{profile:{exactG1G2Certified:boolean;certifiedOrder:number|null};station:{stationG2Certified:boolean}}
  }
  if(kind!=='brep'||!construction.continuousBound||!construction.boundaryCertificate.withinBudget
   ||!construction.profileSmoothness.profile.exactG1G2Certified||construction.profileSmoothness.profile.certifiedOrder!==2||!construction.profileSmoothness.station.stationG2Certified)
   throw Error('Reconstructed periodic profile complete bound/G2 unproved')
  const reversed=(face:number)=>{
   const uses=model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===face))
   if(uses.length!==1)throw Error('Polynomial reference needs one owner per face')
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
  return {file:'rush-reconstructed-periodic-profile.step',model:model as NurbsBrep,
   faces:model.faces.length,shells:1,capHoleFaces:0,capFaces:caps,requireNativeSolid:true,
   construction,sourceFile,sourceSha256:createHash('sha256').update(source).digest('hex'),
   replayRouting:JSON.parse(built.nativeGeometry!.geometryJson).sweepMiterReplay??null,
   volumeReferenceMethod:'polynomial-boundary-divergence-integral',
   wallFaceReversed:model.faces.flatMap((_,id)=>caps.includes(id)?[]:[reversed(id)]),
   outwardCapContours,expectedVolume:0}
 })(),
 ...(['root','placed','rational-placed'] as const).map(mode=>{
  const sourceFile=mode==='rational-placed'?'examples/rush/closed-rational-frame-guide-affine-hollow-placed.r':`examples/rush/closed-periodic-frame-guide-affine-hollow${mode==='placed'?'-placed':''}.r`
  const source=readFileSync(sourceFile,'utf8'),graph=compileRushFrontend(source).document
  const built=buildOwnNurbs(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
  const {kind,...model}=built.report.definitions[built.nativeGeometry!.nodeId] as unknown as NurbsBrep & {kind:string}
  const construction=built.report.construction![built.nativeGeometry!.nodeId] as {continuousBound:boolean;boundaryCertificate:{withinBudget:boolean}}
  if(kind!=='brep'||!construction.continuousBound||!construction.boundaryCertificate.withinBudget)throw Error('Closed periodic profile body unproved')
  if(mode==='rational-placed'&&!(construction as unknown as {profileSmoothness:{profileG1Certified:boolean}}).profileSmoothness.profileG1Certified)throw Error('Closed rational profile G1 unproved')
  const wallFaceReversed=model.faces.map((_,face)=>{
   const uses=model.shells.flatMap(shell=>shell.faces.filter(use=>use.face===face))
   if(uses.length!==1)throw Error('Closed polynomial reference needs one shell owner per face')
   return uses[0]!.reversed
  })
  return {file:`rush-closed-periodic-profile-${mode}.step`,model:model as NurbsBrep,
   faces:model.faces.length,shells:2,capHoleFaces:0,capFaces:[],requireNativeSolid:true,
   construction,sourceFile,sourceSha256:createHash('sha256').update(source).digest('hex'),
   replayRouting:JSON.parse(built.nativeGeometry!.geometryJson).sweepMiterReplay??null,
   volumeReferenceMethod:mode==='rational-placed'?'rational-boundary-gauss-reference':'polynomial-boundary-divergence-integral',wallFaceReversed,
   outwardCapContours:[],expectedVolume:0}
 }),
 ...(['authored-caps','automatic-caps','automatic-affine-caps','authored-affine-caps'] as const).map(capMode=>{
  const sourceFile=capMode==='authored-affine-caps'?'examples/rush/miter-periodic-moving-axis-guide-affine-hollow-authored-caps-placed.r':capMode==='authored-caps'?'examples/rush/miter-periodic-moving-axis-guide-affine-hollow-authored-caps.r':capMode==='automatic-affine-caps'?'examples/rush/miter-periodic-moving-axis-guide-affine-hollow-placed.r':'examples/rush/miter-periodic-moving-axis-guide-affine-hollow.r'
  const source=readFileSync(sourceFile,'utf8'),graph=compileRushFrontend(source).document
  const node=graph.nodes.find(n=>n.op==='brep_progressive_miter_sweep')!
  const built=buildOwnNurbs(graph,{action:'build',display:{segments:4,subdivisionLevels:0}})
  const {kind,...model}=built.report.definitions[built.nativeGeometry!.nodeId] as unknown as NurbsBrep & {kind:string}
  const construction=built.report.construction![built.nativeGeometry!.nodeId] as {continuousBound:boolean;boundaryCertificate:{withinBudget:boolean}}
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
  return {file:`rush-periodic-moving-axis-${capMode}.step`,model:model as NurbsBrep,faces:model.faces.length,
   shells:1,capHoleFaces:2,requireNativeSolid:true,construction,sourceFile,
   replayRouting:JSON.parse(built.nativeGeometry!.geometryJson).sweepMiterReplay??null,
   sourceSha256:createHash('sha256').update(source).digest('hex'),
   volumeReferenceMethod:'polynomial-boundary-divergence-integral',
   wallFaceReversed:model.faces.flatMap((_,id)=>caps.includes(id)?[]:[reversed(id)]),outwardCapContours,
   // The independent Python reference must fill this before OCCT verification.
   expectedVolume:0}
 }),
 ...(['unclamped','periodic','periodic-frame-guide-affine'] as const).map(mode=>{
  const source=readFileSync(`examples/rush/miter-${mode}-hollow.r`,'utf8')
  const graph=compileRushFrontend(source).document
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
 (()=>{
  const fixture=authoredRushFixture('examples/rush/nonuniform-station-g2-hollow-miter.r','rush-nonuniform-station-g2-hollow-miter.step',1)
  // Independent source annulus area times the original total straight length.
  return {...fixture,expectedVolume:Math.PI*(.5**2-.25**2)*34,requireStationG2:true}
 })(),
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
  ...('volumeBudgets' in fixture?fixture.volumeBudgets:{}),
 }
 const caps='capFaces' in fixture?fixture.capFaces:fixture.capHoleFaces?[model.faces.length-2,model.faces.length-1]:[]
 const nativeProfileSmoothness=inspectMiterProfileSmoothness(model,caps)
 if('requireStationG2' in fixture&&fixture.requireStationG2&&
   (nativeProfileSmoothness.stationContinuity!=='G2'||nativeProfileSmoothness.station.edgeIds.length<24)){
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
 return {...fixture,capFaces:caps,nativeAuditBudgets:auditBudgets,nativeProfileSmoothness,nativeRetainedWallCharts,nativeVolume,genericNativeRetainedWallCharts,genericNativeVolume,nativeReplayFinalModelBound:replay?.resultModelBound??null,nativeBoundary,nativeExactUses,exactWork,edges:model.edges.length,faceLoops:model.faces.map(face=>({outer:model.loops[face.outer]!.coedges.map(c=>c.edge),holes:face.holes.map(loop=>model.loops[loop]!.coedges.map(c=>c.edge))})),edgeCurves:model.edges.map(edge=>({curve:edge.curve,samples:[0,.25,.5,.75,1].map(u=>({u,point:evaluateNurbsCurve(edge.curve,u).point}))})),wallCoedges:walls.map(face=>model.loops[face.outer]!.coedges.map(c=>({edge:c.edge,reversed:c.reversed,pcurve:c.pcurve}))),wallSurfaces:walls.map(face=>face.surface),wallFaceReversed,wallSamples,surfaceToleranceMm:1e-8,solids:1,sha256:createHash('sha256').update(text).digest('hex'),relativeVolumeTolerance:1e-7}
})
// Complete reports can exceed V8's maximum single string length. Preserve
// every field while writing one case at a time; publish only the finished file.
const stagedManifest=resolve(root,`manifest-${process.pid}.json.writing`)
writeFileSync(stagedManifest,JSON.stringify({schema:'sweep-external-step/2',units:'mm',artifactProvenance}).slice(0,-1)+',"cases":[\n')
for(let index=0;index<cases.length;index++)appendFileSync(stagedManifest,(index?',\n':'')+JSON.stringify(cases[index]))
appendFileSync(stagedManifest,'\n]}\n')
renameSync(stagedManifest,resolve(root,'manifest.json'))
console.log(root)
