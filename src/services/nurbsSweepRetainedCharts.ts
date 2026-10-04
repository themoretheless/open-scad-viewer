import type {NurbsBrep} from './geometry/brep'
import {inspectSurfaceLinearInjectivity,type SurfaceLinearInjectivityAudit} from './nurbsSweepAudit'
export interface SweepRetainedChartEvidence {
 allChartsCertified:boolean
 cells:number
 charts:{face:number;audit:SurfaceLinearInjectivityAudit}[]
 unresolvedFaces:number[]
 globalEmbeddingCertified:false
}
/** Actual decomposed B-rep wall charts; the cell budget is shared across them. */
export function inspectSweepRetainedWallCharts(model:NurbsBrep,capFaces:number[],maxCells:number,checkAbort=()=>{}):SweepRetainedChartEvidence {
 if(!Number.isInteger(maxCells)||maxCells<0||maxCells>100000||model.faces.length>1024||capFaces.length>16||new Set(capFaces).size!==capFaces.length||capFaces.some(id=>!Number.isInteger(id)||id<0||id>=model.faces.length))throw new Error('Invalid retained wall chart selection or budget')
 let cells=0
 const capSet=new Set(capFaces),charts:{face:number;audit:SurfaceLinearInjectivityAudit}[]=[]
 for(const [face,value]of model.faces.entries()){
  if(capSet.has(face))continue
  checkAbort()
  const audit=inspectSurfaceLinearInjectivity(value.surface,maxCells-cells)
  cells+=audit.cells;charts.push({face,audit})
  checkAbort()
 }
 const unresolvedFaces=charts.filter(chart=>!chart.audit.certified).map(chart=>chart.face)
 return {allChartsCertified:charts.length>0&&unresolvedFaces.length===0,cells,charts,unresolvedFaces,globalEmbeddingCertified:false}
}
