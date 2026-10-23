export type CustomizerValue = number | boolean | string

export interface CustomizerParameter {
  name: string
  label: string
  value: CustomizerValue
  valueStart: number
  valueEnd: number
  min?: number
  max?: number
  step?: number
  options?: CustomizerValue[]
}

const ASSIGNMENT = /(^|\n)([ \t]*)([A-Za-z_$][A-Za-z0-9_$]*)([ \t]*=[ \t]*)(-?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][+-]?\d+)?|true|false|"(?:\\.|[^"\\])*")[ \t]*;([^\n]*)/g

function decodeLiteral(raw: string): CustomizerValue {
  if (raw === 'true') return true
  if (raw === 'false') return false
  if (raw.startsWith('"')) {
    try { return JSON.parse(raw) as string }
    catch { return raw.slice(1, -1) }
  }
  const value = Number(raw)
  if (!Number.isFinite(value)) throw new RangeError('Customizer numeric literals must be finite')
  return value
}

function parseMetadata(raw: string, value: CustomizerValue): Pick<CustomizerParameter, 'min' | 'max' | 'step' | 'options'> {
  const comment = raw.match(/\/\/\s*\[([^\]]+)\]/)?.[1]?.trim()
  if (!comment) return {}
  if (typeof value === 'number') {
    const range = comment.split(':').map(Number)
    if ((range.length === 2 || range.length === 3) && range.every(Number.isFinite)) {
      const [min, middle, last] = range
      const max = range.length === 2 ? middle : last
      const step = range.length === 3 ? middle : undefined
      if (max >= min && (step === undefined || step > 0)) return { min, max, step }
    }
  }
  const options = comment.split(',').map(part => part.trim()).filter(Boolean).map(part => {
    if (/^-?(?:\d+(?:\.\d*)?|\.\d+)$/.test(part)) return Number(part)
    if (part === 'true' || part === 'false') return part === 'true'
    return part.replace(/^"|"$/g, '')
  })
  return options.length ? { options } : {}
}

/** Extract OpenSCAD Customizer-style top-level literal assignments. */
export function extractCustomizerParameters(source: string): CustomizerParameter[] {
  const parameters: CustomizerParameter[] = []
  let cursor = 0
  let depth = 0
  let quote = ''
  let lineComment = false
  let blockComment = false

  // Assignment matches are ordered. Advancing this lexer only once avoids the
  // former O(assignments * source length) prefix rescan on large parameter sets.
  const advanceTo = (end: number) => {
    while (cursor < end) {
      const character = source[cursor]
      const next = source[cursor + 1]
      if (lineComment) {
        if (character === '\n') lineComment = false
        cursor++
        continue
      }
      if (blockComment) {
        if (character === '*' && next === '/') {
          blockComment = false
          cursor += 2
        } else cursor++
        continue
      }
      if (quote) {
        if (character === '\\') cursor += Math.min(2, end - cursor)
        else {
          if (character === quote) quote = ''
          cursor++
        }
        continue
      }
      if (character === '"' || character === "'") {
        quote = character
        cursor++
      } else if (character === '/' && next === '/') {
        lineComment = true
        cursor += 2
      } else if (character === '/' && next === '*') {
        blockComment = true
        cursor += 2
      } else {
        if (character === '{') depth++
        else if (character === '}') depth = Math.max(0, depth - 1)
        cursor++
      }
    }
  }

  for (const match of source.matchAll(ASSIGNMENT)) {
    const statementStart = (match.index ?? 0) + match[1].length
    advanceTo(statementStart)
    if (depth !== 0 || quote || lineComment || blockComment) continue
    const name = match[3]
    if (name.startsWith('$')) continue
    const rawValue = match[5]
    // Structural offset: indent + name + "=" run. The previous
    // indexOf(rawValue) could match INSIDE the variable name (`x1 = 1;`
    // found "1" at offset 1), corrupting the source on replacement.
    const valueStart = statementStart + match[2].length + name.length + match[4].length
    const value = decodeLiteral(rawValue)
    const label = name.replace(/[_-]+/g, ' ').replace(/^./, character => character.toUpperCase())
    parameters.push({
      name,
      label,
      value,
      valueStart,
      valueEnd: valueStart + rawValue.length,
      ...parseMetadata(match[6], value),
    })
  }
  return parameters
}

export function replaceCustomizerValue(source: string, parameter: CustomizerParameter, value: CustomizerValue): string {
  const encoded = encodeCustomizerValue(value)
  return source.slice(0, parameter.valueStart) + encoded + source.slice(parameter.valueEnd)
}

export function encodeCustomizerValue(value: CustomizerValue): string {
  return typeof value === 'string' ? JSON.stringify(value) : String(value)
}
