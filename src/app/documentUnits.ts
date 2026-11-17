/**
 * Document length units (roadmap N5). Kernel geometry is always millimetres;
 * the unit is document metadata applied at import/export and display.
 * Documents without `units` are millimetre documents.
 */
export const DOCUMENT_LENGTH_UNITS = ['mm', 'cm', 'm', 'in'] as const
export type DocumentLengthUnit = (typeof DOCUMENT_LENGTH_UNITS)[number]

/** Exact factors; the inch is 25.4 mm by definition. */
export const MM_PER_UNIT: Readonly<Record<DocumentLengthUnit, number>> = { mm: 1, cm: 10, m: 1000, in: 25.4 }

export const DEFAULT_DOCUMENT_UNIT: DocumentLengthUnit = 'mm'

export function isDocumentLengthUnit(value: unknown): value is DocumentLengthUnit {
  return typeof value === 'string' && (DOCUMENT_LENGTH_UNITS as readonly string[]).includes(value)
}

export function documentUnit(document: { units?: DocumentLengthUnit }): DocumentLengthUnit {
  return document.units ?? DEFAULT_DOCUMENT_UNIT
}

/** Display/export: millimetres to document units. */
export function fromMillimetres(valueMm: number, unit: DocumentLengthUnit): number {
  return unit === 'mm' ? valueMm : valueMm / MM_PER_UNIT[unit]
}

/** Import/input: document units to millimetres. */
export function toMillimetres(value: number, unit: DocumentLengthUnit): number {
  return unit === 'mm' ? value : value * MM_PER_UNIT[unit]
}

/** Coordinate factor for importing a file authored in `source` into kernel millimetres. */
export function importScaleToMillimetres(source: DocumentLengthUnit): number {
  return MM_PER_UNIT[source]
}

/** Coordinate factor for exporting kernel millimetres into a file in `target` units. */
export function exportScaleFromMillimetres(target: DocumentLengthUnit): number {
  return 1 / MM_PER_UNIT[target]
}

export type UnitExpressionKind = 'length' | 'angle' | 'scalar'
export type UnitExpressionResult =
  { valid: true; value: number } | { valid: false; reason: 'number' | 'unit' | 'range' }

const EXPRESSION_UNITS: Readonly<Record<UnitExpressionKind, Readonly<Record<string, number>>>> = {
  length: MM_PER_UNIT,
  angle: { deg: 1, '°': 1, rad: 180 / Math.PI },
  scalar: {},
}
const ANY_UNIT = /^(mm|cm|m|in|deg|rad|°)(?![a-z_0-9])/i
const NUMBER = /^(?:\d+(?:[.,]\d*)?|[.,]\d+)(?:e[+-]?\d+)?/i
const IDENTIFIER = /^[a-z_][a-z_0-9]*/i

/**
 * Expression with units on individual terms (roadmap C6), e.g. `10mm+1in` or `2*pi*r`.
 * Each number may carry its own unit; bare numbers use `defaultUnit` (millimetres for
 * lengths, degrees for angles). Named `variables` are substituted as base values
 * (millimetres or degrees). The result is in base units. Arithmetic is delegated to
 * `evaluate` (the app's safe evaluator, no eval/Function).
 */
export function evaluateUnitExpression(
  text: string,
  evaluate: (expression: string) => number | null,
  options: {
    kind?: UnitExpressionKind
    defaultUnit?: string
    variables?: Readonly<Record<string, number>>
    min?: number
    max?: number
  } = {},
): UnitExpressionResult {
  const kind = options.kind ?? 'length'
  const units = EXPRESSION_UNITS[kind]
  const input = text.trim()
  if (!input || input.length > 256) return { valid: false, reason: 'number' }
  const defaultFactor = options.defaultUnit === undefined ? 1 : units[options.defaultUnit]
  if (defaultFactor === undefined) return { valid: false, reason: 'unit' }
  let out = ''
  for (let pos = 0; pos < input.length;) {
    const rest = input.slice(pos)
    const number = NUMBER.exec(rest)
    if (number) {
      pos += number[0].length
      const spaces = /^\s*/.exec(input.slice(pos))![0]
      const unit = ANY_UNIT.exec(input.slice(pos + spaces.length))
      let factor = defaultFactor
      if (unit) {
        const name = unit[0].toLowerCase()
        if (!Object.hasOwn(units, name)) return { valid: false, reason: 'unit' }
        factor = units[name]!
        pos += spaces.length + unit[0].length
      }
      out += `(${number[0].replace(',', '.')}*${factor})`
      continue
    }
    const identifier = IDENTIFIER.exec(rest)
    if (identifier) {
      const variable =
        options.variables && Object.hasOwn(options.variables, identifier[0])
          ? options.variables[identifier[0]]
          : undefined
      if (variable !== undefined) {
        if (!Number.isFinite(variable)) return { valid: false, reason: 'number' }
        out += `(${variable})`
      } else out += identifier[0]
      pos += identifier[0].length
      continue
    }
    out += input[pos++]
  }
  const value = evaluate(out)
  if (value === null || !Number.isFinite(value)) return { valid: false, reason: 'number' }
  if ((options.min !== undefined && value < options.min) || (options.max !== undefined && value > options.max))
    return { valid: false, reason: 'range' }
  return { valid: true, value: Object.is(value, -0) ? 0 : value }
}

/** Factor converting a length in `source` document units into `target` document units. */
export function documentUnitScale(source: DocumentLengthUnit, target: DocumentLengthUnit): number {
  return source === target ? 1 : MM_PER_UNIT[source] / MM_PER_UNIT[target]
}

export function formatLength(valueMm: number, unit: DocumentLengthUnit, decimals = 2): string {
  return `${fromMillimetres(valueMm, unit).toFixed(decimals)} ${unit}`
}
