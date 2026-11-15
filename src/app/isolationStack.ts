/**
 * Nested isolation (roadmap C6). Each level narrows the visible set; stepping out
 * pops one level. The innermost level decides what the renderer shows.
 */
export type IsolationStack = readonly (readonly string[])[]

/** Push a level made of the `ids` visible at the current level. A level that narrows nothing is a no-op. */
export function isolateWithin(stack: IsolationStack, ids: readonly string[]): IsolationStack {
  const current = stack.at(-1)
  const level = [...new Set(current ? ids.filter(id => current.includes(id)) : ids)]
  if (!level.length || (current && level.length === current.length)) return stack
  return [...stack, level]
}

export const stepOutIsolation = (stack: IsolationStack): IsolationStack => stack.slice(0, -1)

export const isolatedInView = (stack: IsolationStack, id: string): boolean => {
  const level = stack.at(-1)
  return !level || level.includes(id)
}

/** Drop missing ids from every level, and levels that become empty. Returns the same stack when nothing changed. */
export function pruneIsolation(stack: IsolationStack, exists: (id: string) => boolean): IsolationStack {
  let changed = false
  const next: string[][] = []
  for (const level of stack) {
    const kept = level.filter(exists)
    if (kept.length !== level.length) changed = true
    if (kept.length) next.push(kept)
  }
  return changed ? next : stack
}

/** Newly created objects join every level so they stay visible where they were made. */
export const addToIsolation = (stack: IsolationStack, ids: readonly string[]): IsolationStack =>
  stack.length && ids.length ? stack.map(level => [...new Set([...level, ...ids])]) : stack

/** Accept the persisted form: a flat id list (older workspaces) or a list of levels. */
export function readIsolation(value: unknown, exists: (id: string) => boolean): IsolationStack {
  if (!Array.isArray(value)) return []
  const levels: unknown[] = value.every(v => typeof v === 'string') ? (value.length ? [value] : []) : value
  const stack: string[][] = []
  for (const level of levels.slice(0, 16)) {
    if (!Array.isArray(level)) return []
    const ids = level.filter((id): id is string => typeof id === 'string' && exists(id)).slice(0, 1000)
    if (ids.length) stack.push(ids)
  }
  return stack
}
