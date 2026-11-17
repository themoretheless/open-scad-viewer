/** Object selection remembered per history state, so Undo/Redo brings back the selection that
 * was active in the restored state instead of dropping or keeping an unrelated one. */
export interface HistorySelection {
  readonly primary: string
  readonly extra: readonly string[]
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
      byState.set(state, { primary: selection.primary, extra: [...selection.extra] })
    },
    recall(state, exists) {
      const saved = byState.get(state)
      if (!saved) return undefined
      const ids = [...new Set([saved.primary, ...saved.extra].filter(id => id !== '' && exists(id)))]
      return { primary: ids[0] ?? '', extra: ids.slice(1) }
    },
  }
}
