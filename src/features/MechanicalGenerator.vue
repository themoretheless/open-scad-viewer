<script setup lang="ts">
import { BOX_DEFAULTS, generateBox } from '../services/boxGenerator'
import { computed, nextTick, ref, watch } from 'vue'
import { stringifyMeshJson } from '../services/meshJson'
import { compileRushGraph } from '../services/rushGraphCompiler'
import { createMechanicalDocument, GEAR_DEFAULTS, PLANETARY_DEFAULTS, THREAD_DEFAULTS } from '../services/mechanicalGeneratorContract'
const props=defineProps<{open:boolean;locale:'ru'|'en'}>()
const emit=defineEmits<{close:[];generate:[source:string,name:string];downloadCurrent:[]}>()
const dialog=ref<HTMLDialogElement|null>(null)
const kind=ref<'gear'|'planetary_gears'|'thread'|'box'>('box')
const values=ref<Record<string,number|boolean>>({...BOX_DEFAULTS,print_optimized:false,hook_chamfer:true})
const bendEnabled=ref(false)
const visibleValues=computed(()=>Object.fromEntries(Object.entries(values.value).filter(([key])=>{
  if(kind.value!=='box')return true
  if(['bend_radius','bend_reverse','bend_segments'].includes(key))return false
  if(key.startsWith('hook_'))return false
  if(['hole_diameter','hole_pitch'].includes(key))return !values.value.organic||Boolean(values.value.drain&&!values.value.floor_mesh)
  if(['floor_cell_size','floor_rib'].includes(key))return Boolean(values.value.floor_mesh)
  if(key==='drain')return !values.value.floor_mesh
  if(['cell_size','rib','pattern_seed'].includes(key))return Boolean(values.value.organic)
  if(['stress_gradient','mesh_density'].includes(key))return Boolean((values.value.organic&&values.value.porous)||values.value.floor_mesh)
  if(key==='print_optimized')return Boolean(values.value.porous)
  if(key==='overhang_angle')return Boolean(values.value.porous&&values.value.print_optimized)
  return true
})))
const gridKeys=new Set(['porous','organic','print_optimized','overhang_angle','cell_size','rib','stress_gradient','mesh_density','pattern_seed','floor_mesh','floor_cell_size','floor_rib','drain','hole_diameter','hole_pitch'])
const shapeValues=computed(()=>Object.fromEntries(Object.entries(visibleValues.value).filter(([key])=>!gridKeys.has(key))))
const gridValues=computed(()=>Object.fromEntries(Object.entries(visibleValues.value).filter(([key])=>gridKeys.has(key))))
const hookValues=computed(()=>Object.fromEntries(Object.entries(values.value).filter(([key])=>key.startsWith('hook_'))))
const bendValues=computed(()=>Object.fromEntries(Object.entries(values.value).filter(([key])=>['bend_radius','bend_reverse','bend_segments'].includes(key))))
const error=ref('')
const generated=ref<{source:string;document?:ReturnType<typeof compileRushGraph>['document']}|null>(null)
const ru=computed(()=>props.locale==='ru')
const names:Record<string,[string,string]>={
  bend_radius:['Радиус цилиндрической стены/трубы, мм','Cylinder / pipe radius, mm'],bend_reverse:['Вогнуть внутрь','Bend inward'],bend_segments:['Сегменты дуги','Arc segments'],
  corner_radius:['Скругление углов коробки, мм','Corner fillet radius, mm'],front_angle:['Наклон передней стенки, ° (+ наружу)','Front wall tilt, ° (+ outward)'],
  width:['Ширина, мм','Width, mm'],depth:['Глубина, мм','Depth, mm'],height:['Высота, мм','Height, mm'],floor:['Толщина дна, мм','Floor thickness, mm'],porous:['Открытые стенки','Open walls'],organic:['Органический каркас','Organic skeleton'],print_optimized:['Ячейки для печати дном вниз','Upright FDM cells'],overhang_angle:['Предельный свес от вертикали, °','Maximum overhang from vertical, °'],cell_size:['Размер ячейки стенок, мм','Wall cell size, mm'],rib:['Перемычки стенок, мм','Wall rib width, mm'],stress_gradient:['Градиент сопромата (0 — равномерно, 1 — авто)','Structural gradient (0 = uniform, 1 = auto)'],mesh_density:['Плотность сетки (0.5–2, 1 — норма)','Mesh density scale (0.5–2, 1 = normal)'],floor_mesh:['Сетка на дне','Mesh floor'],floor_cell_size:['Размер ячейки дна, мм','Floor cell size, mm'],floor_rib:['Перемычки дна, мм','Floor rib width, mm'],pattern_seed:['Вариант рисунка','Pattern seed'],hole_diameter:['Диаметр отверстий, мм','Hole diameter, mm'],hole_pitch:['Шаг отверстий, мм','Hole pitch, mm'],dividers:['Перегородки','Dividers'],drain:['Дренаж в дне','Drain holes'],hooks:['Крючки','Hooks'],hook_count:['Количество крючков','Hook count'],hook_width:['Ширина крючка, мм','Hook width, mm'],hook_thickness:['Толщина крючка, мм','Hook thickness, mm'],hook_spacing:['Между центрами, мм (0 — автоматически)','Center spacing, mm (0 = automatic)'],hook_gap:['Зазор под рейку, мм','Rail gap, mm'],hook_drop:['Длина зацепа, мм','Hook drop, mm'],hook_chamfer:['Косынка 45° под основанием крючка','45° root gusset under hook'],
  teeth:['Число зубьев','Teeth'],module:['Модуль, мм','Module, mm'],pressure_angle:['Угол давления, °','Pressure angle, °'],thickness:['Толщина, мм','Thickness, mm'],bore:['Отверстие, мм','Bore diameter, mm'],backlash:['Боковой зазор на шестерню, мм','Backlash per gear, mm'],clearance:['Зазор, мм','Clearance, mm'],internal:['Внутренний вариант','Internal'],rim_width:['Толщина обода, мм','Rim width, mm'],flank_segments:['Сегменты боковой линии зуба','Flank segments'],sun_teeth:['Зубья солнечной шестерни','Sun teeth'],planet_teeth:['Зубья сателлита','Planet teeth'],planet_count:['Число сателлитов','Planets'],carrier_angle:['Поворот водила, °','Carrier angle, °'],diameter:['Диаметр резьбы, мм','Thread diameter, mm'],pitch:['Шаг резьбы, мм','Thread pitch, mm'],length:['Длина, мм','Length, mm'],wall:['Толщина стенки, мм','Wall thickness, mm'],starts:['Число заходов','Starts'],left_handed:['Левая резьба','Left handed'],segments_per_turn:['Сегменты на оборот','Segments per turn'],
}
watch(kind,()=>{bendEnabled.value=false;values.value={...(kind.value==='box'?{...BOX_DEFAULTS,print_optimized:false,hook_chamfer:true}:kind.value==='gear'?GEAR_DEFAULTS:kind.value==='thread'?THREAD_DEFAULTS:PLANETARY_DEFAULTS)};generated.value=null;error.value=''})
watch(bendEnabled,enabled=>{if(kind.value==='box')values.value.bend_radius=enabled?Math.max(150,Number(values.value.bend_radius)||150):0})
watch(()=>values.value.internal,internal=>{if(kind.value==='gear'&&internal){values.value.bore=0;if(Number(values.value.teeth)<36)values.value.teeth=72}})
watch(values,()=>{generated.value=null;error.value=''},{deep:true})
watch(()=>props.open,async open=>{await nextTick();if(open&&!dialog.value?.open)dialog.value?.showModal();else if(!open&&dialog.value?.open)dialog.value.close()})
function generate(){
  error.value=''
  try {
    const boxPayload={...values.value,bend_radius:bendEnabled.value?values.value.bend_radius:0}
    generated.value=kind.value==='box'?{source:generateBox(boxPayload)}:compileRushGraph(createMechanicalDocument({kind:kind.value,...values.value}))
    if(kind.value==='box')openEditor(false)
  }
  catch(e){generated.value=null;error.value=e instanceof Error?e.message:String(e)}
}
function download(){
  if(!generated.value)return
  const url=URL.createObjectURL(new Blob([kind.value==='box'?generated.value.source:stringifyMeshJson(generated.value.document, 2)],{type:kind.value==='box'?'text/plain':'application/json'}))
  const a=document.createElement('a');a.href=url;a.download=`${kind.value}.${kind.value==='box'?'scad':'rush.json'}`;a.click();setTimeout(()=>URL.revokeObjectURL(url),1000)
}
function openEditor(save:boolean){
  if(!generated.value)return
  if(save)emit('downloadCurrent')
  emit('generate',generated.value.source,`${kind.value}.scad`);emit('close')
}
</script>
<template>
  <Teleport to="body">
    <dialog ref="dialog" class="mechanical-dialog" aria-labelledby="mechanical-title" @cancel.prevent="emit('close')">
      <header><h2 id="mechanical-title">{{ru?'Генераторы деталей':'Part generators'}}</h2><button type="button" :aria-label="ru?'Закрыть':'Close'" @click="emit('close')">×</button></header>
      <form @submit.prevent="generate">
        <label class="kind-label">{{ru?'Что построить':'Part type'}}
          <select v-model="kind" :aria-label="ru?'Что построить':'Part type'"><option value="box">{{ru?'Коробка / органайзер':'Box / organizer'}}</option><option value="gear">{{ru?'Шестерня':'Gear'}}</option><option value="planetary_gears">{{ru?'Планетарная передача':'Planetary gearset'}}</option><option value="thread">{{ru?'Резьба':'Thread'}}</option></select>
        </label>
        <p class="hint">{{kind==='box'?(ru?'Радиус изгиба гнёт всю коробку дугой; 0 оставляет её прямой. При изгибе ширина — длина дуги задней стенки, глубина — по радиусу. Радиус углов скругляет только углы; положительный наклон раскрывает перед наружу, задняя стенка остаётся вертикальной. В режиме FDM ячейки сходятся наклонными рёбрами без горизонтальных потолков; учитываются изгиб и наклон корпуса. Угол свеса измеряется от вертикали. Крючки с загнутым вниз концом требуют отдельной проверки поддержек. Толщина стенки задаёт глубину рёбер, толщина перемычки — их ширину. Крючки под рейку требуют проверки посадки; нагрузка не испытана.':'Bend radius curves the whole box; 0 keeps it straight. When bent, width is the rear-wall arc length and depth is radial. Corner radius only rounds corners; positive tilt opens the front outward while the back stays vertical. FDM cells close with sloped ribs, without horizontal ceilings, accounting for body bend and tilt. Overhang is measured from vertical. Downturned hooks require a separate support check. Wall thickness controls rib depth; rib width controls the material between cells. Rail hooks require a fit test; load capacity is untested.'):kind==='planetary_gears'?(ru?'Солнце, сателлиты и неподвижный венец. Водило и оси нужно моделировать отдельно.':'Sun, planets and fixed ring. Model the carrier and shafts separately.'):kind==='thread'?(ru?'Наружный стержень или втулка с внутренней резьбой. Зазор подбирайте под свой принтер.':'Externally threaded rod or internally threaded sleeve. Adjust clearance for your printer.'):(ru?'Прямозубая эвольвентная шестерня или внутренний венец.':'Involute spur gear or internal ring.')}}</p>
        <div class="fields"><label v-for="(value,key) in shapeValues" :key="key" :class="{check:typeof value==='boolean'}">
          <span>{{names[key]?.[ru?0:1]??key}}</span>
          <input v-if="typeof value==='boolean'" v-model="values[key]" type="checkbox">
          <input v-else v-model.number="values[key]" type="number" step="any" required>
        </label></div>
        <fieldset v-if="kind==='box'" class="hook-fields grid-fields">
          <legend>{{ru?'Сетка стенок и дна':'Wall and floor mesh'}}</legend>
          <p class="hint">{{ru?'Органическая сотовая/Вороной-сетка с 3-лучевыми узлами: градиент сопромата регулирует утолщение перемычек стенок к основанию и перемычек дна к контуру (0 — одинаковая толщина, 1 — расчётный градиент, до 2 — усиленный), а плотность сетки масштабирует частоту ячеек.':'Organic Voronoi/honeycomb web with trivalent nodes: structural gradient controls rib thickening toward the wall base and floor perimeter (0 = uniform, 1 = standard moment grading, up to 2 = reinforced), while mesh density scales cell frequency.'}}</p>
          <div class="fields"><label v-for="(value,key) in gridValues" :key="key" :class="{check:typeof value==='boolean'}">
            <span>{{names[key]?.[ru?0:1]??key}}</span>
            <input v-if="typeof value==='boolean'" v-model="values[key]" type="checkbox">
            <input v-else v-model.number="values[key]" type="number" step="any" required>
          </label></div>
        </fieldset>
        <fieldset v-if="kind==='box' && values.hooks" class="hook-fields">
          <legend>{{ru?'Размеры крючков':'Hook dimensions'}}</legend>
          <p class="hint">{{ru?'Толщина крючка независима от толщины стенки. Зазор — свободное расстояние под рейку; длина зацепа измеряется от низа верхней перекладины. Шаг 0 размещает крючки по сопромату: в точках Бесселя (~0.21 ширины от угловых стоек) или прямо напротив внутренних перегородок.':'Hook thickness is independent of wall thickness. Gap is clear rail space; drop is measured from the underside of the top bar. Spacing 0 places hooks structurally at Bessel minimum-sag points (~0.21 of span from corner posts) or aligned with internal dividers.'}}</p>
          <div class="fields"><label v-for="(value,key) in hookValues" :key="key" :class="{check:typeof value==='boolean'}">
            <span>{{names[key]?.[ru?0:1]??key}}</span>
            <input v-if="typeof value==='boolean'" v-model="values[key]" type="checkbox">
            <input v-else v-model.number="values[key]" type="number" step="any" required>
          </label></div>
        </fieldset>
        <fieldset v-if="kind==='box'" class="hook-fields">
          <legend>{{ru?'Изгиб корпуса по дуге (опционально)':'Curved body bend (optional)'}}</legend>
          <div class="fields">
            <label class="check">
              <span>{{ru?'Изогнуть коробку по дуге (для круглой трубы/колонны)':'Curve box along an arc (for round pipe/column)'}}</span>
              <input v-model="bendEnabled" type="checkbox">
            </label>
            <template v-if="bendEnabled">
              <label v-for="(value,key) in bendValues" :key="key" :class="{check:typeof value==='boolean'}">
                <span>{{names[key]?.[ru?0:1]??key}}</span>
                <input v-if="typeof value==='boolean'" v-model="values[key]" type="checkbox">
                <input v-else v-model.number="values[key]" type="number" step="any" required>
              </label>
            </template>
          </div>
        </fieldset>
        <p v-if="kind==='box' && values.print_optimized && values.hooks" class="hint" role="note">{{ru?'Оптимизация относится к ячейкам при печати дном вниз. Под загнутыми вниз крючками остаются свесы: проверьте поддержки в слайсере или отключите крючки.':'Optimization applies to cells printed upright. Downturned hooks still overhang: check supports in the slicer or disable hooks.'}}</p>
        <p v-if="error" class="error" role="alert">{{error}}</p>
        <button class="primary" type="submit">{{ru?'Создать модель':'Generate model'}}</button>
      </form>
      <section v-if="generated" class="generated" aria-live="polite">
        <p>{{ru?'Модель создана. Открытие заменит текущий текст в редакторе.':'Model generated. Opening it replaces the current editor text.'}}</p>
        <div class="actions"><button type="button" @click="download">{{kind==='box'?(ru?'Скачать SCAD':'Download SCAD'):(ru?'Скачать RushGraph':'Download RushGraph')}}</button><button type="button" @click="openEditor(true)">{{ru?'Скачать текущую и открыть':'Save current and open'}}</button><button class="primary" type="button" @click="openEditor(false)">{{ru?'Открыть в редакторе':'Open in editor'}}</button></div>
      </section>
    </dialog>
  </Teleport>
