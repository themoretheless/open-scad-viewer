import { describe, expect, it, vi } from 'vitest'
import { createRenderer, defineComponent, h } from 'vue'
import FormaRegion from '../src/components/FormaRegion.vue'
import { parseFormaUi } from '../src/forma/parser'
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
    const viewer = mountForma(viewerToolbarUi, {
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
    })
    const fitBtn = viewer.nodes.find(
      n => n.tag === 'button' && n.props['aria-label'] === 'Вписать',
    )
    expect(fitBtn).toBeDefined()
    fitBtn!.props.onClick({} as Event)
    expect(viewer.onAction).toHaveBeenCalledWith('fitView', [], expect.anything())
    viewer.unmount()

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
