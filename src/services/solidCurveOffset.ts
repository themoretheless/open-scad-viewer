import {validCurveOffsetDiagnostics,type CurveOffsetDiagnostics} from './curveOffsetDiagnostics'
import type {DirectDocument} from './directModeling'
import type {NurbsCurve} from './nurbsCurve'
import {callNurbsRust} from './geometry/nurbs'

export interface SolidCurveOffsetOptions {
 id:string
 createdId:string
 distance:number
 toleranceMm:number
 maxCells:number
 maxPairs:number
 join?:'bevel'
}
export interface CurveOffsetReport {
 accepted:true
 closed:boolean
 errorUpperMm:number
 toleranceMm:number
 wholeCurve:boolean
 wholeWire?:true
 regionTrimmed?:false
 method:'outward-rational-jets-chord-bound/1'|'outward-source-offset-bevel-wire/1'
 cells:{domain:[number,number];errorUpperMm:number;source?:{kind:'source-offset';domain:[number,number]}|{kind:'bevel';sourceKnot:number}}[]
 chainDiagnostics:CurveOffsetDiagnostics|null
 offsetRegularityCertified:false
 regionTopologyCertified:false
}
/** Preserve the source and retain the bounded chain as separate editable curves. */
export function offsetSolidCurve(source:DirectDocument,p:SolidCurveOffsetOptions):{document:DirectDocument;report:CurveOffsetReport} {
 const selected=source.curves?.find(c=>c.id===p.id)
 if(!selected)throw Error('Select an existing NURBS curve.')
 if(!p.createdId||!Number.isFinite(p.distance)||!Number.isFinite(p.toleranceMm)||p.toleranceMm<=0
  ||!Number.isInteger(p.maxCells)||p.maxCells<1||p.maxCells>65536
  ||!Number.isInteger(p.maxPairs)||p.maxPairs<1||p.maxPairs>1000000)throw Error('Use a finite distance, positive tolerance and valid calculation limits.')
 const points=selected.curve.controlPoints,dimension=points[0]?.length,z=points[0]?.[2]
 if(!points.length||(dimension!==2&&dimension!==3)||points.some(q=>q.length!==dimension||q.some(v=>!Number.isFinite(v))||(dimension===3&&q[2]!==z)))
  throw Error('Offset requires a curve in an XY plane. Use a planar curve with a constant Z coordinate.')
 const curve={...selected.curve,controlPoints:points.map(q=>q.slice(0,2))}
 const bevel=p.join==='bevel'&&p.distance!==0
 const result=callNurbsRust<{curves:NurbsCurve[];report:CurveOffsetReport}>(bevel?'curve_offset_bevel_wire':'curve_offset_bounded',{
  curve,distance:p.distance,toleranceMm:p.toleranceMm,maxCells:p.maxCells,maxPairs:p.maxPairs,
 })
 const r=result.report
 if(!r||r.accepted!==true||(bevel?(r.wholeCurve!==false||r.wholeWire!==true||r.regionTrimmed!==false||r.method!=='outward-source-offset-bevel-wire/1'):(r.wholeCurve!==true||r.method!=='outward-rational-jets-chord-bound/1'))
  ||r.regionTopologyCertified!==false||r.offsetRegularityCertified!==false||typeof r.closed!=='boolean'
  ||r.toleranceMm!==p.toleranceMm||!Number.isFinite(r.errorUpperMm)||r.errorUpperMm<0||r.errorUpperMm>p.toleranceMm
  ||!Array.isArray(r.cells)||r.cells.length>p.maxCells||!Array.isArray(result.curves)||!result.curves.length
  ||result.curves.length>Math.max(1,Math.ceil(p.maxCells/255)))throw Error('Offset returned an invalid positional certificate.')
 for(const cell of r.cells)if(!Array.isArray(cell.domain)||cell.domain.length!==2||!cell.domain.every(Number.isFinite)
  ||cell.domain[0]>=cell.domain[1]||!Number.isFinite(cell.errorUpperMm)||cell.errorUpperMm<0||cell.errorUpperMm>r.errorUpperMm)
  throw Error('Offset returned an invalid cell certificate.')
 if(r.cells.length? !validCurveOffsetDiagnostics(r.chainDiagnostics,r.cells.length):r.chainDiagnostics!==null)throw Error('Offset returned invalid chain diagnostics.')
 if((source.curves?.length??0)+(source.surfaces?.length??0)+result.curves.length>128)throw Error('Offset exceeds the document limit of 128 NURBS objects. Reduce the curve count or increase tolerance.')
 if(p.distance!==0){
  let cellIndex=0
  for(let i=0;i<result.curves.length;i++){
   const c=result.curves[i]!,points=c.controlPoints
   if(c.degree!==1||c.periodic===true||c.weights.length!==points.length||c.weights.some(w=>w!==1)
    ||c.knots.length!==points.length+2||c.knots[0]!==c.knots[1]||c.knots.at(-1)!==c.knots.at(-2))throw Error('Offset returned an invalid retained chord chain.')
   if(i){const previous=result.curves[i-1]!.controlPoints.at(-1)!;if(previous.length!==points[0]?.length||previous.some((v,j)=>v!==points[0]![j]))throw Error('Offset chain has a disconnected chunk.')}
   for(let edge=0;edge<points.length-1;edge++){
    const cell=r.cells[cellIndex++]
    if(!cell||c.knots[edge+1]!==cell.domain[0]||c.knots[edge+2]!==cell.domain[1])throw Error('Offset cells do not match the retained geometry.')
   }
  }
  if(cellIndex!==r.cells.length)throw Error('Offset cells do not match the retained geometry.')
  if(r.closed){const a=result.curves[0]!.controlPoints[0]!,b=result.curves.at(-1)!.controlPoints.at(-1)!;if(a.some((v,i)=>v!==b[i]))throw Error('Offset returned an open chain labelled closed.')}
 }
 if(bevel){
  const start=selected.curve.knots[selected.curve.degree]!,end=selected.curve.knots[selected.curve.controlPoints.length]!
  let cursor=start,lastJoin=false
  for(const [i,cell] of r.cells.entries()){
   const role=cell.source
   if(!role)throw Error('Offset returned invalid source roles.')
   if(role.kind==='source-offset'){
    if(!Array.isArray(role.domain)||role.domain.length!==2||!role.domain.every(Number.isFinite)||role.domain[0]!==cursor||role.domain[1]<=cursor||role.domain[1]>end)throw Error('Offset returned invalid source roles.')
    cursor=role.domain[1];lastJoin=false
   }else{
    const seam=i===r.cells.length-1&&r.closed&&role.sourceKnot===start&&cursor===end
    if(role.kind!=='bevel'||!Number.isFinite(role.sourceKnot)||i===0||lastJoin||(i===r.cells.length-1&&!seam)||(!seam&&role.sourceKnot!==cursor))throw Error('Offset returned invalid source roles.')
    lastJoin=true
   }
  }
  if(cursor!==end)throw Error('Offset source roles do not cover the whole curve.')
 }
 const occupied=new Set([...source.sketches,...source.bodies,...source.curves??[],...source.surfaces??[]].map(o=>o.id))
 const additions=result.curves.map((c,i)=>{
  const id=i===0?p.createdId:`${p.createdId}:${i}`
  if(occupied.has(id))throw Error('Each offset curve needs a unique identity.')
  if(!Array.isArray(c.controlPoints)||!c.controlPoints.length||c.controlPoints.length>256
   ||c.controlPoints.some(q=>q.length!==2||q.some(v=>!Number.isFinite(v))))throw Error('Offset returned invalid curve coordinates.')
  return {id,name:`${selected.name} · offset ${i+1}`,group:selected.group,
   curve:p.distance===0?structuredClone(selected.curve):{...c,controlPoints:c.controlPoints.map(q=>dimension===3?[q[0]!,q[1]!,z!]:[...q])}}
 })
 const document=structuredClone(source)
 ;(document.curves??=[]).push(...additions)
 return {document,report:structuredClone(r)}
}
