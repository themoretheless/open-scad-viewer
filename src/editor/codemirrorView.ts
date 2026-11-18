import { Compartment, EditorState, type Extension, type Text } from '@codemirror/state'
import {
  Decoration,
  EditorView,
  drawSelection,
  highlightActiveLine,
  highlightActiveLineGutter,
  keymap,
  lineNumbers,
  ViewPlugin,
  type ViewUpdate,
  type DecorationSet,
} from '@codemirror/view'
import { defaultKeymap, historyKeymap } from '@codemirror/commands'
import {
  HighlightStyle,
  StreamLanguage,
  foldGutter,
  foldService,
  syntaxHighlighting,
  type StringStream,
} from '@codemirror/language'
import { tags } from '@lezer/highlight'
import { editorBlocks, type EditorBlock } from '../language/editorBlocks'
import type { EditorCompositionPhase } from './EditorAdapter'
import type { CodeMirrorEngine } from './codemirrorAdapter'
import { coreExtensions, createEngineCore, highlightsField, isProgrammatic } from './codemirrorCore'

/**
 * Mounted CodeMirror 6 view for the source editor. Loaded lazily by the app
 * shell, so none of `@codemirror/*` lands in the entry chunk.
 */

const KEYWORDS = new Set([
  'match',
  'foreach',
  'yield',
  'continue',
  'break',
  'fn',
  'return',
  'ret',
  'struct',
  'trait',
  'impl',
  'type',
  'Self',
  'with',
  'int',
  'f32',
  'f64',
  'str',
  'Geometry',
  'in',
  'by',
  'count',
  'where',
  'validate',
  'param',
  'range',
  'show',
  'module',
  'function',
  'let',
  'each',
  'for',
  'if',
  'else',
  'assert',
  'echo',
  'include',
  'use',
  'true',
  'false',
  'undef',
])

interface ScadStreamState {
  inBlockComment: boolean
}

/**
 * Incremental OpenSCAD / Rush tokenizer with the same lexical classes as
 * `highlightCode` (comment, string, property, number, keyword, function,
 * operator). StreamLanguage re-tokenizes only from the edited line onward and
 * only for the visible range, instead of the whole document per keystroke.
 */
