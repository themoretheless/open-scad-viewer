export interface PaletteCommand {
  id: string
  label: string
  detail?: string
  shortcut?: string
  keywords?: string | readonly string[]
  /** Alternative localized names, for example both Russian and English. */
  aliases?: readonly string[]
  /** Omit or set to true when the command can be executed in the current context. */
  enabled?: boolean
  /** Human-readable explanation shown for a command that is currently unavailable. */
  disabledReason?: string
  /** Lower values are more recent. Used only to resolve otherwise equal ranks. */
  mruRank?: number
}

interface SearchField {
  value: string
  bias: number
}

interface ScoredCommand {
  command: PaletteCommand
  score: number
  order: number
}

const NO_MATCH = Number.POSITIVE_INFINITY

/**
 * Rank commands without letting usage history override textual relevance.
 * Match tiers, in order, are exact, phrase prefix, token, substring,
 * subsequence and bounded edit-distance typo matching.
 */
export function rankPaletteCommands(
  commands: readonly PaletteCommand[],
  rawQuery: string,
): PaletteCommand[] {
  const query = normalizeCommandText(rawQuery)
  if (!query) {
    return commands
      .map((command, order) => ({ command, order }))
      .sort((a, b) => {
        const availability = Number(!isPaletteCommandEnabled(a.command))
          - Number(!isPaletteCommandEnabled(b.command))
        return availability
          || compareMru(a.command, b.command)
          || a.order - b.order
      })
      .map(({ command }) => command)
  }

  return commands
    .map((command, order): ScoredCommand => ({
      command,
      order,
      score: scoreCommand(command, query),
    }))
    .filter(item => Number.isFinite(item.score))
    .sort((a, b) => (
      a.score - b.score
      || compareMru(a.command, b.command)
      || a.order - b.order
    ))
    .map(({ command }) => command)
}

export function isPaletteCommandEnabled(command: PaletteCommand | undefined): boolean {
  return command !== undefined && command.enabled !== false
}

/**
 * Find the next executable option, wrapping around the list. Disabled options
 * remain visible in the listbox but keyboard navigation deliberately skips them.
 */
export function nextEnabledCommandIndex(
  commands: readonly PaletteCommand[],
  currentIndex: number,
  direction: 1 | -1,
): number {
  if (!commands.length) return -1
  const start = currentIndex >= 0 && currentIndex < commands.length
    ? currentIndex
    : direction === 1 ? -1 : 0

  for (let step = 1; step <= commands.length; step += 1) {
    const index = (start + direction * step + commands.length) % commands.length
    if (isPaletteCommandEnabled(commands[index])) return index
  }
  return -1
}

/** Move through every visible option so keyboard users can inspect disabled reasons. */
export function nextPaletteCommandIndex(
  commands: readonly PaletteCommand[],
  currentIndex: number,
  direction: 1 | -1,
): number {
  if (!commands.length) return -1
  const start = currentIndex >= 0 && currentIndex < commands.length
    ? currentIndex
    : direction === 1 ? -1 : 0
  return (start + direction + commands.length) % commands.length
}

export function normalizeCommandText(value: string): string {
  return value
    .normalize('NFKD')
    .replace(/\p{M}+/gu, '')
    .toLocaleLowerCase()
    .replace(/[^\p{L}\p{N}]+/gu, ' ')
    .trim()
    .replace(/\s+/g, ' ')
}

function scoreCommand(command: PaletteCommand, query: string): number {
  const keywordFields = typeof command.keywords === 'string'
    ? [command.keywords]
    : command.keywords ?? []
  const fields: SearchField[] = [
    { value: command.label, bias: 0 },
    ...(command.aliases ?? []).map(value => ({ value, bias: 4 })),
    ...keywordFields.map(value => ({ value, bias: 8 })),
    ...(command.detail ? [{ value: command.detail, bias: 12 }] : []),
  ]

  let best = NO_MATCH
  for (const field of fields) {
    const value = normalizeCommandText(field.value)
    if (!value) continue
    best = Math.min(best, scoreField(value, query) + field.bias)
  }
  return best
}

function scoreField(value: string, query: string): number {
  if (value === query) return 0
  if (value.startsWith(query)) return 100 + lengthPenalty(value, query)

  const valueTokens = value.split(' ')
  const queryTokens = query.split(' ')
  const exactTokenIndex = valueTokens.indexOf(query)
  if (exactTokenIndex >= 0) return 200 + exactTokenIndex

  const tokenScore = scoreQueryTokens(valueTokens, queryTokens)
  if (Number.isFinite(tokenScore)) return 240 + tokenScore

  const substringIndex = value.indexOf(query)
  if (substringIndex >= 0) return 320 + Math.min(40, substringIndex)

  const compactValue = value.replaceAll(' ', '')
  const compactQuery = query.replaceAll(' ', '')
  const subsequencePenalty = scoreSubsequence(compactValue, compactQuery)
  if (Number.isFinite(subsequencePenalty)) return 400 + subsequencePenalty

  const fuzzyScore = scoreFuzzyTokens(valueTokens, queryTokens)
  return Number.isFinite(fuzzyScore) ? 520 + fuzzyScore : NO_MATCH
}

