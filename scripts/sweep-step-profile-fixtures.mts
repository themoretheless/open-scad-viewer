

import {readFileSync} from 'node:fs'

import {deepStrictEqual} from 'node:assert'
import {createHash} from 'node:crypto'
import {bezierNurbsCurve,circleNurbsCurve} from '../src/services/nurbsConstructors'

import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {createProgressiveBrepProfileBody} from '../src/services/geometry/brep'
import type {ProgressiveGuidedSurfaceSweepOptions as ProgressiveBrepSweepOptions} from '../src/services/nurbsConstructors'

import {compileRushFrontend} from '../src/services/rushFrontend'
import {buildOwnNurbs} from '../src/services/rushGraphNurbsKernel'
import {readSweepBodyBoundaryViewportEvidence,readSweepViewportEvidence} from '../src/services/sweepViewportEvidence'

import type {NurbsBrep} from '../src/services/geometry/brep'

export function buildProfileStepFixtures(law:(a:number,b:number)=>{degree:number;knots:number[];values:number[];weights:number[]}){
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

return {closedContactBodyFixture,sharpProfileStationG2Fixture,closedSpatialRmfTubeFixture,correctedFrenetBodyFixture,closedAuthoredTubeFixture,multispanAuthoredBodyFixture,certifiedRectangularBodyFixture,obliqueRmfHollowFixture,frenetHollowFixture,closedPlanarRmfBodyFixture,closedGuidedTubeFixture,closedArcRmfTubeFixture,fixedFrameFixture}
}
