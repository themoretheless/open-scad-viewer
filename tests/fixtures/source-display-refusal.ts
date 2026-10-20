// Qualification page: the production notice rendered with a saved real-worker denial.
import {createApp,h,ref} from 'vue'
import SourceBodyFailureNotice from '../../src/components/SourceBodyFailureNotice.vue'
import {SourceBodyDisplayError} from '../../src/services/sourceBodyDisplay'
import type {SourceBodyDefinition,SourceBodyResult} from '../../src/services/sourceBody'
import request from '../../docs/qualification/mapped-retained-carrier-2026-10-08/multispan-root-body-step-request-2026-10-10.json'
import report from '../../docs/qualification/mapped-retained-carrier-2026-10-08/multispan-root-worker-2026-10-10.json'
const error=new SourceBodyDisplayError(report.refused.diagnostics as SourceBodyResult['diagnostics'],request.definition as SourceBodyDefinition,'knot-root-chain')
createApp({setup(){
 const locale=ref('ru'),retries=ref(0)
 return ()=>h('main',{style:'max-width:720px;margin:32px auto;padding:16px;font:13px system-ui;color:#222;background:#fafafa'},[
  h('h1',{style:'font-size:16px'},'Native source refusal · browser qualification'),
  h('p','Production notice with the saved WASM worker refusal. Retry here records the UI event only.'),
  h('button',{onClick:()=>locale.value=locale.value==='ru'?'en':'ru'},'RU / EN'),
  h('section',{style:'margin-top:16px'},[h(SourceBodyFailureNotice,{name:'Knot root chain',id:'knot-root-chain',error,locale:locale.value,onRetry:()=>retries.value++})]),
  h('output',{'aria-label':'Retry events',style:'display:block;margin-top:16px'},String(retries.value)),
 ])
}}).mount('#app')
