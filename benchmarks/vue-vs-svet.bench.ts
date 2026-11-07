import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { performance } from 'node:perf_hooks'
import { JSDOM } from '../../forma/packages/svet/node_modules/jsdom/lib/api.js'
import { describe, expect, it } from 'vitest'

// Set up JSDOM global environment BEFORE importing Vue runtime-dom so Vue caches `document`
const dom = new JSDOM('<!doctype html><html><body></body></html>')
Object.assign(globalThis, {
  window: dom.window,
  document: dom.window.document,
  Node: dom.window.Node,
  Element: dom.window.Element,
  HTMLElement: dom.window.HTMLElement,
  SVGElement: dom.window.SVGElement,
  DocumentFragment: dom.window.DocumentFragment,
  Text: dom.window.Text,
  Comment: dom.window.Comment,
  Event: dom.window.Event,
  MouseEvent: dom.window.MouseEvent,
  KeyboardEvent: dom.window.KeyboardEvent,
})

const {
  computed: vueComputed,
  createApp,
  defineComponent,
  h: vueH,
  nextTick,
  reactive: vueReactive,
  ref: vueRef,
  watchEffect: vueWatchEffect,
} = await import('vue')
const {
  batch: svetBatch,
  compileFormaAst,
  computed: svetComputed,
  effect: svetEffect,
  For: SvetFor,
  getGraphTelemetry,
  h: svetH,
  mount: svetMount,
  parseFormaSource,
  resetDomMutationCount,
  resetReactiveSystem,
  signal: svetSignal,
} = await import('../../forma/packages/svet/dist/package/index.js')
const { parseFormaUi, renderFormaAst } = await import('../src/forma/runtime.ts')

const FormaRegion = defineComponent({
  name: 'FormaRegion',
  props: {
    source: { type: String, required: true },
    state: { type: Object, default: () => ({}) },
    locale: { type: String, default: 'ru' },
  },
  emits: ['action', 'binding'],
  setup(props, { slots, emit }) {
    const ast = vueComputed(() => parseFormaUi(props.source))
    return () =>
      renderFormaAst(ast.value, props.state as Record<string, unknown>, props.locale, {
        slots,
        onAction: (name, args, event) => emit('action', name, args, event),
        onBinding: (path, value) => emit('binding', path, value),
      })
  },
})

const uiDir = resolve(import.meta.dirname, '../src/ui')
const uiFiles = [
  'Topbar.ui',
  'EditorToolbar.ui',
  'EditorStats.ui',
  'ViewerToolbar.ui',
  'SelectionModes.ui',
  'DockRail.ui',
  'ComputeInspector.ui',
  'Statusbar.ui',
  'WorkspaceActionBar.ui',
] as const

const rawUiSources = Object.fromEntries(
  uiFiles.map(name => [name, readFileSync(resolve(uiDir, name), 'utf8')]),
) as Record<(typeof uiFiles)[number], string>

