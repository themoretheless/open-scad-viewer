/** Evidence for one caller-selected fiber; never whole-surface absence. */
export interface SurfaceIsoSelfIntersectionReport {
 schema:'nurbs-surface-iso-self-intersection/1'
 status:'present'|'iso-simple'|'unresolved'
 axis:0|1
 parameter:number
 maxSpans:number
 cells:number
 presenceProven:boolean
 isoAbsenceProven:boolean
 surfaceAbsenceProven:false
 isoCoverageComplete:boolean
 allContactBranchesEnumerated:false
 witnessCoordinates:'normalized-original-active-domain'
 crossings:{method:'original-planar-krawczyk';parameterBoxes:[[[number,number],[number,number]],[[number,number],[number,number]]];positionBounds:[[number,number],[number,number],[number,number]]}[]
 unresolvedCurveParameters:[[number,number],[number,number]][]
}
const interval=(x:unknown):x is [number,number]=>Array.isArray(x)&&x.length===2&&x.every(v=>typeof v==='number'&&Number.isFinite(v))&&x[0]<=x[1]
export function isSurfaceIsoAuditReport(value:unknown):value is SurfaceIsoSelfIntersectionReport {
 if(!value||typeof value!=='object')return false
 const r=value as SurfaceIsoSelfIntersectionReport
 if(r.schema!=='nurbs-surface-iso-self-intersection/1'||!['present','iso-simple','unresolved'].includes(r.status)||![0,1].includes(r.axis)
  ||typeof r.parameter!=='number'||!Number.isFinite(r.parameter)||r.parameter<=0||r.parameter>=1
  ||!Number.isInteger(r.maxSpans)||r.maxSpans<1||r.maxSpans>100000||!Number.isInteger(r.cells)||r.cells<0||r.cells>r.maxSpans
  ||r.presenceProven!==(r.status==='present')||r.isoAbsenceProven!==(r.status==='iso-simple')
  ||r.surfaceAbsenceProven!==false||r.allContactBranchesEnumerated!==false||typeof r.isoCoverageComplete!=='boolean'
  ||r.witnessCoordinates!=='normalized-original-active-domain'||!Array.isArray(r.crossings)||r.crossings.length>r.cells
  ||!Array.isArray(r.unresolvedCurveParameters)||r.unresolvedCurveParameters.length>100000)return false
 if((r.crossings.length>0)!==r.presenceProven)return false
 if(r.isoAbsenceProven&&!r.isoCoverageComplete)return false
 if(r.isoCoverageComplete&&r.unresolvedCurveParameters.length>0)return false
 for(const d of r.unresolvedCurveParameters)if(!Array.isArray(d)||d.length!==2||!d.every(interval))return false
 for(const c of r.crossings){
  if(!c||c.method!=='original-planar-krawczyk'||!Array.isArray(c.parameterBoxes)||c.parameterBoxes.length!==2
   ||!c.parameterBoxes.every(uv=>Array.isArray(uv)&&uv.length===2&&uv.every(x=>interval(x)&&x[0]>0&&x[1]<1))
   ||!Array.isArray(c.positionBounds)||c.positionBounds.length!==3||!c.positionBounds.every(interval))return false
  const [a,b]=c.parameterBoxes
  if(![a,b].every(uv=>uv[1-r.axis]![0]===r.parameter&&uv[1-r.axis]![1]===r.parameter))return false
  if(!(a[r.axis]![1]<b[r.axis]![0]||b[r.axis]![1]<a[r.axis]![0]))return false
 }
 return true
}
