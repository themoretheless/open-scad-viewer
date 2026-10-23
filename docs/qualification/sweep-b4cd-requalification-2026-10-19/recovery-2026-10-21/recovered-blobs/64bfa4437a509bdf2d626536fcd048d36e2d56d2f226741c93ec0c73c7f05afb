<script setup lang="ts">
import {computed,onUnmounted,ref,shallowRef,watch} from 'vue'
import type {MeshData} from '../core/mesh'
import type {StructuralSections} from '../services/structuralSections'
import {computeStructuralSections} from '../services/mainSolidWorker'
const props=defineProps<{meshes:MeshData[];selection:number[];source:string;ready:boolean;locale:string}>()
const label=(ru:string,en:string)=>props.locale==='ru'?ru:en
const axis=ref<'x'|'y'|'z'>('z'),positions=ref('5'),busy=ref(false),error=ref(''),url=ref('')
const result=shallowRef<StructuralSections|null>(null)
const mesh=computed(()=>props.selection.length===1?props.meshes[props.selection[0]]:null)
let revision=0,controller:AbortController|undefined
function clear(){revision++;controller?.abort();controller=undefined;busy.value=false;result.value=null;error.value='';if(url.value)URL.revokeObjectURL(url.value);url.value=''}
watch([axis,positions,()=>props.source,()=>props.ready,mesh,()=>mesh.value?.vertices,()=>mesh.value?.indices,()=>mesh.value?.transform],clear,{flush:'sync'})
watch(()=>props.selection,clear,{deep:true,flush:'sync'})
onUnmounted(clear)
async function inspect(){
  clear();if(!props.ready||!mesh.value)return
  const ticket=revision;controller=new AbortController();busy.value=true
  try{
    const stations=positions.value.trim()?positions.value.trim().split(/[\s,;]+/).map(Number):[]
    const value=await computeStructuralSections(mesh.value,axis.value,stations,{signal:controller.signal})
    if(ticket!==revision)return
    result.value=value
    url.value=URL.createObjectURL(new Blob([JSON.stringify(value,null,2)],{type:'application/json'}))
  }catch(cause){if(ticket===revision)error.value=String(cause)}
  finally{if(ticket===revision){busy.value=false;controller=undefined}}
}
</script>
<template>
  <details class="structural-sections"><summary>{{label('Сечения готового тела','Finished-body sections')}}</summary>
    <p>{{label('Анализ выбранной геометрии с оболочкой и отверстиями. Сначала примените облегчение и выберите результат. Это геометрические характеристики, не расчёт прочности детали.','Analyze the selected geometry including shells and holes. Apply lightening and select its result first. These are geometric properties, not a part-strength calculation.')}}</p>
    <label>{{label('Ось сечения','Section axis')}}<select v-model="axis" :aria-label="label('Ось сечения','Section axis')"><option>x</option><option>y</option><option>z</option></select></label>
    <label>{{label('Положения сечений, mm (по возрастанию)','Section positions, mm (increasing)')}}<input v-model="positions" maxlength="2048" placeholder="1, 5, 9"></label>
    <button :disabled="busy||!ready||!mesh" @click="inspect">{{label('Рассчитать сечения','Inspect sections')}}</button>
    <button v-if="busy" @click="clear">{{label('Отмена','Cancel')}}</button>
    <p v-if="error" role="alert">{{error}}</p>
    <section v-if="result" aria-label="Finished-body section results">
      <p>{{label('Площадь материала и моменты инерции относительно центра сечения.','Material area and centroidal second moments.')}} u={{'xyz'[result.planeAxes[0]]}}, v={{'xyz'[result.planeAxes[1]]}}.</p>
      <p>{{label('Компонентов поверхности по общим рёбрам:','Edge-connected boundary components:')}} {{result.connectivity.components.length}}.
        {{label('Общих вершин между компонентами:','Vertices shared between components:')}} {{result.connectivity.sharedVertices.length}}.</p>
      <p v-if="result.connectivity.sharedVertices.length" role="alert">{{label('Обнаружены компоненты, соединённые только вершинами сетки. Такое соединение не подтверждает передачу нагрузки.','Boundary components share mesh vertices without shared edges. This does not establish load transfer.')}}</p>
      <template v-if="result.materialAudit.status==='classified'">
        <p>{{label('Отдельных областей материала:','Separate material regions:')}} {{result.materialAudit.materialRegions}}.
          {{label('Допуск геометрической проверки:','Geometry audit tolerance:')}} {{result.materialAudit.toleranceMm.toExponential(2)}} mm.</p>
        <div class="scroll"><table><thead><tr><th>{{label('Поверхность','Boundary')}}</th><th>{{label('Тип','Type')}}</th><th>{{label('Внутри','Inside')}}</th></tr></thead>
          <tbody><tr v-for="s in result.materialAudit.shells" :key="s.id"><td>{{s.id+1}}</td><td>{{s.kind==='cavity'?label('Полость','Cavity'):label('Материал','Material')}}</td><td>{{s.parent===null?'—':s.parent+1}}</td></tr></tbody></table></div>
        <p>{{label('Пересечения и вложенность проверены с численным допуском. Прочность соединений и сцепление линий печати не рассчитаны.','Intersections and nesting checked at numerical tolerance. Joint strength and printed-line bonding are not calculated.')}}</p>
      </template>
      <p v-else role="alert">{{label('Связность материала не установлена:','Material connectivity unresolved:')}} {{result.materialAudit.message}}</p>
      <div class="scroll"><table><thead><tr><th>mm</th><th>A, mm²</th><th>Iuu, mm⁴</th><th>Ivv, mm⁴</th><th>Iuv, mm⁴</th></tr></thead><tbody><tr v-for="s in result.sections" :key="s.positionMm"><td>{{s.positionMm}}</td><td>{{s.properties?.areaMm2.toPrecision(6)??'—'}}</td><td>{{s.properties?.iuuMm4.toPrecision(6)??'—'}}</td><td>{{s.properties?.ivvMm4.toPrecision(6)??'—'}}</td><td>{{s.properties?.iuvMm4.toPrecision(6)??'—'}}</td></tr></tbody></table></div>
      <p>{{label('Пустое сечение обозначено «—». Выборочные сечения сами по себе не доказывают связность между ними.','Empty sections show “—”. Sampled sections alone do not establish connectivity between them.')}}</p>
      <a :href="url" download="structural-sections.json">structural-sections.json</a>
    </section>
  </details>
</template>
<style scoped>
.structural-sections{font-size:12px;margin:10px 0;min-width:0}p{line-height:1.4;overflow-wrap:anywhere}summary{font-weight:600;cursor:pointer}label{display:flex;flex-direction:column;gap:4px;margin:6px 0}
input,select,button{font:inherit;background:var(--surface-raised);color:var(--text);border:1px solid var(--border);border-radius:4px;padding:5px;min-width:0;box-sizing:border-box}input,select{width:100%}button{white-space:normal;cursor:pointer}button:disabled{opacity:.5}.scroll{overflow:auto;max-height:240px}table{border-collapse:collapse;width:100%}th,td{padding:4px;text-align:right;white-space:nowrap;border-bottom:1px solid var(--border)}a{color:var(--accent)}[role=alert]{color:var(--danger)}
</style>
