import { createRenderer, h, nextTick, shallowReactive } from 'vue'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import GcodePanel from '../src/features/GcodePanel.vue'
import type { MeshData } from '../src/core/mesh'
import { checkGcodePreviewJob, GCODE_PREVIEW_MAX_BYTES, type GcodePreviewDocument } from '../src/services/gcodePreviewProtocol'

const service = vi.hoisted(() => ({ run: vi.fn(), cancel: vi.fn(), dispose: vi.fn(), download: vi.fn(), draw: vi.fn() }))
vi.mock('../src/services/gcodePreviewWorker', () => ({ createGcodePreviewWorker: () => ({ run: service.run, cancel: service.cancel, dispose: service.dispose }) }))
vi.mock('../src/services/cadDrawing', () => ({ downloadCad: service.download }))
vi.mock('../src/services/gcodePreviewGeometry', async importOriginal => ({ ...await importOriginal<object>(), drawGcodeLayer: service.draw }))

class Node {
  parent: Node | null = null
  children: Node[] = []
  props: Record<string, any> = {}
  text = ''
  value: any = ''
  constructor(public tag: string) {}
  get tagName() { return this.tag.toUpperCase() }
  getRootNode() { return { activeElement: null } }
  addEventListener() {}
  removeEventListener() {}
}
const renderer = createRenderer<Node, Node>({
  createElement: tag => new Node(tag), createText: text => { const node = new Node('#text'); node.text = text; return node }, createComment: () => new Node('#comment'),
  setText: (node, text) => { node.text = text }, setElementText: (node, text) => { node.children = []; node.text = text },
  parentNode: node => node.parent, nextSibling: node => node.parent?.children[node.parent.children.indexOf(node) + 1] ?? null,
  patchProp: (node, key, _old, value) => { node.props[key] = value; if (key === 'value') node.value = value },
  insert: (node, parent, anchor) => { if (node.parent) node.parent.children = node.parent.children.filter(child => child !== node); node.parent = parent; const index = anchor ? parent.children.indexOf(anchor) : -1; index < 0 ? parent.children.push(node) : parent.children.splice(index, 0, node) },
  remove: node => { if (node.parent) node.parent.children = node.parent.children.filter(child => child !== node) },
  setScopeId: () => {}, insertStaticContent: () => { throw new Error('Unexpected static content') },
})
const result = (): GcodePreviewDocument => ({
  dialect: 'open-scad-viewer/print-preview 2', gcode: '; validated preview',
  preview: { layers: 2, extrusionMm: 2, depositedVolumeMm3: 4, travelDistanceMm: 3, printDistanceMm: 10, estimatedTimeS: 60,
    bounds: { min: [2, 3, .2], max: [7, 3, .4] }, moves: [
      { x: 2, y: 3, z: .2, e: 0, extruded: false, feedrateMmS: 100, layerIndex: 0 },
      { x: 7, y: 3, z: .2, e: 1, extruded: true, feedrateMmS: 50, layerIndex: 0 },
      { x: 2, y: 3, z: .4, e: 1, extruded: false, feedrateMmS: 100, layerIndex: 1 },
      { x: 7, y: 3, z: .4, e: 2, extruded: true, feedrateMmS: 50, layerIndex: 1 },
    ] },
})
function mesh(z = 12): MeshData {
  return { vertices: new Float32Array([0, 0, 0, 0, 0, 1, 5, 0, 0, 0, 0, 1, 0, 5, 2, 0, 0, 1]), indices: new Uint32Array([0, 1, 2]), transform: new Float32Array([1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, z, 0, 0, 0, 1]) } as MeshData
}
const unmounts: Array<() => void> = []
beforeEach(() => {
  vi.clearAllMocks()
  service.run.mockReset().mockImplementation(async job => { checkGcodePreviewJob(job); return result() })
  vi.stubGlobal('Document', class {})
  vi.stubGlobal('ShadowRoot', class {})
  vi.stubGlobal('document', { activeElement: null })
})
afterEach(() => { unmounts.splice(0).forEach(unmount => unmount()); vi.unstubAllGlobals() })

