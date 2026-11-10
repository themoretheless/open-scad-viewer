import {callGeometryRust,GeometryKernelError} from './kernel'
import type {NurbsCurve} from '../nurbsCurve'
import type {NurbsCurveCurveIntersectionCertificate} from '../nurbsFoundation'

export interface ProfileSegmentReference {loop:number;curve:number}
export interface ProfileIntersectionReport {
  /** Distinct definitions only; does not search self-intersections inside one curve. */
  scope:'distinct-profile-segment-pairs'
  complete:boolean
  totalPairs:number
  visitedPairs:number
  resolvedPairs:number
  unvisitedPairs:number
  boxesVisited:number
  maxPairs:number
  maxBoxes:number
  toleranceMm:number
  pairs:Array<{first:ProfileSegmentReference;second:ProfileSegmentReference;report:NurbsCurveCurveIntersectionCertificate}>
}
export interface ProfileIntersectionOptions {toleranceMm?:number;maxPairs?:number;maxBoxes?:number}

const record=(value:unknown):value is Record<string,unknown>=>typeof value==='object'&&value!==null&&!Array.isArray(value)
const integer=(value:unknown):value is number=>typeof value==='number'&&Number.isSafeInteger(value)&&value>=0
const finite=(value:unknown):value is number=>typeof value==='number'&&Number.isFinite(value)
const interval=(value:unknown):value is [number,number]=>Array.isArray(value)&&value.length===2&&value.every(finite)&&value[0]<=value[1]
const invalid=():never=>{throw new GeometryKernelError('CAD_PROTOCOL','Invalid profile intersection report')}

/** Validates source ownership and aggregate work before a report enters the UI. */
export function validateProfileIntersectionReport(value:unknown,loops:readonly (readonly NurbsCurve[])[],options:Required<ProfileIntersectionOptions>):ProfileIntersectionReport {
  const sources=loops.flatMap((loop,l)=>loop.map((curve,c)=>({loop:l,curve:c,definition:curve})))
  if(loops.length<1||loops.length>64||loops.some(loop=>loop.length===0)||sources.length<2||sources.length>254||
    !finite(options.toleranceMm)||options.toleranceMm<=0||!integer(options.maxPairs)||options.maxPairs<1||options.maxPairs>10000||
    !integer(options.maxBoxes)||options.maxBoxes<1||options.maxBoxes>1000000) return invalid()
  const total=sources.length*(sources.length-1)/2
  if(!record(value)||value.scope!=='distinct-profile-segment-pairs'||typeof value.complete!=='boolean'||
    value.totalPairs!==total||value.maxPairs!==options.maxPairs||value.maxBoxes!==options.maxBoxes||value.toleranceMm!==options.toleranceMm||
    !integer(value.visitedPairs)||!integer(value.resolvedPairs)||!integer(value.unvisitedPairs)||!integer(value.boxesVisited)||
    value.visitedPairs>Math.min(total,options.maxPairs)||value.resolvedPairs>value.visitedPairs||value.unvisitedPairs!==total-value.visitedPairs||
    value.boxesVisited>options.maxBoxes||!Array.isArray(value.pairs)||value.pairs.length!==value.visitedPairs) return invalid()
  let visited=0,boxes=0,resolved=0
  for(let a=0;a<sources.length;a++) for(let b=a+1;b<sources.length;b++) {
    if(visited===value.visitedPairs) break
    const pair=value.pairs[visited++]
    const first=sources[a]!,second=sources[b]!
    if(!record(pair)||!record(pair.first)||!record(pair.second)||pair.first.loop!==first.loop||pair.first.curve!==first.curve||pair.second.loop!==second.loop||pair.second.curve!==second.curve||!record(pair.report)) return invalid()
    const report=pair.report,coverage=report.coverage
    if(report.version!=='nurbs-foundation/5'||report.kind!=='curve_curve'||report.rounding!=='binary64-nextafter-outward'||
      !record(coverage)||typeof coverage.method!=='string'||typeof coverage.complete!=='boolean'||
      !integer(coverage.boxesVisited)||!integer(coverage.resourceLimit)||coverage.resourceLimit!==Math.min(options.maxBoxes-boxes,8192)||coverage.boxesVisited>coverage.resourceLimit||
      !integer(coverage.bernsteinExcluded)||!integer(coverage.krawczykIsolated)||
      !Array.isArray(report.components)||!Array.isArray(report.unresolved)||coverage.complete!==(report.unresolved.length===0)||!record(report.evidence)) return invalid()
    const domain=(curve:NurbsCurve)=>[curve.knots[curve.degree]!,curve.knots[curve.controlPoints.length]!]
    const inDomain=(range:unknown,curve:NurbsCurve)=>interval(range)&&range[0]>=domain(curve)[0]!&&range[1]<=domain(curve)[1]!
    for(const component of report.components) {
      if(!record(component)||!interval(component.firstInterval)||!interval(component.secondInterval)||!inDomain(component.firstInterval,first.definition)||!inDomain(component.secondInterval,second.definition)||
        !Array.isArray(component.geometryEnclosure)||component.geometryEnclosure.length!==3||!component.geometryEnclosure.every(interval)) return invalid()
      if(component.kind==='point') {
        if(!finite(component.first)||!finite(component.second)||!finite(component.residual)||component.residual<0||
          !Array.isArray(component.point)||component.point.length!==3||!component.point.every(finite)||
          component.point.some((coordinate,k)=>coordinate<(component.geometryEnclosure as [number,number][])[k]![0]||coordinate>(component.geometryEnclosure as [number,number][])[k]![1])||
          component.first<component.firstInterval[0]||component.first>component.firstInterval[1]||component.second<component.secondInterval[0]||component.second>component.secondInterval[1]) return invalid()
      } else if(component.kind==='overlap') {
        if(typeof component.reversed!=='boolean'||!finite(component.maxControlResidual)||component.maxControlResidual<0) return invalid()
      } else return invalid()
    }
    for(const unresolved of report.unresolved) {
      if(!record(unresolved)||typeof unresolved.reason!=='string'||!Array.isArray(unresolved.parameterBox)||unresolved.parameterBox.length!==4||
        !inDomain(unresolved.parameterBox.slice(0,2),first.definition)||!inDomain(unresolved.parameterBox.slice(2,4),second.definition)) return invalid()
    }
    boxes+=coverage.boxesVisited
    if(coverage.complete) resolved++
  }
  if(boxes!==value.boxesVisited||resolved!==value.resolvedPairs||value.complete!==(value.visitedPairs===total&&resolved===total)) return invalid()
  return value as unknown as ProfileIntersectionReport
}

/** Read-only pair diagnostics; contacts include ordinary joins and never authorize trimming. */
export function inspectProfileIntersections(loops:NurbsCurve[][],options:ProfileIntersectionOptions={}):ProfileIntersectionReport {
  const limits={toleranceMm:options.toleranceMm??1e-7,maxPairs:options.maxPairs??128,maxBoxes:options.maxBoxes??32768}
  return validateProfileIntersectionReport(callGeometryRust('brep_profile_intersections',{loops,...limits}),loops,limits)
}
