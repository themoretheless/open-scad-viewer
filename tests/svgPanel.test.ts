import { createRenderer, h, nextTick, reactive } from 'vue'
import { afterEach, beforeEach, expect, it, vi } from 'vitest'
import SvgPanel from '../src/features/SvgPanel.vue'

const geometry = vi.hoisted(() => ({ preview: vi.fn(), contours: vi.fn(), exportContours: vi.fn(), extrude: vi.fn(), project: vi.fn(), run: vi.fn(), cancel: vi.fn(), dispose: vi.fn() }))
const storage = vi.hoisted(() => ({ load: vi.fn(), save: vi.fn() }))
vi.mock('../src/services/svgDraftStore', () => ({ svgDraftStore: () => storage }))
vi.mock('../src/services/svgWorkerClient', () => ({ createSvgWorkerClient: () => ({
  cancel: geometry.cancel, dispose: geometry.dispose,
  run: async (job: any, control: any) => {
    geometry.run(job, control)
    if (control.signal.aborted) throw new DOMException('Cancelled', 'AbortError')
    if (job.kind === 'preview') return geometry.preview(job.svg, job.options)
    if (job.kind === 'project') return geometry.preview(geometry.exportContours(await geometry.project(job.meshes, { axis: job.axis, ...(job.face ? { face: job.face } : {}) })), job.options)
    const profile = await geometry.contours(job.svg, job.options)
    return { ...profile, svg: geometry.exportContours(profile.contours), ...(job.kind === 'extrude' ? { source: geometry.extrude(profile.contours, job.height) } : {}) }
  },
}) }))

class Node {
  parent: Node | null = null
  children: Node[] = []
  props: Record<string, any> = {}
  text = ''
  value: any = ''
  selected = false
  constructor(public tag: string) {}
  get tagName() { return this.tag.toUpperCase() }
  get options() { return this.children.filter(child => child.tag === 'option') }
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
const unmounts: Array<() => void> = []
const blobs = new Map<string, Blob>()
const downloads: { href: string; download: string }[] = []
const rings = [[[0, 0], [10, 0], [10, 10], [0, 10]]]
const artwork = '<svg xmlns="http://www.w3.org/2000/svg"><defs><linearGradient id="g"/></defs><path fill="url(#g)"/></svg>'
const contourSvg = '<svg xmlns="http://www.w3.org/2000/svg"><path fill="black"/></svg>'
const result = { svg: artwork, widthMm: 40, heightMm: 30, warnings: [] as string[] }
const profileResult = { contours: rings, widthMm: 40, heightMm: 30, warnings: [] as string[] }

beforeEach(() => {
  vi.clearAllMocks()
  storage.load.mockReset().mockResolvedValue(null)
  storage.save.mockReset().mockResolvedValue(undefined)
  blobs.clear(); downloads.length = 0
  geometry.preview.mockReset().mockResolvedValue(result)
  geometry.contours.mockReset().mockResolvedValue(profileResult)
  geometry.exportContours.mockReset().mockReturnValue(contourSvg)
  geometry.extrude.mockReset().mockReturnValue('linear_extrude(height=5) polygon();')
  geometry.project.mockReset().mockResolvedValue(rings)
  vi.spyOn(URL, 'createObjectURL').mockImplementation(blob => { const url = `blob:svg-test-${blobs.size}`; blobs.set(url, blob as Blob); return url })
  vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => {})
  vi.stubGlobal('Document', class {})
  vi.stubGlobal('ShadowRoot', class {})
  vi.stubGlobal('document', { createElement: () => { const link = { href: '', download: '', click() { downloads.push({ href: this.href, download: this.download }) } }; return link }, activeElement: null })
})
afterEach(() => { unmounts.splice(0).forEach(unmount => unmount()); vi.restoreAllMocks(); vi.unstubAllGlobals(); vi.useRealTimers() })

async function mount(props: Record<string, unknown> = {}) {
  const root = new Node('root'), state = reactive({ meshes: [], hit: null, available: false, locale: 'en', ...props })
  const app = renderer.createApp({ render: () => h(SvgPanel, state as any) })
  app.mount(root)
  unmounts.push(() => app.unmount())
  await nextTick()
  const all = (node: Node = root): Node[] => [node, ...node.children.flatMap(all)]
  const text = (node: Node = root): string => node.text + node.children.map(child => text(child)).join('')
  const button = (name: string) => { const node = all().find(node => node.tag === 'button' && text(node) === name); expect(node, `button ${name}`).toBeDefined(); return node! }
  const input = (label: string) => { const node = all().find(node => node.props['aria-label'] === label); expect(node, `input ${label}`).toBeDefined(); return node! }
  const edit = async (label: string, value: unknown) => { input(label).props['onUpdate:modelValue'](value); await nextTick() }
  const click = async (name: string) => { const node = button(name); expect(node.props.disabled).toBeFalsy(); await node.props.onClick({}); await Promise.resolve(); await nextTick() }
  const upload = async (label: string, files: unknown[]) => { const node = input(label); await node.props.onChange({ target: { files, value: 'selected' } }); await nextTick() }
  return { all, text, button, input, edit, click, upload, setProps: async (values: Record<string, unknown>) => { Object.assign(state, values); await nextTick() }, unmount: () => app.unmount() }
}

