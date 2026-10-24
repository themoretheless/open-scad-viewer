import { storageGet, storageSet } from '../../services/safeStorage'
import { type PaletteCommand } from '../../services/commandSearch'
import { primitiveKinds, download, isProfileCommand, profileBooleanOperation, type Pane } from './helpers'
import { isProfilePreparationInput } from '../../services/solidProfilePreparation'
import { defaultDirectCamera, type OrbitCamera } from '../../services/directModelingTools'
import type { SolidFace } from '../../services/directSolidTools'
import { directBodiesScad, type DirectDocument, type DirectBody, type DirectSketch } from '../../services/directModeling'
import { type SurfaceJetBoundary } from '../../services/nurbsConstructors'
import { type BrepBooleanOperation } from '../../services/geometry/brep'
import { type Vec3 } from '../../services/directSketchGeometry'
import { type NurbsScreenProjection } from '../../services/nurbsCurve'
import { type SolidNurbsCurve, type SolidNurbsSurface } from '../../services/solidNurbs'
import { computed, ref, shallowRef, watch, nextTick, type Ref, type ShallowRef, type ComputedRef } from 'vue'

export interface CommandPaletteOptions {
  applyMinkowski:()=>void|Promise<void>
  addNurbs: (kind: "curve" | "surface") => void
  addPrimitive: (kind: (typeof primitiveKinds)[number]) => Promise<void>
  advanced: Ref<{ runoutPosition:number;runoutTangentLength:number;runoutFlip:boolean;offsetJoin: "smooth" | "bevel" | "trim-nonzero" | "trim-evenodd"; patchPrepare: boolean; patchError: number; profileGap: number; curveEndA: "start" | "end"; curveEndB: "start" | "end"; curveAngle: number; prepareOpenPeriodic: boolean; matchOrder: 1 | 2; matchBoundaryA: SurfaceJetBoundary; matchBoundaryB: SurfaceJetBoundary; matchScale: number; matchReverse: boolean; matchError: number; sweepMode: "translation" | "framed"; sweepSections: number; sweepDeviation: number; sweepNormalX: number; sweepNormalY: number; sweepNormalZ: number; surfaceAxis: "u" | "v"; rebuildControls: number; reduceDegree: number; maxError: number; distance: number; radius: number; endRadius: number; filletMode: "constant" | "variable" | "corner" | "partial-preview"; axis: "x" | "y" | "z"; x: number; y: number; z: number; angle: number; scale: number; cx: number; cy: number; start: number; sweep: number; end: "start" | "end"; }, { runoutPosition:number;runoutTangentLength:number;runoutFlip:boolean;offsetJoin: "smooth" | "bevel" | "trim-nonzero" | "trim-evenodd"; patchPrepare: boolean; patchError: number; profileGap: number; curveEndA: "start" | "end"; curveEndB: "start" | "end"; curveAngle: number; prepareOpenPeriodic: boolean; matchOrder: 1 | 2; matchBoundaryA: SurfaceJetBoundary; matchBoundaryB: SurfaceJetBoundary; matchScale: number; matchReverse: boolean; matchError: number; sweepMode: "translation" | "framed"; sweepSections: number; sweepDeviation: number; sweepNormalX: number; sweepNormalY: number; sweepNormalZ: number; surfaceAxis: "u" | "v"; rebuildControls: number; reduceDegree: number; maxError: number; distance: number; radius: number; endRadius: number; filletMode: "constant" | "variable" | "corner" | "partial-preview"; axis: "x" | "y" | "z"; x: number; y: number; z: number; angle: number; scale: number; cx: number; cy: number; start: number; sweep: number; end: "start" | "end"; } | { runoutPosition:number;runoutTangentLength:number;runoutFlip:boolean;offsetJoin: "smooth" | "bevel" | "trim-nonzero" | "trim-evenodd"; patchPrepare: boolean; patchError: number; profileGap: number; curveEndA: "start" | "end"; curveEndB: "start" | "end"; curveAngle: number; prepareOpenPeriodic: boolean; matchOrder: 1 | 2; matchBoundaryA: SurfaceJetBoundary; matchBoundaryB: SurfaceJetBoundary; matchScale: number; matchReverse: boolean; matchError: number; sweepMode: "translation" | "framed"; sweepSections: number; sweepDeviation: number; sweepNormalX: number; sweepNormalY: number; sweepNormalZ: number; surfaceAxis: "u" | "v"; rebuildControls: number; reduceDegree: number; maxError: number; distance: number; radius: number; endRadius: number; filletMode: "constant" | "variable" | "corner" | "partial-preview"; axis: "x" | "y" | "z"; x: number; y: number; z: number; angle: number; scale: number; cx: number; cy: number; start: number; sweep: number; end: "start" | "end"; }>
  advancedOp: Ref<"split" | "push" | "shell" | "curve" | "transform" | "offset" | "loft" | "chamfer" | "instance-create" | "instance-transform" | "instance-place" | "nurbs-loft" | "nurbs-sweep" | "nurbs-patch" | "nurbs-runout" | "nurbs-rebuild" | "nurbs-reduce" | "nurbs-surface-rebuild" | "nurbs-surface-reduce" | "profile-union" | "profile-difference" | "profile-intersection" | "extend" | "edge-fillet" | "profile-prepare" | "nurbs-offset" | "nurbs-point-trim" | "nurbs-match" | "nurbs-prepare" | "nurbs-curve-match" | null>
  appendBodies: () => void
  applyBrepBoolean: (operation: BrepBooleanOperation) => void
  baseZ: Ref<number>
  beginAdvanced: (kind: CommandPaletteOptions['advancedOp']['value']) => void
  beginCorner: (kind: "fillet" | "dogear") => void
  beginExtrude: (kind?: "extrude" | "revolve") => void
  beginSketch: (value: CommandPaletteOptions['tool']['value']) => void
  beginSubtract: () => void
  boxSelect: Ref<boolean>
  brepSegments: Ref<number>
  camera: Ref<OrbitCamera>
  canExtrudeSketch: ComputedRef<boolean>
  cancelCommand: () => void
  clearanceOpen: Ref<boolean>
  commandActive: ComputedRef<boolean>
  commitDirectTransform: (before: DirectDocument, ids: string[], delta: Vec3, rotation?: number, factor?: number, localSketch?: boolean, resetFields?: boolean, detach?: boolean, metadata?: { operation: "group-create" | "group-move"; group: string; }) => Promise<true | undefined>
  copyCount: Ref<number>
  copySweep: Ref<number>
  copyX: Ref<number>
  copyY: Ref<number>
  cornerRadius: Ref<number>
  diagnosticPanel: Ref<HTMLElement | undefined>
  diagnosticsOpen: Ref<boolean>
  document: ShallowRef<DirectDocument>
  downloadProject: () => void
  duplicate: () => void
  edgeIndex: Ref<number>
  exportBlender: () => Promise<void>
  exportBodySvg: () => Promise<void>
  extraSelection: Ref<string[]>
  extrusionMode: Ref<"union" | "difference" | "new">
  faceSketch: () => Promise<void>
  filletSegments: Ref<number>
  fit: (pane: Pane, selectedOnly?: boolean) => void
  floorVisible: Ref<boolean>
  gizmoMode: Ref<"scale" | "rotate" | "move">
  height: Ref<number>
  kernelReady: Ref<boolean>
  kernelLoadError: Ref<string>
  profileBridgePanel: Ref<{ open: () => void } | null>
  label: (a: string, b: string) => string
  measureSelectedBrep: () => void
  measureTarget: Ref<string>
  measurementOpen: Ref<boolean>
  mode: Ref<"2d" | "3d">
  movingBody: Ref<boolean>
  operation: Ref<"array" | "extrude" | "revolve" | "fillet" | "dogear" | null>
  pickMode: Ref<"body" | "vertex" | "edge" | "face">
  pickObject: (id: string, pane: Pane, add?: boolean) => void
  pointTrimCut: ComputedRef<number[] | undefined>
  pointTrimPick: ShallowRef<{ point: [number, number]; matrix: NurbsScreenProjection; radius: number; } | null>
  primitiveLabel: (kind: (typeof primitiveKinds)[number]) => string
  props: Readonly<{ paletteRequest?: number; open: boolean; canAppend: boolean }>
  redoable: Ref<boolean>
  remove: () => void
  restoringDraft: Ref<boolean>
  retessellateSelectedBrep: () => void
  revolveAngle: Ref<number>
  revolveAxis: Ref<"x" | "y">
  revolveGeometry: Ref<"exact" | "faceted">
  revolveOffset: Ref<number>
  revolveSegments: Ref<number>
  run: (action: () => void) => void
  selectEdgeChain: () => void
  selectedBody: ComputedRef<DirectBody | undefined>
  selectedBrepBodies: ComputedRef<(DirectBody | undefined)[]>
  selectedCurvePair: ComputedRef<string[] | null>
  selectedFace: ComputedRef<SolidFace>
  selectedIds: ComputedRef<string[]>
  selectedNurbs: ComputedRef<SolidNurbsCurve | SolidNurbsSurface | undefined>
  selectedNurbsCurve: ComputedRef<SolidNurbsCurve | undefined>
  selectedNurbsSurface: ComputedRef<SolidNurbsSurface | undefined>
  selectedSketch: ComputedRef<DirectSketch | undefined>
  selectedSurfacePair: ComputedRef<string[] | null>
  selection: Ref<string>
  sendToMesh: () => void
  showHelp: Ref<boolean>
  sketchPaneOpen: Ref<boolean>
  smoothDisplay: Ref<boolean>
  snapDocument: ShallowRef<DirectDocument>
  subtract: Ref<{ a: string[]; b: string[]; active: "a" | "b"; } | null, { a: string[]; b: string[]; active: "a" | "b"; } | { a: string[]; b: string[]; active: "a" | "b"; } | null>
  surfaceInputs: Ref<string[]>
  surfaceReversed: Ref<boolean[]>
  toggleSketchPane: (open?: boolean) => void
  tool: Ref<"trim" | "select" | "slot" | "circle" | "polyline" | "rectangle" | "arc">
  twoSelectedBodies: ComputedRef<boolean>
  undo: (redo?: boolean) => Promise<void>
  undoable: Ref<boolean>
  workspace: Ref<HTMLElement | undefined>
}

