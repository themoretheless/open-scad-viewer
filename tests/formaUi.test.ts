import { describe, expect, it, vi } from 'vitest'
import { createRenderer, defineComponent, h, nextTick, reactive } from 'svet'
import CustomizerPanel from '../src/components/CustomizerPanel.svet'
import FormaRegion from '../src/components/FormaRegion.svet'
import SceneOutliner from '../src/components/SceneOutliner.svet'
import { parseFormaUi } from '../src/forma/parser'
import { batch, computed as svetComputed, effect, signal } from '../src/forma/svet'
import { resolveFormaTokens } from '../src/forma/tokens'
import computeInspectorUi from '../src/ui/ComputeInspector.ui?raw'
import dockRailUi from '../src/ui/DockRail.ui?raw'
import editorStatsUi from '../src/ui/EditorStats.ui?raw'
import editorToolbarUi from '../src/ui/EditorToolbar.ui?raw'
import selectionModesUi from '../src/ui/SelectionModes.ui?raw'
import statusbarUi from '../src/ui/Statusbar.ui?raw'
import topbarUi from '../src/ui/Topbar.ui?raw'
import viewerToolbarUi from '../src/ui/ViewerToolbar.ui?raw'
import workspaceActionBarUi from '../src/ui/WorkspaceActionBar.ui?raw'

class TestNode {
  parent: TestNode | null = null
  children: TestNode[] = []
  props: Record<string, any> = {}
  text = ''
  constructor(public tag: string) {}
}

const renderer = createRenderer<TestNode, TestNode>({
  createElement: tag => new TestNode(tag),
  createText: text => Object.assign(new TestNode('#text'), { text }),
  createComment: () => new TestNode('#comment'),
  setText: (n, text) => { n.text = text },
  setElementText: (n, text) => { n.children = []; n.text = text },
  parentNode: n => n.parent,
  nextSibling: n => n.parent?.children[n.parent.children.indexOf(n) + 1] ?? null,
  patchProp: (n, key, _old, value) => { n.props[key] = value },
  insert: (n, parent, anchor) => {
    n.parent = parent
    const i = anchor ? parent.children.indexOf(anchor) : -1
    if (i < 0) parent.children.push(n)
    else parent.children.splice(i, 0, n)
  },
  remove: n => {
    if (n.parent) n.parent.children = n.parent.children.filter(c => c !== n)
  },
  setScopeId: () => {},
  insertStaticContent: () => { throw new Error('Unexpected static content') },
})

function allNodes(root: TestNode): TestNode[] {
  return [root, ...root.children.flatMap(allNodes)]
}

function textContent(node: TestNode): string {
  return node.text + node.children.map(textContent).join('')
}

function mountForma(
  source: string,
  state: Record<string, unknown>,
  onAction = vi.fn(),
  onBinding = vi.fn(),
) {
  const root = new TestNode('root')
  const Host = defineComponent({
    setup() {
      return () =>
        h(FormaRegion, {
          source,
          state,
          locale: 'ru',
          onAction,
          onBinding,
        })
    },
  })
  const app = renderer.createApp(Host)
  app.mount(root)
  return { root, nodes: allNodes(root), onAction, onBinding, unmount: () => app.unmount() }
}

