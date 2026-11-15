import {
  EditorSelection as CmSelection,
  EditorState,
  StateEffect,
  StateField,
  Transaction,
  type Extension,
  type TransactionSpec,
} from '@codemirror/state'
import { history, isolateHistory, redo, redoDepth, undo, undoDepth } from '@codemirror/commands'
import type { EditorHighlightRange, EditorSelectMode, EditorSelection, EditorSelectionDirection } from './EditorAdapter'

/**
 * Headless logic layer of the CodeMirror 6 editor engine (ROADMAP P4).
 *
 * Depends only on `@codemirror/state` and `@codemirror/commands`, so the undo
 * contract from `editHistory.ts` is testable without a DOM: CodeMirror's own
 * history groups typing within `HISTORY_GROUP_MS`, joins IME composition
 * transactions (`input.type.compose`) into one event, and every programmatic
 * edit below is isolated into exactly one undo step.
 */
export const HISTORY_GROUP_MS = 500

/** User event tagged on app-made edits (format, find/replace, customizer, preset). */
export const PROGRAMMATIC_USER_EVENT = 'input.programmatic'

export const setHighlightsEffect = StateEffect.define<readonly EditorHighlightRange[]>()

/** Source↔scene / find / diagnostic ranges, mapped through edits; the view renders them as decorations. */
export const highlightsField = StateField.define<readonly EditorHighlightRange[]>({
  create: () => [],
  update(value, tr) {
    for (const effect of tr.effects) if (effect.is(setHighlightsEffect)) return effect.value
    if (!tr.docChanged || !value.length) return value
    return value
      .map(range => ({
        ...range,
        start: tr.changes.mapPos(range.start, 1),
        end: tr.changes.mapPos(range.end, -1),
      }))
      .filter(range => range.end >= range.start)
  },
})

/** The state extensions shared by the headless engine and the mounted view. */
export function coreExtensions(): Extension[] {
  return [history({ newGroupDelay: HISTORY_GROUP_MS }), highlightsField]
}

export function createCoreState(text: string, extensions: Extension[] = []) {
  return EditorState.create({ doc: text, extensions: [coreExtensions(), extensions] })
}

/** The part of `EditorView` the engine needs; a plain object satisfies it headlessly. */
export interface CodeMirrorHost {
  readonly state: EditorState
  dispatch(...specs: TransactionSpec[]): void
}

/** Minimal common-prefix/suffix diff so whole-document replacements keep scroll and decorations. */
export function minimalChange(before: string, after: string) {
  let start = 0
  const max = Math.min(before.length, after.length)
  while (start < max && before.charCodeAt(start) === after.charCodeAt(start)) start++
  let endBefore = before.length
  let endAfter = after.length
  while (endBefore > start && endAfter > start && before.charCodeAt(endBefore - 1) === after.charCodeAt(endAfter - 1)) {
    endBefore--
    endAfter--
  }
  return { from: start, to: endBefore, insert: after.slice(start, endAfter) }
}

function mapSelection(
  state: EditorState,
  from: number,
  to: number,
  insert: string,
  select: EditorSelectMode,
): CmSelection | undefined {
  const newEnd = from + insert.length
  if (select === 'select') return CmSelection.single(from, newEnd)
  if (select === 'start') return CmSelection.single(from)
  if (select === 'end') return CmSelection.single(newEnd)
  // 'preserve': DOM setRangeText semantics — positions inside the replaced span collapse to its start.
  const delta = insert.length - (to - from)
  const map = (p: number) => (p > to ? p + delta : p > from ? from : p)
  const main = state.selection.main
  return CmSelection.single(map(main.anchor), map(main.head))
}

/** One isolated, undoable programmatic edit. */
export function programmaticSpec(
  state: EditorState,
  from: number,
  to: number,
  insert: string,
  select: EditorSelectMode = 'preserve',
): TransactionSpec {
  const length = state.doc.length
  const start = Math.max(0, Math.min(from, length))
  const end = Math.max(start, Math.min(to, length))
  return {
    changes: { from: start, to: end, insert },
    selection: mapSelection(state, start, end, insert, select),
    userEvent: PROGRAMMATIC_USER_EVENT,
    annotations: [isolateHistory.of('full'), Transaction.addToHistory.of(true)],
  }
}