export function useCommandPalette(options: CommandPaletteOptions) {
  const { applyMinkowski, addNurbs, addPrimitive, advanced, advancedOp, appendBodies, applyBrepBoolean, baseZ, beginAdvanced, beginCorner, beginExtrude, beginSketch, beginSubtract, boxSelect, brepSegments, camera, canExtrudeSketch, cancelCommand, clearanceOpen, commandActive, commitDirectTransform, copyCount, copySweep, copyX, copyY, cornerRadius, diagnosticPanel, diagnosticsOpen, document, downloadProject, duplicate, edgeIndex, exportBlender, exportBodySvg, extraSelection, extrusionMode, faceSketch, filletSegments, fit, floorVisible, gizmoMode, height, kernelReady, kernelLoadError, profileBridgePanel, label, measureSelectedBrep, measureTarget, measurementOpen, mode, movingBody, operation, pickMode, pickObject, pointTrimCut, pointTrimPick, primitiveLabel, props, redoable, remove, restoringDraft, retessellateSelectedBrep, revolveAngle, revolveAxis, revolveGeometry, revolveOffset, revolveSegments, run, selectEdgeChain, selectedBody, selectedBrepBodies, selectedCurvePair, selectedFace, selectedIds, selectedNurbs, selectedNurbsCurve, selectedNurbsSurface, selectedSketch, selectedSurfacePair, selection, sendToMesh, showHelp, sketchPaneOpen, smoothDisplay, snapDocument, subtract, surfaceInputs, surfaceReversed, toggleSketchPane, tool, twoSelectedBodies, undo, undoable, workspace } = options
// Command palette: every tool, primitive and context action of the workspace, searchable by its Russian or English name.
const paletteOpen = ref(false)
const notice = ref('')
// Creation strips stay out of the way once the scene has content; the Create button brings them back.
const createOpen = ref(document.value.bodies.length === 0 && document.value.sketches.length === 0)
const dockOpen = ref(storageGet('scad-solid-dock') === 'true')
const dockTab = ref<'scene' | 'props'>('scene')
watch(dockOpen, open => storageSet('scad-solid-dock', String(open)))
const exactCardOpen = computed({ get: () => dockOpen.value && dockTab.value === 'props', set: open => { if (open) { dockOpen.value = true; dockTab.value = 'props' } else dockTab.value = 'scene' } })
watch(() => props.paletteRequest, request => { if (request && props.open) paletteOpen.value = true })
type SolidCommand = PaletteCommand & { run: () => void }
const canDuplicate=computed(()=>selectedIds.value.length>0&&selectedIds.value.every(id=>document.value.bodies.some(body=>body.id===id)||document.value.sketches.some(sketch=>sketch.id===id)))
const availableSolidCommands = computed<SolidCommand[]>(() => {
  const sketch = selectedSketch.value, body = selectedBody.value, nurbs = selectedNurbs.value
  const anySelection = !!(sketch || body || nurbs)
  const needSelection = label('Сначала выберите объект', 'Select an object first')
  const needClosed = label('Нужен замкнутый эскиз', 'A closed sketch is required')
  const needFace = label('Выберите грань тела', 'Select a body face')
  const needEdge = label('Выберите ребро тела', 'Select a body edge')
  const twoBodies = twoSelectedBodies.value
  const anyBody = document.value.bodies.length > 0
  const twoBrep = selectedBrepBodies.value.length === 2 && selectedIds.value.length === 2
  const booleanDetail = twoBrep ? label('Точный B-rep', 'Exact B-rep') : label('По сетке', 'Mesh boolean')
  const needTwo = label('Выберите два тела: Shift + клик', 'Select two bodies: Shift + click')
  const cmd = (id: string, ru: string, en: string, run: () => void, extra: Partial<PaletteCommand> = {}): SolidCommand =>
    ({ id, label: label(ru, en), aliases: [ru, en], run, ...extra })
  const toolCmd = (value: typeof tool.value, ru: string, en: string, shortcut?: string) =>
    cmd(`tool-${value}`, ru, en, () => beginSketch(value), { detail: label('Инструмент 2D', '2D tool'), shortcut })
  const list: SolidCommand[] = [
    cmd('focus-selection','Фокус на выбранном','Focus selection',()=>fit(mode.value,true),{enabled:anySelection,disabledReason:needSelection,shortcut:'F'}),
    cmd('edge-chain','Продолжить цепочку рёбер','Extend edge chain',selectEdgeChain,{enabled:!!body&&edgeIndex.value>=0,disabledReason:needEdge,detail:label('До разветвления','Stops at junctions')}),
    toolCmd('select', 'Выбор', 'Select', 'V'), toolCmd('rectangle', 'Прямоугольник', 'Rectangle', 'R'), toolCmd('circle', 'Круг', 'Circle', 'C'),
    toolCmd('slot', 'Паз', 'Slot'), toolCmd('arc', 'Дуга', 'Arc'), toolCmd('trim', 'Обрезать', 'Trim'), toolCmd('polyline', 'Ломаная', 'Polyline', 'L'),
    cmd('box-select', 'Рамка', 'Box select', () => { boxSelect.value = !boxSelect.value }, { detail: label('Выделение', 'Selection') }),
    ...primitiveKinds.map(kind => cmd(`add-${kind}`, primitiveLabel(kind), primitiveLabel(kind), () => addPrimitive(kind), { detail: label('Добавить примитив', 'Add primitive'), keywords: ['primitive', 'примитив', kind], enabled: kernelReady.value, disabledReason: kernelLoadError.value ? label('Повторите загрузку геометрического ядра.', 'Retry loading the geometry kernel.') : label('Геометрическое ядро загружается.', 'Geometry kernel is loading.') })),
    cmd('add-curve', 'NURBS-кривая', 'NURBS curve', () => addNurbs('curve'), { detail: label('Добавить', 'Add') }),
    cmd('nurbs-surface-reduce','Снизить степень поверхности','Reduce surface degree',()=>{beginAdvanced('nurbs-surface-reduce');advanced.value.surfaceAxis=(selectedNurbsSurface.value?.surface.degreeU??1)>1?'u':'v';mode.value='3d'},{enabled:!!selectedNurbsSurface.value&&(selectedNurbsSurface.value.surface.degreeU>1||selectedNurbsSurface.value.surface.degreeV>1),disabledReason:label('Выберите NURBS-поверхность степени выше 1','Select a NURBS surface with degree above 1')}),
    cmd('nurbs-prepare','Подготовить границы поверхностей','Prepare surface boundaries',()=>beginAdvanced('nurbs-prepare'),{enabled:!!selectedSurfacePair.value,disabledReason:label('Выберите две NURBS-поверхности','Select two NURBS surfaces')}),
    cmd('profile-union','Объединить точные профили','Union exact profiles',()=>beginAdvanced('profile-union'),{enabled:selectedIds.value.length>=2&&selectedIds.value.every(id=>document.value.sketches.some(s=>s.id===id&&s.closed)),disabledReason:label('Выберите два замкнутых профиля','Select two closed profiles')}),
    cmd('profile-difference','Вычесть области профилей','Subtract profile regions',()=>beginAdvanced('profile-difference'),{enabled:selectedIds.value.length>=2&&selectedIds.value.every(id=>document.value.sketches.some(s=>s.id===id&&s.closed)),disabledReason:label('Выберите два замкнутых профиля','Select two closed profiles')}),
    cmd('profile-intersection','Пересечь точные профили','Intersect exact profiles',()=>beginAdvanced('profile-intersection'),{enabled:selectedIds.value.length>=2&&selectedIds.value.every(id=>document.value.sketches.some(s=>s.id===id&&s.closed)),disabledReason:label('Выберите два замкнутых профиля','Select two closed profiles')}),
    cmd('profile-prepare','Собрать профиль','Prepare profile',()=>beginAdvanced('profile-prepare'),{enabled:!!selectedIds.value.length&&selectedIds.value.every(id=>isProfilePreparationInput(document.value,id)),disabledReason:label('Выберите открытые ломаные, дуги и NURBS в одной плоскости','Select open polylines, arcs and NURBS in one plane')}),
    cmd('nurbs-offset','Смещение NURBS-кривой','Offset NURBS curve',()=>beginAdvanced('nurbs-offset'),{enabled:!!selectedNurbsCurve.value,disabledReason:label('Выберите NURBS-кривую','Select a NURBS curve')}),
    cmd('nurbs-point-trim','Обрезать NURBS по точке','Trim NURBS at point',()=>beginAdvanced('nurbs-point-trim'),{enabled:!!selectedNurbsCurve.value,disabledReason:label('Выберите NURBS-кривую','Select a NURBS curve')}),
    cmd('nurbs-curve-match','Согласовать кривые G1','Match curves G1',()=>beginAdvanced('nurbs-curve-match'),{enabled:!!selectedCurvePair.value,disabledReason:label('Выберите две NURBS-кривые','Select two NURBS curves')}),
    cmd('profile-bridge-check','Проверить переход профилей','Inspect profile transition',()=>void profileBridgePanel.value?.open(),{enabled:!!selectedSurfacePair.value&&!commandActive.value,disabledReason:label('Выберите две NURBS-поверхности','Select two NURBS surfaces')}),
    cmd('nurbs-match','Согласовать поверхности G1/G2','Match surfaces G1/G2',()=>beginAdvanced('nurbs-match'),{enabled:!!selectedSurfacePair.value,disabledReason:label('Выберите две NURBS-поверхности','Select two NURBS surfaces')}),
    cmd('nurbs-surface-rebuild','Перестроить поверхность','Rebuild surface',()=>{beginAdvanced('nurbs-surface-rebuild');mode.value='3d'},{enabled:!!selectedNurbsSurface.value,disabledReason:label('Выберите NURBS-поверхность','Select a NURBS surface')}),
    cmd('nurbs-rebuild','Перестроить кривую','Rebuild curve',()=>{beginAdvanced('nurbs-rebuild');mode.value='3d'},{enabled:!!selectedNurbsCurve.value&&!selectedNurbsCurve.value.bridge,disabledReason:label('Выберите независимую NURBS-кривую','Select an independent NURBS curve')}),
    cmd('nurbs-reduce','Снизить степень кривой','Reduce curve degree',()=>{beginAdvanced('nurbs-reduce');mode.value='3d'},{enabled:!!selectedNurbsCurve.value&&selectedNurbsCurve.value.curve.degree>1&&!selectedNurbsCurve.value.bridge,disabledReason:label('Выберите независимую NURBS-кривую степени выше 1','Select an independent NURBS curve with degree above 1')}),
    cmd('nurbs-runout','Концевой переход G2','G2 end transition',()=>beginAdvanced('nurbs-runout'),{enabled:selectedIds.value.length===3&&selectedIds.value.filter(id=>document.value.surfaces?.some(s=>s.id===id)).length===2&&selectedIds.value.filter(id=>document.value.curves?.some(c=>c.id===id&&c.curve.controlPoints[0]?.length===3)).length===1,disabledReason:label('Выберите две поверхности и пространственную кривую на торце','Select two surfaces and a spatial end curve')}),
    cmd('nurbs-patch','Coons patch','Coons patch',()=>beginAdvanced('nurbs-patch'),{enabled:selectedIds.value.length===4&&selectedIds.value.every(id=>document.value.curves?.some(c=>c.id===id)),disabledReason:label('Выберите четыре граничные NURBS-кривые','Select four boundary NURBS curves')}),
    cmd('nurbs-sweep','Перенос профиля по пути','Sweep',()=>beginAdvanced('nurbs-sweep'),{enabled:!!selectedCurvePair.value,disabledReason:label('Выберите две NURBS-кривые: профиль, затем путь','Select two NURBS curves: profile, then path')}),
    cmd('nurbs-loft','Поверхность по сечениям','NURBS loft',()=>beginAdvanced('nurbs-loft'),{enabled:selectedIds.value.length>=2&&selectedIds.value.every(id=>document.value.curves?.some(c=>c.id===id)),disabledReason:label('Выберите NURBS-сечения по порядку','Select NURBS sections in order')}),
    cmd('add-surface', 'NURBS-поверхность', 'NURBS surface', () => addNurbs('surface'), { detail: label('Добавить', 'Add') }),
    cmd('pick-body', 'Выбирать тела', 'Pick bodies', () => { pickMode.value = 'body'; advancedOp.value = null; boxSelect.value = false }, { detail: label('Режим выбора 3D', '3D pick mode') }),
    cmd('pick-face', 'Выбирать грани', 'Pick faces', () => { pickMode.value = 'face'; advancedOp.value = null; boxSelect.value = false }, { detail: label('Режим выбора 3D', '3D pick mode') }),
    cmd('pick-edge', 'Выбирать рёбра', 'Pick edges', () => { pickMode.value = 'edge'; advancedOp.value = null; boxSelect.value = false }, { detail: label('Режим выбора 3D', '3D pick mode') }),
    cmd('pick-vertex', 'Выбирать вершины', 'Pick vertices', () => { pickMode.value = 'vertex'; advancedOp.value = null; boxSelect.value = false }, { detail: label('Режим выбора 3D', '3D pick mode') }),
    cmd('gizmo-move', 'Манипулятор: двигать', 'Gizmo: move', () => { gizmoMode.value = 'move' }),
    cmd('gizmo-rotate', 'Манипулятор: вращать', 'Gizmo: rotate', () => { gizmoMode.value = 'rotate' }),
    cmd('gizmo-scale', 'Манипулятор: масштаб', 'Gizmo: scale', () => { gizmoMode.value = 'scale' }),
    cmd('orbit', 'Обзор камерой', 'Orbit camera', () => { movingBody.value = false }, { detail: label('Вид', 'View') }),
    cmd('move-body', 'Двигать тело', 'Move body', () => { movingBody.value = true }, { detail: label('Вид', 'View'), shortcut: 'G' }),
    cmd('sketch-pane', sketchPaneOpen.value ? 'Скрыть панель эскизов 2D' : 'Показать панель эскизов 2D', sketchPaneOpen.value ? 'Hide 2D sketch pane' : 'Show 2D sketch pane', () => toggleSketchPane(), { detail: label('Вид', 'View') }),
    cmd('scene-panel', dockOpen.value ? 'Скрыть панель сцены' : 'Показать панель сцены', dockOpen.value ? 'Hide scene panel' : 'Show scene panel', () => { dockOpen.value = !dockOpen.value }, { detail: label('Вид', 'View'), shortcut: '\\' }),
    cmd('fit', 'Вписать', 'Fit', () => fit(mode.value), { detail: label('Вид', 'View'), shortcut: 'F' }),
    cmd('iso', 'Изометрия', 'Isometric view', () => { camera.value = defaultDirectCamera(); fit('3d') }, { detail: label('Вид', 'View') }),
    cmd('smooth', smoothDisplay.value ? 'Показывать B-rep гранёным' : 'Показывать B-rep гладким', smoothDisplay.value ? 'Show B-rep faceted' : 'Show B-rep smooth', () => { smoothDisplay.value = !smoothDisplay.value }, { detail: label('Вид', 'View') }),
    cmd('grid', floorVisible.value ? 'Скрыть сетку 3D' : 'Показать сетку 3D', floorVisible.value ? 'Hide 3D grid' : 'Show 3D grid', () => { floorVisible.value = !floorVisible.value }, { detail: label('Вид', 'View') }),
    cmd('undo', 'Отменить', 'Undo', () => undo(), { shortcut: 'Ctrl Z', enabled: undoable.value, disabledReason: label('Нечего отменять', 'Nothing to undo') }),
    cmd('redo', 'Повторить', 'Redo', () => undo(true), { shortcut: 'Ctrl Shift Z', enabled: redoable.value, disabledReason: label('Нечего повторять', 'Nothing to redo') }),
    cmd('brep-union', 'Объединить тела', 'Union bodies', () => applyBrepBoolean('union'), { detail: booleanDetail, aliases: ['Объединить тела', 'Union bodies', 'B-rep объединить', 'B-rep Union'], keywords: ['boolean', 'булев', 'union', 'merge', 'слить'], enabled: twoBodies, disabledReason: needTwo }),
    cmd('brep-difference', 'Вычесть: A − B', 'Subtract: A − B', () => beginSubtract(), { detail: label('Панель: A − B, тела кликом, Shift добавляет', 'Panel: A − B, click bodies, Shift adds'), aliases: ['Вычесть: A − B', 'Subtract: A − B', 'B-rep A − B'], keywords: ['boolean', 'булев', 'union', 'merge', 'слить'], enabled: anyBody, disabledReason: needSelection }),
    cmd('brep-intersection', 'Пересечение тел', 'Intersect bodies', () => applyBrepBoolean('intersection'), { detail: booleanDetail, aliases: ['Пересечение тел', 'Intersect bodies', 'B-rep пересечение', 'B-rep Intersection'], keywords: ['boolean', 'булев', 'union', 'merge', 'слить'], enabled: twoBodies, disabledReason: needTwo }),
    cmd('brep-xor', 'B-rep XOR', 'B-rep XOR', () => applyBrepBoolean('xor'), { detail: label('Два точных тела B-rep', 'Two exact B-rep bodies'), enabled: twoBrep, disabledReason: label('Нужны два точных тела B-rep', 'Two exact B-rep bodies are required') }),
    cmd('push', 'Push / Pull', 'Push / Pull', () => beginAdvanced('push'), { detail: label('Грань', 'Face'), enabled: !!(body && selectedFace.value), disabledReason: needFace }),
    cmd('face-sketch', 'Эскиз на грани', 'Sketch on face', faceSketch, { detail: label('Грань', 'Face'), enabled: !!(body && selectedFace.value), disabledReason: needFace }),
    cmd('shell', 'Shell', 'Shell', () => beginAdvanced('shell'), { detail: label('Грань', 'Face'), enabled: !!(body && selectedFace.value), disabledReason: needFace }),
    cmd('chamfer', 'Фаска 3D', 'Chamfer 3D', () => beginAdvanced('chamfer'), { detail: label('Ребро', 'Edge'), enabled: !!body && edgeIndex.value >= 0, disabledReason: needEdge }),
    cmd('edge-fillet', 'Скруглить 3D', 'Fillet 3D', () => beginAdvanced('edge-fillet'), { detail: label('Ребро', 'Edge'), enabled: !!body && edgeIndex.value >= 0, disabledReason: needEdge }),
    cmd('split', 'Разрезать', 'Split', () => beginAdvanced('split'), { detail: label('Тело', 'Body'), enabled: !!body, disabledReason: label('Выберите тело', 'Select a body') }),
    cmd('brep-properties', 'Свойства B-rep', 'B-rep properties', measureSelectedBrep, { detail: label('Тело', 'Body'), enabled: !!body?.brep, disabledReason: label('Выберите тело B-rep', 'Select a B-rep body') }),
    cmd('retessellate', 'Перестроить mesh', 'Retessellate', retessellateSelectedBrep, { detail: label('Тело', 'Body'), enabled: !!body?.brep, disabledReason: label('Выберите тело B-rep', 'Select a B-rep body') }),
    cmd('curve', 'Параметры кривой', 'Curve parameters', () => beginAdvanced('curve'), { detail: label('Эскиз', 'Sketch'), enabled: !!sketch?.analytic, disabledReason: label('Выберите круг или дугу', 'Select a circle or arc') }),
    cmd('offset', 'Offset', 'Offset', () => beginAdvanced('offset'), { detail: label('Эскиз', 'Sketch'), enabled: !!(sketch && (sketch.closed || sketch.analytic)), disabledReason: needClosed }),
    cmd('extend', 'Продлить', 'Extend', () => beginAdvanced('extend'), { detail: label('Эскиз', 'Sketch'), enabled: !!(sketch && !sketch.closed && !sketch.analytic), disabledReason: label('Выберите незамкнутую линию', 'Select an open line') }),
    cmd('instance-create','Создать связанный экземпляр','Create linked instance',()=>beginAdvanced('instance-create'),{enabled:!!body&&!body.instance,disabledReason:label('Выберите независимое тело-источник','Select an independent source body')}),
    cmd('instance-transform','Преобразовать экземпляр','Transform instance',()=>beginAdvanced('instance-transform'),{enabled:!!body?.instance,disabledReason:label('Выберите связанный экземпляр','Select a linked instance')}),
    cmd('instance-place','Разместить экземпляр','Place instance',()=>beginAdvanced('instance-place'),{enabled:!!body?.instance,disabledReason:label('Выберите связанный экземпляр','Select a linked instance')}),
    cmd('instance-detach','Сделать независимым','Make independent',()=>run(()=>commitDirectTransform(snapDocument.value,[...selectedIds.value],[0,0,0],0,1,false,false,true)),{enabled:!!body?.instance,disabledReason:label('Выберите связанный экземпляр','Select a linked instance')}),
    cmd('instance-source','Выбрать источник экземпляра','Select instance source',()=>{if(body?.instance)pickObject(body.instance.sourceId,'3d')},{enabled:!!body?.instance,disabledReason:label('Выберите связанный экземпляр','Select a linked instance')}),
    cmd('body-clearance','Зазор двух тел по сетке','Two-body mesh clearance',()=>{measureTarget.value=selectedIds.value.find(id=>id!==selection.value)??'';exactCardOpen.value=true;measurementOpen.value=true;clearanceOpen.value=true;workspace.value?.focus()},{enabled:!!body&&selectedIds.value.length===2&&selectedIds.value.every(id=>document.value.bodies.some(b=>b.id===id)),disabledReason:label('Выберите два тела с Shift','Shift-select two bodies')}),
    cmd('measurements','Измерить вершины / ребро','Measure vertices / edge',()=>{exactCardOpen.value=true;measurementOpen.value=true;workspace.value?.focus()},{enabled:!!body,disabledReason:needSelection}),
    cmd('diagnostics','Диагностика тела','Body diagnostics',()=>{exactCardOpen.value=true;diagnosticsOpen.value=true;void nextTick(()=>diagnosticPanel.value?.focus())},{enabled:!!body,disabledReason:needSelection}),
    cmd('transform', 'Преобразовать выбор', 'Transform selection', () => beginAdvanced('transform'), { enabled: anySelection && !nurbs, disabledReason: needSelection }),
    cmd('dimensions', 'Размеры эскиза', 'Sketch dimensions', () => { exactCardOpen.value = true }, { enabled: !!sketch, disabledReason: label('Выберите эскиз', 'Select a sketch') }),
    cmd('exact-transform', 'Точные преобразования', 'Exact transforms', () => { exactCardOpen.value = true }, { detail: label('Числовой ввод', 'Numeric input'), enabled: !!(sketch || body), disabledReason: needSelection }),
    cmd('brep-detail', 'Детализация B-rep', 'B-rep detail', () => { exactCardOpen.value = true }, { detail: label('Числовой ввод', 'Numeric input'), enabled: !!body?.brep, disabledReason: label('Выберите тело B-rep', 'Select a B-rep body') }),
    cmd('extrude', 'Выдавить', 'Extrude', () => beginExtrude(), { detail: label('Эскиз', 'Sketch'), shortcut: 'E', enabled: canExtrudeSketch.value, disabledReason: needClosed }),
    cmd('revolve', 'Вращение', 'Revolve', () => beginExtrude('revolve'), { detail: label('Эскиз', 'Sketch'), enabled: canExtrudeSketch.value, disabledReason: needClosed }),
    cmd('loft', 'B-rep loft', 'B-rep loft', () => beginAdvanced('loft'), { detail: label('Эскиз', 'Sketch'), enabled: !!sketch && selectedIds.value.length >= 2, disabledReason: label('Выберите два и более эскиза', 'Select two or more sketches') }),
    cmd('fillet', 'Скруглить', 'Fillet', () => beginCorner('fillet'), { detail: label('Эскиз', 'Sketch'), enabled: canExtrudeSketch.value, disabledReason: needClosed }),
    cmd('dogear', 'DogEar', 'DogEar', () => beginCorner('dogear'), { detail: label('Эскиз', 'Sketch'), enabled: canExtrudeSketch.value, disabledReason: needClosed }),
    cmd('array', 'Круговые копии', 'Circular copies', () => { const active = operation.value === 'array'; cancelCommand(); if (!active) operation.value = 'array' }, { detail: label('Эскиз', 'Sketch'), enabled: !!sketch, disabledReason: label('Выберите эскиз', 'Select a sketch') }),
    cmd('duplicate', 'Копия', 'Duplicate', duplicate, { shortcut: 'Ctrl D', enabled: canDuplicate.value&&!commandActive.value, disabledReason: commandActive.value?label('Завершите текущую команду','Finish the current command'):label('Для копирования выберите только тела или эскизы','Select only bodies or sketches to duplicate') }),
    cmd('delete', 'Удалить', 'Delete', remove, { shortcut: 'Del', enabled: anySelection, disabledReason: needSelection }),
    cmd('bake', 'Bake в Code', 'Bake into Code', appendBodies, { detail: label('Файл', 'File'), enabled: document.value.bodies.length > 0 && props.canAppend, disabledReason: label('Нет тел для переноса', 'No bodies to bake') }),
    cmd('to-mesh', 'Открыть в Mesh', 'Open in Mesh', sendToMesh, { detail: label('Файл', 'File'), enabled: document.value.bodies.length > 0, disabledReason: label('Нет тел', 'No bodies') }),
    cmd('download-svg', 'SVG выбранного тела · проекция XY', 'Selected body SVG · XY projection', exportBodySvg, { detail: label('Файл', 'File'), enabled: !!selectedBody.value, disabledReason: label('Выберите тело', 'Select a body') }),
    cmd('download-blender', 'Экспорт для Blender', 'Export for Blender', exportBlender, { detail: label('Файл', 'File'), enabled: document.value.bodies.length > 0, disabledReason: label('Нет тел', 'No bodies') }),
    cmd('download-json', 'Скачать проект JSON', 'Download JSON project', downloadProject, { enabled:!restoringDraft.value, disabledReason:label('Дождитесь восстановления документа.','Wait for document recovery.'), detail: label('Файл', 'File') }),
    cmd('download-scad', 'Экспорт SCAD', 'Export SCAD', () => download(directBodiesScad(document.value), 'solid-bodies.scad'), { detail: label('Файл', 'File'), enabled: document.value.bodies.length > 0, disabledReason: label('Нет тел', 'No bodies') }),
    cmd('help', 'Горячие клавиши', 'Keyboard shortcuts', () => { showHelp.value = !showHelp.value }, { shortcut: '?' }),
  ]
  return list
})
function captureCommand() {
  const id = subtract.value ? 'brep-difference' : advancedOp.value ?? operation.value
  if (!id) return null
  return { id, surfaceInputs:[...surfaceInputs.value], inputSelection:[...selectedIds.value], surfaceReversed:[...surfaceReversed.value], advanced: { ...advanced.value,...(id==='nurbs-point-trim'&&pointTrimPick.value&&pointTrimCut.value?{x:pointTrimCut.value[0],y:pointTrimCut.value[1],z:pointTrimCut.value[2]??0}:{}) }, height: height.value, baseZ: baseZ.value,
    cornerRadius: cornerRadius.value, revolveAxis: revolveAxis.value, revolveOffset: revolveOffset.value,
    revolveAngle: revolveAngle.value, revolveSegments: revolveSegments.value, revolveGeometry: revolveGeometry.value,
    filletSegments: filletSegments.value, brepSegments: brepSegments.value, copyCount: copyCount.value, copySweep: copySweep.value,
    copyX: copyX.value, copyY: copyY.value, extrusionMode: extrusionMode.value }
}
const lastCommand = shallowRef<ReturnType<typeof captureCommand>>(null)
// A profile merge keeps its first identity. After undo, the other inputs exist
// again but the selection still contains only that first identity.
const repeatProfileInputs = computed(() => {
 const saved=lastCommand.value
 if(!saved||!isProfileCommand(saved.id)||selectedIds.value.length!==1||selectedIds.value[0]!==saved.surfaceInputs[0])return []
 return saved.surfaceInputs.every(id=>profileBooleanOperation(saved.id)?document.value.sketches.some(s=>s.id===id&&s.closed):isProfilePreparationInput(document.value,id))?saved.surfaceInputs:[]
})
const repeatEntry = computed(() => availableSolidCommands.value.find(c => c.id === lastCommand.value?.id))
const repeatReason = computed(() => {
  if (!lastCommand.value) return label('Сначала примените команду', 'Apply a command first')
  if (commandActive.value) return label('Завершите текущую команду', 'Finish the current command')
  if (!repeatEntry.value || (repeatEntry.value.enabled === false && !repeatProfileInputs.value.length)) return repeatEntry.value?.disabledReason ?? label('Нужен другой выбор', 'Choose compatible geometry')
  if (['extrude','revolve'].includes(lastCommand.value.id) && lastCommand.value.extrusionMode !== 'new' && !selectedSketch.value?.supportBodyId)
    return label('Выберите эскиз на целевом теле', 'Select a sketch on the target body')
  return ''
})
function repeatCommand() {
  if (repeatReason.value || !lastCommand.value || !repeatEntry.value) return
  const saved = lastCommand.value
  const restoredInputs=[...repeatProfileInputs.value]
  cancelCommand()
  if(restoredInputs.length){selection.value=restoredInputs[0];extraSelection.value=restoredInputs.slice(1)}
  repeatEntry.value.run()
  if((isProfileCommand(saved.id))&&saved.surfaceInputs.every(id=>document.value.sketches.some(s=>s.id===id)||saved.id==='profile-prepare'&&isProfilePreparationInput(document.value,id))&&saved.inputSelection.every(id=>selectedIds.value.includes(id)))surfaceInputs.value=[...saved.surfaceInputs]
  if(saved.id==='nurbs-curve-match'&&saved.surfaceInputs.every(id=>document.value.curves?.some(c=>c.id===id))&&saved.inputSelection.every(id=>selectedIds.value.includes(id)))surfaceInputs.value=[...saved.surfaceInputs]
  if((saved.id==='nurbs-match'||saved.id==='nurbs-prepare')&&saved.surfaceInputs.every(id=>document.value.surfaces?.some(s=>s.id===id))&&saved.inputSelection.every(id=>selectedIds.value.includes(id)))surfaceInputs.value=[...saved.surfaceInputs]
  if(['nurbs-loft','nurbs-sweep','nurbs-patch'].includes(saved.id)) {
    const order=saved.surfaceInputs.map(id=>saved.inputSelection.indexOf(id))
    if(saved.inputSelection.length===selectedIds.value.length&&saved.inputSelection.every((id,i)=>id===selectedIds.value[i])&&saved.surfaceInputs.every(id=>document.value.curves?.some(c=>c.id===id)))surfaceInputs.value=[...saved.surfaceInputs]
    else if(order.length===surfaceInputs.value.length&&order.every(i=>i>=0&&i<surfaceInputs.value.length))surfaceInputs.value=order.map(i=>surfaceInputs.value[i])
  }
  surfaceReversed.value=surfaceInputs.value.map((_,i)=>saved.surfaceReversed[i]??false)
  advanced.value = { ...saved.advanced }; height.value = saved.height; baseZ.value = saved.baseZ
  cornerRadius.value = saved.cornerRadius; revolveAxis.value = saved.revolveAxis; revolveOffset.value = saved.revolveOffset
  revolveAngle.value = saved.revolveAngle; revolveSegments.value = saved.revolveSegments; revolveGeometry.value = saved.revolveGeometry
  filletSegments.value = saved.filletSegments; brepSegments.value = saved.brepSegments; copyCount.value = saved.copyCount; copySweep.value = saved.copySweep
  copyX.value = saved.copyX; copyY.value = saved.copyY; extrusionMode.value = saved.extrusionMode
}
const solidCommands = computed<SolidCommand[]>(() => [...availableSolidCommands.value, {
  id:'repeat', label:label('Повторить команду','Repeat command'), shortcut:'Shift R', run:repeatCommand,
  enabled:!repeatReason.value, disabledReason:repeatReason.value, detail:repeatEntry.value?.label,
}])

function executeSolidCommand(id: string) {
  paletteOpen.value = false
  const target = solidCommands.value.find(command => command.id === id)
  if (target && target.enabled !== false) run(target.run)
}
  return { paletteOpen, notice, createOpen, dockOpen, dockTab, exactCardOpen, captureCommand, lastCommand, repeatEntry, repeatReason, repeatCommand, solidCommands, executeSolidCommand }
}
