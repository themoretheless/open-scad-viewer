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

const modifierGuards: Record<string, (e: any, modifiers: string[]) => boolean | void> = {
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

export function withModifiers(fn: (...args: any[]) => any, modifiers: string[]): (event: any, ...args: any[]) => any {
  const cache = (fn as any)._withMods || ((fn as any)._withMods = {})
  const cacheKey = modifiers.join('.')
  return (
    cache[cacheKey] ||
    (cache[cacheKey] = (event: any, ...args: any[]) => {
      for (let i = 0; i < modifiers.length; i++) {
        const guard = modifierGuards[modifiers[i]]
        if (guard && guard(event, modifiers)) return
      }
      return fn(event, ...args)
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

export function withKeys(fn: (...args: any[]) => any, modifiers: string[]): (event: any) => any {
  return (event: any) => {
    if (!event || !('key' in event)) return
    const eventKey = String(event.key || '').toLowerCase()
    const hyphenKey = hyphenate(String(event.key || ''))
    const matched = modifiers.some(mod => {
      const m = mod.toLowerCase()
      if (m === eventKey || m === hyphenKey) return true
      const aliases = keyAliases[m]
      return aliases ? aliases.includes(eventKey) || aliases.includes(hyphenKey) : false
    })
    if (matched) return fn(event)
  }
}

export function withDirectives(vnode: SvetVNode, directives: Array<[Directive, any?, string?, Record<string, boolean>?]>): SvetVNode {
  const bindings: DirectiveBinding[] = vnode.dirs ?? (vnode.dirs = [])
  for (let i = 0; i < directives.length; i++) {
    const [dir, value, arg, modifiers = {}] = directives[i]
    if (!dir) continue
    const rawValue = isRef(value) ? value.value : value
    bindings.push({ dir, value: rawValue, oldValue: undefined, arg, modifiers })
  }
  return vnode
}

function toNumber(val: any): any {
  const n = parseFloat(val)
  return Number.isNaN(n) ? val : n
}

function syncModelUpdaterProp(el: any, vnode: SvetVNode, onlyIfMissing = false): void {
  const updater = vnode.props?.['onUpdate:modelValue']
  if (updater && el.props && (!onlyIfMissing || !('onUpdate:modelValue' in el.props))) {
    el.props['onUpdate:modelValue'] = updater
  }
}

function resolveOptionValue(opt: any, vnodeProps?: Record<string, any> | null): any {
  return '_value' in opt ? opt._value : ('value' in opt ? opt.value : (opt.props ?? vnodeProps)?.value)
}

export const vModelText: Directive = {
  created(el, { modifiers: { lazy, trim, number } }, vnode) {
    const updater = vnode.props?.['onUpdate:modelValue']
    if (!updater) return
    const castValue = (raw: any) => {
      let val = raw
      if (trim && typeof val === 'string') val = val.trim()
      if (number || vnode.props?.type === 'number' || vnode.props?.type === 'range') val = toNumber(val)
      return val
    }
    el._assign = (raw: any) => updater(castValue(raw))
    el.addEventListener?.(lazy ? 'change' : 'input', (e: any) => {
      if (e?.target?.composing) return
      el._assign(e?.target ? e.target.value : el.value)
    })
  },
  mounted(el, { value }, vnode) {
    syncModelUpdaterProp(el, vnode)
    el.value = value == null ? '' : value
  },
  beforeUpdate(el, { value, modifiers: { trim, number } }, vnode) {
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
  created(el, _binding, vnode) {
    const updater = vnode.props?.['onUpdate:modelValue']
    if (!updater) return
    el.addEventListener?.('change', (e: any) => {
      const checked = e?.target ? Boolean(e.target.checked) : Boolean(el.checked)
      updater(checked)
    })
  },
  mounted(el, { value }, vnode) {
    syncModelUpdaterProp(el, vnode)
    el.checked = Boolean(value)
  },
  beforeUpdate(el, { value }, vnode) {
    syncModelUpdaterProp(el, vnode)
    el.checked = Boolean(value)
  },
}

export const vModelRadio: Directive = {
  created(el, _binding, vnode) {
    const updater = vnode.props?.['onUpdate:modelValue']
    if (!updater) return
    el.addEventListener?.('change', () => {
      const radioVal = vnode.props && '_value' in el ? el._value : vnode.props?.value
      updater(radioVal)
    })
  },
  mounted(el, { value }, vnode) {
    syncModelUpdaterProp(el, vnode)
    const radioVal = '_value' in el ? el._value : vnode.props?.value
    el.checked = Object.is(value, radioVal)
  },
  beforeUpdate(el, { value }, vnode) {
    syncModelUpdaterProp(el, vnode)
    const radioVal = '_value' in el ? el._value : vnode.props?.value
    el.checked = Object.is(value, radioVal)
  },
}

function setSelected(el: any, value: any, number: boolean): void {
  const options = el.options
  if (!options) {
    el.value = value
    return
  }
  for (let i = 0; i < options.length; i++) {
    const opt = options[i]
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
  created(el, { modifiers: { number } }, vnode) {
    const updater = vnode.props?.['onUpdate:modelValue']
    if (!updater) return
    el.addEventListener?.('change', () => {
      const options = el.options
      if (options && options.length > 0) {
        const selectedOpt = Array.from(options as any[]).find((o: any) => o.selected) ?? options[el.selectedIndex]
        if (selectedOpt) {
          const rawVal = resolveOptionValue(selectedOpt)
          updater(number ? toNumber(rawVal) : rawVal)
          return
        }
      }
      updater(number ? toNumber(el.value) : el.value)
    })
  },
  mounted(el, { value, modifiers: { number } }, vnode) {
    syncModelUpdaterProp(el, vnode, true)
    setSelected(el, value, Boolean(number))
  },
  beforeUpdate(el, _binding, vnode) {
    syncModelUpdaterProp(el, vnode)
  },
  updated(el, { value, modifiers: { number } }) {
    setSelected(el, value, Boolean(number))
  },
}

export const vShow: Directive = {
  beforeMount(el, { value }) {
    if (!el.style) el.style = {}
    el._vod = el.style.display === 'none' ? '' : (el.style.display ?? '')
    el.style.display = value ? el._vod : 'none'
  },
  updated(el, { value, oldValue }) {
    if (!value === !oldValue) return
    if (!el.style) el.style = {}
    el.style.display = value ? (el._vod ?? '') : 'none'
  },
}
