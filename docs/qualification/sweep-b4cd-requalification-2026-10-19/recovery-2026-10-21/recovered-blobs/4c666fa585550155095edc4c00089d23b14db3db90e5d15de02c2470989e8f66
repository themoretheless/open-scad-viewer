export interface SourceSplice {
  readonly from: number
  readonly to: number
  readonly insert: string
  readonly expected: string
  readonly origin: 'customizer' | 'quick-fix' | 'refactor'
}

export interface AppliedSourceSplice {
  readonly source: string
  readonly selection: readonly [anchor: number, head: number]
  readonly inverse: SourceSplice
}

/** Plan one stale-safe, byte-exact source transaction and its inverse. */
export function applySourceSplice(source: string, splice: SourceSplice): AppliedSourceSplice {
  if (!Number.isSafeInteger(splice.from) || !Number.isSafeInteger(splice.to)
    || splice.from < 0 || splice.to < splice.from || splice.to > source.length) {
    throw new RangeError('Source splice range is invalid')
  }
  if (source.slice(splice.from, splice.to) !== splice.expected) throw new Error('Source splice target is stale')
  const next = source.slice(0, splice.from) + splice.insert + source.slice(splice.to)
  return {
    source: next,
    selection: [splice.from, splice.from + splice.insert.length],
    inverse: {
      from: splice.from,
      to: splice.from + splice.insert.length,
      insert: splice.expected,
      expected: splice.insert,
      origin: splice.origin,
    },
  }
}
