import {Fragment,h} from 'vue'
import CadQuantityInput from './CadQuantityInput.vue'
import type {CoonsBoundaryPreparation} from '../services/nurbsConstructors'

interface Props {
 enabled:boolean;budget:number;locale:string;reports?:CoonsBoundaryPreparation['report'][]
 'onUpdate:enabled'?:(value:boolean)=>void
 'onUpdate:budget'?:(value:number)=>void
 onValidity:(valid:boolean)=>void
}
/** Stateless controls share the command card's layout and quantity validation. */
export default function CoonsPreparationControls(p:Props){
 const label=(ru:string,en:string)=>p.locale==='ru'?ru:en
 const budgetLabel=label('Допуск подготовки, мм','Preparation budget, mm')
 return h(Fragment,[
  h('label',[h('input',{checked:p.enabled,type:'checkbox','aria-label':'Prepare patch boundary weights',onChange:(event:Event)=>{p['onUpdate:enabled']?.((event.target as HTMLInputElement).checked);p.onValidity(true)}}),label('Согласовать веса границ','Match boundary weights')]),
  p.enabled?h('label',[budgetLabel,h(CadQuantityInput,{'aria-label':budgetLabel,modelValue:p.budget,kind:'length',locale:p.locale,min:0,'onUpdate:modelValue':p['onUpdate:budget'],onValidity:p.onValidity})]):null,
  p.reports?h('small',{'data-diagnostic':'patch-preparation'},label('Верхняя оценка ошибки границ: ','Boundary error upper bound: ')+Math.max(...p.reports.map(r=>r.errorUpper)).toPrecision(4)+' mm'):null,
 ])
}