it('previews and downloads validated artwork without losing its colors and effects', async () => {
  const ui = await mount()
  expect(ui.all().some(node => node.tag === 'img')).toBe(false)
  await ui.click('Preview')
  expect(geometry.preview).toHaveBeenCalledWith(expect.stringContaining('<rect'), expect.objectContaining({ dpi: 96, tolerance: 0.05 }))
  const image = ui.all().find(node => node.tag === 'img')!
  expect(image.props.src).toMatch(/^blob:/)
  expect(await blobs.get(image.props.src)!.text()).toBe(artwork)
  expect(ui.text()).toContain('40 × 30 mm')
  expect(geometry.contours).not.toHaveBeenCalled()
  await ui.click('Download artwork SVG')
  expect(downloads).toHaveLength(1)
  expect(downloads[0].download).toBe('profile.svg')
  expect(await blobs.get(downloads[0].href)!.text()).toBe(artwork)
  expect(geometry.contours).not.toHaveBeenCalled()
})

it('exports CAD contours separately while retaining the artwork preview', async () => {
  const ui = await mount()
  await ui.click('Preview')
  const previous = ui.all().find(node => node.tag === 'img')!.props.src
  await ui.click('Download contours SVG')
  expect(geometry.exportContours).toHaveBeenCalledWith(rings)
  expect(downloads[0].download).toBe('profile-contours.svg')
  expect(await blobs.get(downloads[0].href)!.text()).toBe(contourSvg)
  expect(ui.all().find(node => node.tag === 'img')!.props.src).toBe(previous)
})

it('passes physical scale, curve quality, silhouette settings and extrusion height to geometry', async () => {
  const append = vi.fn(), ui = await mount({ onAppend: append })
  await ui.edit('DPI', 144)
  await ui.edit('Tolerance, mm', 0.02)
  await ui.edit('3D profile', 'silhouette')
  expect(ui.text()).toContain('pixel approximation')
  await ui.edit('Resolution, px', 1024)
  await ui.edit('Opacity threshold', 0.3)
  await ui.edit('Height, mm', 7)
  await ui.click('Add extrusion to model')
  expect(geometry.contours).toHaveBeenCalledWith(expect.any(String), { dpi: 144, tolerance: 0.02, fonts: [], geometryMode: 'silhouette', rasterSize: 1024, alphaThreshold: 0.3 })
  expect(geometry.extrude).toHaveBeenCalledWith(rings, 7)
  expect(append).toHaveBeenCalledWith('linear_extrude(height=5) polygon();')
})

it.each(['Preview', 'Download artwork SVG', 'Download contours SVG', 'Add extrusion to model'])('discards a stale %s result after markup is edited', async action => {
  let resolve!: (value: any) => void
  const pending = new Promise(resolveResult => { resolve = resolveResult })
  if (action.includes('contours') || action.includes('extrusion')) geometry.contours.mockReturnValueOnce(pending)
  else geometry.preview.mockReturnValueOnce(pending)
  const append = vi.fn(), ui = await mount({ onAppend: append })
  const operation = ui.click(action)
  await nextTick()
  expect(ui.button('Preview').props.disabled).toBe(true)
  await ui.edit('SVG', '<svg><circle r="2"/></svg>')
  resolve(action.includes('contours') || action.includes('extrusion') ? profileResult : result)
  await operation
  expect(ui.all().some(node => node.tag === 'img')).toBe(false)
  expect(downloads).toHaveLength(0)
  expect(append).not.toHaveBeenCalled()
  expect(ui.input('SVG').value).toContain('<circle')
  expect(ui.button('Preview').props.disabled).toBe(false)
})

