import { computed, nextTick, ref } from 'svet'
import { describe, expect, it, vi } from 'vitest'
import type { SourceProvenanceRow } from '../src/components/cadPanels.types'
import type { MeshData } from '../src/core/mesh'
import { useEditorFind } from '../src/composables/useEditorFind'
import { useEditorFormatting } from '../src/composables/useEditorFormatting'
import { useSourceProvenanceCaret } from '../src/composables/useSourceProvenanceCaret'
import { createTextareaEditorAdapter, type EditorSelectMode, type TextareaLike } from '../src/editor/EditorAdapter'
import { EditHistory } from '../src/editor/editHistory'
import { vModelText } from '../src/forma/svet-directives'
import type { WebGPURenderer } from '../src/render/webgpuRenderer'

/** Minimal textarea with DOM `setRangeText` selection semantics and event dispatch. */
class FakeTextarea implements TextareaLike {
  value = ''
  selectionStart = 0
  selectionEnd = 0
  selectionDirection: string | null = 'none'
  scrollTop = 0
  scrollLeft = 0
  clientHeight = 400
  composing?: boolean
  focused = false
  private listeners = new Map<string, Set<(event: Record<string, unknown>) => void>>()
  setRangeText(text: string, start: number, end: number, mode: EditorSelectMode = 'preserve') {
    const oldStart = this.selectionStart
    const oldEnd = this.selectionEnd
    this.value = this.value.slice(0, start) + text + this.value.slice(end)
    const newEnd = start + text.length
    if (mode === 'select') [this.selectionStart, this.selectionEnd] = [start, newEnd]
    else if (mode === 'start') [this.selectionStart, this.selectionEnd] = [start, start]
    else if (mode === 'end') [this.selectionStart, this.selectionEnd] = [newEnd, newEnd]
    else {
      const delta = text.length - (end - start)
      const map = (p: number) => (p > end ? p + delta : p > start ? start : p)
      this.selectionStart = Math.min(map(oldStart), this.value.length)
      this.selectionEnd = Math.min(map(oldEnd), this.value.length)
    }
  }
  setSelectionRange(start: number, end: number, direction: 'forward' | 'backward' | 'none' = 'none') {
    this.selectionStart = start
    this.selectionEnd = end
    this.selectionDirection = direction
  }
  focus() {
    this.focused = true
  }
  addEventListener(type: string, listener: (event: Record<string, unknown>) => void) {
    if (!this.listeners.has(type)) this.listeners.set(type, new Set())
    this.listeners.get(type)!.add(listener)
  }
  removeEventListener(type: string, listener: (event: Record<string, unknown>) => void) {
    this.listeners.get(type)?.delete(listener)
  }
  dispatchEvent(event: Event) {
    this.fire(event.type, {})
    return true
  }
  fire(type: string, init: Record<string, unknown>) {
    for (const listener of this.listeners.get(type) ?? []) listener({ type, target: this, ...init })
  }
  /** A user keystroke: the browser mutates the value, then fires `input`. */
  typeText(text: string) {
    this.setRangeText(text, this.selectionStart, this.selectionEnd, 'end')
    this.fire('input', { data: text })
  }
}

function mount(initial = '') {
  const el = new FakeTextarea()
  el.value = initial
  const elementRef = ref<FakeTextarea | null>(el)
  const adapter = createTextareaEditorAdapter(elementRef, {
    lineHeight: () => 20,
    createInputEvent: () => ({ type: 'input' }) as Event,
  })
  return { el, elementRef, adapter }
}

/** Binds the real Svet `v-model` directive to the fake textarea, as App.svt does. */
function bindModel(el: FakeTextarea, code: { value: string }, onInput: () => void) {
  type Created = (el: unknown, binding: unknown, vnode: unknown, prev: unknown) => void
  const vnode = { props: { 'onUpdate:modelValue': (v: string) => (code.value = v) } }
  ;(vModelText.created as Created)(el, { modifiers: {} }, vnode, null)
  el.addEventListener('input', onInput)
}

