import { callGeometryRust } from './geometry/kernel'
export type QuantityKind = 'length' | 'angle' | 'scalar'
export type QuantityResult = { valid: true; value: number } | { valid: false; reason: 'number' | 'unit' | 'range' }

const UNIT_FACTORS: Readonly<Record<QuantityKind, Readonly<Record<string, number>>>> = {
  length: { mm: 1, cm: 10, m: 1000, in: 25.4 },
  angle: { deg: 1, '°': 1, rad: 180 / Math.PI },
  scalar: {},
}

const KNOWN_UNITS = new Set(['mm', 'cm', 'm', 'in', 'deg', '°', 'rad'])

/**
 * Safe recursive-descent math expression evaluator (no eval/Function).
 * Supports +, -, *, /, ^, parentheses, constants (pi, e, tau), and functions
 * (sqrt, sin, cos, tan, abs, min, max, round, floor, ceil).
 * Trigonometric functions take degrees, matching OpenSCAD conventions.
 */
export function evaluateSafeMathExpression(expression: string): number | null {
  const input = expression.trim()
  if (!input || input.length > 256) return null
  let pos = 0
  let depth = 0

  const skipSpaces = () => {
    while (pos < input.length && /\s/.test(input[pos])) pos++
  }

  const parseExpression = (): number => {
    let left = parseTerm()
    skipSpaces()
    while (pos < input.length && (input[pos] === '+' || input[pos] === '-')) {
      const op = input[pos++]
      const right = parseTerm()
      left = op === '+' ? left + right : left - right
      skipSpaces()
    }
    return left
  }

  const parseTerm = (): number => {
    let left = parsePower()
    skipSpaces()
    while (pos < input.length && (input[pos] === '*' || input[pos] === '/')) {
      const op = input[pos++]
      const right = parsePower()
      if (op === '/') {
        if (right === 0) return Number.NaN
        left /= right
      } else {
        left *= right
      }
      skipSpaces()
    }
    return left
  }

  const parsePower = (): number => {
    const base = parseUnary()
    skipSpaces()
    if (pos < input.length && input[pos] === '^') {
      pos++
      const exp = parsePower()
      return Math.pow(base, exp)
    }
    return base
  }

  const parseUnary = (): number => {
    skipSpaces()
    if (pos < input.length && (input[pos] === '+' || input[pos] === '-')) {
      const sign = input[pos++] === '-' ? -1 : 1
      return sign * parseUnary()
    }
    return parsePrimary()
  }

  const parsePrimary = (): number => {
    skipSpaces()
    if (pos >= input.length) return Number.NaN
    if (input[pos] === '(') {
      if (++depth > 32) return Number.NaN
      pos++
      const val = parseExpression()
      skipSpaces()
      if (pos >= input.length || input[pos] !== ')') return Number.NaN
      pos++
      depth--
      return val
    }
    // Identifier (function call or constant)
    const idMatch = /^[A-Za-z_][A-Za-z0-9_]*/.exec(input.slice(pos))
    if (idMatch) {
      const name = idMatch[0].toLowerCase()
      pos += idMatch[0].length
      skipSpaces()
      if (pos < input.length && input[pos] === '(') {
        if (++depth > 32) return Number.NaN
        pos++
        const args: number[] = []
        skipSpaces()
        if (pos < input.length && input[pos] !== ')') {
          args.push(parseExpression())
          skipSpaces()
          while (pos < input.length && input[pos] === ',') {
            pos++
            args.push(parseExpression())
            skipSpaces()
          }
        }
        if (pos >= input.length || input[pos] !== ')') return Number.NaN
        pos++
        depth--
        if (name === 'sqrt' && args.length === 1) return args[0] < 0 ? Number.NaN : Math.sqrt(args[0])
        if (name === 'abs' && args.length === 1) return Math.abs(args[0])
        if (name === 'sin' && args.length === 1) return Math.sin((args[0] * Math.PI) / 180)
        if (name === 'cos' && args.length === 1) return Math.cos((args[0] * Math.PI) / 180)
        if (name === 'tan' && args.length === 1) return Math.tan((args[0] * Math.PI) / 180)
        if (name === 'round' && args.length === 1) return Math.round(args[0])
        if (name === 'floor' && args.length === 1) return Math.floor(args[0])
        if (name === 'ceil' && args.length === 1) return Math.ceil(args[0])
        if (name === 'min' && args.length >= 1) return Math.min(...args)
        if (name === 'max' && args.length >= 1) return Math.max(...args)
        return Number.NaN
      }
      if (name === 'pi') return Math.PI
      if (name === 'e') return Math.E
      if (name === 'tau') return Math.PI * 2
      return Number.NaN
    }
    // Numeric literal (supports decimal point or decimal comma when not inside function arg list)
    const numMatch = /^(?:\d+(?:[.,]\d*)?|[.,]\d+)(?:[eE][+-]?\d+)?/.exec(input.slice(pos))
    if (!numMatch) return Number.NaN
    pos += numMatch[0].length
    const normalized = numMatch[0].replace(',', '.')
    const num = Number(normalized)
    return Number.isFinite(num) ? num : Number.NaN
  }

  const value = parseExpression()
  skipSpaces()
  if (pos !== input.length || !Number.isFinite(value) || Math.abs(value) > 1e300) return null
  return Object.is(value, -0) ? 0 : value
}

export function parseCadQuantity(text: string, kind: QuantityKind, min?: number, max?: number): QuantityResult {
  const direct = callGeometryRust('cad_quantity', { text, kind, min, max }) as QuantityResult
  if (direct.valid) return direct
  const trimmed = text.trim()
  if (!trimmed) return direct
  // Extract optional trailing unit (e.g. "10 + 5 mm" or "(25.4 * 2) in" or "sqrt(2) * 10")
  const unitMatch = /^(.*?)(?:\s*(mm|cm|m|in|deg|rad|°))?$/i.exec(trimmed)
  if (!unitMatch) return direct
  const exprPart = unitMatch[1]?.trim() ?? ''
  const rawUnit = unitMatch[2]?.toLowerCase() ?? ''
  if (!exprPart || !/[+\-*/^()]/.test(exprPart)) return direct
  if (rawUnit && !KNOWN_UNITS.has(rawUnit)) return { valid: false, reason: 'unit' }
  const allowedUnits = UNIT_FACTORS[kind]
  if (rawUnit && !(rawUnit in allowedUnits)) return { valid: false, reason: 'unit' }
  const evaluated = evaluateSafeMathExpression(exprPart)
  if (evaluated === null || !Number.isFinite(evaluated)) return { valid: false, reason: 'number' }
  const factor = rawUnit ? allowedUnits[rawUnit] : 1
  const converted = evaluated * factor
  if (!Number.isFinite(converted)) return { valid: false, reason: 'number' }
  if ((min !== undefined && converted < min) || (max !== undefined && converted > max)) {
    return { valid: false, reason: 'range' }
  }
  return { valid: true, value: Object.is(converted, -0) ? 0 : converted }
}
