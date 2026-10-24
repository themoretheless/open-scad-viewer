// Test-only fault injection. Open on a separate origin to isolate saved drafts.
import {createApp,h,ref} from 'vue'
import DirectModeler from '../../src/features/DirectModeler.vue'
import {warmGeometryKernel} from '../../src/services/geometry/kernel'
import {createBrepBox,tessellateNurbsBrep} from '../../src/services/geometry/brep'
import type {MainSolidRequest} from '../../src/services/mainSolidProtocol'
const NativeWorker=window.Worker,requests=ref(0),injected=ref(false)
window.Worker=class extends NativeWorker {
 override postMessage(message:MainSolidRequest,transfer?:Transferable[]){
  if(message?.job?.kind==='topology'){
   requests.value++
   if(!injected.value){
    injected.value=true
    queueMicrotask(()=>this.dispatchEvent(new ErrorEvent('error',{message:'Qualification: injected topology worker failure'})))
    return
   }
  }
  super.postMessage(message,transfer??[])
 }
}
await warmGeometryKernel()
const brep=createBrepBox([0,0,0],[20,20,20])
const initialDocument={version:1 as const,sketches:[],curves:[],surfaces:[],bodies:[{
 id:'topology-fixture-cube',name:'Контрольный куб',brep,mesh:tessellateNurbsBrep(brep),
}]}
createApp({setup:()=>()=>h('main',[
 h('p',{role:'status','aria-label':'Qualification topology requests',style:'margin:8px'},`Topology requests: ${requests.value}; injected failure: ${injected.value}`),
 h(DirectModeler,{open:true,locale:'ru',embedded:false,canAppend:false,remainingSource:0,
  initialDocument,initialSelection:'topology-fixture-cube'}),
])}).mount('#app')
