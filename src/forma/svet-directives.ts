/**
 * # Svet Directives & Event Modifiers (`svet-directives.ts`)
 *
 * Pure Svet implementation of template directives (`vModelText`, `vModelCheckbox`,
 * `vModelRadio`, `vModelSelect`, `vShow`) and event modifier helpers (`withModifiers`,
 * `withKeys`, `withDirectives`).
 */
import { hyphenate } from './svet-props'
import { isRef } from './svet-reactivity'
import type { Directive, DirectiveBinding, SvetVNode } from './svet-vnode'

/** Loosely typed event: guards must tolerate plain objects and partial events in tests and SSR. */
type GuardEvent = Partial<MouseEvent & KeyboardEvent> & Record<string, unknown>

/** Host element as the model directives see it: a real form control or a test double with the same fields. */
interface ModelElement {
  value?: unknown
  checked?: boolean
  _value?: unknown
  _vod?: string
  _assign?: (raw: unknown) => void
  props?: Record<string, unknown>
  options?: ArrayLike<ModelOption>
  selectedIndex?: number
  style?: { display?: string }
  addEventListener?: (type: string, listener: (e: ModelEvent) => void) => void
}
interface ModelOption {
  selected?: boolean
  _value?: unknown
  value?: unknown
  props?: Record<string, unknown> | null
}
type ModelEvent = { target?: { composing?: boolean; value?: unknown; checked?: unknown } | null } | null | undefined
type Handler = (...args: never[]) => unknown
type EventHandler = (event: unknown, ...args: unknown[]) => unknown
type ModelUpdater = (value: unknown) => void

const modifierGuards: Record<string, (e: GuardEvent | null | undefined, modifiers: string[]) => unknown> = {
  stop: e => e?.stopPropagation?.(),
  prevent: e => e?.preventDefault?.(),
  self: e => e && e.target !== undefined && e.currentTarget !== undefined && e.target !== e.currentTarget,
  ctrl: e => !e?.ctrlKey,
  shift: e => !e?.shiftKey,
  alt: e => !e?.altKey,
  meta: e => !e?.metaKey,
  left: e => e && 'button' in e && e.button !== 0,
  middle: e => e && 'button' in e && e.button !== 1,
  right: e => e && 'button' in e && e.button !== 2,
  exact: (e, modifiers) => ['ctrl', 'shift', 'alt', 'meta'].some(m => e?.[`${m}Key`] && !modifiers.includes(m)),
}

export function withModifiers(fn: Handler, modifiers: string[]): EventHandler {
  const cached = fn as Handler & { _withMods?: Record<string, EventHandler> }
  const cache = cached._withMods || (cached._withMods = {})
  const cacheKey = modifiers.join('.')
  return (
    cache[cacheKey] ||
    (cache[cacheKey] = (event: unknown, ...args: unknown[]) => {
      for (let i = 0; i < modifiers.length; i++) {
        const guard = modifierGuards[modifiers[i]!]
        if (guard && guard(event as GuardEvent | null | undefined, modifiers)) return
      }
      return (fn as unknown as EventHandler)(event, ...args)
    })
  )
}

const keyAliases: Record<string, string[]> = {
  esc: ['escape', 'esc'],
  escape: ['escape', 'esc'],
  space: [' ', 'spacebar', 'space'],
  up: ['arrowup', 'up', 'arrow-up'],
  left: ['arrowleft', 'left', 'arrow-left'],
  right: ['arrowright', 'right', 'arrow-right'],
  down: ['arrowdown', 'down', 'arrow-down'],
  delete: ['backspace', 'delete', 'del'],
}

export function withKeys(fn: Handler, modifiers: string[]): (event: unknown) => unknown {
  return (rawEvent: unknown) => {
    const event = rawEvent as { key?: unknown } | null | undefined
    if (!event || !('key' in event)) return
    const eventKey = String(event.key || '').toLowerCase()
    const hyphenKey = hyphenate(String(event.key || ''))
    const matched = modifiers.some(mod => {
      const m = mod.toLowerCase()
      if (m === eventKey || m === hyphenKey) return true
      const aliases = keyAliases[m]
      return aliases ? aliases.includes(eventKey) || aliases.includes(hyphenKey) : false
    })
    if (matched) return (fn as unknown as EventHandler)(event)
    return undefined
  }
}

export function withDirectives(
  vnode: SvetVNode,
  directives: Array<[Directive, unknown?, string?, Record<string, boolean>?]>,
): SvetVNode {
  const bindings: DirectiveBinding[] = vnode.dirs ?? (vnode.dirs = [])
  for (let i = 0; i < directives.length; i++) {
    const [dir, value, arg, modifiers = {}] = directives[i]!
    if (!dir) continue
    const rawValue = isRef(value) ? value.value : value
    bindings.push({ dir, value: rawValue, oldValue: undefined, arg, modifiers })
  }
  return vnode
}

function toNumber(val: unknown): unknown {
  const n = parseFloat(val as string)
  return Number.isNaN(n) ? val : n
}

function syncModelUpdaterProp(el: ModelElement, vnode: SvetVNode, onlyIfMissing = false): void {
  const updater = vnode.props?.['onUpdate:modelValue']
  if (updater && el.props && (!onlyIfMissing || !('onUpdate:modelValue' in el.props))) {
    el.props['onUpdate:modelValue'] = updater
  }
}

function resolveOptionValue(opt: ModelOption, vnodeProps?: Record<string, unknown> | null): unknown {
  return '_value' in opt ? opt._value : 'value' in opt ? opt.value : (opt.props ?? vnodeProps)?.value
}

