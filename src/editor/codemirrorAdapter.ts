import type { Ref } from 'svet'
import type { EditorAdapter, EditorCompositionPhase, EditorScrollState } from './EditorAdapter'
import type { CodeMirrorEngineCore } from './codemirrorCore'

/**
 * A mounted CodeMirror engine: the headless core plus the DOM-facing parts of
 * `EditorView`. `codemirrorView.ts` builds it in the lazily loaded chunk; tests
 * build one over `createHeadlessHost`.
 */
export interface CodeMirrorEngine extends CodeMirrorEngineCore {
  element(): HTMLElement | null
  focus(options?: FocusOptions): void
  hasFocus(): boolean
  getScroll(): EditorScrollState
  setScroll(scroll: { top?: number; left?: number }): void
  lineHeightPx(): number
  isComposing(): boolean
  onComposition(listener: (phase: EditorCompositionPhase, data: string) => void): () => void
  destroy(): void
}

/**
 * `EditorAdapter` over CodeMirror 6. This module carries no runtime CodeMirror
 * import, so the app shell can create the adapter eagerly while the engine
 * chunk loads on demand; until `engineRef` is set every method is a no-op.
 */
export function createCodeMirrorEditorAdapter(engineRef: Ref<CodeMirrorEngine | null>): EditorAdapter {
  const listeners = new Set<(phase: EditorCompositionPhase, data: string) => void>()
  let detach: (() => void) | null = null
  let attached: CodeMirrorEngine | null = null
  const sync = () => {
    const engine = engineRef.value
    if (engine === attached) return engine
    detach?.()
    detach = null
    attached = engine
    if (engine) detach = engine.onComposition((phase, data) => listeners.forEach(l => l(phase, data)))
    return engine
  }
  const with_ = <T>(fallback: T, run: (engine: CodeMirrorEngine) => T): T => {
    const engine = sync()
    return engine ? run(engine) : fallback
  }
  return {
    get mounted() {
      return sync() !== null
    },
    element: () => with_(null, e => e.element()),
    getText: () => with_('', e => e.getText()),
    setText: (text, select) => with_(undefined, e => e.setText(text, select)),
    replaceRange: (text, start, end, select) => with_(undefined, e => e.replaceRange(text, start, end, select)),
    getSelection: () => with_({ start: 0, end: 0, direction: 'none' as const }, e => e.getSelection()),
    setSelection: (start, end, direction) => with_(undefined, e => e.setSelection(start, end, direction)),
    focus: options => with_(undefined, e => e.focus(options)),
    hasFocus: () => with_(false, e => e.hasFocus()),
    getScroll: () => with_({ top: 0, left: 0, clientHeight: 0 }, e => e.getScroll()),
    setScroll: scroll => with_(undefined, e => e.setScroll(scroll)),
    lineHeightPx: () => with_(0, e => e.lineHeightPx()),
    undo: () => with_(false, e => e.undo()),
    redo: () => with_(false, e => e.redo()),
    setHighlights: ranges => with_(undefined, e => e.setHighlights(ranges)),
    getHighlights: () => with_([], e => e.getHighlights()),
    isComposing: () => with_(false, e => e.isComposing()),
    onComposition(listener) {
      sync()
      listeners.add(listener)
      return () => listeners.delete(listener)
    },
  }
}

/** Which engine backs the source editor. `?editor=textarea` (or the stored flag) selects the fallback. */
export type EditorEngineKind = 'codemirror' | 'textarea'

export function resolveEditorEngine(search: string, stored: string | null): EditorEngineKind {
  const param = new URLSearchParams(search).get('editor')
  const choice = param ?? stored
  return choice === 'textarea' ? 'textarea' : 'codemirror'
}
