import { type refitSolidNurbs, isSolidNurbsRefit } from '../../services/solidCurveReduction'
import { type matchSolidCurve } from '../../services/solidCurveMatching'
import { type matchSolidSurface, type prepareSolidSurfaceSeams } from '../../services/solidSurfaceMatching'
import { type prepareSolidProfile } from '../../services/solidProfilePreparation'
import { evaluateNurbsSurface } from '../../services/nurbsSurface'
import { type FramedSweepResult, type SurfaceJetBoundary } from '../../services/nurbsConstructors'
import { type DirectDocument, parseDirectDocument, type DirectHistory, type Point2, type DirectBody } from '../../services/directModeling'
import { stringifyMeshJson } from '../../services/meshJson'
import { profileBooleanOperation, type Pane } from './helpers'
import { type NurbsScreenProjection } from '../../services/nurbsCurve'
import { type SolidNurbsCurve, type SolidNurbsSurface } from '../../services/solidNurbs'
import { computed, shallowRef, type Ref, type ShallowRef, type ComputedRef } from 'vue'

export interface NurbsMatchingOptions {
  advanced: Ref<{ offsetJoin: "smooth" | "bevel" | "trim-nonzero" | "trim-evenodd"; patchPrepare: boolean; patchError: number; profileGap: number; curveEndA: "start" | "end"; curveEndB: "start" | "end"; curveAngle: number; prepareOpenPeriodic: boolean; matchOrder: 1 | 2; matchBoundaryA: SurfaceJetBoundary; matchBoundaryB: SurfaceJetBoundary; matchScale: number; matchReverse: boolean; matchError: number; sweepMode: "translation" | "framed"; sweepSections: number; sweepDeviation: number; sweepNormalX: number; sweepNormalY: number; sweepNormalZ: number; surfaceAxis: "u" | "v"; rebuildControls: number; reduceDegree: number; maxError: number; distance: number; radius: number; endRadius: number; filletMode: "constant" | "variable" | "corner" | "partial-preview"; axis: "x" | "y" | "z"; x: number; y: number; z: number; angle: number; scale: number; cx: number; cy: number; start: number; sweep: number; end: "start" | "end"; }, { offsetJoin: "smooth" | "bevel" | "trim-nonzero" | "trim-evenodd"; patchPrepare: boolean; patchError: number; profileGap: number; curveEndA: "start" | "end"; curveEndB: "start" | "end"; curveAngle: number; prepareOpenPeriodic: boolean; matchOrder: 1 | 2; matchBoundaryA: SurfaceJetBoundary; matchBoundaryB: SurfaceJetBoundary; matchScale: number; matchReverse: boolean; matchError: number; sweepMode: "translation" | "framed"; sweepSections: number; sweepDeviation: number; sweepNormalX: number; sweepNormalY: number; sweepNormalZ: number; surfaceAxis: "u" | "v"; rebuildControls: number; reduceDegree: number; maxError: number; distance: number; radius: number; endRadius: number; filletMode: "constant" | "variable" | "corner" | "partial-preview"; axis: "x" | "y" | "z"; x: number; y: number; z: number; angle: number; scale: number; cx: number; cy: number; start: number; sweep: number; end: "start" | "end"; } | { offsetJoin: "smooth" | "bevel" | "trim-nonzero" | "trim-evenodd"; patchPrepare: boolean; patchError: number; profileGap: number; curveEndA: "start" | "end"; curveEndB: "start" | "end"; curveAngle: number; prepareOpenPeriodic: boolean; matchOrder: 1 | 2; matchBoundaryA: SurfaceJetBoundary; matchBoundaryB: SurfaceJetBoundary; matchScale: number; matchReverse: boolean; matchError: number; sweepMode: "translation" | "framed"; sweepSections: number; sweepDeviation: number; sweepNormalX: number; sweepNormalY: number; sweepNormalZ: number; surfaceAxis: "u" | "v"; rebuildControls: number; reduceDegree: number; maxError: number; distance: number; radius: number; endRadius: number; filletMode: "constant" | "variable" | "corner" | "partial-preview"; axis: "x" | "y" | "z"; x: number; y: number; z: number; angle: number; scale: number; cx: number; cy: number; start: number; sweep: number; end: "start" | "end"; }>
  advancedOp: Ref<"split" | "push" | "shell" | "curve" | "transform" | "offset" | "loft" | "chamfer" | "instance-create" | "instance-transform" | "instance-place" | "nurbs-loft" | "nurbs-sweep" | "nurbs-patch" | "nurbs-runout" | "nurbs-rebuild" | "nurbs-reduce" | "nurbs-surface-rebuild" | "nurbs-surface-reduce" | "profile-union" | "profile-difference" | "profile-intersection" | "extend" | "edge-fillet" | "profile-prepare" | "nurbs-offset" | "nurbs-point-trim" | "nurbs-match" | "nurbs-prepare" | "nurbs-curve-match" | null>
  /** Read lazily: the body-edit state is declared after this composable runs. */
  bodyEditError: () => Ref<string>
  document: ShallowRef<DirectDocument>
  edgeIndex: Ref<number>
  edgeIndexes: Ref<number[]>
  history: DirectHistory
  label: (a: string, b: string) => string
  pointTrimPick: ShallowRef<{ point: [number, number]; matrix: NurbsScreenProjection; radius: number; } | null>
  project: (p: ArrayLike<number>, pane: Pane) => Point2
  selectedBody: ComputedRef<DirectBody | undefined>
  selectedNurbsCurve: ComputedRef<SolidNurbsCurve | undefined>
  selectedNurbsSurface: ComputedRef<SolidNurbsSurface | undefined>
  selection: Ref<string>
  surfaceInputLabel: (i: number) => string
  surfaceInputs: Ref<string[]>
}

