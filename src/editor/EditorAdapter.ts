import { watch, type Ref } from 'svet'

/**
 * Engine-neutral seam between the app shell and the source editor.
 *
 * The app and its composables read and write the editor only through this
 * interface, so the current `<textarea>` overlay can later be swapped for
 * CodeMirror 6 or Monaco without touching call sites (ROADMAP P4).
 */
export type EditorSelectionDirection = 'forward' | 'backward' | 'none'

export interface EditorSelection {
  start: number
  end: number
  direction: EditorSelectionDirection
}

/** Where the selection lands after a programmatic replacement (DOM `setRangeText` semantics). */
export type EditorSelectMode = 'select' | 'start' | 'end' | 'preserve'

/** A source range the editor should mark, e.g. the span picked in the 3D scene. */
export interface EditorHighlightRange {
  start: number
  end: number
  kind: 'source' | 'find' | 'diagnostic'
}

export interface EditorScrollState {
  top: number
  left: number
  clientHeight: number
}

export type EditorCompositionPhase = 'start' | 'update' | 'end'

export interface EditorAdapter {
  /** True while an engine instance is mounted. Every method is a safe no-op otherwise. */
  readonly mounted: boolean
  /** The engine's root element, for layout observers. */
  element(): HTMLElement | null
  getText(): string
  /** Replaces the whole document as one programmatic edit. */
  setText(text: string, select?: EditorSelectMode): void
  /** Replaces `[start, end)` as one programmatic edit. */
  replaceRange(text: string, start: number, end: number, select?: EditorSelectMode): void
  getSelection(): EditorSelection
  setSelection(start: number, end: number, direction?: EditorSelectionDirection): void
  focus(options?: FocusOptions): void
  hasFocus(): boolean
  getScroll(): EditorScrollState
  setScroll(scroll: { top?: number; left?: number }): void
  /** Rendered line height in CSS pixels. */
  lineHeightPx(): number
  /** Engine-native undo/redo; returns false when the engine had nothing to apply. */
  undo(): boolean
  redo(): boolean
  /** Decorations for source↔scene navigation; the textarea engine keeps them as data only. */
  setHighlights(ranges: readonly EditorHighlightRange[]): void
  getHighlights(): readonly EditorHighlightRange[]
  /** True between compositionstart and compositionend (IME input in progress). */
  isComposing(): boolean
  onComposition(listener: (phase: EditorCompositionPhase, data: string) => void): () => void
}

/** The fields of DOM input/composition events the adapter reads. */
export interface EditorDomEvent {
  data?: string | null
}

/** The subset of `HTMLTextAreaElement` the textarea adapter relies on. */
export interface TextareaLike {
  value: string
  selectionStart: number
  selectionEnd: number
  selectionDirection: string | null
  scrollTop: number
  scrollLeft: number
  readonly clientHeight: number
  setRangeText(replacement: string, start: number, end: number, selectionMode?: EditorSelectMode): void
  setSelectionRange(start: number, end: number, direction?: EditorSelectionDirection): void
  focus(options?: FocusOptions): void
  addEventListener(type: string, listener: (event: EditorDomEvent) => void): void
  removeEventListener(type: string, listener: (event: EditorDomEvent) => void): void
  dispatchEvent?(event: Event): boolean
  readonly ownerDocument?: Document | null
  /** Flag read by Svet's `v-model` input handler to skip mid-composition updates. */
  composing?: boolean
}

export interface TextareaEditorAdapterOptions {
  /** Computes the rendered line height; injected so tests need no layout engine. */
  lineHeight?: (element: TextareaLike) => number
  /** Creates the trailing `input` event dispatched after compositionend. */
  createInputEvent?: () => Event
}

function defaultLineHeight(element: TextareaLike) {
  return parseFloat(getComputedStyle(element as unknown as Element).lineHeight) || 0
}

/**
 * Adapter over the current `<textarea>` editor. Behaviour matches direct DOM
 * access; the only addition is the composition guard, which makes the existing
 * `v-model` skip intermediate IME text and commit once on compositionend.
 */