describe('textarea editor adapter', () => {
  it('reads and writes text, selection and scroll like direct textarea access', () => {
    const { el, adapter } = mount('cube(1);')
    expect(adapter.mounted).toBe(true)
    adapter.setSelection(0, 4, 'backward')
    expect(adapter.getSelection()).toEqual({ start: 0, end: 4, direction: 'backward' })
    adapter.replaceRange('sphere', 0, 4, 'select')
    expect(adapter.getText()).toBe('sphere(1);')
    expect(adapter.getSelection()).toMatchObject({ start: 0, end: 6 })
    adapter.setText('x', 'start')
    expect(el.value).toBe('x')
    adapter.setScroll({ top: 40, left: 3 })
    expect(adapter.getScroll()).toEqual({ top: 40, left: 3, clientHeight: 400 })
    expect(adapter.lineHeightPx()).toBe(20)
    adapter.focus()
    expect(el.focused).toBe(true)
  })

  it('is a safe no-op while unmounted', () => {
    const { elementRef, adapter } = mount('a')
    elementRef.value = null
    expect(adapter.mounted).toBe(false)
    expect(adapter.getText()).toBe('')
    expect(() => adapter.setText('b')).not.toThrow()
    expect(adapter.getSelection()).toEqual({ start: 0, end: 0, direction: 'none' })
    expect(adapter.undo()).toBe(false)
  })

  it('keeps highlight decorations as data', () => {
    const { adapter } = mount('cube(1);')
    adapter.setHighlights([{ start: 0, end: 4, kind: 'source' }])
    expect(adapter.getHighlights()).toEqual([{ start: 0, end: 4, kind: 'source' }])
  })
})

describe('IME composition', () => {
  it('commits composed text once, on compositionend, not mid-composition', () => {
    const { el, adapter } = mount('a = ;')
    const code = ref(el.value)
    const committed: string[] = []
    bindModel(el, code, () => {
      if (!adapter.isComposing()) committed.push(code.value)
    })
    const phases: string[] = []
    adapter.onComposition(phase => phases.push(phase))
    el.setSelectionRange(4, 4)
    el.fire('compositionstart', { data: '' })
    expect(adapter.isComposing()).toBe(true)
    for (const step of ['n', 'に', 'にほ']) {
      el.setRangeText(step, 4, 4 + (el.value.length - 5), 'end')
      el.fire('compositionupdate', { data: step })
      el.fire('input', { data: step, isComposing: true })
    }
    expect(code.value).toBe('a = ;')
    expect(committed).toEqual([])
    el.setRangeText('日本', 4, 6, 'end')
    el.fire('compositionend', { data: '日本' })
    expect(adapter.isComposing()).toBe(false)
    expect(code.value).toBe('a = 日本;')
    expect(committed).toEqual(['a = 日本;'])
    expect(phases).toEqual(['start', 'update', 'update', 'update', 'end'])
  })

  it('detaches composition listeners when the element is replaced', () => {
    const { el, elementRef, adapter } = mount('x')
    elementRef.value = null
    el.fire('compositionstart', {})
    expect(adapter.isComposing()).toBe(false)
  })
})

describe('find and replace through the adapter', () => {
  it('seeds the query from the selection and replaces as one programmatic edit', async () => {
    const { el, adapter } = mount('cube(1); cube(2);')
    const code = ref(el.value)
    const onEdited = vi.fn()
    const find = useEditorFind({
      code,
      editor: adapter,
      input: ref(null),
      translate: k => k,
      notify: () => {},
      onEdited,
    })
    adapter.setSelection(0, 4)
    find.openEditorFind(true)
    expect(find.findQuery.value).toBe('cube')
    await nextTick()
    find.findReplacement.value = 'sphere'
    find.navigateFind(1)
    expect(adapter.getSelection()).toMatchObject({ start: 9, end: 13 })
    find.replaceCurrentFindMatch()
    expect(code.value).toBe('cube(1); sphere(2);')
    expect(onEdited).toHaveBeenCalledTimes(1)
    find.replaceAllFindMatches()
    expect(code.value).toBe('sphere(1); sphere(2);')
    expect(el.value).toBe(code.value)
  })
})

describe('formatting through the adapter', () => {
  it('indents the selection with Tab and keeps the model in sync', () => {
    const { el, adapter } = mount('a;\nb;')
    const code = ref(el.value)
    const handleEditorInput = vi.fn()
    const { handleEditorKey } = useEditorFormatting({
      code,
      editor: adapter,
      foldedLines: ref(new Set<number>()),
      syncHighlightScroll: () => {},
      handleEditorInput,
      dispatchKeyboardCommand: () => {},
    })
    adapter.setSelection(0, 4)
    handleEditorKey({
      key: 'Tab',
      shiftKey: false,
      ctrlKey: false,
      metaKey: false,
      altKey: false,
      isComposing: false,
      preventDefault() {},
    } as KeyboardEvent)
    expect(code.value).toBe(el.value)
    expect(code.value.split('\n').every(line => /^\s+/.test(line))).toBe(true)
    expect(handleEditorInput).toHaveBeenCalledTimes(1)
  })

  it('does not indent while an IME composition is open', () => {
    const { el, adapter } = mount('a;')
    const code = ref(el.value)
    const dispatch = vi.fn()
    const { handleEditorKey } = useEditorFormatting({
      code,
      editor: adapter,
      foldedLines: ref(new Set<number>()),
      syncHighlightScroll: () => {},
      handleEditorInput: () => {},
      dispatchKeyboardCommand: dispatch,
    })
    handleEditorKey({ key: 'Tab', isComposing: true, preventDefault() {} } as KeyboardEvent)
    expect(code.value).toBe('a;')
    expect(dispatch).toHaveBeenCalledTimes(1)
  })
})