describe('Forma UI engine & open-scad-viewer declarative .ui regions', () => {
  it('parses all open-scad-viewer .ui specifications with design tokens', () => {
    for (const [name, src] of [
      ['Topbar', topbarUi],
      ['EditorToolbar', editorToolbarUi],
      ['EditorStats', editorStatsUi],
      ['ViewerToolbar', viewerToolbarUi],
      ['SelectionModes', selectionModesUi],
      ['DockRail', dockRailUi],
      ['ComputeInspector', computeInspectorUi],
      ['Statusbar', statusbarUi],
      ['WorkspaceActionBar', workspaceActionBarUi],
    ] as const) {
      const ast = parseFormaUi(src)
      expect(ast.name).toBe(name)
      expect(ast.children.length).toBeGreaterThan(0)
      expect(Object.keys(ast.tokens).length).toBeGreaterThan(0)
    }
  })

  it('resolves Forma Unified Style Guide CSS tokens for dark and light themes', () => {
    const dark = resolveFormaTokens('dark', { '--accent': '#ff9900' })
    const light = resolveFormaTokens('light')
    expect(dark['--forma-accent']).toBe('#ff9900')
    expect(dark['--forma-bg-canvas']).toBe('#1e1d1b')
    expect(light['--forma-bg-surface']).toBe('#ffffff')
  })

  it('renders Topbar.ui and dispatches workspace mode and editor toggle actions', () => {
    const { nodes, onAction, unmount } = mountForma(topbarUi, {
      workspaceModeGroupLabel: 'Режим работы',
      workspaceMode: 'solid',
      modes: [
        { id: 'solid', label: 'Solid', hint: 'Точные тела' },
        { id: 'mesh', label: 'Mesh', hint: 'Полигональная сетка' },
      ],
      workspaceConflict: false,
      persistenceError: false,
      editorOpen: false,
      sourceLabel: 'Исходный код',
      commandHelp: 'Команды',
      commandsLabel: 'Команды',
      exportTitle: 'Экспорт модели',
      exportLabel: 'Экспорт',
    })

    const sourceBtn = nodes.find(
      n => n.tag === 'button' && String(n.props.class ?? '').includes('source-toggle'),
    )
    expect(sourceBtn).toBeDefined()
    expect(sourceBtn!.props['aria-label']).toBe('Исходный код')
    sourceBtn!.props.onClick({} as Event)
    expect(onAction).toHaveBeenCalledWith('toggleEditor', [], expect.anything())

    const modeGroup = nodes.find(
      n => n.tag === 'div' && String(n.props.class ?? '').includes('mode-switch'),
    )
    expect(modeGroup).toBeDefined()
    const modeButtons = modeGroup!.children.filter(n => n.tag === 'button')
    expect(modeButtons.length).toBe(2)
    modeButtons[1].props.onClick({} as Event)
    expect(onAction).toHaveBeenCalledWith('openWorkspaceMode', ['mesh'], expect.anything())
    unmount()
  })

  it('renders EditorToolbar.ui and binds autoRender checkbox + render action', () => {
    const { nodes, onAction, onBinding, unmount } = mountForma(editorToolbarUi, {
      rendering: false,
      renderLabel: 'Собрать',
      cancelBuildLabel: 'Отменить сборку',
      autoRender: true,
      autoLabel: 'Авто',
      solidBuilding: false,
      cancelSolidLabel: 'Отмена',
    })

    const renderBtn = nodes.find(
      n => n.tag === 'button' && String(n.props.class ?? '').includes('btn-primary'),
    )
    expect(renderBtn).toBeDefined()
    expect(textContent(renderBtn!)).toContain('Собрать')
    renderBtn!.props.onClick({} as Event)
    expect(onAction).toHaveBeenCalledWith('renderFull', [], expect.anything())

    const checkbox = nodes.find(n => n.tag === 'input' && n.props.type === 'checkbox')
    expect(checkbox).toBeDefined()
    expect(checkbox!.props.checked).toBe(true)
    checkbox!.props.onChange({ target: { checked: false } } as unknown as Event)
    expect(onBinding).toHaveBeenCalledWith('autoRender', false)
    unmount()
  })

  it('renders ViewerToolbar.ui, SelectionModes.ui, DockRail.ui, and Statusbar.ui', () => {
    const viewerSlots = mountForma(
      viewerToolbarUi,
      {
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
      },
    )
    const fitBtn = viewerSlots.nodes.find(
      n => n.tag === 'button' && n.props['aria-label'] === 'Вписать',
    )
    expect(fitBtn).toBeDefined()
    fitBtn!.props.onClick({} as Event)
    expect(viewerSlots.onAction).toHaveBeenCalledWith('fitView', [], expect.anything())
    viewerSlots.unmount()

    const slotRoot = new TestNode('root')
    const slotApp = renderer.createApp({
      render: () =>
        h(
          FormaRegion,
          { source: viewerToolbarUi, state: { toolbarClass: 'viewer-toolbar', displayMode: 'shaded' } },
          {
            standardViewSelect: () => h('select', { class: 'view-select' }),
            scanToggle: () => h('button', { class: 'scan-toggle' }, 'Scan'),
            dockToggle: () => h('button', { class: 'dock-toggle' }, 'Dock'),
          },
        ),
    })
    slotApp.mount(slotRoot)
    try {
      const slotNodes = allNodes(slotRoot)
      expect(slotNodes.some(n => n.tag === 'select' && n.props.class === 'view-select')).toBe(true)
      expect(slotNodes.some(n => n.tag === 'button' && n.props.class === 'scan-toggle')).toBe(true)
      expect(slotNodes.some(n => n.tag === 'button' && n.props.class === 'dock-toggle')).toBe(true)
    } finally {
      slotApp.unmount()
    }

    const sel = mountForma(selectionModesUi, {
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
      selectedSummary: 'Выбрано: Объект 1 / 2',
      isolateLabel: 'Изолировать',
      focusLabel: 'Фокус',
      isolated: false,
      isolateTitle: 'Изолировать',
      deselectLabel: 'Снять выделение',
    })
    const hud = sel.nodes.find(
      n => n.tag === 'div' && String(n.props.class ?? '').includes('selection-hud'),
    )
    expect(hud).toBeDefined()
    expect(textContent(hud!)).toContain('Выбрано: Объект 1 / 2')
    sel.unmount()

    const status = mountForma(statusbarUi, {
      statusAriaLabel: 'Состояние',
      gpuDotClass: 'ok',
      backendLabel: 'WebGPU',
      showSortedTransparency: false,
      transparencySortedLabel: '',
      showPersistence: true,
      persistenceLabel: 'Черновик сохранён',
      conversionPending: false,
      conversionLabel: '',
      conversionMax: 2,
      conversionValue: 0,
      cancelLabel: 'Отмена',
      hintLabel: 'Орбита: перетаскивание',
    })
    const statusbar = status.nodes.find(
      n => n.tag === 'footer' && String(n.props.class ?? '').includes('statusbar'),
    )
    expect(statusbar).toBeDefined()
    expect(textContent(statusbar!)).toContain('Manifold · Forma')
    status.unmount()
  })
})

