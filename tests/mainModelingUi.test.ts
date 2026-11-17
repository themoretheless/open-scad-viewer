import { createRenderer, nextTick } from 'svet'
import { it, expect, vi, afterEach } from 'vitest'
import MainModelingOverlay from '../src/features/MainModelingOverlay.svt'
import MainSketchTools from '../src/features/MainSketchTools.svt'
import CadWorkbenchPanel from '../src/features/CadWorkbenchPanel.svt'
import { previewMeshes } from '../src/app/mainModeling'
import { extrudeDirectSketch, type DirectDocument } from '../src/app/directModeling'
import { sampleCurve } from '../src/app/directSketchGeometry'
import { inspectPolygonMesh } from '../src/kernel/geometry/polygon'
class Node {
  parent: Node | null = null
  children: Node[] = []
  props: Record<string, any> = {}
  style: Record<string, any> = {}
  text = ''
  value: any = ''
  selected = false
  constructor(public tag: string) {}
  get tagName() {
    return this.tag.toUpperCase()
  }
  get options() {
    return this.children.filter(n => n.tag === 'option')
  }
  get ownerSVGElement(): Node | null {
    return this.tag === 'svg' ? this : (this.parent?.ownerSVGElement ?? null)
  }
  getBoundingClientRect() {
    return { left: 0, top: 0, width: 1000, height: 1000 }
  }
  get clientWidth() {
    return 1000
  }
  get clientHeight() {
    return 1000
  }
  getRootNode() {
    return { activeElement: null }
  }
  focus() {}
  addEventListener() {}
  removeEventListener() {}
  setPointerCapture() {}
  hasPointerCapture() {
    return false
  }
  getScreenCTM() {
    return { inverse: () => ({}) }
  }
  setAttribute(key: string, v: any) {
    this.props[key] = v
  }
  removeAttribute(key: string) {
    delete this.props[key]
  }
}
const renderer = createRenderer<Node, Node>({
  createElement: tag => new Node(tag),
  createText: text => {
    const n = new Node('#text')
    n.text = text
    return n
  },
  createComment: () => new Node('#comment'),
  setText: (n, t) => {
    n.text = t
  },
  setElementText: (n, t) => {
    n.children = []
    n.text = t
  },
  parentNode: n => n.parent,
  nextSibling: n => n.parent?.children[n.parent.children.indexOf(n) + 1] ?? null,
  patchProp: (n, k, _old, v) => {
    n.props[k] = v
    if (k === 'value') n.value = v
  },
  insert: (n, p, anchor) => {
    if (n.parent) n.parent.children = n.parent.children.filter(c => c !== n)
    n.parent = p
    const i = anchor ? p.children.indexOf(anchor) : -1
    i < 0 ? p.children.push(n) : p.children.splice(i, 0, n)
  },
  remove: n => {
    if (n.parent) n.parent.children = n.parent.children.filter(c => c !== n)
  },
  setScopeId: () => {},
  insertStaticContent: () => {
    throw Error('Unexpected static content')
  },
})
const mounts: Array<() => void> = []
afterEach(() => {
  mounts.splice(0).forEach(f => f())
  vi.unstubAllGlobals()
})

