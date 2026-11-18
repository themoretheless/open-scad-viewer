/**
 * Undo-granularity contract for source editor engines.
 *
 * The textarea engine relies on the browser's native undo stack; this model
 * states the granularity any replacement engine (CodeMirror 6, Monaco) must
 * keep and is what the adapter tests check:
 * - adjacent typed insertions within `coalesceMs` merge into one step;
 * - a programmatic edit (format, find/replace, customizer, preset) is exactly one step;
 * - nothing is recorded while an IME composition is open; the composed text
 *   becomes one step on compositionend.
 */
export interface HistoryEdit {
  from: number
  to: number
  insert: string
  removed: string
}

export interface HistoryStep {
  edits: HistoryEdit[]
  origin: 'typing' | 'programmatic' | 'composition'
  at: number
}

export interface EditHistoryOptions {
  coalesceMs?: number
}

export class EditHistory {
  private readonly coalesceMs: number
  private past: HistoryStep[] = []
  private future: HistoryStep[] = []
  private composing = false

  constructor(
    private text: string,
    options: EditHistoryOptions = {},
  ) {
    this.coalesceMs = options.coalesceMs ?? 500
  }

  get value() {
    return this.text
  }
  get undoDepth() {
    return this.past.length
  }
  get redoDepth() {
    return this.future.length
  }

  private apply(edit: Omit<HistoryEdit, 'removed'>): HistoryEdit {
    const removed = this.text.slice(edit.from, edit.to)
    this.text = this.text.slice(0, edit.from) + edit.insert + this.text.slice(edit.to)
    return { ...edit, removed }
  }

  /** A keystroke-sized edit from the user. */
  type(from: number, to: number, insert: string, at: number) {
    const edit = this.apply({ from, to, insert })
    if (this.composing) return
    this.future = []
    const last = this.past.at(-1)
    const prev = last?.edits.at(-1)
    if (
      last?.origin === 'typing' &&
      prev &&
      at - last.at <= this.coalesceMs &&
      from === to &&
      prev.from + prev.insert.length === from &&
      !/\n/.test(insert)
    ) {
      last.edits.push(edit)
      last.at = at
      return
    }
    this.past.push({ edits: [edit], origin: 'typing', at })
  }

  /** A whole-step edit made by the app rather than by keystrokes. */
  replace(from: number, to: number, insert: string, at: number) {
    const edit = this.apply({ from, to, insert })
    this.future = []
    this.past.push({ edits: [edit], origin: 'programmatic', at })
  }

  compositionStart() {
    this.composing = true
    this.compositionBase = this.text
  }
  private compositionBase = ''

  /** Closes the composition; the net change becomes one undo step. */
  compositionEnd(at: number) {
    if (!this.composing) return
    this.composing = false
    const before = this.compositionBase
    const after = this.text
    if (before === after) return
    let start = 0
    while (start < before.length && start < after.length && before[start] === after[start]) start++
    let endBefore = before.length
    let endAfter = after.length
    while (endBefore > start && endAfter > start && before[endBefore - 1] === after[endAfter - 1]) {
      endBefore--
      endAfter--
    }
    this.future = []
    this.past.push({
      edits: [
        { from: start, to: endBefore, insert: after.slice(start, endAfter), removed: before.slice(start, endBefore) },
      ],
      origin: 'composition',
      at,
    })
  }

  undo(): boolean {
    const step = this.past.pop()
    if (!step) return false
    for (const edit of [...step.edits].reverse()) {
      this.text = this.text.slice(0, edit.from) + edit.removed + this.text.slice(edit.from + edit.insert.length)
    }
    this.future.push(step)
    return true
  }

  redo(): boolean {
    const step = this.future.pop()
    if (!step) return false
    for (const edit of step.edits) {
      this.text = this.text.slice(0, edit.from) + edit.insert + this.text.slice(edit.to)
    }
    this.past.push(step)
    return true
  }
}
