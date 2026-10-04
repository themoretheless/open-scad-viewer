import type {NurbsBrep} from '../../geometry/brep'
import {callGeometryRust} from '../../geometry/kernel'
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
 nestingPairs:number
 nestingCells:number
 nestingDomainCells:number
 orientationCells:number
 orientationDomainCells:number
 orientationSpans:number
}
export interface SweepVolumeAudit {
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
 maxLinearCells:20000,maxTrimPairs:10000,maxTrimCells:100000,maxTrimDomainCells:1000000,
 maxPairs:10000,maxCells:100000,maxDomainCells:1000000,cellsPerPair:1000,domainCellsPerPair:10000,
 nestingPairs:1000,nestingCells:100000,nestingDomainCells:1000000,
 orientationCells:100000,orientationDomainCells:1000000,orientationSpans:100,
}
/** Empty cap selection supports closed no-cap shells; all material stages remain mandatory. */
export function inspectSweepVolume(model:NurbsBrep,capFaces:number[],budgets:SweepVolumeBudgets):SweepVolumeAudit {
 return callGeometryRust('brep_sweep_volume_audit',{model,capFaces,...budgets})
}
