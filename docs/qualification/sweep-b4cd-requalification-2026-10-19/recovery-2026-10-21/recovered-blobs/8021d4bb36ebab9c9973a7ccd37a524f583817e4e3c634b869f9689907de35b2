import type {NurbsCurve} from './nurbsCurve'
import type {CurveOffsetDiagnostics} from './curveOffsetDiagnostics'
export function offsetPreviewSegments(curves:NurbsCurve[],diagnostics:CurveOffsetDiagnostics|null){
 const segments=curves.flatMap(c=>c.degree===1?c.controlPoints.slice(1).map((b,i)=>({a:c.controlPoints[i]!,b})):[])
 const bad=new Set([...(diagnostics?.crossings??[]).flat(),...(diagnostics?.contacts??[]).flat(),...(diagnostics?.degenerate??[])])
 const uncertain=new Set((diagnostics?.uncertain??[]).flat())
 return segments.map((s,index)=>({...s,index,state:bad.has(index)?'error' as const:uncertain.has(index)?'uncertain' as const:'preview' as const}))
}
