import {h} from 'vue'
import type {SolidNurbsCurve} from '../services/solidNurbs'
import type {CurveOffsetReport} from '../services/solidCurveOffset'
import {offsetPreviewSegments} from '../services/curveOffsetPreview'
interface Props {curves:SolidNurbsCurve[];id?:string;report?:CurveOffsetReport|null;ids?:string[];diagnostics?:import('../services/curveOffsetDiagnostics').CurveOffsetDiagnostics;project:(point:[number,number,number])=>number[];sample?:(curve:SolidNurbsCurve['curve'])=>number[][]}
export default function CurveOffsetPreview(p:Props){
 const curves=p.diagnostics?p.curves:p.ids?p.ids.map(id=>p.curves.find(c=>c.id===id)!).filter(Boolean):p.curves.filter(c=>c.id===p.id||c.id.startsWith(p.id+':'))
 const xy=(q:number[])=>p.project([q[0]!,q[1]!,q[2]??0]).join(',')
 const segments=offsetPreviewSegments(curves.map(c=>c.curve),p.diagnostics??p.report?.chainDiagnostics??null)
 return h('g',{'data-preview':'curve-offset','pointer-events':'none'},[
  ...curves.map(c=>h('polyline',{key:c.id,points:(c.curve.degree===1?c.curve.controlPoints:p.sample?.(c.curve)??[]).map(xy).join(' '),fill:'none',stroke:'#77eac5','stroke-width':3,'vector-effect':'non-scaling-stroke'})),
  ...segments.filter(s=>s.state!=='preview').map(s=>h('polyline',{key:s.index,'data-diagnostic':'curve-offset-'+s.state,'data-segment':s.index,points:[s.a,s.b].map(xy).join(' '),fill:'none',stroke:s.state==='error'?'#ff6978':'#ffc977','stroke-width':5,'vector-effect':'non-scaling-stroke'})),
 ])
}
