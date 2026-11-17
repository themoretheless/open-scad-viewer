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

/** Strict parse for persisted or user-supplied units; unknown names are refused, never defaulted. */
export function parseDocumentLengthUnit(value: unknown): DocumentLengthUnit {
  if (isDocumentLengthUnit(value)) return value
  throw Error(
    `Unknown document length unit ${JSON.stringify(value)}; expected one of ${DOCUMENT_LENGTH_UNITS.join(', ')}.`,
  )
}

/** STL carries no units: its coordinates are read in the document unit and converted to millimetres. */
export function stlImportPositionsToMillimetres(positions: ArrayLike<number>, unit: DocumentLengthUnit): Float64Array {
  const factor = MM_PER_UNIT[unit]
  return Float64Array.from(positions, v => (unit === 'mm' ? v : v * factor))
}

/** STL export writes coordinates in the document unit. */
export function stlExportPositionsFromMillimetres(
  positions: ArrayLike<number>,
  unit: DocumentLengthUnit,
): Float64Array {
  const factor = MM_PER_UNIT[unit]
  return Float64Array.from(positions, v => (unit === 'mm' ? v : v / factor))
}

const STEP_SI_PREFIX: Readonly<Record<string, DocumentLengthUnit>> = { $: 'm', '.MILLI.': 'mm', '.CENTI.': 'cm' }
const STEP_ENTITY = /#(\d+)\s*=\s*((?:[^;']|'(?:[^']|'')*')*);/g

/**
 * Length unit declared by a STEP exchange file: SI metre with no, MILLI or CENTI prefix, or the
 * conversion-based INCH defined as 25.4 mm. Any other declaration is refused by name.
 */
export function stepLengthUnit(text: string): DocumentLengthUnit {
  const entities = new Map<string, string>()
  for (const m of text.matchAll(STEP_ENTITY)) entities.set(m[1]!, m[2]!)
  const siPrefix = (body: string | undefined) =>
    /SI_UNIT\(\s*(\$|\.\w+\.)\s*,\s*\.METRE\.\s*\)/i.exec(body ?? '')?.[1]?.toUpperCase()
  const lengthUnits = [...entities.values()].filter(body => /\bLENGTH_UNIT\s*\(/i.test(body))
  const declared = new Set<string>()
  for (const body of lengthUnits) {
    const conversion = /CONVERSION_BASED_UNIT\s*\(\s*'((?:[^']|'')*)'\s*,\s*#(\d+)\s*\)/i.exec(body)
    if (conversion) {
      const name = conversion[1]!.trim().toUpperCase()
      if (name !== 'INCH') throw Error(`Unknown STEP length unit '${name}'; supported: mm, cm, m, inch.`)
      const measure = /LENGTH_MEASURE\s*\(\s*([^)]+)\)\s*,\s*#(\d+)/i.exec(entities.get(conversion[2]!) ?? '')
      const prefix = siPrefix(entities.get(measure?.[2] ?? ''))
      const baseUnit = prefix ? STEP_SI_PREFIX[prefix] : undefined
      const mm = measure && baseUnit ? Number(measure[1]!.replace(/D/i, 'E')) * MM_PER_UNIT[baseUnit] : NaN
      if (!(Math.abs(mm - 25.4) <= 1e-9)) throw Error('STEP INCH unit must be defined as 25.4 mm.')
      declared.add('in')
      continue
    }
    const prefix = siPrefix(body)
    if (!prefix) continue
    const unit = STEP_SI_PREFIX[prefix]
    if (!unit) throw Error(`Unknown STEP length unit prefix ${prefix}; supported: mm, cm, m, inch.`)
    declared.add(unit)
  }
  // The INCH definition references its SI base unit; the conversion-based unit is the declared one.
  if (declared.has('in')) {
    const referenced = new Set<string>()
    for (const body of lengthUnits) {
      const conversion = /CONVERSION_BASED_UNIT\s*\(\s*'[^']*'\s*,\s*#(\d+)/i.exec(body)
      const base = conversion && /,\s*#(\d+)\s*\)\s*$/.exec(entities.get(conversion[1]!) ?? '')
      if (base) referenced.add(siPrefix(entities.get(base[1]!)) ?? '')
    }
    for (const unit of [...declared]) if (unit !== 'in' && referenced.has(unitPrefix(unit))) declared.delete(unit)
  }
  if (!declared.size) throw Error('STEP declares no length unit.')
  if (declared.size !== 1) throw Error('STEP declares more than one length unit.')
  return [...declared][0] as DocumentLengthUnit
}

function unitPrefix(unit: string): string {
  return Object.entries(STEP_SI_PREFIX).find(([, value]) => value === unit)?.[0] ?? ''
}

/**
 * STEP length-unit entities for `unit`. `add` appends one entity and returns its `#id`;
 * the returned id is the length unit to reference from contexts and measures.
 */
export function stepLengthUnitEntities(unit: DocumentLengthUnit, add: (entity: string) => string): string {
  if (unit === 'mm') return add('(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.))')
  if (unit === 'cm') return add('(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.CENTI.,.METRE.))')
  if (unit === 'm') return add('(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT($,.METRE.))')
  const mm = add('(LENGTH_UNIT() NAMED_UNIT(*) SI_UNIT(.MILLI.,.METRE.))')
  const measure = add(`LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(25.4),${mm})`)
  const exponents = add('DIMENSIONAL_EXPONENTS(1.,0.,0.,0.,0.,0.,0.)')
  return add(`(CONVERSION_BASED_UNIT('INCH',${measure}) LENGTH_UNIT() NAMED_UNIT(${exponents}))`)
}

/** Dimension label in document units; angles are unaffected by length units. */
export function formatDimensionValue(valueMm: number | null, unit: DocumentLengthUnit, angle = false): string {
  if (valueMm === null) return '—'
  return angle ? `${valueMm.toFixed(2)}°` : formatLength(valueMm, unit)
}
