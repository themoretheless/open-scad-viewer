export const MAX_TEXT_SEARCH_MATCHES = 10_000
export const MAX_TEXT_SEARCH_QUERY_LENGTH = 4_096
export const MAX_TEXT_SEARCH_SOURCE_LENGTH = 250_000

export interface TextMatch {
  readonly start: number
  readonly end: number
  readonly text: string
}

export interface TextSearchResult {
  readonly matches: readonly TextMatch[]
  readonly truncated: boolean
}

export function findLiteralMatches(
  source: string,
  query: string,
  options: { caseSensitive?: boolean; maxMatches?: number } = {},
): TextSearchResult {
  if (!query) return { matches: [], truncated: false }
  if (query.length > MAX_TEXT_SEARCH_QUERY_LENGTH || source.length > MAX_TEXT_SEARCH_SOURCE_LENGTH) {
    return { matches: [], truncated: true }
  }
  const cap = Math.max(1, Math.min(MAX_TEXT_SEARCH_MATCHES, Math.trunc(options.maxMatches ?? MAX_TEXT_SEARCH_MATCHES)))
  const expression = new RegExp(escapeRegExp(query), options.caseSensitive ? 'gu' : 'giu')
  const matches: TextMatch[] = []
  for (const match of source.matchAll(expression)) {
    const start = match.index
    const text = match[0]
    if (matches.length === cap) return { matches, truncated: true }
    matches.push({ start, end: start + text.length, text })
  }
  return { matches, truncated: false }
}

export function wrappedMatchIndex(current: number, count: number, direction: 1 | -1): number {
  if (count <= 0) return -1
  if (current < 0 || current >= count) return direction === 1 ? 0 : count - 1
  return (current + direction + count) % count
}

export function replaceExpectedMatch(source: string, match: TextMatch, replacement: string): string | null {
  if (match.start < 0 || match.end < match.start || match.end > source.length
    || source.slice(match.start, match.end) !== match.text) return null
  const nextLength = source.length - (match.end - match.start) + replacement.length
  if (nextLength > MAX_TEXT_SEARCH_SOURCE_LENGTH) return null
  return source.slice(0, match.start) + replacement + source.slice(match.end)
}

export type ReplaceAllResult =
  | { readonly status: 'unchanged'; readonly source: string; readonly count: 0 }
  | { readonly status: 'replaced'; readonly source: string; readonly count: number }
  | { readonly status: 'too-many'; readonly source: string; readonly count: number }
  | { readonly status: 'too-large'; readonly source: string; readonly count: number }

export function replaceAllLiteral(
  source: string,
  query: string,
  replacement: string,
  options: { caseSensitive?: boolean; maxMatches?: number } = {},
): ReplaceAllResult {
  const result = findLiteralMatches(source, query, options)
  if (result.truncated) return { status: 'too-many', source, count: result.matches.length }
  if (!result.matches.length) return { status: 'unchanged', source, count: 0 }
  const removed = result.matches.reduce((sum, match) => sum + match.end - match.start, 0)
  const nextLength = source.length - removed + result.matches.length * replacement.length
  if (!Number.isSafeInteger(nextLength) || nextLength > MAX_TEXT_SEARCH_SOURCE_LENGTH) {
    return { status: 'too-large', source, count: result.matches.length }
  }
  let next = ''
  let cursor = 0
  for (const match of result.matches) {
    next += source.slice(cursor, match.start) + replacement
    cursor = match.end
  }
  return { status: 'replaced', source: next + source.slice(cursor), count: result.matches.length }
}

function escapeRegExp(value: string): string {
  return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')
}