describe('source ↔ scene navigation through the adapter', () => {
  const source = 'union() { cube(1); sphere(2); }'
  const cubeStart = source.indexOf('cube')
  const sphereStart = source.indexOf('sphere')
  const mesh = (id: number, start: number, end: number) =>
    ({ provenance: [{ source: { id, start, end } }] }) as unknown as MeshData

  function setup() {
    const { el, adapter } = mount(source)
    const setSourceHighlight = vi.fn()
    const renderer = { setSourceHighlight } as unknown as WebGPURenderer
    const meshes = [
      mesh(1, 0, source.length),
      mesh(2, cubeStart, cubeStart + 8),
      mesh(3, sphereStart, sphereStart + 10),
    ]
    const caret = useSourceProvenanceCaret({
      code: ref(source),
      editor: adapter,
      selectedEditorName: ref(''),
      sceneMeshes: computed(() => meshes),
      sourceMatchesEditor: computed(() => true),
      getRenderer: () => renderer,
      showNotice: () => {},
      translate: k => k,
    })
    return { el, adapter, caret, setSourceHighlight }
  }

  it('highlights the innermost scene span under the editor caret', () => {
    const { adapter, caret, setSourceHighlight } = setup()
    adapter.setSelection(sphereStart + 2, sphereStart + 2)
    caret.syncSourceHighlightFromEditor()
    expect(setSourceHighlight).toHaveBeenLastCalledWith(3)
    adapter.setSelection(2, 2)
    caret.syncSourceHighlightFromEditor()
    expect(setSourceHighlight).toHaveBeenLastCalledWith(1)
  })

  it('reveals a scene pick as an editor selection and decoration', () => {
    const { el, adapter, caret, setSourceHighlight } = setup()
    caret.revealSource({ sourceId: 2, sourceStart: cubeStart, sourceEnd: cubeStart + 8 } as SourceProvenanceRow)
    expect(el.focused).toBe(true)
    expect(adapter.getSelection()).toEqual({ start: cubeStart, end: cubeStart + 8, direction: 'forward' })
    expect(adapter.getHighlights()).toEqual([{ start: cubeStart, end: cubeStart + 8, kind: 'source' }])
    expect(setSourceHighlight).toHaveBeenLastCalledWith(2)
  })
})

describe('undo granularity contract', () => {
  it('coalesces adjacent typing within the window into one step', () => {
    const h = new EditHistory('')
    'cube'.split('').forEach((ch, i) => h.type(i, i, ch, i * 100))
    expect(h.value).toBe('cube')
    expect(h.undoDepth).toBe(1)
    h.type(4, 4, '(', 2000)
    expect(h.undoDepth).toBe(2)
    h.undo()
    expect(h.value).toBe('cube')
    h.undo()
    expect(h.value).toBe('')
    h.redo()
    h.redo()
    expect(h.value).toBe('cube(')
  })

  it('splits typing at newlines and non-adjacent positions', () => {
    const h = new EditHistory('ab')
    h.type(2, 2, 'c', 0)
    h.type(0, 0, 'x', 10)
    h.type(1, 1, '\n', 20)
    expect(h.undoDepth).toBe(3)
  })

  it('records each programmatic edit as exactly one step and clears redo', () => {
    const h = new EditHistory('cube(1);')
    h.type(8, 8, ' ', 0)
    h.replace(0, h.value.length, 'cube(2);', 10)
    h.replace(5, 6, '3', 11)
    expect(h.undoDepth).toBe(3)
    h.undo()
    expect(h.value).toBe('cube(2);')
    h.replace(0, 0, '//', 12)
    expect(h.redoDepth).toBe(0)
  })

  it('records an IME composition as one step on compositionend', () => {
    const h = new EditHistory('a = ;')
    h.compositionStart()
    h.type(4, 4, 'n', 0)
    h.type(4, 5, 'に', 1)
    h.type(4, 5, '日本', 2)
    expect(h.undoDepth).toBe(0)
    h.compositionEnd(3)
    expect(h.undoDepth).toBe(1)
    h.undo()
    expect(h.value).toBe('a = ;')
    h.redo()
    expect(h.value).toBe('a = 日本;')
  })
})