function scoreQueryTokens(valueTokens: readonly string[], queryTokens: readonly string[]): number {
  let score = 0
  let searchFrom = 0
  for (const queryToken of queryTokens) {
    let bestIndex = -1
    let bestPenalty = NO_MATCH
    for (let index = searchFrom; index < valueTokens.length; index += 1) {
      const token = valueTokens[index] ?? ''
      const penalty = token === queryToken
        ? 0
        : token.startsWith(queryToken) ? 8 + lengthPenalty(token, queryToken) : NO_MATCH
      if (penalty < bestPenalty) {
        bestIndex = index
        bestPenalty = penalty
      }
    }
    if (bestIndex < 0) return NO_MATCH
    score += bestPenalty + (bestIndex - searchFrom)
    searchFrom = bestIndex + 1
  }
  return score
}

function scoreSubsequence(value: string, query: string): number {
  if (!query || query.length > value.length) return NO_MATCH
  let queryIndex = 0
  let firstIndex = -1
  let previousIndex = -1
  let gaps = 0
  for (let index = 0; index < value.length && queryIndex < query.length; index += 1) {
    if (value[index] !== query[queryIndex]) continue
    if (firstIndex < 0) firstIndex = index
    if (previousIndex >= 0) gaps += index - previousIndex - 1
    previousIndex = index
    queryIndex += 1
  }
  return queryIndex === query.length ? Math.min(99, (firstIndex < 0 ? 0 : firstIndex) + gaps) : NO_MATCH
}

function scoreFuzzyTokens(valueTokens: readonly string[], queryTokens: readonly string[]): number {
  let score = 0
  for (const queryToken of queryTokens) {
    const threshold = typoThreshold(queryToken.length)
    if (!threshold) return NO_MATCH
    let best = NO_MATCH
    for (const valueToken of valueTokens) {
      if (Math.abs(valueToken.length - queryToken.length) > threshold) continue
      best = Math.min(best, damerauLevenshtein(valueToken, queryToken, threshold))
    }
    if (!Number.isFinite(best)) return NO_MATCH
    score += best * 12
  }
  return score
}

function typoThreshold(length: number): number {
  if (length < 4) return 0
  return length < 7 ? 1 : 2
}

/** Bounded optimal-string-alignment distance, including adjacent transpositions. */
function damerauLevenshtein(a: string, b: string, maximum: number): number {
  const left = Array.from(a)
  const right = Array.from(b)
  if (Math.abs(left.length - right.length) > maximum) return NO_MATCH

  const matrix = Array.from({ length: left.length + 1 }, () => new Array<number>(right.length + 1).fill(0))
  for (let i = 0; i <= left.length; i += 1) matrix[i]![0] = i
  for (let j = 0; j <= right.length; j += 1) matrix[0]![j] = j

  for (let i = 1; i <= left.length; i += 1) {
    let rowMinimum = NO_MATCH
    for (let j = 1; j <= right.length; j += 1) {
      const substitution = left[i - 1] === right[j - 1] ? 0 : 1
      let distance = Math.min(
        matrix[i - 1]![j]! + 1,
        matrix[i]![j - 1]! + 1,
        matrix[i - 1]![j - 1]! + substitution,
      )
      if (
        i > 1
        && j > 1
        && left[i - 1] === right[j - 2]
        && left[i - 2] === right[j - 1]
      ) {
        distance = Math.min(distance, matrix[i - 2]![j - 2]! + 1)
      }
      matrix[i]![j] = distance
      rowMinimum = Math.min(rowMinimum, distance)
    }
    if (rowMinimum > maximum) return NO_MATCH
  }

  const result = matrix[left.length]![right.length]!
  return result <= maximum ? result : NO_MATCH
}

function lengthPenalty(value: string, query: string): number {
  return Math.min(49, Math.max(0, value.length - query.length))
}

function compareMru(a: PaletteCommand, b: PaletteCommand): number {
  return normalizedMru(a.mruRank) - normalizedMru(b.mruRank)
}

function normalizedMru(value: number | undefined): number {
  return value !== undefined && Number.isFinite(value) ? Math.max(0, value) : Number.MAX_SAFE_INTEGER
}
