import { MAX_DOCUMENT_CHARACTERS } from './directDocumentLimits'
import { inspectNurbsBrep, type NurbsBrep } from './geometry/brep'

/** Exact-content memoization: cloned bodies hit, mutated geometry must be inspected again. */
export class BrepInspectionCache {
  private readonly keys = new Set<string>()
  private characters = 0
  private readonly maxEntries: number
  private readonly maxCharacters: number

  constructor(
    limits = { maxEntries: 128, maxCharacters: MAX_DOCUMENT_CHARACTERS },
    private readonly inspectModel: (model: NurbsBrep) => void = inspectNurbsBrep,
  ) {
    for (const limit of [limits.maxEntries, limits.maxCharacters]) {
      if (!Number.isSafeInteger(limit) || limit < 1) throw new RangeError('Invalid B-rep inspection cache limit')
    }
    this.maxEntries = limits.maxEntries
    this.maxCharacters = limits.maxCharacters
  }

  get size() { return this.keys.size }
  /** UTF-16 code units retained as keys, not a measurement of JavaScript heap bytes. */
  get retainedCharacters() { return this.characters }

  inspect(model: NurbsBrep): void {
    const key = JSON.stringify(model)
    if (this.keys.has(key)) return
    this.inspectModel(model)
    // Large or rejected inputs must not evict the useful working set.
    if (key.length > this.maxCharacters) return
    while (this.keys.size >= this.maxEntries || this.characters + key.length > this.maxCharacters) {
      const oldest = this.keys.values().next().value!
      this.keys.delete(oldest)
      this.characters -= oldest.length
    }
    this.keys.add(key)
    this.characters += key.length
  }
}
