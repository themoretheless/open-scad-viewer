/** Object selection remembered per history state, so Undo/Redo brings back the selection that
 * was active in the restored state instead of dropping or keeping an unrelated one. */
import type { SubshapeAnchor } from './subshapeSelection'

export interface HistorySelection {
  readonly primary: string
  readonly extra: readonly string[]
  /** Face/edge selection on `primary`, by stable identity (see `subshapeSelection.ts`). */
  readonly subshape?: SubshapeAnchor | null
}

export interface SelectionHistory {
  /** Record the selection active while `state` (a `DirectHistory.stateToken`) is current. */
  remember(state: object, selection: HistorySelection): void
  /** The selection recorded for `state`, limited to identities `exists` still accepts; undefined
   * when nothing was recorded, so the caller keeps its own pruning. */
  recall(state: object, exists: (id: string) => boolean): HistorySelection | undefined
}

export function createSelectionHistory(): SelectionHistory {
  // Weak keys: dropped history states (trimmed stacks, a replaced history) release their entry.
  const byState = new WeakMap<object, HistorySelection>()
  return {
    remember(state, selection) {
      byState.set(state, {
        primary: selection.primary,
        extra: [...selection.extra],
        subshape: selection.subshape ?? null,
      })
    },
    recall(state, exists) {
      const saved = byState.get(state)
      if (!saved) return undefined
      const ids = [...new Set([saved.primary, ...saved.extra].filter(id => id !== '' && exists(id)))]
      const primary = ids[0] ?? ''
      // A face/edge selection belongs to its body: it is dropped with the body.
      const subshape = saved.subshape && saved.subshape.body === primary ? saved.subshape : null
      return { primary, extra: ids.slice(1), subshape }
    },
  }
}
