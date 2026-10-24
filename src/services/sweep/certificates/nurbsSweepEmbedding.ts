import {inspectSweepRetainedWallCharts,type SweepRetainedChartEvidence} from '../../nurbsSweepRetainedCharts'
import type {NurbsBrep} from '../../geometry/brep'
import {callGeometryRust,NurbsCurveError} from '../../geometry/kernel'
import type {NativeSweepCapContactBudgets} from './nurbsSweepCapContacts'

export interface SweepEmbeddingBudgets {
 toleranceUv:number
 maxExactWork:number
 maxTrimPairs:number
 maxTrimCells:number
 maxTrimDomainCells:number
 maxSpans:number
 maxLinearCells:number
 maxPairs:number
 maxCells:number
 maxDomainCells:number
 cellsPerPair:number
 domainCellsPerPair:number
 capBudgets:NativeSweepCapContactBudgets
}
export interface SweepEmbeddingAudit {
 boundaryEmbeddingCertified:boolean
 solidGeometryCertified:false
 exactBoundaryCertified:boolean
 trimCertified:boolean
 allFacesInjective:boolean
 allPairsClassified:boolean
 linearCells:number
 spans:number
 totalPairs:number
 individualPairs:number
 groupedPairs:number
 nextPair:[number,number]|null
 unresolvedFaces:number[]
 pairs:{faces:[number,number];reason:string;allowedBoundary:boolean}[]
 caps:{face:number;capCertified:boolean;allowedBoundaries:[number,number][];unresolvedWalls:number[];reason:string|null}[]
}
export const DEFAULT_SWEEP_EMBEDDING_BUDGETS:SweepEmbeddingBudgets={
 toleranceUv:1e-8,maxExactWork:1000000,maxTrimPairs:1000,maxTrimCells:10000,
 maxTrimDomainCells:100000,maxSpans:1000,maxLinearCells:1000,maxPairs:1000,
 maxCells:10000,maxDomainCells:100000,cellsPerPair:16,domainCellsPerPair:1000,
 capBudgets:{maxWalls:1024,maxExactWork:1000000,maxChartCells:1000,
  maxTrimPairs:100000,maxTrimCells:100000,maxTrimDomainCells:1000000},
}
/** Embedded boundary evidence; shell nesting and material orientation remain separate. */
export function inspectSweepEmbedding(model:NurbsBrep,capFaces:number[],budgets:SweepEmbeddingBudgets):SweepEmbeddingAudit {
 return callGeometryRust('brep_sweep_embedding_audit',{model,capFaces,...budgets})
}
export interface SweepVolumeBudgets extends SweepEmbeddingBudgets {
 maxRegularityCells?:number
 nestingPairs:number
 nestingCells:number
 nestingDomainCells:number
 orientationCells:number
 orientationDomainCells:number
 orientationSpans:number
}
export interface SweepVolumeAudit {
 linearCells?:number
 totalPairs?:number
 regularBoundarySolidCertified?:boolean
 boundaryRegularity?:{allFacesRegular:boolean;allFacesRegularModuloPoles:boolean;cells:number;maxCells:number;unresolvedFaces:number[];poleQuotients:unknown[]}
 individualPairs?:number
 groupedPairs?:number
 solidGeometryCertified:boolean
 boundaryEmbeddingCertified:boolean
 allFacesInjective:boolean
 allPairsClassified:boolean
 nextPair:[number,number]|null
 nesting:null|{rolesConsistent:boolean|null;parents:(number|null)[]|null;totalPairs:number;visitedPairs:number;cells:number;domainCells:number;pairs:{shells:[number,number];reason:string;separationLower:number;boundarySeparationCertified:boolean}[]}
 orientationCells:number
 orientationDomainCells:number
 orientations:{shell:number;expectedOutward:boolean;outward:boolean|null;attempts:number}[]
}
export const DEFAULT_SWEEP_VOLUME_BUDGETS:SweepVolumeBudgets={
 ...DEFAULT_SWEEP_EMBEDDING_BUDGETS,
 maxLinearCells:30000,maxTrimPairs:10000,maxTrimCells:100000,maxTrimDomainCells:1000000,
 maxPairs:10000,maxCells:100000,maxDomainCells:1000000,cellsPerPair:1000,domainCellsPerPair:10000,
 nestingPairs:1000,nestingCells:100000,nestingDomainCells:1000000,
 orientationCells:100000,orientationDomainCells:1000000,orientationSpans:100,
}
/** Empty cap selection supports closed no-cap shells; all material stages remain mandatory. */
export function inspectSweepVolume(model:NurbsBrep,capFaces:number[],budgets:SweepVolumeBudgets):SweepVolumeAudit {
 return callGeometryRust('brep_sweep_volume_audit',{model,capFaces,...budgets})
}

export function inspectSweepVolumeAndCharts(model:NurbsBrep,capFaces:number[],budgets:SweepVolumeBudgets):SweepVolumeAudit & {retainedWallCharts:SweepRetainedChartEvidence} {
 try{return callGeometryRust('brep_sweep_volume_charts_audit',{model,capFaces,...budgets})}
 catch(error){
  // Older geometry kernels route this unknown operation to their surface parser.
  // Keep those installed kernels usable through the original two native audits.
  if(!(error instanceof NurbsCurveError)||error.code!=='NURBS_INVALID_INPUT'||error.message!=='Invalid surface: Expected object')throw error
  const retainedWallCharts=inspectSweepRetainedWallCharts(model,capFaces,budgets.maxLinearCells)
  return {...inspectSweepVolume(model,capFaces,budgets),retainedWallCharts}
 }
}

export async function inspectSweepVolumeAndChartsAsync(model:NurbsBrep,capFaces:number[],budgets:SweepVolumeBudgets,
 control:{shouldAbort?:()=>boolean;onYield?:()=>void}={}):Promise<SweepVolumeAudit & {retainedWallCharts:SweepRetainedChartEvidence}> {
 const check=()=>{if(control.shouldAbort?.())throw new DOMException('Build cancelled','AbortError')}
 check()
 const start=callGeometryRust<{session:string}>('brep_sweep_volume_begin',{model,capFaces,...budgets})
 let maxUnits=1
 try {
  for(;;){
   check()
   const started=performance.now()
   const step=callGeometryRust<{done:boolean;advancedUnits:number;completedSteps:number;report?:SweepVolumeAudit & {retainedWallCharts:SweepRetainedChartEvidence}}>(
    'brep_sweep_volume_step',{session:start.session,maxUnits})
   const elapsed=performance.now()-started
   maxUnits=elapsed<8?Math.min(64,maxUnits*2):elapsed>50?Math.max(1,Math.floor(maxUnits/2)):maxUnits
   check()
   if(step.advancedUnits>0)control.onYield?.()
   if(step.done){if(!step.report)throw new Error('Missing completed native volume report');return step.report}
   await new Promise<void>(resolve=>setTimeout(resolve,0))
  }
 }finally {callGeometryRust('brep_sweep_volume_drop',{session:start.session})}
}
