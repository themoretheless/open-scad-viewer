import { afterEach, describe, expect, it, vi } from 'vitest'
import { createRenderer, h, nextTick, shallowReactive } from 'svet'
import CommandActions from '../src/components/CommandActions.svt'

class El {
  parent: El | null = null
  children: El[] = []
  props: Record<string, any> = {} // eslint-disable-line @typescript-eslint/no-explicit-any -- host props hold handlers and attributes
  text = ''
  constructor(public tag: string) {}
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
  querySelector: () => null,
  setScopeId: () => {},
  insertStaticContent: () => {
    throw new Error('Unexpected static content')
  },
})

const mounts: (() => void)[] = []
afterEach(() => mounts.splice(0).forEach(unmount => unmount()))

const all = (node: El): El[] => [node, ...node.children.flatMap(all)]
const text = (node: El): string => node.text + node.children.map(text).join('')
const key = (name: string, extra: Partial<KeyboardEvent> = {}) => ({
  key: name,
  isComposing: false,
  preventDefault: vi.fn(),
  ...extra,
})

async function mount(initial: { ready: boolean; reason?: string; locale?: string; handleKeys?: boolean }) {
  const props = shallowReactive({ locale: 'en', ...initial })
  const onApply = vi.fn()
  const onCancel = vi.fn()
  const root = new El('root')
  const app = renderer.createApp({ setup: () => () => h(CommandActions, { ...props, onApply, onCancel }) })
  app.mount(root)
  mounts.push(() => app.unmount())
  await nextTick()
  const buttons = () => all(root).filter(node => node.tag === 'button')
  const group = () => all(root).find(node => node.props.role === 'group')!
  return { root, props, onApply, onCancel, buttons, group }
}

describe('CommandActions', () => {
  it('renders one Apply and one Cancel and emits on click', async () => {
    const ui = await mount({ ready: true })
    const [apply, cancel] = ui.buttons()
    expect(ui.buttons()).toHaveLength(2)
    expect(text(apply)).toBe('Apply · Enter')
    expect(text(cancel)).toBe('Cancel · Esc')
    expect(apply.props.disabled).toBe(false)
    apply.props.onClick()
    cancel.props.onClick()
    expect(ui.onApply).toHaveBeenCalledTimes(1)
    expect(ui.onCancel).toHaveBeenCalledTimes(1)
  })

  it('explains a disabled Apply with visible text linked by aria-describedby', async () => {
    const ui = await mount({ ready: false, reason: 'Select cutting bodies B.' })
    const apply = ui.buttons()[0]
    expect(apply.props.disabled).toBe(true)
    expect(apply.props.title).toBe('Select cutting bodies B.')
    const hint = all(ui.root).find(node => node.props.id === apply.props['aria-describedby'])!
    expect(text(hint)).toBe('Select cutting bodies B.')
    ui.props.ready = true
    await nextTick()
    expect(ui.buttons()[0].props['aria-describedby']).toBeUndefined()
    expect(all(ui.root).some(node => node.props.class === 'command-blocked')).toBe(false)
  })

  it('maps Enter to apply and Escape to cancel when it owns the keys', async () => {
    const ui = await mount({ ready: false, reason: 'Wait', handleKeys: true })
    const enter = key('Enter')
    ui.group().props.onKeydown(enter)
    expect(ui.onApply).not.toHaveBeenCalled()
    ui.props.ready = true
    await nextTick()
    ui.group().props.onKeydown(key('Enter', { isComposing: true }))
    expect(ui.onApply).not.toHaveBeenCalled()
    ui.group().props.onKeydown(key('Enter'))
    expect(ui.onApply).toHaveBeenCalledTimes(1)
    const escape = key('Escape')
    ui.group().props.onKeydown(escape)
    expect(ui.onCancel).toHaveBeenCalledTimes(1)
    expect(escape.preventDefault).toHaveBeenCalled()
  })

  it('leaves keys to the host by default and localizes labels', async () => {
    const ui = await mount({ ready: true, locale: 'ru' })
    ui.group().props.onKeydown(key('Enter'))
    ui.group().props.onKeydown(key('Escape'))
    expect(ui.onApply).not.toHaveBeenCalled()
    expect(ui.onCancel).not.toHaveBeenCalled()
    expect(ui.buttons().map(text)).toEqual(['Готово · Enter', 'Отмена · Esc'])
  })
})
