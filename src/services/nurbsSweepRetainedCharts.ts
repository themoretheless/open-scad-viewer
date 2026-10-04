import type {NurbsBrep} from './geometry/brep'
import type {SurfaceLinearInjectivityAudit} from './nurbsSweepAudit'
import {callGeometryRust} from './geometry/kernel'
export interface SweepRetainedChartEvidence {
 allChartsCertified:boolean
 cells:number
 charts:{face:number;audit:SurfaceLinearInjectivityAudit}[]
 unresolvedFaces:number[]
 globalEmbeddingCertified:false
}
/** Actual decomposed B-rep wall charts; the cell budget is shared across them. */
export function inspectSweepRetainedWallCharts(model:NurbsBrep,capFaces:number[],maxCells:number,checkAbort=()=>{}):SweepRetainedChartEvidence {
 checkAbort()
 const report=callGeometryRust<SweepRetainedChartEvidence>('brep_sweep_retained_wall_charts_audit',{model,capFaces,maxCells})
 checkAbort()
 return report
}
