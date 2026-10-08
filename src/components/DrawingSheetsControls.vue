<script setup lang="ts">
import {stringifyMeshJson} from '../services/meshJson'
import {ref,computed,onUnmounted} from 'vue'
import type {DirectDocument} from '../services/directModeling'
import {type DrawingSheet,type DrawingDimension,parseDrawingDimensions} from '../services/drawingSheets'
import {createSolidPreviewWorker} from '../services/solidPreviewWorker'
const props=defineProps<{document:DirectDocument;locale?:string}>()
const open=ref(false),template=ref<'A5'|'A4'|'A3'|'A2'|'Letter'>('A4'),portrait=ref(false),hidden=ref<'show'|'hide'|'dash'>('dash'),dimensions=ref(true),scale=ref(.5),title=ref('Drawing'),author=ref(''),perBody=ref(true),pending=ref(false),error=ref(''),sheets=ref<DrawingSheet[]>([]),page=ref(0),pdfUrl=ref('')
const occlusion=ref<'mesh'|'brep'>('mesh'),frame=ref<'simple'|'detail'|'assembly'>('simple'),drawingNumber=ref(''),revision=ref(''),material=ref('')
const vertexLabels=ref(false)
const linkedDimensions=ref<DrawingDimension[]>([]),dimensionBody=ref(''),dimensionA=ref(0),dimensionB=ref(1),dimensionView=ref<'top'|'front'|'right'>('top'),dimensionAxis=ref<'horizontal'|'vertical'>('horizontal')
const dimensionReference=computed(()=>sheets.value.flatMap(s=>s.references??[]).find(r=>r.bodyId===dimensionBody.value))
function addDimension(){const r=dimensionReference.value;if(!r)return;linkedDimensions.value.push({bodyId:r.bodyId,vertices:[dimensionA.value,dimensionB.value],topologySignature:r.topologySignature,view:dimensionView.value,axis:dimensionAxis.value})}
const dimensionUrl=computed(()=>'data:application/json;charset=utf-8,'+encodeURIComponent(JSON.stringify(linkedDimensions.value)))
async function importDimensions(event:Event){const input=event.target as HTMLInputElement;const file=input.files?.[0];try{if(!file)return;if(file.size>1048576)throw Error('Dimension definitions exceed 1 MiB.');linkedDimensions.value=parseDrawingDimensions(await file.text())}catch(e){error.value=String(e)}finally{input.value=''}}
let disposed=false
function clearPdf(){if(pdfUrl.value)URL.revokeObjectURL(pdfUrl.value);pdfUrl.value=''}
const worker=createSolidPreviewWorker(),label=(ru:string,en:string)=>props.locale==='en'?en:ru
async function preview(){pending.value=true;error.value='';clearPdf();const before=stringifyMeshJson(props.document);try{const prepared=await worker.run({kind:'drawingSheets',bodies:JSON.parse(stringifyMeshJson(props.document.bodies)),options:{template:template.value,scale:scale.value,title:title.value,author:author.value,perBody:perBody.value,portrait:portrait.value,hidden:hidden.value,dimensions:dimensions.value,vertexLabels:vertexLabels.value,occlusion:occlusion.value,frame:frame.value,drawingNumber:drawingNumber.value,revision:revision.value,material:material.value,linkedDimensions:JSON.parse(JSON.stringify(linkedDimensions.value))}});if(disposed)return;if(stringifyMeshJson(props.document)!==before)throw Error(label('Модель изменилась. Подготовьте листы заново.','The model changed. Prepare the sheets again.'));sheets.value=prepared;page.value=0;if(!dimensionBody.value)dimensionBody.value=prepared[0]?.references?.[0]?.bodyId??''}catch(e){error.value=String(e)}finally{pending.value=false}}
async function pdf(){pending.value=true;error.value='';try{const bytes=await worker.run({kind:'drawingPdf',sheets:JSON.parse(JSON.stringify(sheets.value))});if(disposed)return;if(pdfUrl.value)URL.revokeObjectURL(pdfUrl.value);pdfUrl.value=URL.createObjectURL(new Blob([bytes as BlobPart],{type:'application/pdf'}))}catch(e){error.value=String(e)}finally{pending.value=false}}
onUnmounted(()=>{disposed=true;worker.dispose();clearPdf()})
</script>
<template>
 <div class="modeling-feature"><button type="button" :aria-expanded="open" @click="open=!open">{{label('Листы чертежей','Drawing sheets')}}</button>
  <fieldset v-if="open"><legend>{{label('Многостраничный чертёж','Multipage drawing')}}</legend>
   <label>{{label('Шаблон','Template')}}<select v-model="template"><option>A5</option><option>A4</option><option>A3</option><option>A2</option><option>Letter</option></select></label>
   <label><input v-model="portrait" type="checkbox">{{label('Книжный лист','Portrait sheet')}}</label>
   <label><input v-model="dimensions" type="checkbox">{{label('Габаритные размеры','Overall dimensions')}}</label>
   <label>{{label('Скрытые линии','Hidden lines')}}<select v-model="hidden"><option value="dash">{{label('Штриховые','Dashed')}}</option><option value="hide">{{label('Скрыть','Hide')}}</option><option value="show">{{label('Показать','Show')}}</option></select></label>
   <label>{{label('Геометрия скрытия','Occlusion geometry')}}<select v-model="occlusion"><option value="mesh">{{label('Сетка','Mesh')}}</option><option value="brep">{{label('Грани B-rep — плоские','B-rep — planar faces')}}</option></select></label>
   <label>{{label('Основная надпись','Title block')}}<select v-model="frame"><option value="simple">{{label('Простой лист','Simple')}}</option><option value="detail">{{label('Деталь','Detail')}}</option><option value="assembly">{{label('Сборка','Assembly')}}</option></select></label>
   <template v-if="frame!=='simple'"><label>{{label('Обозначение','Drawing number')}}<input v-model="drawingNumber" maxlength="80"></label><label>{{label('Редакция','Revision')}}<input v-model="revision" maxlength="20"></label><label v-if="frame==='detail'">{{label('Материал','Material')}}<input v-model="material" maxlength="80"></label></template>
   <label>{{label('Масштаб листа','Sheet scale')}}<input v-model.number="scale" type="number" min=".001" max="100" step=".1"></label>
   <label>{{label('Название','Title')}}<input v-model="title" maxlength="120"></label>
   <label>{{label('Подпись','Author')}}<input v-model="author" maxlength="80"></label>
   <label><input v-model="perBody" type="checkbox">{{label('Один лист на тело','One sheet per body')}}</label>
   <small>{{label('Три проекции рёбер B-rep. Сетка: 0,02 мм. Плоские грани B-rep: 0,00001 мм, с контурами отверстий. Криволинейные поверхности требуют режима сетки.','Three B-rep edge projections. Mesh: 0.02 mm; planar trimmed B-rep faces: 0.00001 mm. Curved surfaces require mesh mode.')}}</small>
   <button type="button" :disabled="pending||!document.bodies.length" @click="preview">{{label('Подготовить листы','Prepare sheets')}}</button>
   <fieldset v-if="sheets.length"><legend>{{label('Размеры по вершинам','Vertex dimensions')}}</legend>
    <label><input v-model="vertexLabels" type="checkbox">{{label('Номера вершин на листе','Vertex numbers on sheet')}}</label>
    <label>{{label('Тело размера','Dimension body')}}<select v-model="dimensionBody"><option v-for="b in document.bodies.filter(b=>b.brep)" :key="b.id" :value="b.id">{{b.name}}</option></select></label>
    <template v-if="dimensionReference"><label>{{label('Первая вершина','First vertex')}}<select v-model.number="dimensionA"><option v-for="(_,i) in dimensionReference.vertices" :key="i" :value="i">{{i+1}}</option></select></label><label>{{label('Вторая вершина','Second vertex')}}<select v-model.number="dimensionB"><option v-for="(_,i) in dimensionReference.vertices" :key="i" :value="i">{{i+1}}</option></select></label></template>
    <label>{{label('Проекция размера','Dimension view')}}<select v-model="dimensionView"><option value="top">XY</option><option value="front">XZ</option><option value="right">YZ</option></select></label>
    <label>{{label('Направление размера','Dimension direction')}}<select v-model="dimensionAxis"><option value="horizontal">{{label('По горизонтали','Horizontal')}}</option><option value="vertical">{{label('По вертикали','Vertical')}}</option></select></label>
    <button type="button" :disabled="!dimensionReference||dimensionA===dimensionB||linkedDimensions.length>=64" @click="addDimension">{{label('Добавить привязанный размер','Add linked dimension')}}</button>
    <button v-if="linkedDimensions.length" type="button" @click="linkedDimensions=[]">{{label('Очистить размеры','Clear dimensions')}} ({{linkedDimensions.length}})</button>
    <a v-if="linkedDimensions.length" :href="dimensionUrl" download="drawing-dimensions.json">{{label('Сохранить привязки размеров','Save dimension references')}}</a>
    <label>{{label('Загрузить привязки размеров','Load dimension references')}}<input type="file" accept=".json,application/json" @change="importDimensions"></label>
    <small>{{label('После добавления подготовьте листы заново. Размеры следуют за вершинами; изменение топологии требует новой привязки.','Prepare sheets again after adding. Dimensions follow vertices; topology changes require reattachment.')}}</small>
   </fieldset>
   <template v-if="sheets.length"><label>{{label('Лист','Sheet')}}<select v-model.number="page"><option v-for="(s,i) in sheets" :key="i" :value="i">{{i+1}} · {{s.title}}</option></select></label><img :src="'data:image/svg+xml;charset=utf-8,'+encodeURIComponent(sheets[page].svg)" :alt="label('Предпросмотр чертежа','Drawing preview')"><a :href="'data:image/svg+xml;charset=utf-8,'+encodeURIComponent(sheets[page].svg)" :download="'drawing-'+(page+1)+'.svg'">{{label('Скачать лист SVG','Download SVG sheet')}}</a><button type="button" :disabled="pending" @click="pdf">{{label('Подготовить PDF','Prepare PDF')}}</button><a v-if="pdfUrl" :href="pdfUrl" download="drawings.pdf">{{label('Скачать PDF','Download PDF')}}</a></template>
   <p v-if="error" role="alert">{{error}}</p>
  </fieldset>
 </div>
</template>
<style scoped>fieldset{display:grid;gap:6px;border:0;padding:6px 0;font-size:11px}label{display:flex;gap:6px;justify-content:space-between}input,select{min-width:0;max-width:130px}img{width:100%;background:white}small{line-height:1.4}button{font-size:11px}p{color:#ef8c7e}</style>