it('shows validation errors without inserting or previewing unvalidated SVG', async () => {
  const ui = await mount()
  await ui.click('Preview')
  const url = ui.all().find(node => node.tag === 'img')!.props.src
  await ui.edit('SVG', '<svg onload="alert(1)"><script>alert(1)</script></svg>')
  expect(URL.revokeObjectURL).toHaveBeenCalledWith(url)
  geometry.preview.mockRejectedValueOnce(new Error('Active SVG content is not allowed.'))
  await ui.click('Preview')
  expect(ui.all().some(node => node.tag === 'img')).toBe(false)
  expect(ui.all().some(node => node.props.innerHTML)).toBe(false)
  expect(ui.all().find(node => node.props.role === 'alert')?.text).toBe('Active SVG content is not allowed.')
  expect(downloads).toHaveLength(0)
})

it('loads a validated file, retains its filename and enforces the upload size limit', async () => {
  const ui = await mount()
  const source = '<svg><circle r="2"/></svg>'
  await ui.upload('Open SVG', [{ size: source.length, name: 'Знак.svg', text: async () => source }])
  expect(ui.input('SVG').value).toBe(source)
  expect(ui.text()).toContain('Знак.svg')
  await ui.click('Download artwork SVG')
  expect(downloads[0].download).toBe('Знак.svg')
  const read = vi.fn()
  await ui.upload('Open SVG', [{ size: 4 * 1024 * 1024 + 1, name: 'large.svg', text: read }])
  expect(read).not.toHaveBeenCalled()
  expect(ui.all().find(node => node.props.role === 'alert')?.text).toContain('4 MiB')
  expect(ui.input('SVG').value).toBe(source)
})

it('loads local fonts for both preview and extrusion, and rejects oversized font files', async () => {
  const ui = await mount()
  await ui.upload('Load fonts', [{ name: 'Brand.ttf', size: 3, arrayBuffer: async () => new Uint8Array([1, 2, 3]).buffer }])
  expect(ui.text()).toContain('Brand.ttf')
  expect(geometry.preview).toHaveBeenLastCalledWith(expect.any(String), expect.objectContaining({ fonts: [new Uint8Array([1, 2, 3])] }))
  await ui.click('Add extrusion to model')
  expect(geometry.contours).toHaveBeenLastCalledWith(expect.any(String), expect.objectContaining({ fonts: [new Uint8Array([1, 2, 3])] }))
  const read = vi.fn()
  await ui.upload('Load fonts', [{ name: 'TooLarge.ttf', size: 4 * 1024 * 1024 + 1, arrayBuffer: read }])
  expect(read).not.toHaveBeenCalled()
  expect(ui.all().find(node => node.props.role === 'alert')?.text).toContain('4 MiB')
  await ui.click('Clear fonts')
  expect(ui.text()).not.toContain('Brand.ttf')
})

it('exports model projections and selected faces without replacing the artwork until explicitly opened', async () => {
  const meshes = [{}], hit = { meshIndex: 0, triangleIndex: 7 }, ui = await mount({ meshes, hit, available: true })
  const original = ui.input('SVG').value
  geometry.preview.mockResolvedValue({ ...result, svg: contourSvg })
  await ui.edit('Projection axis', 'x')
  await ui.click('Model projection → SVG')
  expect(geometry.project).toHaveBeenLastCalledWith(meshes, { axis: 'x' })
  expect(ui.input('SVG').value).toBe(original)
  expect(downloads[0].download).toBe('model-x.svg')
  await ui.click('Selected face → SVG')
  expect(geometry.project).toHaveBeenLastCalledWith(meshes, { axis: 'x', face: hit })
  expect(downloads[1].download).toBe('selected-face.svg')
  await ui.click('Open exported SVG in editor')
  expect(ui.input('SVG').value).toBe(contourSvg)
})

it('disables exports without a model or selection and releases the preview URL on unmount', async () => {
  const ui = await mount()
  expect(ui.button('Model projection → SVG').props.disabled).toBe(true)
  expect(ui.button('Selected face → SVG').props.disabled).toBe(true)
  await ui.click('Preview')
  const url = ui.all().find(node => node.tag === 'img')!.props.src
  ui.unmount()
  expect(URL.revokeObjectURL).toHaveBeenCalledWith(url)
})

it('provides reusable artwork and text examples with Russian interface copy', async () => {
  const ui = await mount({ locale: 'ru' })
  await ui.click('Стили и обрезка')
  expect(ui.input('SVG').value).toContain('<style>')
  expect(ui.input('SVG').value).toContain('<use href=')
  expect(ui.input('SVG').value).toContain('<clipPath')
  expect(geometry.preview).toHaveBeenCalled()
  await ui.click('Текст')
  expect(ui.input('SVG').value).toContain('<text')
  expect(ui.text()).toContain('Скачать рисунок SVG')
})


