import { afterEach, expect, it, vi } from 'vitest'
import { createRenderer, nextTick } from 'vue'
import VrControls from '../src/components/VrControls.vue'
import type { MeshData } from '../src/core/mesh'

class Node {
  parent: Node | null = null
  children: Node[] = []
  props: Record<string, any> = {}
  text = ''
  constructor(public tag: string) {}
}
const renderer = createRenderer<Node, Node>({
  createElement: tag => new Node(tag),
  createText: text => Object.assign(new Node('#text'), { text }),
  createComment: () => new Node('#comment'),
  setText: (n, text) => { n.text = text },
  setElementText: (n, text) => { n.children = []; n.text = text },
  parentNode: n => n.parent,
  nextSibling: n => n.parent?.children[n.parent.children.indexOf(n) + 1] ?? null,
  patchProp: (n, key, _old, value) => { n.props[key] = value },
  insert: (n, parent, anchor) => {
    n.parent = parent
    const i = anchor ? parent.children.indexOf(anchor) : -1
    if (i < 0) parent.children.push(n); else parent.children.splice(i, 0, n)
  },
  remove: n => { if (n.parent) n.parent.children = n.parent.children.filter(child => child !== n) },
  setScopeId: () => {},
  insertStaticContent: () => { throw new Error('Unexpected static content') },
})
const mounts: (() => void)[] = []
afterEach(() => { mounts.splice(0).forEach(unmount => unmount()); vi.unstubAllGlobals() })
async function mount(props: Record<string, unknown>) {
  vi.stubGlobal('navigator', { xr: Object.assign(new EventTarget(), { isSessionSupported: async () => true }) })
  vi.stubGlobal('window', { isSecureContext: true })
  const root = new Node('root')
  const app = renderer.createApp(VrControls, { locale: 'en', ...props })
  app.mount(root); mounts.push(() => app.unmount())
  await Promise.resolve(); await nextTick()
  const all = (n: Node = root): Node[] => [n, ...n.children.flatMap(all)]
  all().find(n => n.props['aria-label'] === 'VR mode')!.props.onClick()
  await nextTick()
  const text = (n: Node): string => n.text + n.children.map(text).join('')
  return all().find(n => n.tag === 'button' && text(n) === 'Enter VR')!
}
const mesh = { indices: new Uint32Array([0, 1, 2]) } as MeshData
it('enables Source VR for visible geometry when available is omitted', async () => {
  expect((await mount({ meshes: [mesh], visibility: [true], isolated: false })).props.disabled).toBe(false)
})
it('disables entry for hidden geometry or an empty source scene', async () => {
  expect((await mount({ meshes: [mesh], visibility: [false] })).props.disabled).toBe(true)
  expect((await mount({ meshes: [] })).props.disabled).toBe(true)
})
it('respects explicit workspace availability', async () => {
  expect((await mount({ available: true, getScene: () => [] })).props.disabled).toBe(false)
  expect((await mount({ available: false, meshes: [mesh] })).props.disabled).toBe(true)
})
