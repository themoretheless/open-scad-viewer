import {buildMiterStepFixtures} from './sweep-step-miter-fixtures.mts'
import {buildProfileStepFixtures} from './sweep-step-profile-fixtures.mts'
import {callGeometryRust} from '../src/services/geometry/kernel'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
import {mkdirSync,writeFileSync,readFileSync,appendFileSync,renameSync} from 'node:fs'
import {resolve} from 'node:path'

import {createHash} from 'node:crypto'
import {bezierNurbsCurve} from '../src/services/nurbsConstructors'
import type {ProgressiveGuidedSurfaceSweepOptions} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve,evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {createProgressiveBrepProfileBody} from '../src/services/geometry/brep'

import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {exportDirectStepV5} from '../src/services/cadNurbsStep'
import {inspectSweepVolume,inspectSweepEmbedding,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'
import {inspectMiterProfileSmoothness} from '../src/services/miterProfileSmoothness'
import {inspectSweepRetainedWallCharts} from '../src/services/nurbsSweepRetainedCharts'
import {inspectSweepCoedgeExact} from '../src/services/nurbsSweepAudit'
import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'

import {miterLawMatrixModes} from './miter-law-matrix-sources'
import type {NurbsBrep} from '../src/services/geometry/brep'
const root=resolve(process.argv[2]??'docs/qualification/sweep-coverage-2026-10-01/external-step')
mkdirSync(root,{recursive:true})
const artifactProvenance=sweepStepArtifactProvenance()
const {law,miterLawMatrixFixture,authoredRushFixture,unsegmentedFixture,circleModel,circleConstruction,circleSource,progressiveVolume,spatialCircleModel,spatialCircleConstruction,spatialCircleSource,affineCircleModel,affineCircleConstruction,affineCircleSource,obliqueCircleModel,obliqueCircleConstruction,obliqueCircleSource,curvedGuideModel,curvedGuideConstruction,curvedGuideSource,curvedGuideVolume,stationModel,stationConstruction,stationSource,g1Model,g1Construction,g1Source,closedJoint,closedJointVolume,closedAffine,straight,progressive,body,correctedSpatial,correction,correctionCorrespondence,constructorCorrected,closed,oblique,obliqueTransform,obliqueCaps,dyadicOblique,dyadicCaps,affineRushModel,affineRushSource,affineRushConstruction}=buildMiterStepFixtures(root)
const {closedContactBodyFixture,sharpProfileStationG2Fixture,closedSpatialRmfTubeFixture,correctedFrenetBodyFixture,closedAuthoredTubeFixture,multispanAuthoredBodyFixture,certifiedRectangularBodyFixture,obliqueRmfHollowFixture,frenetHollowFixture,closedPlanarRmfBodyFixture,closedGuidedTubeFixture,closedArcRmfTubeFixture,fixedFrameFixture}=buildProfileStepFixtures(law)
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
   replayArtifact:built.nativeGeometry!,replayRouting:JSON.parse(built.nativeGeometry!.geometryJson).sweepMiterReplay,
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
   replayArtifact:built.nativeGeometry!,replayRouting:JSON.parse(built.nativeGeometry!.geometryJson).sweepMiterReplay??null,
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
   replayArtifact:built.nativeGeometry!,replayRouting:JSON.parse(built.nativeGeometry!.geometryJson).sweepMiterReplay??null,
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
   replayArtifact:built.nativeGeometry!,replayRouting:JSON.parse(built.nativeGeometry!.geometryJson).sweepMiterReplay??null,
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
const qualificationCatalog=JSON.parse(readFileSync(new URL('../docs/design/sweep-qualification-catalog.json',import.meta.url),'utf8'))
writeFileSync(resolve(root,'fixture-inventory.json'),JSON.stringify(fixtures.map(f=>({file:f.file,requireNativeSolid:'requireNativeSolid' in f&&f.requireNativeSolid})),null,2)+'\n')
if(process.argv.includes('--inventory-only'))process.exit(0)
const catalogFiles:string[]=qualificationCatalog.step.baseline.map((entry:{file:string})=>entry.file)
if(new Set(fixtures.map(f=>f.file)).size!==fixtures.length||fixtures.length!==catalogFiles.length||catalogFiles.some(file=>!fixtures.some(f=>f.file===file)))throw Error('STEP fixtures differ from the shared qualification catalog: '+JSON.stringify({fixtures:fixtures.length,catalog:catalogFiles.length,missing:catalogFiles.filter(file=>!fixtures.some(f=>f.file===file)),extra:fixtures.filter(f=>!catalogFiles.includes(f.file)).map(f=>f.file)}))
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
 const replay='replayRouting' in fixture&&fixture.replayRouting&&fixture.replayRouting.matrix
  ?callGeometryRust<any>('brep_miter_owned_place',{...fixture.replayRouting,expectedResultModel:model,requireSolid:true,wallCells:100000,volumeBudgets:auditBudgets}):null
 if(replay&&(replay.resultModelBound!==true||replay.solidGeometryCertified!==true))throw Error('Native replay final-model/material binding refused: '+fixture.file)
 const nativeRetainedWallCharts=replay?.retainedWallCharts??genericNativeRetainedWallCharts
 const replayAdmission='replayArtifact' in fixture&&fixture.replayRouting
  ?inspectProgressiveSweepSolidAdmission(fixture.replayArtifact,model):null
 if('replayArtifact' in fixture&&fixture.replayRouting&&!replayAdmission?.solidGeometryCertified)throw Error('Native original-source snapshot admission refused: '+fixture.file)
 const nativeVolume=replayAdmission??replay?.volume??genericNativeVolume
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
 return {...fixture,capFaces:caps,nativeAuditBudgets:auditBudgets,nativeProfileSmoothness,nativeRetainedWallCharts,nativeVolume,genericNativeRetainedWallCharts,genericNativeVolume,nativeReplayFinalModelBound:replayAdmission?.solidGeometryCertified??replay?.resultModelBound??null,nativeBoundary,nativeExactUses,exactWork,edges:model.edges.length,faceLoops:model.faces.map(face=>({outer:model.loops[face.outer]!.coedges.map(c=>c.edge),holes:face.holes.map(loop=>model.loops[loop]!.coedges.map(c=>c.edge))})),edgeCurves:model.edges.map(edge=>({curve:edge.curve,samples:[0,.25,.5,.75,1].map(u=>({u,point:evaluateNurbsCurve(edge.curve,u).point}))})),wallCoedges:walls.map(face=>model.loops[face.outer]!.coedges.map(c=>({edge:c.edge,reversed:c.reversed,pcurve:c.pcurve}))),wallSurfaces:walls.map(face=>face.surface),wallFaceReversed,wallSamples,surfaceToleranceMm:1e-8,solids:1,sha256:createHash('sha256').update(text).digest('hex'),relativeVolumeTolerance:1e-7}
})
// Complete reports can exceed V8's maximum single string length. Preserve
// every field while writing one case at a time; publish only the finished file.
const stagedManifest=resolve(root,`manifest-${process.pid}.json.writing`)
writeFileSync(stagedManifest,JSON.stringify({schema:'sweep-external-step/2',units:'mm',artifactProvenance}).slice(0,-1)+',"cases":[\n')
for(let index=0;index<cases.length;index++)appendFileSync(stagedManifest,(index?',\n':'')+JSON.stringify(cases[index]))
appendFileSync(stagedManifest,'\n]}\n')
renameSync(stagedManifest,resolve(root,'manifest.json'))
console.log(root)
