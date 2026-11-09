const UTF8 = new TextEncoder()

/** Shared admission limits for untrusted exchange and mesh parsers. */
export interface ParserInputBudgets {
  readonly bytes: number
  readonly entities: number
  readonly depth: number
  readonly faces: number
  readonly controlPoints: number
}

export const DEFAULT_PARSER_INPUT_BUDGETS: ParserInputBudgets = Object.freeze({
  bytes: 16 * 1024 * 1024,
  entities: 65_536,
  depth: 32,
  faces: 250_000,
  controlPoints: 16_384,
})

export type ParserFormat = 'step' | 'iges' | 'stl'
export type ParserBudget = keyof ParserInputBudgets
export type ParserInputErrorCode =
  | 'input-too-large'
  | 'entity-limit'
  | 'depth-limit'
  | 'face-limit'
  | 'control-point-limit'

const ERROR_CODE: Readonly<Record<ParserBudget, ParserInputErrorCode>> = Object.freeze({
  bytes: 'input-too-large',
  entities: 'entity-limit',
  depth: 'depth-limit',
  faces: 'face-limit',
  controlPoints: 'control-point-limit',
})

export class ParserInputError extends Error {
  constructor(
    readonly format: ParserFormat,
    readonly budget: ParserBudget,
    readonly observed: number,
    readonly limit: number,
  ) {
    super(`${format.toUpperCase()} ${budget} budget exceeded: ${observed} > ${limit}`)
    this.name = 'ParserInputError'
  }

  get code(): ParserInputErrorCode { return ERROR_CODE[this.budget] }
}

export interface ParserPreflightStats {
  readonly format: ParserFormat
  readonly bytes: number
  readonly entities: number
  readonly depth: number
  readonly faces: number
  readonly controlPoints: number
}

function charge(
  format: ParserFormat,
  budgets: ParserInputBudgets,
  budget: ParserBudget,
  observed: number,
): void {
  if (observed > budgets[budget]) {
    throw new ParserInputError(format, budget, observed, budgets[budget])
  }
}

function stats(format: ParserFormat, values: Omit<ParserPreflightStats, 'format'>): ParserPreflightStats {
  return Object.freeze({ format, ...values })
}

/** Lexical depth only: strings and comments cannot manufacture nesting. */
function stepDepth(text: string): number {
  let depth = 0
  let maximum = 0
  let quote = false
  let comment = false
  for (let i = 0; i < text.length; i++) {
    const c = text[i], next = text[i + 1]
    if (comment) {
      if (c === '*' && next === '/') { comment = false; i++ }
      continue
    }
    if (quote) {
      if (c === "'" && next === "'") i++
      else if (c === "'") quote = false
      continue
    }
    if (c === '/' && next === '*') { comment = true; i++; continue }
    if (c === "'") { quote = true; continue }
    if (c === '(') { depth++; maximum = Math.max(maximum, depth) }
    else if (c === ')') depth = Math.max(0, depth - 1)
  }
  return maximum
}

function stepEntitySlices(text: string): Array<{ type: string; args: string }> {
  const entities: Array<{ type: string; args: string }> = []
  const expression = /#(\d+)\s*=\s*([A-Z_0-9]+)\s*\(((?:[^']|'(?:[^']|'')*')*?)\)\s*;/gis
  for (const match of text.matchAll(expression)) entities.push({ type: match[2]!.toUpperCase(), args: match[3]! })
  return entities
}

export function preflightStepInput(
  text: string,
  budgets: ParserInputBudgets = DEFAULT_PARSER_INPUT_BUDGETS,
): ParserPreflightStats {
  const bytes = UTF8.encode(text).byteLength
  charge('step', budgets, 'bytes', bytes)
  const entities = stepEntitySlices(text)
  charge('step', budgets, 'entities', entities.length)
  const depth = stepDepth(text)
  charge('step', budgets, 'depth', depth)
  const faces = entities.reduce((count, entity) => count + (/^(?:ADVANCED_FACE|FACE_SURFACE|FACE|TRIANGULATED_FACE_SET|COMPLEX_TRIANGULATED_FACE_SET)$/.test(entity.type) ? 1 : 0), 0)
  charge('step', budgets, 'faces', faces)
  // Conservative lexical admission: B-spline reference lists are charged before
  // native semantic expansion. The native parser applies its exact pole/net checks.
  const controlPoints = entities.reduce((total, entity) => {
    if (!/^B_SPLINE_(?:CURVE|SURFACE)/.test(entity.type)) return total
    const refs = entity.args.replace(/'(?:[^']|'')*'/g, "''").match(/#\d+/g)?.length ?? 0
    return total + Math.max(0, refs - 1)
  }, 0)
  charge('step', budgets, 'controlPoints', controlPoints)
  return stats('step', { bytes, entities: entities.length, depth, faces, controlPoints })
}

export function preflightIgesInput(
  text: string,
  budgets: ParserInputBudgets = DEFAULT_PARSER_INPUT_BUDGETS,
): ParserPreflightStats {
  const bytes = UTF8.encode(text).byteLength
  charge('iges', budgets, 'bytes', bytes)
  const lines = text.split(/\r?\n/)
  let entities = 0, faces = 0, controlPoints = 0
  for (const line of lines) {
    if (line.length < 73 || line[72] !== 'D') continue
    const type = Number.parseInt(line.slice(0, 8).trim(), 10)
    const sequence = Number.parseInt(line.slice(73, 80).trim(), 10)
    if (sequence % 2 === 1) {
      entities++
      if (type === 510) faces++
    }
  }
  charge('iges', budgets, 'entities', entities)
  charge('iges', budgets, 'faces', faces)
  // Parameter rows are bounded lexically. Exact composite depth and control-net
  // accounting remains in the native IGES graph pass.
  const parameterRows = lines.reduce((count, line) => count + (line.length >= 73 && line[72] === 'P' ? 1 : 0), 0)
  charge('iges', budgets, 'depth', parameterRows === 0 ? 0 : 1)
  for (const line of lines) {
    if (line.length < 73 || line[72] !== 'P') continue
    const payload = line.slice(0, 64)
    const curve = /^\s*126\s*,\s*(\d+)\s*,/.exec(payload)
    if (curve) controlPoints += Number(curve[1]) + 1
    const surface = /^\s*128\s*,\s*(\d+)\s*,\s*(\d+)(?=\s*[,;])/.exec(payload)
    if (surface) controlPoints += (Number(surface[1]) + 1) * (Number(surface[2]) + 1)
    charge('iges', budgets, 'controlPoints', controlPoints)
  }
  return stats('iges', { bytes, entities, depth: parameterRows === 0 ? 0 : 1, faces, controlPoints })
}

export function preflightStlInput(
  input: ArrayBuffer | Uint8Array,
  budgets: ParserInputBudgets = DEFAULT_PARSER_INPUT_BUDGETS,
): ParserPreflightStats {
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input)
  charge('stl', budgets, 'bytes', bytes.byteLength)
  let entities = 0
  if (bytes.byteLength >= 84) {
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
    const declared = view.getUint32(80, true)
    if (84 + declared * 50 === bytes.byteLength) entities = declared
  }
  if (entities === 0) {
    const decoded = new TextDecoder('utf-8', { fatal: false }).decode(bytes)
    entities = decoded.match(/^\s*facet\s+normal\b/gim)?.length ?? 0
  }
  charge('stl', budgets, 'entities', entities)
  charge('stl', budgets, 'faces', entities)
  return stats('stl', { bytes: bytes.byteLength, entities, depth: entities === 0 ? 0 : 1, faces: entities, controlPoints: 0 })
}