</template>
<style scoped>
.mechanical-dialog{box-sizing:border-box;width:min(650px,calc(100vw - 32px));max-height:90vh;overflow:auto;border:1px solid var(--border);border-radius:14px;background:var(--surface);color:var(--text);padding:24px;box-shadow:0 18px 70px #0005}
.hook-fields{margin:18px 0 0;border:1px solid var(--border);border-radius:8px;padding:12px}.hook-fields legend{font-weight:600;padding:0 6px}.hook-fields .fields{margin:12px 0 0}
.mechanical-dialog::backdrop{background:#0008}header{display:flex;align-items:center;justify-content:space-between;gap:16px}h2{font-size:20px;margin:0 0 16px}button,select,input{font:inherit;color:inherit;border:1px solid var(--border);border-radius:6px;background:var(--surface-raised);padding:8px}button{cursor:pointer}button:focus-visible,input:focus-visible,select:focus-visible{outline:2px solid var(--accent);outline-offset:2px}header button{font-size:24px;border:0;background:none}.kind-label{display:grid;gap:6px}.hint{font-size:13px;line-height:1.5;opacity:.8}.fields{display:grid;grid-template-columns:1fr 1fr;gap:14px;margin:18px 0}.fields label{display:grid;gap:6px;font-size:13px}.fields input{width:100%;box-sizing:border-box}.fields .check{display:flex;align-items:center;justify-content:space-between}.check input{width:auto}.primary{background:var(--accent);color:var(--accent-contrast,#fff)}.error{color:#df6262;overflow-wrap:anywhere}.generated{border-top:1px solid var(--border);margin-top:20px;padding-top:12px}.generated p{font-size:13px}.actions{display:flex;flex-wrap:wrap;gap:8px}@media(max-width:480px){.fields{grid-template-columns:1fr}.mechanical-dialog{padding:16px}}
</style>