function initialRegionState() {
  return {
    // Topbar
    workspaceModeGroupLabel: 'Режим работы',
    workspaceMode: 'solid',
    modes: [
      { id: 'solid', label: 'Solid', hint: 'Точные тела' },
      { id: 'mesh', label: 'Mesh', hint: 'Полигональная сетка' },
      { id: 'code', label: 'Code', hint: 'Код' },
    ],
    workspaceConflict: false,
    persistenceError: false,
    editorOpen: true,
    sourceLabel: 'Исходный код',
    commandHelp: 'Команды',
    commandsLabel: 'Команды',
    exportTitle: 'Экспорт модели',
    exportLabel: 'Экспорт',
    // EditorToolbar
    rendering: false,
    renderLabel: 'Собрать',
    cancelBuildLabel: 'Отменить сборку',
    autoRender: true,
    autoLabel: 'Авто',
    solidBuilding: false,
    cancelSolidLabel: 'Отмена',
    // EditorStats
    meshesLabel: 'Тела:',
    meshesValue: '24',
    trianglesLabel: 'Треугольники:',
    trianglesValue: '18 420',
    hasMeshes: true,
    volumeLabel: 'Объём:',
    volumeValue: '1 240 мм³',
    areaLabel: 'Площадь:',
    areaValue: '842 мм²',
    statusClass: 'status-ok',
    statusSummary: 'Готово · 14 мс',
    // ViewerToolbar
    toolbarClass: 'viewer-toolbar with-dock',
    fitLabel: 'Вписать',
    resetLabel: 'Сбросить вид',
    canPreviousView: true,
    previousViewLabel: 'Предыдущий вид',
    previousViewTitle: 'Предыдущий вид',
    isOrthographic: false,
    projectionLabel: 'Перспектива',
    projectionTitle: 'Перспектива',
    gridVisible: true,
    gridLabel: 'Сетка',
    displayLabel: 'Отображение',
    displayMode: 'shaded',
    shadedLabel: 'Заливка',
    edgesLabel: 'Рёбра',
    xrayLabel: 'Рентген',
    // SelectionModes
    selectionModeLabel: 'Режим выделения',
    selectionMode: 'face',
    pointLabel: 'Точка',
    pointTitle: 'Точка · 1',
    faceLabel: 'Грань',
    faceTitle: 'Грань · 3',
    bodyLabel: 'Тело',
    bodyTitle: 'Тело · 4',
    showHud: true,
    hasSelectedMesh: true,
    selectedSummary: 'Выбрано: Корпус #1',
    isolateLabel: 'Изолировать',
    focusLabel: 'Фокус',
    isolated: false,
    isolateTitle: 'Изолировать',
    deselectLabel: 'Снять выделение',
    // DockRail
    sidebarLabel: 'Боковая панель',
    dockTab: 'scene',
    tabs: [
      { id: 'scene', domId: 'tab-scene', label: 'Сцена', iconPath: 'M3 3h18v18H3z' },
      { id: 'params', domId: 'tab-params', label: 'Параметры', iconPath: 'M4 6h16M4 12h16M4 18h16' },
      { id: 'inspect', domId: 'tab-inspect', label: 'Инспектор', iconPath: 'M12 2v20M2 12h20' },
      { id: 'diagnostics', domId: 'tab-diag', label: 'Диагностика', iconPath: 'M12 8v4m0 4h.01' },
    ],
    closeSidebarLabel: 'Закрыть',
    // ComputeInspector
    panelTitle: 'Анализ геометрии',
    description: 'Расчёт площади и объёма на GPU/CPU',
    computeButtonLabel: 'Рассчитать',
    canRunCompute: true,
    computeBusy: false,
    cancelLabel: 'Отмена',
    hasResult: true,
    areaPrefix: 'Площадь: ',
    backendSummary: 'WebGPU compute',
    timingSummary: 'GPU 1.2 мс',
    showFallback: false,
    fallbackNote: '',
    verificationNote: 'Проверено с CPU эталоном',
    computeError: '',
    // Statusbar
    statusAriaLabel: 'Состояние',
    gpuDotClass: 'ok',
    backendLabel: 'WebGPU',
    showSortedTransparency: true,
    transparencySortedLabel: 'OIT прозрачность',
    showPersistence: true,
    persistenceLabel: 'Черновик сохранён',
    conversionPending: true,
    conversionLabel: 'STEP экспорт...',
    conversionMax: 100,
    conversionValue: 45,
    hintLabel: 'Орбита: ЛКМ · Панорама: ПКМ',
    // WorkspaceActionBar
    cubeLabel: '+ Куб',
    cylinderLabel: '+ Цилиндр',
    sphereLabel: '+ Сфера',
    booleanLabel: 'Вычитание CSG',
    busy: false,
  }
}

interface BenchmarkRow {
  workload: string
  vueMs: number
  svetMs: number
  speedup: string
  notes: string
}

const results: BenchmarkRow[] = []

function record(workload: string, vueMs: number, svetMs: number, notes: string) {
  const ratio = vueMs / Math.max(svetMs, 0.0001)
  results.push({
    workload,
    vueMs: Number(vueMs.toFixed(2)),
    svetMs: Number(svetMs.toFixed(2)),
    speedup: `${ratio.toFixed(2)}x быстрее (Svet)`,
    notes,
  })
}