async function mount(overrides: Record<string, unknown> = {}) {
  const props = shallowReactive({ meshes: [mesh(), mesh(-5)], selection: [0], ready: true, source: 'cube(5);', locale: 'en', ...overrides })
  const root = new Node('root'), app = renderer.createApp({ setup: () => () => h(GcodePanel, props) })
  const unmount = () => { app.unmount(); const index = unmounts.indexOf(unmount); if (index >= 0) unmounts.splice(index, 1) }
  app.mount(root); unmounts.push(unmount); await nextTick()
  const all = (node: Node = root): Node[] => [node, ...node.children.flatMap(all)]
  const text = (node: Node = root): string => node.text + node.children.map(child => text(child)).join('')
  const button = (name: string) => { const node = all().find(node => node.tag === 'button' && text(node) === name); expect(node, `button ${name}`).toBeDefined(); return node! }
  const input = (label: string) => { const node = all().find(node => node.props['aria-label'] === label); expect(node, `input ${label}`).toBeDefined(); return node! }
  const edit = async (label: string, value: unknown) => { input(label).props['onUpdate:modelValue'](value); await nextTick() }
  const click = async (name: string) => { const node = button(name); expect(node.props.disabled).toBeFalsy(); await node.props.onClick({}); await nextTick() }
  const upload = async (file: unknown) => { const target = { files: [file], value: 'selected' }; await input('Open preview G-code').props.onChange({ target }); await nextTick(); expect(target.value).toBe('') }
  return { all, text, button, input, edit, click, upload, props, unmount }
}

it('uses the single selected body’s transformed range, exposes all settings, previews layers and downloads the checked result', async () => {
  const ui = await mount({ selection: [1] })
  expect(ui.input('Z min, mm').value).toBe(-5)
  expect(ui.input('Z max, mm').value).toBe(-3)
  await ui.edit('Line width, mm', .6)
  await ui.edit('Wall count', 3)
  await ui.edit('Infill spacing, mm', 4)
  await ui.edit('Print speed, mm/s', 70)
  await ui.edit('Travel speed, mm/s', 180)
  await ui.edit('Filament diameter, mm', 2.85)
  await ui.edit('Layer height, mm', .3)
  await ui.click('Generate toolpaths')
  expect(service.run).toHaveBeenCalledWith({ kind: 'slice', mesh: ui.props.meshes[1], zMin: -5, zMax: -3,
    settings: { layerHeightMm: .3, lineWidthMm: .6, wallCount: 3, infillSpacingMm: 4, feedrateMmS: 70, travelFeedrateMmS: 180, filamentDiameterMm: 2.85 } })
  expect(ui.text()).toContain('open-scad-viewer/print-preview 2')
  expect(ui.text()).toContain('No heating, homing')
  expect(ui.all().some(node => node.tag === 'canvas')).toBe(true)
  expect(service.draw).toHaveBeenLastCalledWith(expect.anything(), result().preview, 0, true)
  await ui.edit('Preview layer', 1)
  expect(service.draw).toHaveBeenLastCalledWith(expect.anything(), result().preview, 1, true)
  expect(ui.text()).toContain('Layer 2 / 2')
  await ui.click('Download preview G-code')
  expect(service.download).toHaveBeenCalledWith(result().gcode, 'text/plain;charset=utf-8', 'body-2-preview.gcode')
})

it('requires a current build and exactly one valid body', async () => {
  const ui = await mount({ selection: [0, 1] })
  expect(ui.button('Generate toolpaths').props.disabled).toBe(true)
  expect(ui.text()).toContain('Select exactly one body')
  ui.props.selection = [0]; ui.props.ready = false; await nextTick()
  expect(ui.text()).toContain('Build the current source first')
  expect(ui.button('Generate toolpaths').props.disabled).toBe(true)
  ui.props.ready = true; ui.props.selection = [99]; await nextTick()
  expect(ui.button('Generate toolpaths').props.disabled).toBe(true)
  expect(service.run).not.toHaveBeenCalled()
})

it('invalidates a result on settings, source, selection and positioned mesh changes', async () => {
  const ui = await mount()
  await ui.click('Generate toolpaths')
  await ui.edit('Layer height, mm', .4)
  expect(ui.all().some(node => node.tag === 'canvas')).toBe(false)
  expect(ui.text()).toContain('Settings changed')
  await ui.click('Generate toolpaths')
  ui.props.source = 'cube(6);'; await nextTick()
  expect(ui.text()).toContain('Model changed')
  expect(ui.all().some(node => node.tag === 'canvas')).toBe(false)
  ui.props.meshes = [mesh(40)]; await nextTick()
  expect(ui.input('Z min, mm').value).toBe(40)
  expect(ui.input('Z max, mm').value).toBe(42)
  await ui.click('Generate toolpaths')
  ui.props.selection = []; await nextTick()
  expect(ui.all().some(node => node.tag === 'canvas')).toBe(false)
  expect(ui.button('Generate toolpaths').props.disabled).toBe(true)
})