it('keeps live mode state, conditional actions and application slots', async () => {
  const state = reactive({ workspaceMode: 'solid', modes: [{id:'solid',label:'Solid'}, {id:'mesh',label:'Mesh'}], editorOpen:false, workspaceConflict:false })
  const root = new TestNode('root'), actions = vi.fn()
  const app = renderer.createApp({render:()=>h(FormaRegion,{source:topbarUi,state,onAction:actions},{fileMenu:()=>h('mark',{id:'file-slot'},'File menu'),settingsMenu:()=>h('mark',{id:'settings-slot'},'Settings')})})
  app.mount(root)
  const modes=()=>allNodes(root).filter(n=>n.tag==='button'&&String(n.props.class).includes('forma-seg-btn'))
  try {
    expect(modes().map(n=>n.props['aria-pressed'])).toEqual([true,false])
    expect(allNodes(root).filter(n=>n.tag==='mark')).toHaveLength(2)
    state.workspaceMode='mesh';state.editorOpen=true;await nextTick()
    expect(modes().map(n=>n.props['aria-pressed'])).toEqual([false,true])
    expect(allNodes(root).find(n=>String(n.props.class).includes('source-toggle'))!.props['aria-pressed']).toBe(true)
    state.modes.push({id:'code',label:'Code'});await nextTick();modes()[2].props.onClick({})
    expect(actions).toHaveBeenLastCalledWith('openWorkspaceMode',['code'],expect.anything())
  } finally {app.unmount()}
})

it('uses full comparison tokens and unary precedence for live control conditions', () => {
  const source=`component Conditions { Row {
    Button { text: "strict"; active: state.n === 2; }
    Button { text: "different"; active: state.name !== "old"; }
    Button { text: "range"; active: state.n >= 2 && state.n <= 3; }
    Button { text: "negation"; disabled: !state.selected && !state.isolated; }
    Button { text: "alternative"; active: state.n == 2 || state.n != 2; }
  } }`
  for (const selected of [false,true]) for (const isolated of [false,true]) {
    const v=mountForma(source,{n:2,name:'new',selected,isolated})
    try {const buttons=v.nodes.filter(n=>n.tag==='button');expect(buttons.slice(0,3).map(n=>String(n.props.class).includes('active'))).toEqual([true,true,true]);expect(buttons[3].props.disabled).toBe(!selected&&!isolated);expect(String(buttons[4].props.class)).toContain('active')}finally{v.unmount()}
  }
})

it('shows actual compute results and preserves computation cancellation and errors', () => {
  const busy=mountForma(computeInspectorUi,{panelTitle:'Geometry',computeBusy:true,canRunCompute:false,computeButtonLabel:'Compute',cancelLabel:'Cancel',hasResult:false})
  try {expect(textContent(busy.root)).not.toContain('14 912');expect(textContent(busy.root)).not.toContain('100%');const buttons=busy.nodes.filter(n=>n.tag==='button');expect(buttons[0].props.disabled).toBe(true);buttons[1].props.onClick({});expect(busy.onAction).toHaveBeenCalledWith('cancelGeometryAnalysis',[],expect.anything())}finally{busy.unmount()}
  const result=mountForma(computeInspectorUi,{hasResult:true,areaPrefix:'Area: ',areaValue:'42',backendSummary:'CPU fallback',timingSummary:'CPU 2 ms',showFallback:true,fallbackNote:'GPU unavailable',computeError:'Check failed'})
  try {expect(textContent(result.root)).toContain('Area: 42');expect(textContent(result.root)).toContain('CPU fallback');expect(textContent(result.nodes.find(n=>n.props.role==='alert')!)).toBe('Check failed')}finally{result.unmount()}
})