export function createTextareaEditorAdapter(
  elementRef: Ref<TextareaLike | null>,
  options: TextareaEditorAdapterOptions = {},
): EditorAdapter {
  let highlights: readonly EditorHighlightRange[] = []
  let composing = false
  const listeners = new Set<(phase: EditorCompositionPhase, data: string) => void>()
  const emit = (phase: EditorCompositionPhase, data: string) => {
    for (const listener of listeners) listener(phase, data)
  }
  const onStart = (event: EditorDomEvent) => {
    composing = true
    const el = elementRef.value
    if (el) el.composing = true
    emit('start', event?.data ?? '')
  }
  const onUpdate = (event: EditorDomEvent) => emit('update', event?.data ?? '')
  const onEnd = (event: EditorDomEvent) => {
    if (!composing) return
    composing = false
    const el = elementRef.value
    if (el) {
      el.composing = false
      // Commit the composed text through the normal input path exactly once.
      const input = options.createInputEvent?.() ?? new Event('input', { bubbles: true })
      el.dispatchEvent?.(input)
    }
    emit('end', event?.data ?? '')
  }
  watch(
    elementRef,
    (el, previous) => {
      if (previous) {
        previous.removeEventListener('compositionstart', onStart)
        previous.removeEventListener('compositionupdate', onUpdate)
        previous.removeEventListener('compositionend', onEnd)
        previous.composing = false
      }
      composing = false
      if (el) {
        el.addEventListener('compositionstart', onStart)
        el.addEventListener('compositionupdate', onUpdate)
        el.addEventListener('compositionend', onEnd)
      }
    },
    { immediate: true, flush: 'sync' },
  )

  const exec = (command: 'undo' | 'redo') => {
    const el = elementRef.value
    const doc = el?.ownerDocument
    if (!el || !doc || typeof doc.execCommand !== 'function') return false
    el.focus({ preventScroll: true })
    return doc.execCommand(command)
  }

  return {
    get mounted() {
      return elementRef.value !== null
    },
    element: () => (elementRef.value as unknown as HTMLElement | null) ?? null,
    getText: () => elementRef.value?.value ?? '',
    setText(text, select = 'preserve') {
      const el = elementRef.value
      if (el) el.setRangeText(text, 0, el.value.length, select)
    },
    replaceRange(text, start, end, select = 'preserve') {
      elementRef.value?.setRangeText(text, start, end, select)
    },
    getSelection() {
      const el = elementRef.value
      if (!el) return { start: 0, end: 0, direction: 'none' }
      const direction = el.selectionDirection
      return {
        start: el.selectionStart,
        end: el.selectionEnd,
        direction: direction === 'forward' || direction === 'backward' ? direction : 'none',
      }
    },
    setSelection(start, end, direction) {
      const el = elementRef.value
      if (!el) return
      if (direction) el.setSelectionRange(start, end, direction)
      else el.setSelectionRange(start, end)
    },
    focus(focusOptions) {
      elementRef.value?.focus(focusOptions)
    },
    hasFocus() {
      const el = elementRef.value
      return Boolean(el && el.ownerDocument?.activeElement === (el as unknown as Element))
    },
    getScroll() {
      const el = elementRef.value
      return el
        ? { top: el.scrollTop, left: el.scrollLeft, clientHeight: el.clientHeight }
        : { top: 0, left: 0, clientHeight: 0 }
    },
    setScroll({ top, left }) {
      const el = elementRef.value
      if (!el) return
      if (top !== undefined) el.scrollTop = top
      if (left !== undefined) el.scrollLeft = left
    },
    lineHeightPx() {
      const el = elementRef.value
      return el ? (options.lineHeight ?? defaultLineHeight)(el) : 0
    },
    undo: () => exec('undo'),
    redo: () => exec('redo'),
    setHighlights(ranges) {
      highlights = ranges.slice()
    },
    getHighlights: () => highlights,
    isComposing: () => composing,
    onComposition(listener) {
      listeners.add(listener)
      return () => listeners.delete(listener)
    },
  }
}
