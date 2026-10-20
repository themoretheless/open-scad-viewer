// Isolated qualification surface: production panel and real production worker.
// Does not read or write model documents or saved drafts.
import {createApp,h,ref} from 'vue'
import ProfileBridgeCheck from '../../src/components/ProfileBridgeCheck.vue'
import type {NurbsSurface} from '../../src/services/nurbsSurface'
import fixture from '../../docs/qualification/mapped-retained-carrier-2026-10-08/profile-bridge-native-transport-2026-10-10.json'
createApp({setup(){
 const ru=ref(true),curved=ref(false),active=ref(true)
 const surfaces=ref(structuredClone(fixture.request.surfaces) as [NurbsSurface,NurbsSurface])
 function change(){curved.value=!curved.value;surfaces.value=structuredClone(curved.value?fixture.curvedWholeCoverage.request.surfaces:fixture.request.surfaces) as [NurbsSurface,NurbsSurface]}
 return ()=>h('main',{style:'max-width:660px;margin:24px auto;padding:12px;font:13px system-ui;color:#222;background:#fafafa'},[
  h('h1',{style:'font-size:16px'},'Profile coverage · real worker'),
  h('p','Production panel and WASM worker. Isolated surfaces; no model or draft writes.'),
  h('button',{onClick:()=>ru.value=!ru.value},'RU / EN'),
  h('button',{onClick:change,style:'margin-left:8px'},curved.value?'Use planar profiles':'Use curved profiles'),
  h('button',{onClick:()=>active.value=!active.value,style:'margin-left:8px'},active.value?'Activate another command':'Finish other command'),
  h('button',{onClick:()=>{surfaces.value[0].controlPoints[0][0][0]+=0.01},style:'margin-left:8px'},'Edit control point in place'),
  h(ProfileBridgeCheck,{surfaces:surfaces.value,active:active.value,ru:ru.value}),
 ])
}}).mount('#app')
