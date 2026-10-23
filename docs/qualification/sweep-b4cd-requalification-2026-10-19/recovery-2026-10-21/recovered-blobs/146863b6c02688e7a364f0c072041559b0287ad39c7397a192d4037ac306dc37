import CurrentCurveChainInspection from './CurrentCurveChainInspection.vue'
import type {SolidNurbsCurve} from '../services/solidNurbs'
import {h,type FunctionalComponent} from 'vue'
import type {CurveOffsetConstruction} from '../services/curveOffsetConstruction'
const CurveOffsetConstructionInfo:FunctionalComponent<{value?:CurveOffsetConstruction;locale:string;curves:SolidNurbsCurve[];ids:string[];onResult?:(value:{curves:SolidNurbsCurve[];diagnostics:import('../services/curveOffsetDiagnostics').CurveOffsetDiagnostics}|null)=>void}>=({value:v,locale,curves,ids,onResult})=>{
 const ru=locale==='ru',text=(a:string,b:string)=>ru?a:b
 const heading=h('strong',curves.some(c=>c.id===ids[0])?text('NURBS-кривая · CV','NURBS curve · CV'):text('NURBS-поверхность · CV','NURBS surface · CV'))
 const instructions=h('small',{style:{display:'block',marginTop:'6px'}},text('Тяните жёлтые CV прямо в 3D-виде. Перетаскивание идёт в плоскости экрана; точные XYZ и вес — ниже.','Drag yellow CVs directly in the 3D view. Dragging follows the screen plane; exact XYZ and weight are below.'))
 const inspection=h(CurrentCurveChainInspection,{curves,ids,locale,onResult})
 if(!v)return h('section',{style:{display:'grid',gap:'8px'}},[heading,instructions,inspection])
 return h('section',{style:{display:'grid',gap:'8px',minWidth:'0'}},[heading,instructions,h('details',{'data-testid':'offset-construction',open:true},[
  h('summary',text('Offset · при построении','Offset · at construction')),
  h('small',{style:{display:'block',marginTop:'6px'}},text(`Смещение ${v.distanceMm} мм · допуск ${v.toleranceMm} мм · отклонение ≤ ${v.errorUpperMm.toPrecision(6)} мм.`,`Distance ${v.distanceMm} mm · tolerance ${v.toleranceMm} mm · deviation ≤ ${v.errorUpperMm.toPrecision(6)} mm.`)),
  h('small',{style:{display:'block',marginTop:'6px'}},text(`Пересечения ${v.crossings} · контакты ${v.contacts} · неразрешённые пары ${v.uncertain}.`,`Crossings ${v.crossings} · contacts ${v.contacts} · unresolved pairs ${v.uncertain}.`)),
  h('small',{style:{display:'block',marginTop:'6px'}},v.complete?text('Проверка цепочки завершена.','Chain inspection completed.'):text('Проверка цепочки не завершена.','Chain inspection incomplete.')),
  h('small',{style:{display:'block',marginTop:'6px'}},text('Сводка относится к моменту построения. После правок нужна новая проверка. Корректность области не подтверждена.','This summary describes construction time. Edits require a new inspection. Region validity is unverified.')),
 ]),inspection])
}
export default CurveOffsetConstructionInfo