export function useNurbsMatching(options: NurbsMatchingOptions) {
  const { advanced, advancedOp, bodyEditError, document, edgeIndex, edgeIndexes, history, label, pointTrimPick, project, selectedBody, selectedNurbsCurve, selectedNurbsSurface, selection, surfaceInputLabel, surfaceInputs } = options
const rebuildPeriodic=computed(()=>advancedOp.value==='nurbs-rebuild'?!!selectedNurbsCurve.value?.curve.periodic:advancedOp.value==='nurbs-surface-rebuild'?!!(advanced.value.surfaceAxis==='u'?selectedNurbsSurface.value?.surface.periodicU:selectedNurbsSurface.value?.surface.periodicV):false)
const nurbsRefitResult=shallowRef<ReturnType<typeof refitSolidNurbs>|null>(null)
const nurbsReduction=computed(()=>isSolidNurbsRefit(advancedOp.value)?{value:nurbsRefitResult.value,error:bodyEditError().value}:null)
const curveMatchResult=shallowRef<ReturnType<typeof matchSolidCurve>|null>(null),surfaceMatchResult=shallowRef<ReturnType<typeof matchSolidSurface>|null>(null),seamPrepareResult=shallowRef<ReturnType<typeof prepareSolidSurfaceSeams>|null>(null)
const isNurbsMatch=(op:unknown)=>['nurbs-curve-match','nurbs-match','nurbs-prepare'].includes(String(op))
const surfacePreparation=computed(()=>{
 if(advancedOp.value!=='nurbs-prepare')return null
 const message=bodyEditError().value
 return {value:seamPrepareResult.value,error:message.includes('matching periodicity')?label('Для смешанной пары включите «Снять периодическую связь». Форма сохранится в допуске, концы станут независимыми.','For a mixed pair, enable “Remove periodic linkage”. Shape stays within tolerance; the endpoints become independent.'):message}
})
function preparationError(reason:string){return reason==='unproven-parameter-normalization'?label('Не подтверждена точность преобразования параметров. Автоматическая подготовка этих узлов пока недоступна.','Parameter conversion accuracy is unconfirmed. Automatic preparation of these knots is currently unavailable.'):label('Отклонение подготовки превышает допуск. Увеличьте допуск или измените входы.','Preparation deviation exceeds the tolerance. Increase the tolerance or change the inputs.')}
function profilePreparationError(reason:string,kind?:string){
 if(kind==='intersection')return label('Отмеченные сегменты пересекаются. Обрежьте их или измените точки, затем повторите сборку.','The marked segments intersect. Trim them or edit the points, then prepare the profile again.')
 if(kind==='overlap')return label('Отмеченные сегменты перекрываются. Удалите дублирующий участок или измените точки.','The marked segments overlap. Remove the duplicate section or edit the points.')
 if(kind==='unproven')return label('Проверка отмеченных сегментов не завершена. Упростите контур или разделите его на части.','The marked segments could not be verified. Simplify the contour or split it into parts.')
 if(reason==='endpoint-topology')return label('Есть разрывы или неоднозначные соединения. Проверьте отмеченные концы и допуск.','There are gaps or ambiguous connections. Check the marked endpoints and tolerance.')
 if(reason==='disconnected')return label('Выбрано несколько отдельных циклов. Соберите каждый профиль отдельно.','Multiple separate cycles are selected. Prepare each profile separately.')
 return label('Не удалось подтвердить корректность контура. Проверьте пересечения и короткие соединения.','Could not validate the contour. Check intersections and short connectors.')
}
const preparedProfileResult=shallowRef<ReturnType<typeof prepareSolidProfile>|null>(null)
const profilePreparation=computed(()=>{
 if(advancedOp.value!=='profile-prepare')return null
 const message=bodyEditError().value
 const names=surfaceInputs.value.map(id=>document.value.sketches.find(s=>s.id===id)?.name??document.value.curves?.find(c=>c.id===id)?.name).filter(Boolean).join(', ')
 const context=names?names+': ':''
 return {value:preparedProfileResult.value,error:message?context+(message.includes('control deviation exceeds')?label('Управляющие точки выходят из плоскости профиля больше чем на 0,0000001 мм. Перенесите кривые в общую плоскость.','Control points deviate from the profile plane by more than 0.0000001 mm. Move the curves into a common plane.'):message.includes('Select distinct open sketches')?label('Для каждой роли выберите отдельную открытую линию или NURBS-кривую. Уберите повторяющиеся входы.','Choose a distinct open sketch or NURBS curve for each role. Remove duplicate inputs.'):message.includes('exactly clamped')?label('Концы NURBS должны быть зажаты узлами. Подготовьте или обрежьте выбранную кривую.','NURBS endpoints must be clamped. Prepare or trim the selected curve.'):message.includes('same sketch plane')?label('Все линии должны использовать одну плоскость эскиза. Перенесите их в общую плоскость.','All lines must use one sketch plane. Move them into a common plane.'):message.includes('open polylines')?label('Выберите открытые ломаные, дуги или NURBS в одной плоскости.','Select open polylines, arcs or NURBS in one plane.'):message) : ''}
})
function curveMatchError(reason:string){
 if(reason==='unproven-endpoint-regularity')return label('Касательная вырождена. Измените соседнюю управляющую точку.','The tangent is degenerate. Move the adjacent control point.')
 if(reason==='unproven-tangent-orientation')return label('Направление касательной не подтверждено. Выберите другой конец.','Tangent orientation is unconfirmed. Choose another endpoint.')
 return label('Угловая ошибка превышает допуск. Увеличьте допуск или измените входы.','Angular error exceeds tolerance. Increase the tolerance or change the inputs.')
}
const curveMatch=computed(()=>{
 if(advancedOp.value!=='nurbs-curve-match')return null
 const message=bodyEditError().value
 return {value:curveMatchResult.value,error:message.includes('non-periodic')?label('Выберите конец непериодической кривой. У периодической кривой нет независимого конца.','Choose a non-periodic curve endpoint. Periodic curves have no independent endpoint.'):message.includes('exactly clamped')?label('Конец должен быть зажат узлами. Подготовьте или обрежьте кривую.','The endpoint must be clamped. Prepare or trim the curve.'):message.includes('degenerate')?curveMatchError('unproven-endpoint-regularity'):message}
})
const curveMatchGuides=computed(()=>{
 if(advancedOp.value!=='nurbs-curve-match')return []
 const result=curveMatch.value?.value
 return surfaceInputs.value.slice(0,2).flatMap((id,i)=>{
  const curve=(result?.report.accepted?result.document:document.value).curves?.find(c=>c.id===id)?.curve
  if(!curve)return []
  const end=i===0?advanced.value.curveEndA:advanced.value.curveEndB,n=curve.controlPoints.length,k=end==='start'?0:n-1,j=end==='start'?1:n-2
  const point=(index:number)=>{const p=curve.controlPoints[index];return project([p[0],p[1],p[2]??0],'3d')}
  const joint=point(k),handle=point(j)
  return [{id:i,x:joint[0],y:joint[1],points:[joint,handle].map(p=>p.join(',')).join(' '),color:curveMatch.value?.error||result&&!result.report.accepted?'#ff647c':i===0?'#89baff':'#ffc977'}]
 })
})
const surfaceMatch=computed(()=>{
 if(advancedOp.value!=='nurbs-match')return null
 return {value:surfaceMatchResult.value,error:bodyEditError().value}
})
const surfaceMatchLines=computed(()=>{
 if(advancedOp.value!=='nurbs-match'&&advancedOp.value!=='nurbs-prepare')return []
 const result=surfaceMatch.value?.value,p=advanced.value
 return surfaceInputs.value.slice(0,2).flatMap((id,i)=>{
  const prepared=surfacePreparation.value?.value
  const source=(prepared?.report.accepted?prepared.document:document.value).surfaces?.find(s=>s.id===id)?.surface
  const surface=i===1&&result?.report.accepted?result.candidate:source
  if(!surface)return []
  const boundary=i===0?p.matchBoundaryA:p.matchBoundaryB,crossU=boundary[0]==='u',atMax=boundary.endsWith('Max')
  const ku=surface.knotsU,kv=surface.knotsV,nu=surface.controlPoints.length,nv=surface.controlPoints[0].length
  const start=crossU?kv[surface.degreeV]:ku[surface.degreeU],end=crossU?kv[nv]:ku[nu]
  const cross=crossU?ku[atMax?nu:surface.degreeU]:kv[atMax?nv:surface.degreeV]
  const line=(lo:number,hi:number)=>Array.from({length:33},(_,j)=>{const t=lo+(hi-lo)*j/32;return project(evaluateNurbsSurface(surface,crossU?cross:t,crossU?t:cross).point,'3d').join(',')}).join(' ')
  try{
   const regularity=i===0?result?.report.referenceRegularity:result?.report.editedRegularity
   // Edited unresolved intervals describe the candidate, which is not displayed on refusal.
   const intervals=i===0?regularity?.unresolvedIntervals??[]:[]
   return [{key:String(i),points:line(start,end),color:i===0?'#89baff':'#ffc977'},...intervals.map(([a,b],j)=>({key:`${i}-${j}`,points:line(a,b),color:'#ff647c'}))]
  }catch{return []}
 })
})
function surfaceMatchError(reason:string){
 if(reason==='unproven-regularity')return label('Регулярность шва не подтверждена. Выберите другую границу или исправьте вырождение.','Seam regularity is unconfirmed. Choose another boundary or repair the singularity.')
 if(reason==='unproven-tangential-smoothness')return label('Гладкость вдоль шва недостаточна. Снизьте порядок или подготовьте границу.','Insufficient smoothness along the seam. Lower the order or prepare the boundary.')
 return label('Ошибка производных превышает допуск. Измените масштаб или допуск.','Derivative error exceeds the tolerance. Change the scale or tolerance.')
}
function resultAdvanced(onSweepReport?:(report:FramedSweepResult['report'])=>void):DirectDocument {
 const d=history.document,p=advanced.value,id=selection.value,b=d.bodies.find(b=>b.id===id),s=d.sketches.find(s=>s.id===id)
 switch(advancedOp.value){
  case 'profile-prepare': {const r=profilePreparation.value;if(!r?.value)throw Error(r?.error||'Profile preparation unavailable.');if(!r.value.report.accepted)throw Error(profilePreparationError(r.value.report.reason,r.value.report.segmentDefect?.kind));return r.value.document}
  case 'nurbs-curve-match': {const r=curveMatch.value;if(!r?.value)throw Error(r?.error||'Curve matching unavailable.');if(!r.value.report.accepted)throw Error(curveMatchError(r.value.report.reason));return r.value.document}
  case 'nurbs-prepare': {const r=surfacePreparation.value;if(!r?.value)throw Error(r?.error||'Seam preparation unavailable.');if(!r.value.report.accepted)throw Error(preparationError(r.value.report.reason));return r.value.document}
  case 'nurbs-match': {const r=surfaceMatch.value;if(!r?.value)throw Error(r?.error||'Surface matching unavailable.');if(!r.value.report.accepted)throw Error(surfaceMatchError(r.value.report.reason));return r.value.document}
  case 'nurbs-surface-rebuild': case 'nurbs-rebuild': case 'nurbs-reduce': case 'nurbs-surface-reduce': {
   const r=nurbsReduction.value
   if(!r?.value)throw Error(r?.error||'Reduction unavailable.')
   if(!r.value.certificate.accepted||r.value.certificate.rolledBack)throw Error(label('Отклонение превышает допуск. Измените параметры или увеличьте допуск.','Deviation exceeds the tolerance. Adjust the parameters or increase the tolerance.'))
   return r.value.document
  }

 }
 return parseDirectDocument(stringifyMeshJson(d))
}
function bodyCalculationFailure(error:unknown,hasRetryButton=true):string {
 const code=error && typeof error==='object' && 'code' in error?error.code:undefined
 if(code==='BREP_LAYERED_FILLET_REFUSED'||code==='BREP_ANNULAR_FILLET_REFUSED'||code==='BREP_IMPRINT_PIPELINE_REFUSED'||code==='BREP_EXACT_FILLET_REFUSED'||code==='BREP_EXACT_CHAMFER_REFUSED'||code==='BREP_VARIABLE_RADIUS_FILLET_REFUSED'||code==='BREP_VALENCE3_CORNER_BLEND_REFUSED'){
  const message=error instanceof Error?error.message:String(error)
  const edges=(edgeIndexes.value.length?edgeIndexes.value:[edgeIndex.value]).filter(i=>i>=0).map(i=>i+1).join(', ')
  const context=`${selectedBody.value?.name??''} · ${label('рёбра','edges')} ${edges}: `
  if(message.includes('complete corner chain'))return context+label('Выберите всю цепочку рёбер с Shift.','Select the complete edge chain with Shift.')
  if(message.includes('cavity or missing material'))return context+label('Скругление прорезает стенку. Уменьшите радиус.','Fillet crosses the wall. Reduce the radius.')
  if(message.includes('Partial circular rim'))return context+label('Выберите полное кольцо: добавьте остальные дуги с Shift.','Select a complete rim: add the remaining arcs with Shift.')
  if(message.includes('separation is unresolved'))return context+label('Не удалось подтвердить отсутствие пересечений. Измените радиус или набор рёбер и повторите расчёт.','Intersection freedom could not be confirmed. Change the radius or selected edges and retry.')
  return context+(/collid|consum|radius.*(large|fit)|distance.*(large|fit)|too (small|short) for/i.test(message)
   ?label('Размер сопряжения не помещается. Уменьшите радиус или размер фаски.','The feature does not fit. Reduce the radius or chamfer size.')
   :label('Сопряжение не подтверждено для выбранной геометрии. Проверьте условия режима ниже и выберите подходящие рёбра.','The feature is unproven for this geometry. Check the mode requirements below and select suitable edges.'))
 }
 if(code==='BREP_RESOURCE_LIMIT'||code==='BREP_ANALYSIS_INDETERMINATE'){
  const source=selectedBody.value?.name??selectedNurbsCurve.value?.name??label('Выбранная геометрия','Selected geometry')
  return source+': '+(code==='BREP_RESOURCE_LIMIT'
   ?label('Исчерпан лимит вычислений. Модель не изменена. Упростите геометрию или разделите операцию.','The calculation budget was exhausted. The model is unchanged. Simplify geometry or split the operation.')
   :label('Точность расчёта не подтверждена. Модель не изменена. Проверьте замкнутость, ориентацию и вырожденные участки.','Calculation accuracy is unconfirmed. The model is unchanged. Check closure, orientation and degenerate regions.'))
 }
 if(code==='CAD_CRASH')return label('Вычисление прервано из-за сбоя. Модель не изменена. ','The calculation stopped unexpectedly. The model is unchanged. ')+(hasRetryButton?label('Нажмите «Повторить вычисление».','Choose Retry calculation.'):label('Повторите операцию с выбранными телами.','Run the operation again with the selected bodies.'))
 if(code==='CAD_TIMEOUT')return label('Истекло время расчёта. Модель не изменена. Упростите геометрию или повторите расчёт.','The calculation timed out. The model is unchanged. Simplify geometry or retry.')
 if(code==='CAD_TRANSPORT'||code==='CAD_PROTOCOL')return label('Не удалось получить корректный результат вычисления. Модель не изменена. Повторите вычисление.','A valid calculation result could not be received. The model is unchanged. Retry the calculation.')
 return error instanceof Error?error.message:String(error)
}
function surfaceConstructionError(error:unknown):string {
 const message=error instanceof Error?error.message:String(error)
 if(advancedOp.value==='nurbs-point-trim'){
  if(message.includes('ambiguous or unresolved'))return pointTrimPick.value?label('Попадание в проекции неоднозначно. Поверните вид или выберите другую точку.','The projected pick is ambiguous. Rotate the view or choose another point.'):label('Ближайшая точка неоднозначна. Выберите другую точку разреза.','The nearest point is ambiguous. Choose another cut point.')
  if(message.includes('capture distance'))return label('Точка вне допуска. Приблизьте её к кривой или увеличьте допуск.','Outside capture distance. Move closer to the curve or increase the distance.')
  if(message.includes('strictly inside'))return label('Это конец кривой. Выберите внутреннюю точку.','Curve endpoint. Choose an interior point.')
  if(message.includes('open curve'))return label('Нужна открытая непериодическая кривая с разными концами.','An open non-periodic curve with distinct endpoints is required.')
 }
 if(advancedOp.value==='offset'&&message.includes('contains no material'))return label('Смещение удаляет профиль. Уменьшите модуль расстояния.','Offset removes the profile. Reduce the distance magnitude.')
 if(advancedOp.value==='offset'&&message.includes('below coordinate resolution'))return label('Смещение слишком мало для точности этих координат. Увеличьте расстояние.','The offset is below coordinate resolution. Increase the distance.')
 if(advancedOp.value==='offset'&&message.includes('cusp below resolution'))return label('Радиус после смещения слишком мал для проверки. Измените расстояние.','The resulting radius is below resolution. Change the distance.')
 if(profileBooleanOperation(advancedOp.value)&&message.includes('contains no material'))return label('Пустой результат. Измените входные профили или основной профиль.','Empty result. Change profiles or target.')
 if(profileBooleanOperation(advancedOp.value)&&message.includes('common sketch plane'))return label('Профили в разных плоскостях. Перенесите их в общую плоскость.','Move the profiles into one sketch plane.')
 if(advancedOp.value==='nurbs-sweep'&&message.includes('Closed sweep requires'))return label('Для замкнутого пути задайте минимум 4 сечения.','Use at least 4 sections for a closed path.')
 if(advancedOp.value==='nurbs-sweep'&&message.includes('Sweep sampled deviation'))return label('Измеренное отклонение превышает предел. Увеличьте число сечений или измените предел. ','Sampled deviation exceeds the budget. Increase sections or change the budget. ')+message
 if(advancedOp.value==='nurbs-sweep'){
  if(message.includes('zero or nonfinite direction'))return label('Начальное направление нулевое или вдоль касательной. Измените X, Y, Z.','Initial direction is zero or along the tangent. Change X, Y, Z.')
  if(message.includes('tangent discontinuities'))return label('У пути есть разрыв касательной. Разбейте путь в месте излома.','The path has a tangent discontinuity. Split it at the corner.')
 }
 if(message==='Choose distinct curves.')return label('Для каждой роли выберите отдельную кривую.','Choose a distinct curve for each role.')
 if(advancedOp.value!=='nurbs-patch')return message
 if(message.includes('PATCH_BUDGET')){
  const role=message.match(/PATCH_BUDGET:([0-3])/)
  return (role?surfaceInputLabel(Number(role[1]))+': ':'')+label('Подготовка границ превышает допуск. Увеличьте допуск или измените границы.','Boundary preparation exceeds the budget. Increase the budget or change the boundaries.')
 }
 if(message.includes('corner weights are incompatible'))return label(
  'Несовместимые угловые веса. Включите согласование весов границ и задайте допуск.',
  'Incompatible corner weights. Enable boundary weight matching and set a budget.')
 if(message.includes('positive finite weights'))return label(
  'Положительные веса patch не подтверждены. Разбейте patch или измените границы.',
  'Positive patch weights are unproven. Split the patch or change its boundaries.')
 const corner=message.match(/Coons corner ([1-4]) does not coincide/)
 if(corner){
  const pairs=[label('низ и слева','bottom and left'),label('низ и справа','bottom and right'),label('верх и слева','top and left'),label('верх и справа','top and right')]
  return label(`Угол ${corner[1]}: границы «${pairs[Number(corner[1])-1]}» не соединены. Проверьте роли и направление кривых; соедините их концы.`,
   `Corner ${corner[1]}: ${pairs[Number(corner[1])-1]} boundaries do not meet. Check curve roles and direction, then connect their endpoints.`)
 }
 return message
}
  return { rebuildPeriodic, nurbsRefitResult, nurbsReduction, curveMatchResult, surfaceMatchResult, seamPrepareResult, isNurbsMatch, surfacePreparation, preparationError, profilePreparationError, preparedProfileResult, profilePreparation, curveMatchError, curveMatch, curveMatchGuides, surfaceMatch, surfaceMatchLines, surfaceMatchError, resultAdvanced, bodyCalculationFailure, surfaceConstructionError }
}