async function mount(component: any, props: any, stored: string | null = null) {
  vi.stubGlobal('localStorage', { getItem: () => stored, setItem: () => {} })
  vi.stubGlobal('Document', class {})
  vi.stubGlobal('ShadowRoot', class {})
  vi.stubGlobal('document', { activeElement: null })
  vi.stubGlobal('window', { document: { activeElement: null } })
  const root = new Node('root'),
    app = renderer.createApp(component, props)
  app.mount(root)
  mounts.push(() => app.unmount())
  await nextTick()
  const all = (n: Node = root): Node[] => [n, ...n.children.flatMap(all)],
    text = (n: Node): string => n.text + n.children.map(text).join('')
  const event = (n: Node, x = 0, y = 0) => ({
    button: 0,
    target: n,
    currentTarget: n,
    clientX: x,
    clientY: y,
    pointerId: 1,
    preventDefault() {},
    stopPropagation() {},
  })
  return {
    all,
    text,
    event,
    click: async (name: string) => {
      const n = all().find(n => n.tag === 'button' && text(n) === name)!
      n.props.onClick({})
      await nextTick()
    },
  }
}
const body = extrudeDirectSketch(
  {
    id: 's',
    name: 'Box',
    closed: true,
    points: [
      [0, 0],
      [10, 0],
      [10, 10],
      [0, 10],
    ],
  },
  10,
  '0',
)
const parameters = {
  amount: 2,
  x: 0,
  y: 0,
  z: 0,
  axis: 'z',
  edge: 0,
  shape: 'rectangle',
  width: 2,
  height: 2,
  cut: false,
}
it('fits a bounded nominal opening and preserves geometry when print constraints conflict', async () => {
  const ui = await mount(CadWorkbenchPanel, {
    meshes: [],
    selection: [],
    hit: null,
    source: '',
    ready: true,
    locale: 'en',
    initialAction: 'lighten',
  })
  const input = (label: string) =>
    ui
      .all()
      .find(n => n.tag === 'label' && ui.text(n).startsWith(label))!
      .children.find(n => n.tag === 'input')!
  const set = async (label: string, value: unknown) => {
    input(label).props['onUpdate:modelValue'](value)
    await nextTick()
  }
  await set('Limit nominal opening', true)
  await set('Bridge limit', 5)
  await ui.click('Fit geometry to print settings')
  expect(input('Cell, mm').value).toBeCloseTo(6.35, 12)
  expect(input('Rib, mm').value).toBe(1.35)
  await set('Bridge limit', 1)
  await ui.click('Fit geometry to print settings')
  expect(ui.all().find(n => n.props.role === 'alert')?.text).toContain('incompatible')
  expect(input('Cell, mm').value).toBeCloseTo(6.35, 12)
  await set('Bridge limit', '')
  expect(input('Cell, mm').value).toBeCloseTo(6.35, 12)
  await set('Bridge limit', 4)
  await ui.click('Fit geometry to print settings')
  expect(input('Cell, mm').value).toBeCloseTo(5.35, 12)
  expect(ui.all().some(n => n.props.role === 'alert')).toBe(false)
})
it('offers centered lattice patterns with spatial controls and no ineffective randomization', async () => {
  const ui = await mount(CadWorkbenchPanel, {
    meshes: [],
    selection: [],
    hit: null,
    source: '',
    ready: true,
    locale: 'en',
    initialAction: 'lighten',
  })
  const select = ui.all().find(n => n.tag === 'select' && n.options.some(o => o.value === 'bcc'))!
  expect(select).toBeDefined()
  expect(select.options.some(o => o.value === 'octet')).toBe(true)
  expect(select.options.some(o => o.value === 'isogrid')).toBe(true)
  for (const pattern of ['bcc', 'octet']) {
    select.props['onUpdate:modelValue'](pattern)
    await nextTick()
    const labels = ui
      .all()
      .filter(n => n.tag === 'label')
      .map(ui.text)
    expect(labels.some(s => s.includes('Outer skin, mm'))).toBe(true)
    expect(labels.some(s => s.includes('Sampling step, mm'))).toBe(true)
    expect(labels.some(s => s.includes('Channel axis'))).toBe(false)
    expect(labels.some(s => s.includes('Randomness'))).toBe(false)
    expect(ui.all().some(n => n.tag === 'button' && ui.text(n) === 'Next variation')).toBe(false)
  }
  select.props['onUpdate:modelValue']('spatial')
  await nextTick()
  expect(ui.all().some(n => n.tag === 'label' && ui.text(n).includes('Randomness'))).toBe(true)
  select.props['onUpdate:modelValue']('grid')
  await nextTick()
  expect(ui.all().some(n => n.tag === 'label' && ui.text(n).includes('Channel axis'))).toBe(true)
  select.props['onUpdate:modelValue']('isogrid')
  await nextTick()
  expect(ui.all().some(n => n.tag === 'label' && ui.text(n).includes('Channel axis'))).toBe(true)
  expect(ui.all().some(n => n.tag === 'label' && ui.text(n).includes('Outer skin, mm'))).toBe(false)
})
it('drags a main viewport gizmo and commits once on release', async () => {
  const values: any[] = [],
    preview = vi.fn(),
    apply = vi.fn()
  const ui = await mount(MainModelingOverlay, {
    meshes: previewMeshes({ version: 1, sketches: [], bodies: [body] }),
    selected: 0,
    selection: [0],
    hit: null,
    operation: 'move',
    parameters,
    project: (p: number[]) => [p[0] * 10, p[1] * 10],
    revision: 0,
    box: false,
    onParameters: (p: any) => values.push(p),
    onPreview: preview,
    onApply: apply,
  })
  const svg = ui.all().find(n => n.tag === 'svg')!,
    g = ui.all().find(n => n.tag === 'g' && n.props.onPointerdown)!
  g.props.onPointerdown(ui.event(g, 0, 0))
  svg.props.onPointermove(ui.event(svg, 20, 0))
  expect(values.at(-1).x).toBeCloseTo(2)
  expect(preview).toHaveBeenCalledTimes(1)
  expect(apply).not.toHaveBeenCalled()
  svg.props.onPointerup()
  expect(apply).toHaveBeenCalledTimes(1)
})
it('draws and extrudes a sketch directly on the main workplane', async () => {
  const emitted: any[] = [],
    ui = await mount(MainSketchTools, {
      plane: null,
      project: (p: number[]) => [p[0], p[1]],
      ray: (x: number, y: number) => ({ origin: [x, y, 100], direction: [0, 0, -1] }),
      revision: 0,
      locale: 'en',
      onBody: (b: any) => emitted.push(b),
    })
  await ui.click('rectangle')
  const svg = ui.all().find(n => n.tag === 'svg')!
  svg.props.onPointerdown(ui.event(svg, 0, 0))
  svg.props.onPointermove(ui.event(svg, 10, 10))
  svg.props.onPointerup()
  await nextTick()
  await ui.click('Extrude')
  expect(emitted).toHaveLength(1)
  expect(inspectPolygonMesh(emitted[0].mesh).signedVolumeMm3).toBeCloseTo(1000)
})

