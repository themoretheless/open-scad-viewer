import {requirePolylineSketch} from './retainedSketchProfile'
import { callGeometryRust } from './geometry/kernel'
import type { DirectSketch, Point2 } from './directModeling'
import { sampleCurve } from './directSketchGeometry'
import type { NurbsCurve } from './nurbsCurve'
import { solveRushGraphSketch, type SketchConstraint as SolverConstraint } from './rushGraphSketch'
export type SketchDimension = {kind:'length';a:number;b:number} | {kind:'angle';a:number;b:number;c:number} | {kind:'horizontal'|'vertical';a:number;b:number} | {kind:'radius'|'diameter'}
export interface DimensionResult { points:Point2[]; measurements:{value:number|null;label:Point2;lines:Point2[][]}[] }
const measuredCache = new WeakMap<DirectSketch, DimensionResult>()
export function sketchDimensions(sketch:DirectSketch,edit?:{index:number;value:number}):DimensionResult {
  if(edit||sketch.dimensions?.length)requirePolylineSketch(sketch)
  if(!edit && (!sketch.dimensions || sketch.dimensions.length === 0)) {
    return {points:sketch.points,measurements:[]}
  }
  if(!edit && measuredCache.has(sketch)) {
    return measuredCache.get(sketch)!
  }
  if (sketch.analytic && (sketch.dimensions??[]).every(d=>d.kind==='radius'||d.kind==='diameter')) {
    const radius = edit ? (edit.value > 0 ? (sketch.dimensions![edit.index]?.kind === 'diameter' ? edit.value / 2 : edit.value) : (()=>{throw Error('Radius must be positive.')})()) : sketch.analytic.radius
    const result = (sketch.dimensions??[]).map(d => { const value=d.kind==='diameter'?radius*2:radius; const offset=[radius*1.2,0] as Point2; const label:Point2=[sketch.analytic!.center[0]+offset[0],sketch.analytic!.center[1]]; const lines:Point2[][]=[[[sketch.analytic!.center[0],sketch.analytic!.center[1]],[sketch.analytic!.center[0]+radius,sketch.analytic!.center[1]]]]; return {value,label,lines} })
    const out = {points:sketch.points,measurements:result}
    if (!edit) measuredCache.set(sketch, out)
    return out
  }
  // Measurement never runs the solver or changes the geometry being measured.
  const measured = callGeometryRust('cad_dimensions',{points:sketch.points,dimensions:sketch.dimensions??[],...(edit?{edit}:{})}) as DimensionResult
  const dimensions = sketch.dimensions??[]
  if (edit && dimensions.length > 1 && dimensions.every(d=>d.kind==='length') && sketch.points.length>=3 && sketch.points.length<=16) {
    const baseline = measuredCache.get(sketch) ?? (callGeometryRust('cad_dimensions',{points:sketch.points,dimensions}) as DimensionResult)
    const constraints: SolverConstraint[] = dimensions.map((d,i) => ({id:`dimension-${i}`,kind:'distance',a:`p${(d as {a:number}).a}`,b:`p${(d as {b:number}).b}`,value:i===edit.index?edit.value:baseline.measurements[i].value!}))
    const solved = solveRushGraphSketch(sketch.points.map((position,i)=>({id:`p${i}`,position:[...position] as Point2})),constraints)
    if (solved.status==='inconsistent'||solved.status==='not_converged') throw Error('Sketch dimensions conflict or did not converge.')
    const points = solved.points.map(p=>p.position)
    const result = callGeometryRust('cad_dimensions',{points,dimensions:sketch.dimensions??[]}) as DimensionResult
    return {points,measurements:result.measurements}
  }
  if (!edit) measuredCache.set(sketch, measured)
  return measured
}
export function editSketchDimension(sketch:DirectSketch,index:number,value:number):DirectSketch {
  if(sketch.analytic) { const dimension=sketch.dimensions?.[index]; if(!dimension || (dimension.kind!=='radius'&&dimension.kind!=='diameter')) throw Error('Analytic curves support radius and diameter dimensions only.'); const radius=dimension.kind==='diameter'?value/2:value; if(!(radius>0&&Number.isFinite(radius))) throw Error('Radius must be positive.'); const analytic={...sketch.analytic,radius}; return {...sketch,analytic,points:sampleCurve(analytic)} }
  return {...sketch,points:sketchDimensions(sketch,{index,value}).points}
}
export function bridgeNurbsCurves(a:NurbsCurve,b:NurbsCurve,endA:'start'|'end'='end',endB:'start'|'end'='start',tension=1):NurbsCurve {
  return callGeometryRust('cad_bridge_curve',{a,b,endA,endB,tension})
}

const dimensionStatusCache = new WeakMap<DirectSketch, ReturnType<typeof solveRushGraphSketch> | null>()
/** State of the length constraints preserved together by dimension editing. */
export function sketchDimensionStatus(sketch:DirectSketch) {
 if (dimensionStatusCache.has(sketch)) return dimensionStatusCache.get(sketch)!
 const dimensions=sketch.dimensions??[]
 if(sketch.analytic || sketch.points.length<3 || sketch.points.length>16 || !dimensions.length || !dimensions.every(d=>d.kind==='length')) {
  dimensionStatusCache.set(sketch, null)
  return null
 }
 const measured=sketchDimensions(sketch)
 const result = solveRushGraphSketch(sketch.points.map((position,i)=>({id:`p${i}`,position:[...position] as Point2})),dimensions.map((d,i)=>({id:`dimension-${i}`,kind:'distance',a:`p${(d as {a:number}).a}`,b:`p${(d as {b:number}).b}`,value:measured.measurements[i].value!})))
 dimensionStatusCache.set(sketch, result)
 return result
}