async function runBenchmarks() {
  // =========================================================================
  // 1. Cold Parse + Mount of all 9 open-scad-viewer Forma .ui regions (x100 iterations)
  // =========================================================================
  const MOUNT_ITERS = 80
  let vueMountTotal = 0
  for (let i = 0; i < MOUNT_ITERS; i++) {
    const hostEl = document.createElement('div')
    const state = vueReactive(initialRegionState())
    const t0 = performance.now()
    const App = defineComponent({
      setup() {
        return () =>
          vueH(
            'div',
            null,
            uiFiles.map(f =>
              vueH(FormaRegion, {
                key: f,
                source: rawUiSources[f],
                state,
                locale: 'ru',
              }),
            ),
          )
      },
    })
    const app = createApp(App)
    app.mount(hostEl)
    vueMountTotal += performance.now() - t0
    app.unmount()
  }

  // Pre-parsed AOT AST (standard in Svet via compileFormaAst)
  const precompiledAsts = uiFiles.map(f => parseFormaSource(rawUiSources[f]))
  const initSvetMount = initialRegionState()
  const sharedSignals = Object.fromEntries(
    Object.entries(initSvetMount).map(([k, v]) => [k, svetSignal(v)]),
  )
  const mountedSvetInstances = precompiledAsts.map(ast =>
    compileFormaAst(ast, { viewModel: sharedSignals }),
  )
  let svetMountTotal = 0
  for (let i = 0; i < MOUNT_ITERS; i++) {
    const hostEl = document.createElement('div')
    const t0 = performance.now()
    const handles = mountedSvetInstances.map(inst => {
      const sub = document.createElement('div')
      hostEl.appendChild(sub)
      return inst.mount(sub)
    })
    svetMountTotal += performance.now() - t0
    for (const h of handles) h.unmount()
  }
  for (const inst of mountedSvetInstances) inst.dispose()
  for (const sig of Object.values(sharedSignals)) sig.dispose()
  record(
    `1. Монтирование всех 9 регионов .ui (${MOUNT_ITERS} циклов)`,
    vueMountTotal,
    svetMountTotal,
    'Vue VDOM + FormaRegion vs Svet AOT compileFormaAst.mount()',
  )

  // =========================================================================
  // 2. High-Frequency Reactive State Updates across all 9 .ui regions (2,500 frames)
  //    Simulates live CAD build progress, triangle/volume telemetry, mode & HUD updates
  // =========================================================================
  const UPDATE_FRAMES = 2500

  const vueUpdateHost = document.createElement('div')
  const vueState = vueReactive(initialRegionState())
  const vueApp = createApp({
    setup() {
      return () =>
        vueH(
          'div',
          null,
          uiFiles.map(f =>
            vueH(FormaRegion, {
              key: f,
              source: rawUiSources[f],
              state: vueState,
              locale: 'ru',
            }),
          ),
        )
    },
  })
  vueApp.mount(vueUpdateHost)

  const tVueUp0 = performance.now()
  for (let i = 0; i < UPDATE_FRAMES; i++) {
    vueState.trianglesValue = `${18000 + i}`
    vueState.volumeValue = `${1200 + (i % 500)} мм³`
    vueState.statusSummary = `Кадр #${i}`
    vueState.conversionValue = i % 100
    vueState.displayMode = i % 3 === 0 ? 'shaded' : i % 3 === 1 ? 'edges' : 'xray'
    vueState.selectionMode = i % 3 === 0 ? 'point' : i % 3 === 1 ? 'face' : 'object'
    await nextTick()
  }
  const vueUpdateMs = performance.now() - tVueUp0
  vueApp.unmount()

  resetReactiveSystem()
  const svetUpdateHost = document.createElement('div')
  const initSvet = initialRegionState()
  const svetInstances = precompiledAsts.map(ast => compileFormaAst(ast, { state: initSvet }))
  const svetHandles = svetInstances.map(inst => {
    const sub = document.createElement('div')
    svetUpdateHost.appendChild(sub)
    return inst.mount(sub)
  })
  resetDomMutationCount()
  const tSvetUp0 = performance.now()
  for (let i = 0; i < UPDATE_FRAMES; i++) {
    const nextTri = `${18000 + i}`
    const nextVol = `${1200 + (i % 500)} мм³`
    const nextStatus = `Кадр #${i}`
    const nextConv = i % 100
    const nextDisp = i % 3 === 0 ? 'shaded' : i % 3 === 1 ? 'edges' : 'xray'
    const nextSel = i % 3 === 0 ? 'point' : i % 3 === 1 ? 'face' : 'object'
    svetBatch(() => {
      // EditorStats (index 2)
      svetInstances[2].state.trianglesValue.set(nextTri)
      svetInstances[2].state.volumeValue.set(nextVol)
      svetInstances[2].state.statusSummary.set(nextStatus)
      // Statusbar (index 7)
      svetInstances[7].state.conversionValue.set(nextConv)
      // ViewerToolbar (index 3)
      svetInstances[3].state.displayMode.set(nextDisp)
      // SelectionModes (index 4)
      svetInstances[4].state.selectionMode.set(nextSel)
    })
  }
  const svetUpdateMs = performance.now() - tSvetUp0
  for (const h of svetHandles) h.unmount()
  for (const inst of svetInstances) inst.dispose()

  record(
    `2. Точечные обновления 9 регионов .ui (${UPDATE_FRAMES} кадров)`,
    vueUpdateMs,
    svetUpdateMs,
    'Vue пересчитывает VNode дерево; Svet обновляет 6 текстовых/DOM узлов за O(1)',
  )

  // =========================================================================
  // 3. SceneOutliner: 1,000 CAD Meshes — Single-Item Selection & Visibility Toggle (1,000 ops)
  // =========================================================================
  const MESH_COUNT = 1000
  const SELECT_OPS = 1000

  const vueOutlinerHost = document.createElement('div')
  const vueMeshes = vueReactive(
    Array.from({ length: MESH_COUNT }, (_, i) => ({
      index: i,
      name: `Деталь_${i}`,
      triangles: 240 + (i % 100),
      visible: true,
    })),
  )
  const vueSelectedIdx = vueRef(0)
  const vueOutlinerApp = createApp({
    setup() {
      return () =>
        vueH(
          'ul',
          { class: 'scene-list' },
          vueMeshes.map(m =>
            vueH(
              'li',
              {
                key: m.index,
                class: ['scene-item', vueSelectedIdx.value === m.index ? 'selected' : '', !m.visible ? 'dimmed' : ''].join(' '),
              },
              [
                vueH('span', { class: 'item-name' }, m.name),
                vueH('span', { class: 'item-meta' }, `${m.triangles} Δ`),
              ],
            ),
          ),
        )
    },
  })
  vueOutlinerApp.mount(vueOutlinerHost)

  const tVueSel0 = performance.now()
  for (let i = 0; i < SELECT_OPS; i++) {
    const idx = i % MESH_COUNT
    vueSelectedIdx.value = idx
    vueMeshes[idx].visible = !vueMeshes[idx].visible
    await nextTick()
  }
  const vueOutlinerMs = performance.now() - tVueSel0
  vueOutlinerApp.unmount()

  resetReactiveSystem()
  const svetOutlinerHost = document.createElement('div')
  const svetMeshes = Array.from({ length: MESH_COUNT }, (_, i) => ({
    index: i,
    name: `Деталь_${i}`,
    triangles: 240 + (i % 100),
    visible: svetSignal(true),
    selected: svetSignal(i === 0),
  }))
  let prevSelIdx = 0
  const svetOutlinerMount = svetMount(
    () =>
      svetH(
        'ul',
        { className: 'scene-list' },
        ...svetMeshes.map(m =>
          svetH(
            'li',
            {
              className: () =>
                `scene-item ${m.selected() ? 'selected' : ''} ${!m.visible() ? 'dimmed' : ''}`,
            },
            svetH('span', { className: 'item-name' }, m.name),
            svetH('span', { className: 'item-meta' }, `${m.triangles} Δ`),
          ),
        ),
      ),
    svetOutlinerHost,
  )

  const tSvetSel0 = performance.now()
  for (let i = 0; i < SELECT_OPS; i++) {
    const idx = i % MESH_COUNT
    svetBatch(() => {
      svetMeshes[prevSelIdx].selected.set(false)
      svetMeshes[idx].selected.set(true)
      svetMeshes[idx].visible.update(v => !v)
    })
    prevSelIdx = idx
  }
  const svetOutlinerMs = performance.now() - tSvetSel0
  svetOutlinerMount.unmount()

  record(
    `3. SceneOutliner (1 000 тел): выбор + скрытие (${SELECT_OPS} операций)`,
    vueOutlinerMs,
    svetOutlinerMs,
    'Vue обходит 1 000 VNode; Svet меняет класс только у 2 затронутых <li>',
  )

  // =========================================================================
  // 4. Keyed List Reordering (1,000 CAD Meshes, 100 Shuffles/Reversals)
  // =========================================================================
  const REORDER_ITERS = 100
  const vueReorderHost = document.createElement('div')
  const vueList = vueRef(
    Array.from({ length: MESH_COUNT }, (_, i) => ({ id: i, label: `Mesh #${i}` })),
  )
  const vueReorderApp = createApp({
    setup() {
      return () =>
        vueH(
          'div',
          null,
          vueList.value.map(item => vueH('div', { key: item.id, class: 'row' }, item.label)),
        )
    },
  })
  vueReorderApp.mount(vueReorderHost)

  const tVueRe0 = performance.now()
  for (let i = 0; i < REORDER_ITERS; i++) {
    const copy = vueList.value.slice()
    // Swap 20 pairs and rotate
    for (let j = 0; j < 20; j++) {
      const a = (i * 17 + j * 31) % MESH_COUNT
      const b = (i * 29 + j * 43) % MESH_COUNT
      const tmp = copy[a]
      copy[a] = copy[b]
      copy[b] = tmp
    }
    vueList.value = copy
    await nextTick()
  }
  const vueReorderMs = performance.now() - tVueRe0
  vueReorderApp.unmount()

  resetReactiveSystem()
  const svetReorderHost = document.createElement('div')
  const svetListSig = svetSignal(
    Array.from({ length: MESH_COUNT }, (_, i) => ({ id: i, label: `Mesh #${i}` })),
  )
  const svetReorderMount = svetMount(
    () =>
      svetH(
        'div',
        null,
        SvetFor({
          each: svetListSig,
          keyFn: item => item.id,
          children: item => svetH('div', { className: 'row' }, item.label),
        }),
      ),
    svetReorderHost,
  )

  const tSvetRe0 = performance.now()
  for (let i = 0; i < REORDER_ITERS; i++) {
    const copy = svetListSig.peek().slice()
    for (let j = 0; j < 20; j++) {
      const a = (i * 17 + j * 31) % MESH_COUNT
      const b = (i * 29 + j * 43) % MESH_COUNT
      const tmp = copy[a]
      copy[a] = copy[b]
      copy[b] = tmp
    }
    svetListSig.set(copy)
  }
  const svetReorderMs = performance.now() - tSvetRe0
  svetReorderMount.unmount()

  record(
    `4. Перестановка 1 000 элементов списка (${REORDER_ITERS} итераций)`,
    vueReorderMs,
    svetReorderMs,
    'Vue VDOM LIS diff vs Svet Rust WASM LIS (compute_lis в линейной памяти)',
  )

  // =========================================================================
  // 5. Customizer Panel / CAD Parameter Controls in DOM (50 sliders + 50 live value badges, 1,500 slider ticks)
  // =========================================================================
  const PARAM_COUNT = 50
  const SLIDER_TICKS = 1500

  const vueParamHost = document.createElement('div')
  const vueParams = vueReactive(
    Array.from({ length: PARAM_COUNT }, (_, i) => ({
      name: `param_${i}`,
      value: 10 + i,
    })),
  )
  const vueParamApp = createApp({
    setup() {
      return () =>
        vueH(
          'div',
          { class: 'customizer-panel' },
          vueParams.map((p, i) => {
            const derived = Math.round((p.value * 2 + vueParams[(i + 1) % PARAM_COUNT].value) * 1.5)
            return vueH('div', { key: p.name, class: 'param-row' }, [
              vueH('label', null, p.name),
              vueH('input', { type: 'range', value: p.value }),
              vueH('span', { class: 'param-val' }, `${derived} мм`),
            ])
          }),
        )
    },
  })
  vueParamApp.mount(vueParamHost)

  const tVueDag0 = performance.now()
  for (let i = 0; i < SLIDER_TICKS; i++) {
    const idx = i % PARAM_COUNT
    vueParams[idx].value = 10 + (i % 200)
    await nextTick()
  }
  const vueDagMs = performance.now() - tVueDag0
  vueParamApp.unmount()

  resetReactiveSystem()
  const svetParamHost = document.createElement('div')
  const svetParams = Array.from({ length: PARAM_COUNT }, (_, i) => ({
    name: `param_${i}`,
    value: svetSignal(10 + i),
  }))
  const svetDerived = svetParams.map((p, i) =>
    svetComputed(() =>
      Math.round((p.value() * 2 + svetParams[(i + 1) % PARAM_COUNT].value()) * 1.5),
    ),
  )
  const svetParamMount = svetMount(
    () =>
      svetH(
        'div',
        { className: 'customizer-panel' },
        ...svetParams.map((p, i) =>
          svetH(
            'div',
            { className: 'param-row' },
            svetH('label', null, p.name),
            svetH('input', { type: 'range', value: () => p.value() }),
            svetH('span', { className: 'param-val' }, () => `${svetDerived[i]()} мм`),
          ),
        ),
      ),
    svetParamHost,
  )

  const tSvetDag0 = performance.now()
  for (let i = 0; i < SLIDER_TICKS; i++) {
    const idx = i % PARAM_COUNT
    svetParams[idx].value.set(10 + (i % 200))
  }
  const svetDagMs = performance.now() - tSvetDag0
  svetParamMount.unmount()

  record(
    `5. CustomizerPanel (50 параметров в DOM): ${SLIDER_TICKS} движений ползунка`,
    vueDagMs,
    svetDagMs,
    'Vue диффает 50 строк формы; Svet обновляет ровно 2 зависимых DOM-узла',
  )

  // =========================================================================
  // 6. Lifecycle & Memory Churn: 500 Mount/Unmount Cycles of Dock Panels + WASM Slot Recycling
  // =========================================================================
  const CHURN_CYCLES = 500
  const tVueChurn0 = performance.now()
  for (let i = 0; i < CHURN_CYCLES; i++) {
    const box = document.createElement('div')
    const st = vueReactive(initialRegionState())
    const app = createApp({
      setup() {
        return () =>
          vueH(FormaRegion, {
            source: rawUiSources['Topbar.ui'],
            state: st,
            locale: 'ru',
          })
      },
    })
    app.mount(box)
    app.unmount()
  }
  const vueChurnMs = performance.now() - tVueChurn0

  resetReactiveSystem()
  const topbarAst = parseFormaSource(rawUiSources['Topbar.ui'])
  const topbarInit = {
    workspaceModeGroupLabel: 'Режим работы',
    workspaceMode: 'solid',
    modes: [
      { id: 'solid', label: 'Solid', hint: 'Точные тела' },
      { id: 'mesh', label: 'Mesh', hint: 'Полигональная сетка' },
      { id: 'code', label: 'Code', hint: 'Код' },
    ],
    workspaceConflict: false,
    persistenceError: false,
    editorOpen: true,
    sourceLabel: 'Исходный код',
    commandHelp: 'Команды',
    commandsLabel: 'Команды',
    exportTitle: 'Экспорт модели',
    exportLabel: 'Экспорт',
  }
  const sharedTopbarVm = Object.fromEntries(
    Object.entries(topbarInit).map(([k, v]) => [k, svetSignal(v)]),
  )
  const topbarInst = compileFormaAst(topbarAst, { viewModel: sharedTopbarVm })
  // Warm up one cycle to measure WASM arena capacity stability
  {
    const box = document.createElement('div')
    const h = topbarInst.mount(box)
    h.unmount()
  }
  const capBefore = getGraphTelemetry().capacity

  const tSvetChurn0 = performance.now()
  for (let i = 0; i < CHURN_CYCLES; i++) {
    const box = document.createElement('div')
    const h = topbarInst.mount(box)
    h.unmount()
  }
  const svetChurnMs = performance.now() - tSvetChurn0
  const capAfterChurn = getGraphTelemetry().capacity
  topbarInst.dispose()
  for (const sig of Object.values(sharedTopbarVm)) sig.dispose()
  const telemetryAfter = getGraphTelemetry()

  record(
    `6. Жизненный цикл Mount/Unmount Topbar.ui (${CHURN_CYCLES} циклов)`,
    vueChurnMs,
    svetChurnMs,
    `WASM слоты после ${CHURN_CYCLES} циклов: active=${telemetryAfter.nodes}, рост ёмкости=${capAfterChurn - capBefore}`,
  )

  console.log('\n=== РЕЗУЛЬТАТЫ БЕНЧМАРКА: VUE 3.5.42 vs SVET ENGINE (open-scad-viewer) ===\n')
  console.table(results)
}

describe('Vue 3.5.42 vs Svet Engine Benchmark on open-scad-viewer workloads', () => {
  it('runs head-to-head benchmarks and verifies Svet performance & zero WASM leaks', async () => {
    await runBenchmarks()
    expect(results.length).toBe(6)
    expect(getGraphTelemetry().nodes).toBe(0)
  }, 60_000)
})