it('discards delayed success after cancellation or edits and leaves a fresh generation usable', async () => {
  const ui = await mount()
  let finish!: (document: GcodePreviewDocument) => void
  service.run.mockImplementationOnce(() => new Promise(resolve => { finish = resolve }))
  const pending = ui.button('Generate toolpaths').props.onClick(); await nextTick()
  expect(ui.text()).toContain('Processing toolpaths')
  await ui.click('Cancel processing')
  finish(result()); await pending; await nextTick()
  expect(ui.all().some(node => node.tag === 'canvas')).toBe(false)
  service.run.mockImplementationOnce(() => new Promise(resolve => { finish = resolve }))
  const stale = ui.button('Generate toolpaths').props.onClick(); await nextTick()
  await ui.edit('Z max, mm', 13)
  finish(result()); await stale; await nextTick()
  expect(ui.all().some(node => node.tag === 'canvas')).toBe(false)
  await ui.click('Generate toolpaths')
  expect(ui.all().some(node => node.tag === 'canvas')).toBe(true)
})

it('shows validation and worker errors, clears obsolete downloads, and can retry', async () => {
  const ui = await mount()
  await ui.click('Generate toolpaths')
  await ui.edit('Wall count', 1.5)
  await ui.click('Generate toolpaths')
  expect(ui.text()).toContain('Wall count must be an integer')
  expect(ui.all().some(node => node.tag === 'canvas')).toBe(false)
  await ui.edit('Wall count', 2)
  service.run.mockRejectedValueOnce(new Error('The mesh is open.'))
  await ui.click('Generate toolpaths')
  expect(ui.text()).toContain('The mesh is open.')
  await ui.click('Generate toolpaths')
  expect(ui.text()).not.toContain('The mesh is open.')
  expect(ui.all().some(node => node.tag === 'canvas')).toBe(true)
})

it('opens its own preview dialect without a selected model and rejects oversized files before reading', async () => {
  const ui = await mount({ meshes: [], selection: [], ready: false })
  await ui.upload({ name: 'saved.gcode', size: 20, text: async () => '; saved preview' })
  expect(service.run).toHaveBeenCalledWith({ kind: 'parse', gcode: '; saved preview' })
  await ui.click('Download preview G-code')
  expect(service.download).toHaveBeenCalledWith(result().gcode, 'text/plain;charset=utf-8', 'saved.gcode')
  const read = vi.fn()
  await ui.upload({ name: 'large.gcode', size: GCODE_PREVIEW_MAX_BYTES + 1, text: read })
  expect(read).not.toHaveBeenCalled()
  expect(ui.text()).toContain('4 MiB')
  expect(ui.all().some(node => node.tag === 'canvas')).toBe(false)
})

it('does not start stale file parsing after the owner cancels while reading', async () => {
  const ui = await mount()
  let read!: (value: string) => void
  const upload = ui.input('Open preview G-code').props.onChange({ target: { files: [{ name: 'saved.gcode', size: 10, text: () => new Promise(resolve => { read = resolve }) }], value: 'file' } })
  await nextTick(); await ui.click('Cancel processing')
  read('; saved'); await upload; await nextTick()
  expect(service.run).not.toHaveBeenCalled()
  expect(ui.all().some(node => node.tag === 'canvas')).toBe(false)
})

it('cancels work when the section closes and disposes its worker when the CAD panel unmounts', async () => {
  const ui = await mount()
  let finish!: (document: GcodePreviewDocument) => void
  service.run.mockImplementationOnce(() => new Promise(resolve => { finish = resolve }))
  const pending = ui.button('Generate toolpaths').props.onClick(); await nextTick()
  ui.all().find(node => node.tag === 'details')!.props.onToggle({ target: { open: false } })
  await nextTick()
  expect(ui.text()).toContain('Processing cancelled.')
  finish(result()); await pending; await nextTick()
  expect(ui.all().some(node => node.tag === 'canvas')).toBe(false)
  service.run.mockImplementationOnce(() => new Promise(resolve => { finish = resolve }))
  const closing = ui.button('Generate toolpaths').props.onClick(); await nextTick()
  ui.unmount()
  expect(service.dispose).toHaveBeenCalledOnce()
  finish(result()); await closing; await nextTick()
  expect(service.draw).not.toHaveBeenCalled()
})
