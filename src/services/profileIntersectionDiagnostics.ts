import type {NurbsCurve} from './nurbsCurve'
import type {ProfileIntersectionReport,ProfileSegmentReference} from './geometry/profileIntersections'

export interface ProfileIntersectionMarker {
  kind:'intersection'
  point:[number,number]
  first:ProfileSegmentReference
  second:ProfileSegmentReference
  firstInterval:[number,number]
  secondInterval:[number,number]
}
export interface ProfileIntersectionDiagnostics {
  points:ProfileIntersectionMarker[]
  overlaps:Array<{first:ProfileSegmentReference;second:ProfileSegmentReference}>
  unresolved:Array<{first:ProfileSegmentReference;second:ProfileSegmentReference}>
  endpointBands:Array<{first:ProfileSegmentReference;second:ProfileSegmentReference}>
  ordinaryJoins:number
  complete:boolean
  unvisitedPairs:number
}

/** Presentation of validated pair events. Completeness remains distinct-pair scoped. */
export function profileIntersectionDiagnostics(loops:NurbsCurve[][],report:ProfileIntersectionReport):ProfileIntersectionDiagnostics {
  const result:ProfileIntersectionDiagnostics={points:[],overlaps:[],unresolved:[],endpointBands:[],ordinaryJoins:0,complete:report.complete,unvisitedPairs:report.unvisitedPairs}
  for(const pair of report.pairs) {
    const a=loops[pair.first.loop]![pair.first.curve]!,b=loops[pair.second.loop]![pair.second.curve]!
    const exactEndpoint=(curve:NurbsCurve,t:number):number[]|null=>{
      const n=curve.controlPoints.length,p=curve.degree
      if(curve.periodic||curve.knots.slice(0,p+1).some(k=>k!==curve.knots[p])||curve.knots.slice(n).some(k=>k!==curve.knots[n]))return null
      return t===curve.knots[p]?curve.controlPoints[0]!:t===curve.knots[n]?curve.controlPoints[n-1]!:null
    }
    for(const event of pair.report.components) {
      if(event.kind==='overlap'){result.overlaps.push({first:pair.first,second:pair.second});continue}
      const first=event.first as number,second=event.second as number
      const start=exactEndpoint(a,first),end=exactEndpoint(b,second)
      const adjacent=pair.first.loop===pair.second.loop&&
        (pair.second.curve===pair.first.curve+1&&first===a.knots[a.controlPoints.length]&&second===b.knots[b.degree]||
          pair.first.curve===0&&pair.second.curve===loops[pair.first.loop]!.length-1&&first===a.knots[a.degree]&&second===b.knots[b.controlPoints.length])
      if(adjacent&&start&&end&&start.length===end.length&&start.every((x,k)=>x===end[k])){result.ordinaryJoins++;continue}
      // A numerical event enclosure containing a known shared endpoint cannot
      // distinguish that join from an additional contact within the tolerance.
      // Preserve it as uncertainty, rather than hide it or mark a definite defect.
      const joins:number[][]=[]
      if(pair.first.loop===pair.second.loop){
        if(pair.second.curve===pair.first.curve+1)joins.push([a.knots[a.controlPoints.length]!,b.knots[b.degree]!])
        if(pair.first.curve===0&&pair.second.curve===loops[pair.first.loop]!.length-1)joins.push([a.knots[a.degree]!,b.knots[b.controlPoints.length]!])
      }
      if(joins.some(([first,second])=>{
        const aa=exactEndpoint(a,first!),bb=exactEndpoint(b,second!),enclosure=event.geometryEnclosure as [number,number][]|undefined
        return aa&&bb&&aa.length===bb.length&&aa.every((x,k)=>x===bb[k])&&Array.isArray(enclosure)&&enclosure.length===3&&
          [aa[0]!,aa[1]!,0].every((x,k)=>x>=enclosure[k]![0]&&x<=enclosure[k]![1])
      })){
        result.endpointBands.push({first:pair.first,second:pair.second});continue
      }
      const point=event.point as number[]
      result.points.push({kind:'intersection',point:[point[0]!,point[1]!],first:pair.first,second:pair.second,
        firstInterval:event.firstInterval as [number,number],secondInterval:event.secondInterval as [number,number]})
    }
    if(pair.report.unresolved.length)result.unresolved.push({first:pair.first,second:pair.second})
  }
  result.complete=report.complete&&result.endpointBands.length===0
  return result
}
