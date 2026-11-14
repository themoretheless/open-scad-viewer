import { afterEach, describe, expect, it, vi } from 'vitest'
import { createRenderer, h, nextTick, shallowReactive } from 'svet'
import ViewCube from '../src/components/ViewCube.svt'
import KeyboardShortcuts from '../src/components/KeyboardShortcuts.svt'
import ScanPlanePanel from '../src/features/ScanPlanePanel.svt'
import {
  KEYBOARD_ORBIT_PIXELS,
  KEYBOARD_PAN_PIXELS,
  KEYBOARD_ZOOM_PIXELS,
  resolveKeyboardCameraStep,
} from '../src/render/keyboardCamera'

/** Minimal host element: enough to read roles, names and ARIA state from rendered components. */
class El {
  parent: El | null = null
  children: El[] = []
  props: Record<string, unknown> & { [key: string]: any } = {} // eslint-disable-line @typescript-eslint/no-explicit-any -- host props hold handlers and attributes
  text = ''
  style: Record<string, string> = {}
  constructor(public tag: string) {}
  get tabIndex() {
    return this.props.tabindex === undefined
      ? this.tag === 'button' || this.tag === 'input'
        ? 0
        : -1
      : Number(this.props.tabindex)
  }
  focus() {
    doc.activeElement = this
  }
  contains(node: El | null) {
    for (let current = node; current; current = current.parent) if (current === this) return true
    return false
  }
  matches() {
    return false
  }
  getClientRects() {
    return [{}]
  }
  querySelectorAll() {
    const all = (node: El): El[] => node.children.flatMap(child => [child, ...all(child)])
    return all(this).filter(node => node.tag === 'button' || node.tag === 'input' || 'tabindex' in node.props)
  }
}

const body = Object.assign(new El('body'), { style: { overflow: '' } })
const doc = {
  body,
  activeElement: body as El | null,
  hidden: false,
  addEventListener: () => {},
  removeEventListener: () => {},
}
const renderer = createRenderer<El, El>({
  createElement: tag => new El(tag),
  createText: text => Object.assign(new El('#text'), { text }),
  createComment: () => new El('#comment'),
  setText: (node, text) => {
    node.text = text
  },
  setElementText: (node, text) => {
    node.children = []
    node.text = text
  },
  parentNode: node => node.parent,
  nextSibling: node => node.parent?.children[node.parent.children.indexOf(node) + 1] ?? null,
  patchProp: (node, key, _old, value) => {
    node.props[key] = value
  },
  insert: (node, parent, anchor) => {
    node.parent = parent
    const index = anchor ? parent.children.indexOf(anchor) : -1
    if (index < 0) parent.children.push(node)
    else parent.children.splice(index, 0, node)
  },
  remove: node => {
    if (node.parent) node.parent.children = node.parent.children.filter(child => child !== node)
    node.parent = null
  },
  querySelector: selector => (selector === 'body' ? body : null),
  setScopeId: () => {},
  insertStaticContent: () => {
    throw new Error('Unexpected static content')
  },
})

const mounts: (() => void)[] = []
afterEach(() => {
  mounts.splice(0).forEach(unmount => unmount())
  body.children = []
  doc.activeElement = body
  vi.unstubAllGlobals()
})

const all = (root: El = body): El[] => root.children.flatMap(child => [child, ...all(child)])
const textOf = (node: El): string => node.text + node.children.map(textOf).join('')

async function mount(component: unknown, props: Record<string, unknown>) {
  vi.stubGlobal('document', doc)
  vi.stubGlobal('requestAnimationFrame', () => 0)
  vi.stubGlobal('cancelAnimationFrame', () => {})
  const state = shallowReactive({ ...props })
  const host = new El('div')
  body.children.push(host)
  host.parent = body
  const app = renderer.createApp({ setup: () => () => h(component as Parameters<typeof h>[0], { ...state }) })
  app.mount(host)
  mounts.push(() => app.unmount())
  for (let i = 0; i < 4; i++) await nextTick()
  return state
}

/** Every interactive host element must carry an accessible name (aria-label, labelledby or text). */
function expectNamedControls(nodes: El[]) {
  for (const node of nodes) {
    const name = node.props['aria-label'] ?? node.props['aria-labelledby'] ?? textOf(node).trim()
    expect(name, `${node.tag} ${String(node.props.class ?? '')}`).toBeTruthy()
  }
}

