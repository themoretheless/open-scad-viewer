import { callGeometryRust } from './geometry/kernel'
import type { DirectSketch, Point2 } from './directModeling'
import type { NurbsCurve } from './nurbsCurve'
import { solveModelGraphSketch, type SketchConstraint as SolverConstraint } from './modelGraphSketch'
export type SketchDimension = {kind:'length';a:number;b:number} | {kind:'angle';a:number;b:number;c:number} | {kind:'horizontal'|'vertical';a:number;b:number} | {kind:'radius'|'diameter'}
export interface DimensionResult { points:Point2[]; measurements:{value:number|null;label:Point2;lines:Point2[][]}[] }
export function sketchDimensions(sketch:DirectSketch,edit?:{index:number;value:number}):DimensionResult {
  if (sketch.analytic && (sketch.dimensions??[]).every(d=>d.kind==='radius'||d.kind==='diameter')) {
    const radius = edit ? (edit.value > 0 ? (sketch.dimensions![edit.index]?.kind === 'diameter' ? edit.value / 2 : edit.value) : (()=>{throw Error('Radius must be positive.')})()) : sketch.analytic.radius
    const result = (sketch.dimensions??[]).map(d => { const value=d.kind==='diameter'?radius*2:radius; const offset=[radius*1.2,0] as Point2; const label:Point2=[sketch.analytic!.center[0]+offset[0],sketch.analytic!.center[1]]; const lines:Point2[][]=[[[sketch.analytic!.center[0],sketch.analytic!.center[1]],[sketch.analytic!.center[0]+radius,sketch.analytic!.center[1]]]]; return {value,label,lines} })
    return {points:sketch.points,measurements:result}
  }
  const dimensions = (sketch.dimensions??[]).map((d,i)=>edit&&i===edit.index ? ({...d, ...(d.kind==='length'?{value:edit.value}: {})}) : d)
  if ((!edit || dimensions[edit.index]?.kind==='length') && dimensions.every(d=>d.kind==='length'||d.kind==='horizontal'||d.kind==='vertical')) {
    const constraints: SolverConstraint[] = dimensions.map((d,i) => d.kind==='length'
      ? {id:`dimension-${i}`,kind:'distance',a:`p${d.a}`,b:`p${d.b}`,value:edit&&i===edit.index?edit.value:Math.hypot(sketch.points[d.b][0]-sketch.points[d.a][0],sketch.points[d.b][1]-sketch.points[d.a][1])}
      : {id:`dimension-${i}`,kind:d.kind,a:`p${(d as {a:number}).a}`,b:`p${(d as {b:number}).b}`} as SolverConstraint)
    const solved = solveModelGraphSketch(sketch.points.map((position,i)=>({id:`p${i}`,position:[...position] as Point2})),constraints)
    if (solved.status==='inconsistent'||solved.status==='not_converged') throw Error('Sketch dimensions conflict or did not converge.')
    const points = solved.points.map(p=>p.position)
    const result = callGeometryRust('cad_dimensions',{points,dimensions:sketch.dimensions??[]}) as DimensionResult
    return {points,measurements:result.measurements}
  }
  return callGeometryRust('cad_dimensions',{points:sketch.points,dimensions:sketch.dimensions??[],...(edit?{edit}:{})})
}
export function editSketchDimension(sketch:DirectSketch,index:number,value:number):DirectSketch {
  if(sketch.analytic) { const dimension=sketch.dimensions?.[index]; if(!dimension || (dimension.kind!=='radius'&&dimension.kind!=='diameter')) throw Error('Analytic curves support radius and diameter dimensions only.'); const radius=dimension.kind==='diameter'?value/2:value; if(!(radius>0&&Number.isFinite(radius))) throw Error('Radius must be positive.'); return {...sketch,analytic:{...sketch.analytic,radius},points:sketch.points} }
  return {...sketch,points:sketchDimensions(sketch,{index,value}).points}
}
export function bridgeNurbsCurves(a:NurbsCurve,b:NurbsCurve,endA:'start'|'end'='end',endB:'start'|'end'='start',tension=1):NurbsCurve {
  return callGeometryRust('cad_bridge_curve',{a,b,endA,endB,tension})
}
