/** Original-request native construction transport. This report proves boundary
 * error only; it does not authenticate serialized copies for later operations
 * or admit a Solid. Construction and proof derivation live in Rust. */
import {callGeometryRust} from './geometry/kernel'
import type {NurbsCurve} from './nurbsCurve'
import {sweepAffineLawPayload,sweepFrameLawPayload,type ProgressiveMiterReport,type ProgressiveMiterOptions,type NurbsScaleLaw} from './nurbsConstructors'
import {DEFAULT_SWEEP_VOLUME_BUDGETS} from './nurbsSweepEmbedding'
import type {SweepSectionCorrection} from './nurbsSectionProjection'
import type {SweepRetainedChartEvidence} from './nurbsSweepRetainedCharts'
import type {NurbsBrep} from './geometry/brep'
import type {SweepBoundaryCertificate} from './sweepBoundaryCertificate'
export interface MiterOwnedSourceRequest {
 loops:NurbsCurve[][]
 points:[number,number,number][]
 /** Native scalar law controls are [value,0,0]; twist values are radians. */
 scale:NurbsCurve
 twist:NurbsCurve
 normal:[number,number,number]
 closed:boolean
 miterLimit:number
 initialSteps:number
 maxSteps:number
 maxDeviation:number
 axisScale?:NurbsCurve
 centerLaw?:NurbsCurve
 frameAxis?:NurbsCurve
 frameNormal?:NurbsCurve
 orientationGuide?:NurbsCurve
 circleCorrection?:{quantum:number;tolerance:number;maxWork:number|null}
 capCorrection?:{quantum:number;tolerance:number;maxWork:number|null;authoredFrame?:boolean}
 limits?:{
  maxProducts:number;maxFaces:number;exactWork:number
  correspondenceMaxFaces?:number;capMaxEdges?:number;wallCells?:number
  /** Independent cap phases; omitted values retain exactWork as the default. */
  domainExactWork?:number;projectionExactWork?:number
  domainTolerance:number;domainPairs:number;domainCells:number;projectionCells:number
  capRegions:{maxWalls:number;maxExactWork:number;maxChartCells:number;maxTrimPairs:number;maxTrimCells:number;maxTrimDomainCells:number}
 }
}
export interface MiterOwnedBoundaryReport {
 method:'original-request-owned-miter-boundary'
 model:NurbsBrep
 sections:NurbsCurve[][][]
 sourceSections:NurbsCurve[][]
 sectionCorrection:Omit<SweepSectionCorrection,'sections'>|null
 retainedWallCharts:SweepRetainedChartEvidence
 edges:number
 maxSteps:number
 sharpStationIndices:number[]
 closed:boolean
 budget:number
 boundaryCertificate:SweepBoundaryCertificate
 levels:ProgressiveMiterReport[]
 solidGeometryCertified:false
 globalEmbeddingCertified:false
}
export const constructMiterOwnedBoundary=(source:MiterOwnedSourceRequest):MiterOwnedBoundaryReport=>
 callGeometryRust('brep_miter_owned_construct',source)

/** Replays original construction in the receiving realm. No serialized model
 * or supplied numerical certificate participates in native boundary admission. */
export const reconstructMiterOwnedBoundary=(source:MiterOwnedSourceRequest,options:{quantum:number;tolerance:number;maxWork:number;budget:number;requireSolid?:boolean;wallCells?:number;volumeBudgets?:import('./nurbsSweepEmbedding').SweepVolumeBudgets;expectedSourceModel?:NurbsBrep;expectedSourceCertificate?:SweepBoundaryCertificate;expectedSections?:NurbsCurve[][][];expectedSharp?:number[]}):{
 model:NurbsBrep|null
 candidate:import('./geometry/brep').SmoothStationWallCandidate|null
 boundaryCertificate:SweepBoundaryCertificate|null
 sharpStationIndices:number[]
 reason:string
 retainedWallCharts:SweepRetainedChartEvidence|null
 volume:import('./nurbsSweepEmbedding').SweepVolumeAudit|null
 solidGeometryCertified:boolean
 globalEmbeddingCertified:false
}=>callGeometryRust('brep_miter_owned_reconstruct',{source,...options})

/** Wire normalization for the existing public degrees/vector-law API. Native
 * construction owns face planning, refinement, correction and boundary proof. */
export function constructProgressiveMiterOwnedBoundary(
 loops:NurbsCurve[][],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveMiterOptions,
):MiterOwnedBoundaryReport {
 return constructMiterOwnedBoundary(progressiveMiterOwnedRequest(loops,points,scale,twist,options))
}
export function progressiveMiterOwnedRequest(
 loops:NurbsCurve[][],points:[number,number,number][],scale:NurbsScaleLaw,twist:NurbsScaleLaw,options:ProgressiveMiterOptions,
):MiterOwnedSourceRequest {
 const affine=sweepAffineLawPayload(options)
 const frames=options.frameAxis||options.frameNormal?sweepFrameLawPayload({orientation:'authored',frameAxis:options.frameAxis,frameNormal:options.frameNormal}):{}
 const correction=(b:ProgressiveMiterOptions['circleCorrection'])=>b?{...b,maxWork:Number.isFinite(b.maxWork)?b.maxWork:null}:undefined
 const cap=options.capCorrection
 return {loops,points,
  scale:{degree:scale.degree,knots:scale.knots,controlPoints:scale.values.map(x=>[x,0,0]),weights:scale.weights,periodic:false},
  twist:{degree:twist.degree,knots:twist.knots,controlPoints:twist.values.map(x=>[x*Math.PI/180,0,0]),weights:twist.weights,periodic:false},
  normal:options.normal,closed:options.closed??false,miterLimit:options.miterLimit??4,initialSteps:options.initialSteps??1,maxSteps:options.maxSteps??64,maxDeviation:options.maxDeviation,
  ...(affine.axis_scale?{axisScale:affine.axis_scale}:{}),...(affine.center_law?{centerLaw:affine.center_law}:{}),
  ...(frames.frame_axis?{frameAxis:frames.frame_axis,frameNormal:frames.frame_normal}:{}),
  ...(options.orientationGuide?{orientationGuide:options.orientationGuide}:{}),
  circleCorrection:correction(options.circleCorrection),capCorrection:cap?{...correction(cap)!,authoredFrame:cap.authoredFrame}:undefined,
  limits:{maxProducts:options.retainedDecompositionBudgets?.maxProducts??100000,maxFaces:options.retainedDecompositionBudgets?.maxFaces??1024,
   correspondenceMaxFaces:1024,capMaxEdges:1024,wallCells:options.retainedWallMaxInjectivityCells??10000,exactWork:1000000,
   domainExactWork:options.capDomainBudgets?.maxExactWork??1000000,projectionExactWork:options.capProjectionBudgets?.maxExactWork??1000000,
   domainTolerance:options.capDomainBudgets?.tolerance??.001,domainPairs:options.capDomainBudgets?.maxPairs??1000,
   domainCells:options.capDomainBudgets?.maxCells??10000,projectionCells:options.capProjectionBudgets?.maxCells??10000,
   capRegions:(options.volumeBudgets??DEFAULT_SWEEP_VOLUME_BUDGETS).capBudgets},
 }
}