it('surfaces conversion warnings even when users go straight to extrusion or contour export', async () => {
  const ui = await mount()
  geometry.contours.mockResolvedValue({ ...profileResult, warnings: ['Font Brand was replaced with the default font.'] })
  await ui.click('Add extrusion to model')
  expect(ui.text()).toContain('Font Brand was replaced with the default font.')
  expect(ui.text()).toContain('40 × 30 mm')
  geometry.contours.mockResolvedValue({ ...profileResult, warnings: ['Silhouette is a pixel approximation.'] })
  await ui.click('Download contours SVG')
  expect(ui.text()).toContain('Silhouette is a pixel approximation.')
  expect(ui.text()).not.toContain('Font Brand was replaced')
})

it('does not overwrite an edit while an uploaded file is being validated', async () => {
  let resolve!: (value: typeof result) => void
  geometry.preview.mockReturnValueOnce(new Promise(resolveResult => { resolve = resolveResult }))
  const ui = await mount()
  const operation = ui.upload('Open SVG', [{ size: 10, name: 'old.svg', text: async () => '<svg/>' }])
  await nextTick()
  await ui.edit('SVG', '<svg><path id="new"/></svg>')
  resolve(result)
  await operation
  expect(ui.input('SVG').value).toContain('id="new"')
  expect(ui.text()).not.toContain('old.svg')
  expect(ui.all().some(node => node.tag === 'img')).toBe(false)
})


it('exposes the SVG tool scroller as a named keyboard-focusable region', async () => {
  const ui = await mount()
  const scroller = ui.all().find(node => node.props.role === 'region' && node.props['aria-label'] === 'SVG tools')
  expect(scroller).toBeDefined()
  expect(scroller!.props.tabindex).toBe('0')
  expect(scroller!.children.some(node => node.tag === 'textarea')).toBe(true)
})


it('prevents SCAD append to a different document format while keeping SVG export available', async () => {
  const append = vi.fn(), ui = await mount({ canAppend: false, onAppend: append })
  expect(ui.button('Add extrusion to model').props.disabled).toBe(true)
  expect(ui.text()).toContain('Adding an extrusion requires an OpenSCAD (.scad) document.')
  await ui.click('Download contours SVG')
  expect(downloads).toHaveLength(1)
  expect(append).not.toHaveBeenCalled()
})


it('cancels a running conversion, keeps the current source and allows another preview immediately', async () => {
  let resolve!: (value: typeof result) => void
  geometry.preview.mockReturnValueOnce(new Promise(done => { resolve = done }))
  const ui = await mount(), operation = ui.click('Preview')
  await nextTick()
  const signal = geometry.run.mock.calls[0][1].signal as AbortSignal
  await ui.click('Cancel')
  expect(signal.aborted).toBe(true)
  expect(ui.button('Preview').props.disabled).toBe(false)
  expect(ui.text()).toContain('SVG processing cancelled.')
  await ui.click('Preview')
  resolve({ ...result, svg: 'stale' }); await operation
  const image = ui.all().find(node => node.tag === 'img')!
  expect(await blobs.get(image.props.src)!.text()).toBe(artwork)
})

it('rejects oversized extrusion output against the destination document budget', async () => {
  const append = vi.fn(), ui = await mount({ remainingSource: 4, onAppend: append })
  await ui.click('Add extrusion to model')
  expect(append).not.toHaveBeenCalled()
  expect(ui.all().find(node => node.props.role === 'alert')?.text).toContain('remaining document space')
})

it('does not append an extrusion after the destination SCAD document changes', async () => {
  let resolve!: (value: typeof profileResult) => void
  geometry.contours.mockReturnValueOnce(new Promise(done => { resolve = done }))
  const append = vi.fn(), ui = await mount({ appendRevision: 'document-a:1', onAppend: append })
  const operation = ui.click('Add extrusion to model'); await nextTick()
  const signal = geometry.run.mock.calls[0][1].signal as AbortSignal
  await ui.setProps({ appendRevision: 'document-b:1' })
  expect(signal.aborted).toBe(true)
  resolve(profileResult); await operation
  expect(append).not.toHaveBeenCalled()
})

it.each(['meshes', 'hit', 'available'])('cancels pending model export when %s changes', async field => {
  let resolve!: (value: typeof rings) => void
  geometry.project.mockReturnValueOnce(new Promise(done => { resolve = done }))
  const ui = await mount({ meshes: [{}], hit: { meshIndex: 0, triangleIndex: 1 }, available: true })
  const operation = ui.click('Selected face → SVG'); await nextTick()
  const signal = geometry.run.mock.calls[0][1].signal as AbortSignal
  await ui.setProps({ [field]: field === 'meshes' ? [{ changed: true }] : field === 'hit' ? null : false })
  expect(signal.aborted).toBe(true)
  resolve(rings); await operation
  expect(downloads).toHaveLength(0)
})