const constrainedSketches = (curveConstraints: unknown[]): string => {
  const circle = (id: string, center: [number, number], radius: number) => {
    const analytic = { kind: 'circle' as const, center, radius, start: 0, sweep: 360 }
    return { id, name: id, points: sampleCurve(analytic), closed: true, analytic }
  }
  const line = {
    id: 'L',
    name: 'L',
    points: [
      [-10, 0],
      [10, 0],
      [10, 5],
    ],
    closed: false,
  }
  return JSON.stringify({
    version: 1,
    sketches: [circle('R', [0, 0], 5), line, { ...circle('A', [0, 0], 2), curveConstraints }],
    bodies: [],
  })
}
it('re-solves a constrained circle when its own handles are dragged in the main sketch tool', async () => {
  const ui = await mount(
    MainSketchTools,
    {
      plane: null,
      project: (p: number[]) => [p[0], p[1]],
      ray: (x: number, y: number) => ({ origin: [x, y, 100], direction: [0, 0, -1] }),
      revision: 0,
      locale: 'en',
    },
    constrainedSketches([{ kind: 'concentric', ref: 'R' }]),
  )
  const svg = ui.all().find(n => n.tag === 'svg')!,
    curve = () => ui.all().filter(n => n.tag === 'polyline' && n.props['stroke-width'] === '3')[2]!
  curve().props.onPointerdown(ui.event(curve(), 0, 2))
  await nextTick()
  const handles = () => ui.all().filter(n => n.tag === 'circle' && String(n.props.r) === '6')
  const center = handles()[0]!
  center.props.onPointerdown(ui.event(center, 0, 0))
  svg.props.onPointermove(ui.event(svg, 7, 7))
  svg.props.onPointerup()
  await nextTick()
  expect(handles()[0]!.props.cx).toBeCloseTo(0, 6)
  expect(handles()[0]!.props.cy).toBeCloseTo(0, 6)
  const radius = handles()[1]!
  radius.props.onPointerdown(ui.event(radius, 2, 0))
  svg.props.onPointermove(ui.event(svg, 0, 20))
  svg.props.onPointerup()
  await nextTick()
  expect(Math.hypot(handles()[1]!.props.cx, handles()[1]!.props.cy)).toBeCloseTo(20, 6)
  expect(handles()[0]!.props.cx).toBeCloseTo(0, 6)
  expect(ui.all().some(n => n.props.role === 'alert' || n.props.role === 'status')).toBe(false)
})
it('refuses a main sketch handle drag with a notice when the circle constraints cannot be satisfied', async () => {
  const ui = await mount(
    MainSketchTools,
    {
      plane: null,
      project: (p: number[]) => [p[0], p[1]],
      ray: (x: number, y: number) => ({ origin: [x, y, 100], direction: [0, 0, -1] }),
      revision: 0,
      locale: 'en',
    },
    constrainedSketches([
      { kind: 'concentric', ref: 'R' },
      { kind: 'equal_radius', ref: 'R' },
      { kind: 'tangent_line', ref: 'L', a: 0, b: 1 },
    ]),
  )
  const svg = ui.all().find(n => n.tag === 'svg')!,
    curve = () => ui.all().filter(n => n.tag === 'polyline' && n.props['stroke-width'] === '3')[2]!
  curve().props.onPointerdown(ui.event(curve(), 0, 2))
  await nextTick()
  const before = curve().props.points,
    radius = ui.all().filter(n => n.tag === 'circle' && String(n.props.r) === '6')[1]!
  radius.props.onPointerdown(ui.event(radius, 2, 0))
  svg.props.onPointermove(ui.event(svg, 0, 9))
  await nextTick()
  expect(curve().props.points).toEqual(before)
  expect(
    ui.all().find(n => n.props.role === 'status') && ui.text(ui.all().find(n => n.props.role === 'status')!),
  ).toContain('drag was refused')
  svg.props.onPointerup()
  await nextTick()
  expect(curve().props.points).toEqual(before)
})
it('authors Bézier nodes, edits handles, cancels a drag and extrudes from the viewport', async () => {
  const bodies: any[] = [],
    ui = await mount(MainSketchTools, {
      plane: null,
      project: (p: number[]) => [p[0], p[1]],
      ray: (x: number, y: number) => ({ origin: [x, y, 100], direction: [0, 0, -1] }),
      revision: 0,
      locale: 'en',
      onBody: (b: any) => bodies.push(b),
    })
  await ui.click('bezier')
  const svg = ui.all().find(n => n.tag === 'svg')!
  for (const [x, y] of [
    [0, 0],
    [40, 0],
    [40, 40],
    [0, 40],
  ])
    svg.props.onPointerdown(ui.event(svg, x, y))
  await nextTick()
  await ui.click('Close contour')
  expect(ui.all().filter(n => n.tag === 'circle' && String(n.props.r) === '6')).toHaveLength(4)
  const path = () => ui.all().find(n => n.tag === 'polyline' && n.props['stroke-width'] === '3')!.props.points
  const before = path(),
    handle = ui.all().find(n => n.tag === 'circle' && String(n.props.r) === '4')!
  handle.props.onPointerdown(ui.event(handle))
  svg.props.onPointermove(ui.event(svg, 15, -20))
  await nextTick()
  expect(path()).not.toEqual(before)
  svg.props.onPointercancel()
  await nextTick()
  expect(path()).toEqual(before)
  const anchor = ui.all().filter(n => n.tag === 'circle' && String(n.props.r) === '6')[1]
  anchor.props.onPointerdown(ui.event(anchor, 40, 0))
  svg.props.onPointermove(ui.event(svg, 50, 1))
  await nextTick()
  expect(ui.all().some(n => n.tag === 'line' && n.props['stroke-dasharray'] === '5 4')).toBe(true)
  svg.props.onPointerup()
  await nextTick()
  await ui.click('Extrude')
  expect(bodies).toHaveLength(1)
  expect(inspectPolygonMesh(bodies[0].mesh).signedVolumeMm3).toBeGreaterThan(0)
})
it('outlines a closed sketch as a region with a hole before viewport extrusion', async () => {
  const bodies: any[] = [],
    ui = await mount(MainSketchTools, {
      plane: null,
      project: (p: number[]) => [p[0], p[1]],
      ray: (x: number, y: number) => ({ origin: [x, y, 100], direction: [0, 0, -1] }),
      revision: 0,
      locale: 'en',
      onBody: (b: any) => bodies.push(b),
    })
  await ui.click('rectangle')
  const svg = ui.all().find(n => n.tag === 'svg')!
  svg.props.onPointerdown(ui.event(svg, 0, 0))
  svg.props.onPointermove(ui.event(svg, 20, 20))
  svg.props.onPointerup()
  await nextTick()
  await ui.click('Outline Stroke')
  await ui.click('Extrude')
  expect(bodies).toHaveLength(1)
  const volume = inspectPolygonMesh(bodies[0].mesh).signedVolumeMm3
  expect(volume).toBeGreaterThan(750)
  expect(volume).toBeLessThan(850)
})
const coincidentSketches = (): string => {
  const analytic = { kind: 'circle' as const, center: [0, 0] as [number, number], radius: 5, start: 0, sweep: 360 }
  const R = { id: 'R', name: 'R', points: sampleCurve(analytic), closed: true, analytic }
  const L = {
    id: 'L',
    name: 'L',
    points: [
      [0, 0],
      [20, 0],
      [20, 10],
    ],
    closed: false,
    curveConstraints: [{ kind: 'coincident', ref: 'R', point: 0, target: 'center' }],
    constraints: [{ kind: 'horizontal', a: 0, b: 1 }],
  }
  return JSON.stringify({ version: 1, sketches: [R, L], bodies: [] })
}
const sketchProps = {
  plane: null,
  project: (p: number[]) => [p[0], p[1]],
  ray: (x: number, y: number) => ({ origin: [x, y, 100], direction: [0, 0, -1] }),
  revision: 0,
  locale: 'en',
}
it('C7 coincident: shows constraint glyphs in the main sketch overlay, selects one and deletes only it with Delete, undo returns it', async () => {
  const ui = await mount(MainSketchTools, sketchProps, coincidentSketches())
  const glyphs = () => ui.all().filter(n => n.tag === 'g' && n.props['data-constraint-glyph'])
  expect(
    glyphs()
      .map(g => g.props['data-constraint-glyph'])
      .sort(),
  ).toEqual(['coincident', 'horizontal'])
  const coincident = glyphs().find(g => g.props['data-constraint-glyph'] === 'coincident')!
  expect(ui.text(coincident)).toContain('●')
  const rect = ui.all(coincident).find(n => n.tag === 'rect')!
  // About 14 px from vertex 0 at the origin, diagonally.
  expect(Math.hypot(rect.props.x + 9, rect.props.y + 9)).toBeCloseTo(14, 6)
  coincident.props.onPointerdown(ui.event(coincident))
  await nextTick()
  expect(glyphs().find(g => g.props['data-constraint-glyph'] === 'coincident')!.props['aria-pressed']).toBe(true)
  expect(glyphs().find(g => g.props['data-constraint-glyph'] === 'horizontal')!.props['aria-pressed']).toBe(false)
  const svg = ui.all().find(n => n.tag === 'svg')!
  svg.props.onKeydown({ key: 'Delete', preventDefault() {} })
  await nextTick()
  expect(glyphs().map(g => g.props['data-constraint-glyph'])).toEqual(['horizontal'])
  ui.all()
    .find(n => n.tag === 'button' && ui.text(n) === '↶')!
    .props.onClick({})
  await nextTick()
  expect(
    glyphs()
      .map(g => g.props['data-constraint-glyph'])
      .sort(),
  ).toEqual(['coincident', 'horizontal'])
  // Delete without a selected glyph leaves the constraints alone.
  svg.props.onKeydown({ key: 'Delete', preventDefault() {} })
  await nextTick()
  expect(glyphs()).toHaveLength(2)
})
it('C7 coincident: a main sketch vertex drag keeps the coincident vertex on its target and refuses dragging it away', async () => {
  const ui = await mount(MainSketchTools, sketchProps, coincidentSketches())
  const svg = ui.all().find(n => n.tag === 'svg')!,
    line = () => ui.all().filter(n => n.tag === 'polyline' && n.props['stroke-width'] === '3')[1]!
  line().props.onPointerdown(ui.event(line(), 10, 0))
  await nextTick()
  const handles = () => ui.all().filter(n => n.tag === 'circle' && String(n.props.r) === '5')
  const end = handles()[2]!
  end.props.onPointerdown(ui.event(end, 20, 10))
  svg.props.onPointermove(ui.event(svg, 40, 30))
  svg.props.onPointerup()
  await nextTick()
  expect(handles()[2]!.props.cx).toBeCloseTo(40, 6)
  expect(handles()[0]!.props.cx).toBeCloseTo(0, 9)
  expect(handles()[0]!.props.cy).toBeCloseTo(0, 9)
  const before = line().props.points,
    first = handles()[0]!
  first.props.onPointerdown(ui.event(first, 0, 0))
  svg.props.onPointermove(ui.event(svg, -30, -30))
  await nextTick()
  expect(line().props.points).toEqual(before)
  expect(ui.text(ui.all().find(n => n.props.role === 'status')!)).toContain('edit was refused')
  svg.props.onPointerup()
  await nextTick()
  expect(line().props.points).toEqual(before)
})
it('C7 trim notice: the main sketch tool names dimensions the trim dropped', async () => {
  const { warmGeometryKernel } = await import('../src/kernel/geometry/kernel')
  await warmGeometryKernel()
  const stored = JSON.stringify({
    version: 1,
    bodies: [],
    sketches: [
      {
        id: 'L',
        name: 'L',
        closed: false,
        points: [
          [0, 0],
          [2, 0],
          [8, 0],
        ],
        dimensions: [{ kind: 'length', a: 1, b: 2 }],
      },
      {
        id: 'B',
        name: 'B',
        closed: false,
        points: [
          [5, -10],
          [5, 10],
        ],
      },
    ],
  })
  const ui = await mount(
    MainSketchTools,
    {
      plane: null,
      project: (p: number[]) => [p[0], p[1]],
      ray: (x: number, y: number) => ({ origin: [x, y, 100], direction: [0, 0, -1] }),
      revision: 0,
      locale: 'en',
    },
    stored,
  )
  await ui.click('trim')
  const line = ui.all().filter(n => n.tag === 'polyline' && n.props['stroke-width'] === '3')[0]!
  line.props.onPointerdown(ui.event(line, 7, 0))
  await nextTick()
  const status = ui.all().find(n => n.props.role === 'status')
  expect(status && ui.text(status)).toContain('Trim removed sketch dimensions (1): length (2–3).')
})