export function isProgrammatic(tr: Transaction) {
  return tr.isUserEvent(PROGRAMMATIC_USER_EVENT)
}

export function selectionOf(state: EditorState): EditorSelection {
  const main = state.selection.main
  return {
    start: main.from,
    end: main.to,
    direction: main.empty ? 'none' : main.anchor <= main.head ? 'forward' : 'backward',
  }
}

export function selectionSpec(
  state: EditorState,
  start: number,
  end: number,
  direction?: EditorSelectionDirection,
): CmSelection {
  const length = state.doc.length
  const a = Math.max(0, Math.min(start, length))
  const b = Math.max(0, Math.min(end, length))
  return direction === 'backward' ? CmSelection.single(b, a) : CmSelection.single(a, b)
}

/** Text, selection, history and highlight operations over any host. */
export interface CodeMirrorEngineCore {
  getText(): string
  setText(text: string, select?: EditorSelectMode): void
  replaceRange(text: string, start: number, end: number, select?: EditorSelectMode): void
  getSelection(): EditorSelection
  setSelection(start: number, end: number, direction?: EditorSelectionDirection): void
  undo(): boolean
  redo(): boolean
  undoDepth(): number
  redoDepth(): number
  setHighlights(ranges: readonly EditorHighlightRange[]): void
  getHighlights(): readonly EditorHighlightRange[]
}

export function createEngineCore(
  host: CodeMirrorHost,
  options: { scrollIntoView?: boolean } = {},
): CodeMirrorEngineCore {
  const command = (run: typeof undo) => run({ state: host.state, dispatch: tr => host.dispatch(tr) })
  return {
    getText: () => host.state.doc.toString(),
    setText(text, select = 'preserve') {
      const current = host.state.doc.toString()
      if (current === text && select === 'preserve') return
      const change = minimalChange(current, text)
      if (select === 'preserve') {
        host.dispatch(programmaticSpec(host.state, change.from, change.to, change.insert, 'preserve'))
        return
      }
      // Whole-document selection modes refer to the full replaced span.
      const spec = programmaticSpec(host.state, change.from, change.to, change.insert, 'preserve')
      const selection =
        select === 'select'
          ? CmSelection.single(0, text.length)
          : select === 'start'
            ? CmSelection.single(0)
            : CmSelection.single(text.length)
      host.dispatch({ ...spec, selection })
    },
    replaceRange(text, start, end, select = 'preserve') {
      host.dispatch(programmaticSpec(host.state, start, end, text, select))
    },
    getSelection: () => selectionOf(host.state),
    setSelection(start, end, direction) {
      host.dispatch({
        selection: selectionSpec(host.state, start, end, direction),
        scrollIntoView: options.scrollIntoView ?? true,
      })
    },
    undo: () => command(undo),
    redo: () => command(redo),
    undoDepth: () => undoDepth(host.state),
    redoDepth: () => redoDepth(host.state),
    setHighlights(ranges) {
      host.dispatch({ effects: setHighlightsEffect.of(ranges.slice()) })
    },
    getHighlights: () => host.state.field(highlightsField, false) ?? [],
  }
}

/** A DOM-free host for tests and benchmarks: applies transactions to a plain state. */
export function createHeadlessHost(
  text: string,
  extensions: Extension[] = [],
): CodeMirrorHost & {
  transactions: Transaction[]
} {
  const host = {
    state: createCoreState(text, extensions),
    transactions: [] as Transaction[],
    dispatch(...specs: TransactionSpec[]) {
      const tr = specs.length === 1 && specs[0] instanceof Transaction ? specs[0] : host.state.update(...specs)
      host.transactions.push(tr)
      host.state = tr.state
    },
  }
  return host
}
