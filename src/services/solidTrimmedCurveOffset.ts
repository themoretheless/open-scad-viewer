import type {DirectDocument} from './directModeling'
import type {NurbsCurve} from './nurbsCurve'
import {callNurbsRust} from './geometry/nurbs'
import type {SolidCurveOffsetOptions} from './solidCurveOffset'
export interface TrimmedOffsetReport {
 accepted:true;method:'represented-bevel-offset-fill/1';fillRule:'nonzero'|'evenodd';regionTrimmed:true
 topologyScope:'represented-reconstructed-chord-graph';regionTopologyCertified:false;originalOffsetTopologyCertified:false
 sourceWireErrorUpperMm:number;intersectionConstructionErrorUpperMm:number
}
export type TrimmedOffsetOptions=SolidCurveOffsetOptions & {fillRule:'nonzero'|'evenodd';intersectionToleranceMm:number;maxWitnessChecks:number}
export function offsetTrimmedSolidCurve(source:DirectDocument,p:TrimmedOffsetOptions) {
 const selected=source.curves?.find(c=>c.id===p.id)
 if(!selected)throw Error('Select an existing NURBS curve.')
 if(!p.createdId||!Number.isFinite(p.distance)||p.distance===0||!Number.isFinite(p.toleranceMm)||p.toleranceMm<=0
 ||!Number.isFinite(p.intersectionToleranceMm)||p.intersectionToleranceMm<=0
 ||!['nonzero','evenodd'].includes(p.fillRule)
 ||!Number.isInteger(p.maxCells)||p.maxCells<1||p.maxCells>65536
 ||[p.maxPairs,p.maxWitnessChecks].some(n=>!Number.isInteger(n)||n<1||n>1000000))throw Error('Use positive tolerances and valid calculation limits.')
 const points=selected.curve.controlPoints,dimension=points[0]?.length,z=points[0]?.[2]
 if(!points.length||(dimension!==2&&dimension!==3)||points.some(q=>q.length!==dimension||q.some(v=>!Number.isFinite(v))||(dimension===3&&q[2]!==z)))throw Error('Use an XY planar curve with constant Z.')
 const result=callNurbsRust<{loops:NurbsCurve[][];report:TrimmedOffsetReport}>('curve_offset_trimmed_bevel',{
 curve:{...selected.curve,controlPoints:points.map(q=>q.slice(0,2))},distance:p.distance,toleranceMm:p.toleranceMm,maxCells:p.maxCells,
 intersectionToleranceMm:p.intersectionToleranceMm,maxPairs:p.maxPairs,maxWitnessChecks:p.maxWitnessChecks,fillRule:p.fillRule,
 })
 const r=result.report
 if(!r||r.accepted!==true||r.method!=='represented-bevel-offset-fill/1'||r.fillRule!==p.fillRule||r.regionTrimmed!==true
 ||r.topologyScope!=='represented-reconstructed-chord-graph'||r.regionTopologyCertified!==false||r.originalOffsetTopologyCertified!==false
 ||!Number.isFinite(r.sourceWireErrorUpperMm)||r.sourceWireErrorUpperMm<0||r.sourceWireErrorUpperMm>p.toleranceMm
 ||!Number.isFinite(r.intersectionConstructionErrorUpperMm)||r.intersectionConstructionErrorUpperMm<0||r.intersectionConstructionErrorUpperMm>p.intersectionToleranceMm
 ||!Array.isArray(result.loops)||result.loops.length>65536)throw Error('Invalid trimmed offset report.')
 if(!result.loops.length)throw Error('Offset leaves no filled boundary. Reduce the offset or change the fill rule.')
 const count=result.loops.reduce((n,loop)=>n+(Array.isArray(loop)?loop.length:129),0)
 if(count+(source.curves?.length??0)+(source.surfaces?.length??0)>128)throw Error('Trimmed offset exceeds 128 NURBS objects.')
 const occupied=new Set([...source.sketches,...source.bodies,...source.curves??[],...source.surfaces??[]].map(o=>o.id))
 const loopIds:string[][]=[]
 const additions=result.loops.flatMap((loop,l)=>{
  if(!Array.isArray(loop)||!loop.length)throw Error('Invalid trimmed offset loop.')
  const ids:string[]=[];loopIds.push(ids)
  const added=loop.map((c,i)=>{
   const q=c.controlPoints,n=q?.length
   if(c.degree!==1||c.periodic===true||!Array.isArray(q)||n<2||n>256||q.some(v=>!Array.isArray(v)||v.length!==2||v.some(x=>!Number.isFinite(x)))
   ||!Array.isArray(c.weights)||c.weights.length!==n||c.weights.some(w=>w!==1)
   ||!Array.isArray(c.knots)||c.knots.length!==n+2||c.knots.some(x=>!Number.isFinite(x))
   ||c.knots[0]!==c.knots[1]||c.knots.at(-1)!==c.knots.at(-2)||q.slice(1).some((v,j)=>c.knots[j+1]!>=c.knots[j+2]!||v.every((x,k)=>x===q[j]![k])))throw Error('Invalid trimmed boundary geometry.')
   if(i&&loop[i-1]!.controlPoints.at(-1)!.some((v,k)=>v!==q[0]![k]))throw Error('Disconnected trimmed boundary.')
   const id=`${p.createdId}:${l}:${i}`
   if(occupied.has(id))throw Error('Trimmed offset identities already exist.')
   occupied.add(id);ids.push(id)
   return {offsetRegion:{version:1 as const,scope:'at-construction' as const,sourceId:selected.id,loopId:`${p.createdId}:${l}`,part:i,parts:loop.length,fillRule:p.fillRule,originalOffsetTopologyCertified:false as const},id,name:`${selected.name} · offset ${l+1}.${i+1}`,group:selected.group,curve:{...c,controlPoints:q.map(v=>dimension===3?[...v,z!]:[...v])}}
  })
  if(loop[0]!.controlPoints[0]!.some((v,k)=>v!==loop.at(-1)!.controlPoints.at(-1)![k]))throw Error('Open trimmed boundary.')
  return added
 })
 const document=structuredClone(source);(document.curves??=[]).push(...additions)
 return {document,loopIds,report:structuredClone(r)}
}