it('restores source and settings without rendering unvalidated saved markup', async () => {
  const saved = { version: 1, text: '<svg onload="untrusted draft"/>', filename: 'saved.svg', height: 7, axis: 'y', dpi: 144, tolerance: 0.01, geometryMode: 'silhouette', rasterSize: 1024, alphaThreshold: 0.3, fonts: [{ name: 'Saved.ttf', bytes: new Uint8Array([4, 5]) }], opened: true }
  storage.load.mockResolvedValueOnce(saved)
  const ui = await mount()
  expect(ui.input('SVG').value).toBe(saved.text)
  expect(ui.input('DPI').value).toBe(144)
  expect(ui.input('Resolution, px').value).toBe(1024)
  expect(ui.text()).toContain('Saved.ttf')
  expect(ui.all().some(node => node.tag === 'img')).toBe(false)
  expect(geometry.run).not.toHaveBeenCalled()
  ui.unmount()
  expect(storage.save).not.toHaveBeenCalled()
})

it('does not apply a late restored draft over a user preview request', async () => {
  let resolve!: (value: unknown) => void
  storage.load.mockReturnValueOnce(new Promise(done => { resolve = done }))
  const ui = await mount()
  const original = ui.input('SVG').value
  await ui.click('Preview')
  resolve({ version: 1, text: 'old draft' }); await Promise.resolve(); await nextTick()
  expect(ui.input('SVG').value).toBe(original)
  expect(ui.all().some(node => node.tag === 'img')).toBe(true)
})

it('persists edited text and fonts as a cloneable snapshot on unmount', async () => {
  const ui = await mount()
  await ui.edit('SVG', '<svg><path')
  await ui.upload('Load fonts', [{ name: 'Local.ttf', size: 2, arrayBuffer: async () => new Uint8Array([7, 8]).buffer }])
  ui.unmount()
  expect(storage.save).toHaveBeenCalled()
  const saved = storage.save.mock.calls.at(-1)![0]
  expect(structuredClone(saved)).toMatchObject({ text: '<svg><path', fonts: [{ name: 'Local.ttf', bytes: new Uint8Array([7, 8]) }] })
  expect(geometry.dispose).toHaveBeenCalled()
})

it('reports durable save failures instead of implying the SVG survived a reload', async () => {
  vi.useFakeTimers()
  storage.save.mockRejectedValue(new Error('Storage quota exceeded. Download your SVG.'))
  const ui = await mount()
  await ui.edit('SVG', '<svg/>')
  await vi.advanceTimersByTimeAsync(500)
  expect(ui.text()).toContain('Storage quota exceeded. Download your SVG.')
})


it.each(['Height, mm', 'Projection axis'])('cancels work if the active %s setting changes', async label => {
  let resolve!: (value: any) => void
  const wait = new Promise(done => { resolve = done })
  if (label === 'Height, mm') geometry.contours.mockReturnValueOnce(wait)
  else geometry.project.mockReturnValueOnce(wait)
  const append = vi.fn(), ui = await mount({ meshes: [{}], available: true, onAppend: append })
  const task = ui.click(label === 'Height, mm' ? 'Add extrusion to model' : 'Model projection → SVG')
  await nextTick()
  const signal = geometry.run.mock.calls[0][1].signal as AbortSignal
  await ui.edit(label, label === 'Height, mm' ? 7 : 'x')
  expect(signal.aborted).toBe(true)
  resolve(label === 'Height, mm' ? profileResult : rings); await task
  expect(append).not.toHaveBeenCalled(); expect(downloads).toHaveLength(0)
})

it('saves an unfinished numeric input together with the newest artwork', async () => {
  const ui = await mount()
  await ui.edit('SVG', '<svg><new artwork')
  await ui.edit('DPI', '')
  ui.unmount()
  expect(storage.save.mock.calls.at(-1)![0]).toMatchObject({ text: '<svg><new artwork', dpi: '' })
})


it('reports preview publication failures after committing newly opened SVG text', async () => {
  const ui = await mount()
  vi.mocked(URL.createObjectURL).mockImplementationOnce(() => { throw new Error('Preview allocation failed') })
  await ui.upload('Open SVG', [{ size: 6, name: 'new.svg', text: async () => '<svg/>' }])
  expect(ui.input('SVG').value).toBe('<svg/>')
  expect(ui.all().find(node => node.props.role === 'alert')?.text).toContain('Preview allocation failed')
})