export const vModelText: Directive = {
  created(rawEl, { modifiers: { lazy, trim, number } }, vnode) {
    const el = rawEl as ModelElement
    const updater = vnode.props?.['onUpdate:modelValue'] as ModelUpdater | undefined
    if (!updater) return
    const castValue = (raw: unknown) => {
      let val = raw
      if (trim && typeof val === 'string') val = val.trim()
      if (number || vnode.props?.type === 'number' || vnode.props?.type === 'range') val = toNumber(val)
      return val
    }
    el._assign = (raw: unknown) => updater(castValue(raw))
    el.addEventListener?.(lazy ? 'change' : 'input', (e: ModelEvent) => {
      if (e?.target?.composing) return
      el._assign?.(e?.target ? e.target.value : el.value)
    })
  },
  mounted(rawEl, { value }, vnode) {
    const el = rawEl as ModelElement
    syncModelUpdaterProp(el, vnode)
    el.value = value == null ? '' : value
  },
  beforeUpdate(rawEl, { value, modifiers: { trim, number } }, vnode) {
    const el = rawEl as ModelElement
    syncModelUpdaterProp(el, vnode)
    const newVal = value == null ? '' : value
    if (el.value !== newVal) {
      if (trim && typeof el.value === 'string' && el.value.trim() === newVal) return
      if (number && toNumber(el.value) === value) return
      el.value = newVal
    }
  },
}

export const vModelCheckbox: Directive = {
  created(rawEl, _binding, vnode) {
    const el = rawEl as ModelElement
    const updater = vnode.props?.['onUpdate:modelValue'] as ModelUpdater | undefined
    if (!updater) return
    el.addEventListener?.('change', (e: ModelEvent) => {
      const checked = e?.target ? Boolean(e.target.checked) : Boolean(el.checked)
      updater(checked)
    })
  },
  mounted(rawEl, { value }, vnode) {
    const el = rawEl as ModelElement
    syncModelUpdaterProp(el, vnode)
    el.checked = Boolean(value)
  },
  beforeUpdate(rawEl, { value }, vnode) {
    const el = rawEl as ModelElement
    syncModelUpdaterProp(el, vnode)
    el.checked = Boolean(value)
  },
}

export const vModelRadio: Directive = {
  created(rawEl, _binding, vnode) {
    const el = rawEl as ModelElement
    const updater = vnode.props?.['onUpdate:modelValue'] as ModelUpdater | undefined
    if (!updater) return
    el.addEventListener?.('change', () => {
      const radioVal = vnode.props && '_value' in el ? el._value : vnode.props?.value
      updater(radioVal)
    })
  },
  mounted(rawEl, { value }, vnode) {
    const el = rawEl as ModelElement
    syncModelUpdaterProp(el, vnode)
    const radioVal = '_value' in el ? el._value : vnode.props?.value
    el.checked = Object.is(value, radioVal)
  },
  beforeUpdate(rawEl, { value }, vnode) {
    const el = rawEl as ModelElement
    syncModelUpdaterProp(el, vnode)
    const radioVal = '_value' in el ? el._value : vnode.props?.value
    el.checked = Object.is(value, radioVal)
  },
}

function setSelected(el: ModelElement, value: unknown, number: boolean): void {
  const options = el.options
  if (!options) {
    el.value = value
    return
  }
  for (let i = 0; i < options.length; i++) {
    const opt = options[i]!
    const rawOptVal = resolveOptionValue(opt)
    const optVal = number ? toNumber(rawOptVal) : rawOptVal
    const isSelected = Object.is(optVal, value) || (optVal != null && value != null && String(optVal) === String(value))
    opt.selected = isSelected
    if (isSelected) {
      el.selectedIndex = i
      el.value = optVal
      return
    }
  }
  el.value = value
}

export const vModelSelect: Directive = {
  created(rawEl, { modifiers: { number } }, vnode) {
    const el = rawEl as ModelElement
    const updater = vnode.props?.['onUpdate:modelValue'] as ModelUpdater | undefined
    if (!updater) return
    el.addEventListener?.('change', () => {
      const options = el.options
      if (options && options.length > 0) {
        const selectedOpt = Array.from(options).find(o => o.selected) ?? options[el.selectedIndex ?? -1]
        if (selectedOpt) {
          const rawVal = resolveOptionValue(selectedOpt)
          updater(number ? toNumber(rawVal) : rawVal)
          return
        }
      }
      updater(number ? toNumber(el.value) : el.value)
    })
  },
  mounted(rawEl, { value, modifiers: { number } }, vnode) {
    const el = rawEl as ModelElement
    syncModelUpdaterProp(el, vnode, true)
    setSelected(el, value, Boolean(number))
  },
  beforeUpdate(rawEl, _binding, vnode) {
    const el = rawEl as ModelElement
    syncModelUpdaterProp(el, vnode)
  },
  updated(rawEl, { value, modifiers: { number } }) {
    const el = rawEl as ModelElement
    setSelected(el, value, Boolean(number))
  },
}

export const vShow: Directive = {
  beforeMount(rawEl, { value }) {
    const el = rawEl as ModelElement
    if (!el.style) el.style = {}
    el._vod = el.style.display === 'none' ? '' : (el.style.display ?? '')
    el.style.display = value ? el._vod : 'none'
  },
  updated(rawEl, { value, oldValue }) {
    const el = rawEl as ModelElement
    if (!value === !oldValue) return
    if (!el.style) el.style = {}
    el.style.display = value ? (el._vod ?? '') : 'none'
  },
}
