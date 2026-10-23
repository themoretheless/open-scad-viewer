import type {DirectDocument} from './directModeling'
import {callNurbsRust} from './geometry/nurbs'
import {validCurveOffsetDiagnostics,type CurveOffsetDiagnostics} from './curveOffsetDiagnostics'
/** Inspect current ordered retained chords; historical offset summaries are ignored. */
export function inspectCurrentCurveChain(document:DirectDocument,ids:string[],maxPairs=1000000):CurveOffsetDiagnostics {
 if(!ids.length||ids.length>128||new Set(ids).size!==ids.length)throw Error('Select an ordered chord chain without repeated objects.')
 const curves=ids.map(id=>{
  const item=document.curves?.find(c=>c.id===id)
  if(!item)throw Error('Selected chain curve no longer exists.')
  return item.curve
 })
 const segments=curves.reduce((n,c)=>n+c.controlPoints.length-1,0)
 const result=callNurbsRust<CurveOffsetDiagnostics>('curve_chain_diagnostics',{curves,maxPairs})
 if(!validCurveOffsetDiagnostics(result,segments))throw Error('Invalid current chain diagnostics.')
 return result
}
