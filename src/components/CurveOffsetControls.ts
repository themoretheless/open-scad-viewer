import {Fragment,h} from 'vue'
import CadQuantityInput from './CadQuantityInput.vue'
import type {CurveOffsetReport} from '../services/solidCurveOffset'
interface Props {
 locale:string;join:'smooth'|'bevel';distance:number;tolerance:number;report:CurveOffsetReport|null
 'onUpdate:join'?:(value:'smooth'|'bevel')=>void
 'onUpdate:distance'?:(value:number)=>void
 'onUpdate:tolerance'?:(value:number)=>void
 onDistanceValidity:(valid:boolean)=>void
 onToleranceValidity:(valid:boolean)=>void
}
export default function CurveOffsetControls(p:Props){
 const label=(ru:string,en:string)=>p.locale==='ru'?ru:en
 const diagnostic=p.report?.chainDiagnostics
 const distance=label('Смещение, мм','Offset, mm'),tolerance=label('Допуск, мм','Tolerance, mm')
 return h(Fragment,[
  h('label',[distance,h(CadQuantityInput,{modelValue:p.distance,locale:p.locale,'aria-label':distance,'onUpdate:modelValue':p['onUpdate:distance'],onValidity:p.onDistanceValidity})]),
  h('label',[tolerance,h(CadQuantityInput,{modelValue:p.tolerance,locale:p.locale,min:.000001,'aria-label':tolerance,'onUpdate:modelValue':p['onUpdate:tolerance'],onValidity:p.onToleranceValidity})]),
  h('label',[label('Соединения','Joins'),h('select',{value:p.join,'aria-label':label('Соединения','Joins'),onChange:(e:Event)=>p['onUpdate:join']?.((e.target as HTMLSelectElement).value as 'smooth'|'bevel')},[h('option',{value:'smooth'},label('Только гладкая кривая','Smooth curve only')),h('option',{value:'bevel'},'Bevel')])]),
  p.join==='bevel'?h('small',label('Соединения прямые. Внутренние пересечения не обрезаны.','Straight joins. Internal crossings are not trimmed.')):null,
  h('small',label('Кривая в XY с постоянной Z. Исходник сохраняется. Enter — применить, Esc — отменить.','XY curve with constant Z. Source retained. Enter applies, Esc cancels.')),
  diagnostic?h('small',{'data-testid':'curve-offset-diagnostics'},
   label('Цепочка: пересечения ','Chain: crossings ')+diagnostic.crossings.length+label(', контакты ', ', contacts ')+diagnostic.contacts.length
   +label(', неоднозначные пары ', ', unresolved pairs ')+diagnostic.uncertain.length+' · '+diagnostic.checks+'/'+diagnostic.totalPairs
   +(diagnostic.complete?label(' · Проверка завершена',' · Check complete'):label(' · Проверка неполная; уменьшите смещение или разделите кривую.',' · Check incomplete; reduce offset or split the curve.'))):null,
  p.report?h('small',{'data-testid':'curve-offset-report'},label('Отклонение не больше ','Deviation at most ')+p.report.errorUpperMm.toPrecision(6)+' mm · '+label('Топология области не проверена','Region topology unverified')):null,
 ])
}
