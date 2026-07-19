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
  return Number(raw)
}

function bracesBefore(source: string, end: number): number {
  let depth = 0
  let quote = ''
  for (let index = 0; index < end; index++) {
    const character = source[index]
    const next = source[index + 1]
    if (quote) {
      if (character === '\\') index++
      else if (character === quote) quote = ''
      continue
    }
    if (character === '"' || character === "'") { quote = character; continue }
    if (character === '/' && next === '/') {
      const newline = source.indexOf('\n', index + 2)
      if (newline < 0 || newline >= end) break
      index = newline
      continue
    }
    if (character === '/' && next === '*') {
      const close = source.indexOf('*/', index + 2)
      if (close < 0 || close >= end) break
      index = close + 1
      continue
    }
    if (character === '{') depth++
    else if (character === '}') depth = Math.max(0, depth - 1)
  }
  return depth
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
  for (const match of source.matchAll(ASSIGNMENT)) {
    const statementStart = (match.index ?? 0) + match[1].length
    if (bracesBefore(source, statementStart) !== 0) continue
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
  const encoded = typeof value === 'string' ? JSON.stringify(value) : String(value)
  return source.slice(0, parameter.valueStart) + encoded + source.slice(parameter.valueEnd)
}