it('retains all workspace primitive and Boolean actions in the default slot', () => {
  const v=mountForma(workspaceActionBarUi,{})
  try {for (const b of v.nodes.filter(n=>n.tag==='button')) b.props.onClick({});expect(v.onAction.mock.calls.map(c=>c.slice(0,2))).toEqual([['insertPrimitive',['cube']],['insertPrimitive',['cylinder']],['insertPrimitive',['sphere']],['applyBoolean',['difference']]])}finally{v.unmount()}
})

it('retains draft conflict choices and save retry actions', () => {
  const v=mountForma(topbarUi,{modes:[],workspaceConflict:true,conflictWarning:'Conflict',exportDraftLabel:'Export draft',useIndexedLabel:'Use saved',restoreDraftLabel:'Keep local',persistenceError:true,unsavedDraftLabel:'Retry save'})
  try {
    for(const label of ['Export draft','Use saved','Keep local','Retry save'])v.nodes.find(n=>n.tag==='button'&&textContent(n)===label)!.props.onClick({})
    expect(v.onAction.mock.calls.map(c=>c.slice(0,2))).toEqual([['saveSource',[]],['resolveConflict',[false]],['resolveConflict',[true]],['retryPersistence',[]]])
  } finally {v.unmount()}
})

it('renders actual conversion progress and dispatches cancellation', () => {
  const v=mountForma(statusbarUi,{conversionPending:true,conversionLabel:'Exporting',conversionValue:1,conversionMax:3,cancelLabel:'Stop'})
  try {const p=v.nodes.find(n=>n.tag==='progress')!;expect(p.props.value).toBe(1);expect(p.props.max).toBe(3);v.nodes.find(n=>n.tag==='button'&&textContent(n)==='Stop')!.props.onClick({});expect(v.onAction).toHaveBeenCalledWith('cancelConversion',[],expect.anything())}finally{v.unmount()}
})

it('updates Svet signals glitch-free and drives SceneOutliner + CustomizerPanel UX enhancements', async () => {
  const count = signal(2)
  const doubled = svetComputed(() => count() * 2)
  const observed: number[] = []
  const eff = effect(() => {
    observed.push(doubled())
  })
  batch(() => {
    count.set(5)
    count.set(10)
  })
  expect(observed).toEqual([4, 20])
  eff.dispose()

  const outlinerRoot = new TestNode('root')
  const toggleVisibility = vi.fn()
  const outlinerApp = renderer.createApp({
    render: () =>
      h(SceneOutliner, {
        meshes: [
          { id: 1, name: 'Body 1', visible: true, triangleCount: 120 },
          { id: 2, name: 'Body 2', visible: true, triangleCount: 80 },
        ],
        selectedMeshId: 1,
        locale: 'ru',
        onToggleVisibility: toggleVisibility,
      }),
  })
  outlinerApp.mount(outlinerRoot)
  try {
    expect(textContent(outlinerRoot)).toContain('200 тр.')
    const hideAllBtn = allNodes(outlinerRoot).find(
      n => n.tag === 'button' && n.props['aria-label'] === 'Скрыть все объекты',
    )
    expect(hideAllBtn).toBeDefined()
    hideAllBtn!.props.onClick({})
    expect(toggleVisibility).toHaveBeenCalledWith(1, false)
    expect(toggleVisibility).toHaveBeenCalledWith(2, false)
  } finally {
    outlinerApp.unmount()
  }

  const customizerRoot = new TestNode('root')
  const onChange = vi.fn()
  const state = reactive({
    parameters: [
      { name: 'width', label: 'Width', value: 20, valueStart: 0, valueEnd: 2, min: 5, max: 50, step: 1 },
    ],
  })
  const customizerApp = renderer.createApp({
    render: () =>
      h(CustomizerPanel, {
        parameters: state.parameters,
        title: 'Parameters',
        emptyLabel: 'None',
        onChange,
      }),
  })
  customizerApp.mount(customizerRoot)
  try {
    const rangeInput = allNodes(customizerRoot).find(n => n.tag === 'input' && n.props.type === 'range')!
    rangeInput.props.onInput({ target: { value: '35', valueAsNumber: 35 } })
    state.parameters = [{ ...state.parameters[0], value: 35 }]
    await nextTick()
    expect(onChange).toHaveBeenCalledWith('width', 35)
    const resetBtn = allNodes(customizerRoot).find(
      n => n.tag === 'button' && n.props['aria-label'] === 'Reset width',
    )
    expect(resetBtn).toBeDefined()
    resetBtn!.props.onClick({ preventDefault() {} })
    expect(onChange).toHaveBeenLastCalledWith('width', 20)
  } finally {
    customizerApp.unmount()
  }
})