describe('keyboard camera navigation', () => {
  it('maps arrows to orbit, Shift+arrows to pan and +/- to zoom', () => {
    expect(resolveKeyboardCameraStep({ key: 'ArrowLeft' })).toEqual({
      kind: 'orbit',
      dx: -KEYBOARD_ORBIT_PIXELS,
      dy: 0,
    })
    expect(resolveKeyboardCameraStep({ key: 'ArrowDown' })).toEqual({ kind: 'orbit', dx: 0, dy: KEYBOARD_ORBIT_PIXELS })
    expect(resolveKeyboardCameraStep({ key: 'ArrowUp', shiftKey: true })).toEqual({
      kind: 'pan',
      dx: 0,
      dy: -KEYBOARD_PAN_PIXELS,
    })
    expect(resolveKeyboardCameraStep({ key: '+' })).toEqual({ kind: 'zoom', delta: -KEYBOARD_ZOOM_PIXELS })
    expect(resolveKeyboardCameraStep({ key: '=' })).toEqual({ kind: 'zoom', delta: -KEYBOARD_ZOOM_PIXELS })
    expect(resolveKeyboardCameraStep({ key: '-' })).toEqual({ kind: 'zoom', delta: KEYBOARD_ZOOM_PIXELS })
    expect(resolveKeyboardCameraStep({ key: 'Subtract', code: 'NumpadSubtract' })).toEqual({
      kind: 'zoom',
      delta: KEYBOARD_ZOOM_PIXELS,
    })
  })

  it('leaves modified keys and unrelated keys to the command registry', () => {
    expect(resolveKeyboardCameraStep({ key: '=', ctrlKey: true })).toBeNull()
    expect(resolveKeyboardCameraStep({ key: '=', metaKey: true })).toBeNull()
    expect(resolveKeyboardCameraStep({ key: 'ArrowLeft', altKey: true })).toBeNull()
    expect(resolveKeyboardCameraStep({ key: 'f' })).toBeNull()
  })
})

describe('ViewCube semantics', () => {
  it('names every preset, exposes the active one with aria-pressed and announces the view', async () => {
    await mount(ViewCube, { activeView: 'front' })
    const group = all().find(node => node.props.role === 'group')!
    expect(group.props['aria-label']).toMatch(/View presets/)
    expect(all().find(node => node.props.id === group.props['aria-describedby'])).toBeDefined()
    const buttons = all().filter(node => node.props.role === 'button')
    expect(buttons).toHaveLength(7)
    for (const button of buttons) {
      expect(button.props['aria-label']).toBeTruthy()
      expect(button.props.tabindex).toBe('0')
    }
    expect(
      buttons.filter(button => button.props['aria-pressed'] === true).map(button => button.props['aria-label']),
    ).toEqual(['Front / Спереди'])
    const status = all().find(node => node.props.role === 'status')!
    expect(textOf(status)).toMatch(/Front/)
  })

  it('reports a custom orbit with no preset pressed', async () => {
    await mount(ViewCube, { activeView: 'custom' })
    expect(all().filter(node => node.props['aria-pressed'] === true)).toHaveLength(0)
    expect(textOf(all().find(node => node.props.role === 'status')!)).toMatch(/custom/)
  })

  it('emits a stepped orbit for arrow keys on a focused preset', async () => {
    const onOrbitStep = vi.fn()
    await mount(ViewCube, { activeView: 'iso', yaw: 0, pitch: 0, onOrbitStep })
    const group = all().find(node => node.props.role === 'group')!
    group.props.onKeydown({ key: 'ArrowRight', shiftKey: false, preventDefault: vi.fn() })
    expect(onOrbitStep).toHaveBeenCalledWith(expect.objectContaining({ stepDegrees: 15, direction: 'right' }))
  })
})

describe('ScanPlanePanel semantics', () => {
  it('is a labelled, described non-modal dialog with named controls', async () => {
    await mount(ScanPlanePanel, {
      enabled: true,
      axis: 'y',
      offset: 0,
      min: -5,
      max: 5,
      step: 0.1,
      flip: false,
      locale: 'en',
      available: true,
    })
    const dialog = all().find(node => node.props.role === 'dialog')!
    expect(dialog.props['aria-modal']).toBe('false')
    expect(textOf(all().find(node => node.props.id === dialog.props['aria-labelledby'])!)).toBe('Scanning plane')
    expect(all().find(node => node.props.id === dialog.props['aria-describedby'])).toBeDefined()
    expectNamedControls(all().filter(node => node.tag === 'button'))
    const play = all().find(node => node.props.class === 'play')!
    expect(play.props['aria-pressed']).toBe(false)
  })
})

describe('KeyboardShortcuts camera help', () => {
  it('documents keyboard orbit, pan and zoom', async () => {
    await mount(KeyboardShortcuts, { open: true, groups: [], locale: 'en' })
    const dialog = all().find(node => node.props.role === 'dialog')!
    expect(dialog.props['aria-modal']).toBe('true')
    const camera = all().find(node => node.props['data-group'] === 'camera')!
    const text = textOf(camera)
    expect(text).toMatch(/Orbit.*Arrows/)
    expect(text).toMatch(/Pan.*Shift\+Arrows/)
    expect(text).toMatch(/Zoom in \/ out.*\+/)
  })
})