export const scadRushLanguage = StreamLanguage.define<ScadStreamState>({
  name: 'scad-rush',
  startState: () => ({ inBlockComment: false }),
  copyState: state => ({ ...state }),
  token(stream: StringStream, state: ScadStreamState) {
    if (state.inBlockComment) {
      if (stream.skipTo('*/')) {
        stream.pos += 2
        state.inBlockComment = false
      } else stream.skipToEnd()
      return 'comment'
    }
    if (stream.eatSpace()) return null
    if (stream.match('//')) {
      stream.skipToEnd()
      return 'comment'
    }
    if (stream.match('/*')) {
      state.inBlockComment = true
      if (stream.skipTo('*/')) {
        stream.pos += 2
        state.inBlockComment = false
      } else stream.skipToEnd()
      return 'comment'
    }
    if (stream.peek() === '"') {
      stream.next()
      let escaped = false
      for (let ch = stream.next(); ch !== undefined; ch = stream.next()) {
        if (!escaped && ch === '"') break
        escaped = !escaped && ch === '\\'
      }
      return stream.match(/^\s*:/, false) ? 'propertyName' : 'string'
    }
    if (stream.match(/^(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?(?:mm|cm|in|deg|rad|m)?\b/)) return 'number'
    const word = stream.match(/^\$?[A-Za-z_][\w$]*/) as RegExpMatchArray | null
    if (word) {
      if (KEYWORDS.has(word[0])) return 'keyword'
      if (stream.match(/^\s*\(/, false)) return 'function'
      if (stream.match(/^\s*=(?!=)/, false)) return 'propertyName'
      return null
    }
    if (stream.match(/^(?:\.\.<|\.\.|[+*/%=!<>?:&|^-]+)/)) return 'operator'
    stream.next()
    return null
  },
  tokenTable: { function: tags.function(tags.variableName) },
  languageData: { commentTokens: { line: '//', block: { open: '/*', close: '*/' } } },
})

/** Maps highlight tags onto the existing `syntax-*` classes, themed by CSS variables in App.scoped.css. */
const scadHighlightStyle = HighlightStyle.define([
  { tag: tags.comment, class: 'syntax-comment' },
  { tag: tags.keyword, class: 'syntax-keyword' },
  { tag: tags.string, class: 'syntax-string' },
  { tag: tags.number, class: 'syntax-number' },
  { tag: tags.function(tags.variableName), class: 'syntax-function' },
  { tag: tags.propertyName, class: 'syntax-property' },
  { tag: tags.operator, class: 'syntax-operator' },
])

const blocksByDoc = new WeakMap<Text, Map<number, EditorBlock>>()
function blockStarts(state: EditorState) {
  let starts = blocksByDoc.get(state.doc)
  if (!starts) {
    starts = new Map(editorBlocks(state.doc.toString()).map(block => [block.start, block]))
    blocksByDoc.set(state.doc, starts)
  }
  return starts
}

/** Folds the same brace/indent blocks the textarea gutter offered. */
const scadFolding = foldService.of((state, lineStart) => {
  const line = state.doc.lineAt(lineStart)
  const block = blockStarts(state).get(line.number - 1)
  if (!block || block.end + 1 > state.doc.lines) return null
  const end = state.doc.line(block.end + 1)
  return end.from > line.to ? { from: line.to, to: end.to } : null
})

const highlightMarks = {
  source: Decoration.mark({ class: 'cm-source-highlight' }),
  find: Decoration.mark({ class: 'cm-find-highlight' }),
  diagnostic: Decoration.mark({ class: 'cm-diagnostic-highlight' }),
}

const highlightDecorations = EditorView.decorations.compute([highlightsField], (state): DecorationSet => {
  const ranges = state.field(highlightsField)
  const length = state.doc.length
  const marks = ranges
    .map(range => ({ ...range, start: Math.min(range.start, length), end: Math.min(range.end, length) }))
    .filter(range => range.end > range.start)
    .sort((a, b) => a.start - b.start || a.end - b.end)
    .map(range => highlightMarks[range.kind].range(range.start, range.end))
  return Decoration.set(marks, true)
})

const selectionMatchMark = Decoration.mark({ class: 'cm-selectionMatch' })
const WORD_CHAR = /[\w$]/

/**
 * Highlights other whole-word occurrences of a selected word (2+ chars) in the
 * visible ranges. Replaces `@codemirror/search`'s `highlightSelectionMatches`
 * so that package stays out of the editor chunk; find itself is app-owned.
 */
const selectionMatches = ViewPlugin.fromClass(
  class {
    decorations: DecorationSet
    constructor(view: EditorView) {
      this.decorations = this.build(view)
    }
    update(update: ViewUpdate) {
      if (update.docChanged || update.selectionSet || update.viewportChanged) this.decorations = this.build(update.view)
    }
    build(view: EditorView): DecorationSet {
      const { state } = view
      const sel = state.selection
      if (sel.ranges.length > 1) return Decoration.none
      const { from, to } = sel.main
      const query = state.sliceDoc(from, to)
      if (query.length < 2 || query.length > 200 || !/^[\w$]+$/.test(query)) return Decoration.none
      const isWord = (pos: number) => pos >= 0 && pos < state.doc.length && WORD_CHAR.test(state.sliceDoc(pos, pos + 1))
      if (isWord(from - 1) || isWord(to)) return Decoration.none
      const marks = []
      for (const range of view.visibleRanges) {
        const text = state.sliceDoc(range.from, range.to)
        for (let index = text.indexOf(query); index >= 0; index = text.indexOf(query, index + 1)) {
          const start = range.from + index
          const end = start + query.length
          if (start === from || isWord(start - 1) || isWord(end)) continue
          marks.push(selectionMatchMark.range(start, end))
        }
      }
      return Decoration.set(marks, true)
    }
  },
  { decorations: plugin => plugin.decorations },
)

/** Layout rules only; colours come from the app's CSS variables so both themes apply. */
const scadTheme = EditorView.theme({
  '&': { height: '100%', backgroundColor: 'var(--bg)', color: 'var(--text)', fontSize: '12.5px' },
  '&.cm-focused': { outline: '2px solid var(--accent)', outlineOffset: '-2px' },
  '.cm-scroller': { fontFamily: 'var(--font-mono)', lineHeight: '1.6', overflow: 'auto' },
  '.cm-content': { padding: '14px 0', caretColor: 'var(--accent)', tabSize: '2' },
  '.cm-line': { padding: '0 15px 0 6px' },
  '.cm-cursor, .cm-dropCursor': { borderLeftColor: 'var(--accent)' },
  '&.cm-focused > .cm-scroller > .cm-selectionLayer .cm-selectionBackground, .cm-selectionBackground, ::selection': {
    backgroundColor: 'color-mix(in srgb, var(--accent) 35%, transparent)',
  },
  '.cm-gutters': { backgroundColor: 'var(--bg)', color: 'var(--text-dim)', border: 'none' },
  '.cm-lineNumbers .cm-gutterElement': { padding: '0 4px 0 10px', opacity: '.6' },
  '.cm-activeLine, .cm-activeLineGutter': { backgroundColor: 'color-mix(in srgb, var(--hover) 60%, transparent)' },
  '.cm-foldGutter .cm-gutterElement': { color: 'var(--text-dim)', cursor: 'pointer' },
  '.cm-foldPlaceholder': {
    backgroundColor: 'var(--surface-raised, var(--surface))',
    border: 'none',
    color: 'var(--text-dim)',
  },
  '.cm-selectionMatch': { backgroundColor: 'color-mix(in srgb, var(--accent) 23%, transparent)' },
  '.cm-source-highlight': {
    backgroundColor: 'color-mix(in srgb, var(--accent) 28%, transparent)',
    outline: '1px solid color-mix(in srgb, var(--accent) 70%, transparent)',
    borderRadius: '2px',
  },
  '.cm-find-highlight': { backgroundColor: 'color-mix(in srgb, var(--warning) 30%, transparent)' },
  '.cm-diagnostic-highlight': { textDecoration: 'underline wavy var(--danger)' },
})

export interface MountCodeMirrorOptions {
  parent: HTMLElement
  text: string
  ariaLabel: string
  maxLength?: number
  /** Every document change; `user` is false for programmatic edits made through the adapter. */
  onChange(text: string, user: boolean): void
  /** Selection moved or the editor gained focus. */
  onSelection?(): void
  extensions?: Extension[]
}

export interface MountedCodeMirror extends CodeMirrorEngine {
  view: EditorView
  setAriaLabel(label: string): void
}

export function mountCodeMirror(options: MountCodeMirrorOptions): MountedCodeMirror {
  const compositionListeners = new Set<(phase: EditorCompositionPhase, data: string) => void>()
  const emit = (phase: EditorCompositionPhase, data: string) => compositionListeners.forEach(l => l(phase, data))
  const aria = new Compartment()
  const ariaAttributes = (label: string) =>
    EditorView.contentAttributes.of({
      'aria-label': label,
      'aria-multiline': 'true',
      spellcheck: 'false',
      autocorrect: 'off',
      autocapitalize: 'off',
    })
  let lastNotified = options.text
  const notify = (user: boolean) => {
    const text = view.state.doc.toString()
    if (text === lastNotified) return
    lastNotified = text
    options.onChange(text, user)
  }
  const maxLength = options.maxLength
  const view: EditorView = new EditorView({
    parent: options.parent,
    doc: options.text,
    extensions: [
      coreExtensions(),
      lineNumbers(),
      foldGutter(),
      scadFolding,
      highlightActiveLine(),
      highlightActiveLineGutter(),
      drawSelection(),
      scadRushLanguage,
      syntaxHighlighting(scadHighlightStyle),
      selectionMatches,
      highlightDecorations,
      // No Tab binding: the app's keydown handler owns Tab / Shift+Tab indentation.
      keymap.of([...historyKeymap, ...defaultKeymap]),
      maxLength === undefined
        ? []
        : EditorState.transactionFilter.of(tr =>
            tr.docChanged &&
            tr.newDoc.length > maxLength &&
            tr.newDoc.length > tr.startState.doc.length &&
            !isProgrammatic(tr)
              ? []
              : tr,
          ),
      aria.of(ariaAttributes(options.ariaLabel)),
      scadTheme,
      EditorView.updateListener.of(update => {
        if (update.docChanged) {
          const user = !update.transactions.some(isProgrammatic)
          // Mid-composition text is not a source edit; compositionend commits it once.
          if (!(user && update.view.composing)) notify(user)
        }
        if (update.selectionSet || update.focusChanged) options.onSelection?.()
      }),
      EditorView.domEventHandlers({
        compositionstart: event => emit('start', event.data ?? ''),
        compositionupdate: event => emit('update', event.data ?? ''),
        compositionend: event => {
          // CodeMirror applies the final composed text after this event; commit on the next task.
          setTimeout(() => {
            notify(true)
            emit('end', event.data ?? '')
          }, 0)
        },
      }),
      options.extensions ?? [],
    ],
  })
  const core = createEngineCore(view)
  return {
    ...core,
    view,
    element: () => view.dom,
    focus: () => view.focus(),
    hasFocus: () => view.hasFocus,
    getScroll: () => ({
      top: view.scrollDOM.scrollTop,
      left: view.scrollDOM.scrollLeft,
      clientHeight: view.scrollDOM.clientHeight,
    }),
    setScroll({ top, left }) {
      if (top !== undefined) view.scrollDOM.scrollTop = top
      if (left !== undefined) view.scrollDOM.scrollLeft = left
    },
    lineHeightPx: () => view.defaultLineHeight,
    isComposing: () => view.composing,
    onComposition(listener) {
      compositionListeners.add(listener)
      return () => compositionListeners.delete(listener)
    },
    setAriaLabel(label) {
      view.dispatch({ effects: aria.reconfigure(ariaAttributes(label)) })
    },
    destroy: () => view.destroy(),
  }
}
