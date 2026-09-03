/**
 * Kernel-neutral pieces of the OpenSCAD 2021.01 built-in-module contract.
 *
 * These helpers deliberately do not know about the parser, Manifold, or the
 * semantic-plan builder.  Both independent execution lanes can therefore use
 * the same observable language rules without making the upstream runtime a
 * production dependency.
 */

export type OpenScadRgba = readonly [number, number, number, number]

export type OpenScadStableModuleWarningCode =
  | 'OPENSCAD_COLOR_UNKNOWN'
  | 'OPENSCAD_COLOR_CHANNEL_RANGE'
  | 'OPENSCAD_COLOR_ALPHA_RANGE'
  | 'OPENSCAD_FN_NEGATIVE'
  | 'OPENSCAD_FRAGMENT_NON_FINITE'
  | 'OPENSCAD_FA_TOO_SMALL'
  | 'OPENSCAD_FS_TOO_SMALL'
  | 'OPENSCAD_FRAGMENTS_CLAMPED'

export interface OpenScadStableModuleWarning {
  readonly code: OpenScadStableModuleWarningCode
  readonly message: string
  readonly value?: number | string
  readonly limit?: number
}

export interface OpenScadStableModuleSemanticsContext {
  warn(warning: OpenScadStableModuleWarning): void
}

const SILENT_CONTEXT: OpenScadStableModuleSemanticsContext = Object.freeze({ warn() {} })

/* CSS/SVG named colors accepted by OpenSCAD, plus the CSS transparent keyword. */
export const OPENSCAD_NAMED_COLOR_HEX = Object.freeze({
  aliceblue: 'F0F8FF',
  antiquewhite: 'FAEBD7',
  aqua: '00FFFF',
  aquamarine: '7FFFD4',
  azure: 'F0FFFF',
  beige: 'F5F5DC',
  bisque: 'FFE4C4',
  black: '000000',
  blanchedalmond: 'FFEBCD',
  blue: '0000FF',
  blueviolet: '8A2BE2',
  brown: 'A52A2A',
  burlywood: 'DEB887',
  cadetblue: '5F9EA0',
  chartreuse: '7FFF00',
  chocolate: 'D2691E',
  coral: 'FF7F50',
  cornflowerblue: '6495ED',
  cornsilk: 'FFF8DC',
  crimson: 'DC143C',
  cyan: '00FFFF',
  darkblue: '00008B',
  darkcyan: '008B8B',
  darkgoldenrod: 'B8860B',
  darkgray: 'A9A9A9',
  darkgreen: '006400',
  darkgrey: 'A9A9A9',
  darkkhaki: 'BDB76B',
  darkmagenta: '8B008B',
  darkolivegreen: '556B2F',
  darkorange: 'FF8C00',
  darkorchid: '9932CC',
  darkred: '8B0000',
  darksalmon: 'E9967A',
  darkseagreen: '8FBC8F',
  darkslateblue: '483D8B',
  darkslategray: '2F4F4F',
  darkslategrey: '2F4F4F',
  darkturquoise: '00CED1',
  darkviolet: '9400D3',
  deeppink: 'FF1493',
  deepskyblue: '00BFFF',
  dimgray: '696969',
  dimgrey: '696969',
  dodgerblue: '1E90FF',
  firebrick: 'B22222',
  floralwhite: 'FFFAF0',
  forestgreen: '228B22',
  fuchsia: 'FF00FF',
  gainsboro: 'DCDCDC',
  ghostwhite: 'F8F8FF',
  gold: 'FFD700',
  goldenrod: 'DAA520',
  gray: '808080',
  green: '008000',
  greenyellow: 'ADFF2F',
  grey: '808080',
  honeydew: 'F0FFF0',
  hotpink: 'FF69B4',
  indianred: 'CD5C5C',
  indigo: '4B0082',
  ivory: 'FFFFF0',
  khaki: 'F0E68C',
  lavender: 'E6E6FA',
  lavenderblush: 'FFF0F5',
  lawngreen: '7CFC00',
  lemonchiffon: 'FFFACD',
  lightblue: 'ADD8E6',
  lightcoral: 'F08080',
  lightcyan: 'E0FFFF',
  lightgoldenrodyellow: 'FAFAD2',
  lightgray: 'D3D3D3',
  lightgreen: '90EE90',
  lightgrey: 'D3D3D3',
  lightpink: 'FFB6C1',
  lightsalmon: 'FFA07A',
  lightseagreen: '20B2AA',
  lightskyblue: '87CEFA',
  lightslategray: '778899',
  lightslategrey: '778899',
  lightsteelblue: 'B0C4DE',
  lightyellow: 'FFFFE0',
  lime: '00FF00',
  limegreen: '32CD32',
  linen: 'FAF0E6',
  magenta: 'FF00FF',
  maroon: '800000',
  mediumaquamarine: '66CDAA',
  mediumblue: '0000CD',
  mediumorchid: 'BA55D3',
  mediumpurple: '9370DB',
  mediumseagreen: '3CB371',
  mediumslateblue: '7B68EE',
  mediumspringgreen: '00FA9A',
  mediumturquoise: '48D1CC',
  mediumvioletred: 'C71585',
  midnightblue: '191970',
  mintcream: 'F5FFFA',
  mistyrose: 'FFE4E1',
  moccasin: 'FFE4B5',
  navajowhite: 'FFDEAD',
  navy: '000080',
  oldlace: 'FDF5E6',
  olive: '808000',
  olivedrab: '6B8E23',
  orange: 'FFA500',
  orangered: 'FF4500',
  orchid: 'DA70D6',
  palegoldenrod: 'EEE8AA',
  palegreen: '98FB98',
  paleturquoise: 'AFEEEE',
  palevioletred: 'DB7093',
  papayawhip: 'FFEFD5',
  peachpuff: 'FFDAB9',
  peru: 'CD853F',
  pink: 'FFC0CB',
  plum: 'DDA0DD',
  powderblue: 'B0E0E6',
  purple: '800080',
  rebeccapurple: '663399',
  red: 'FF0000',
  rosybrown: 'BC8F8F',
  royalblue: '4169E1',
  saddlebrown: '8B4513',
  salmon: 'FA8072',
  sandybrown: 'F4A460',
  seagreen: '2E8B57',
  seashell: 'FFF5EE',
  sienna: 'A0522D',
  silver: 'C0C0C0',
  skyblue: '87CEEB',
  slateblue: '6A5ACD',
  slategray: '708090',
  slategrey: '708090',
  snow: 'FFFAFA',
  springgreen: '00FF7F',
  steelblue: '4682B4',
  tan: 'D2B48C',
  teal: '008080',
  thistle: 'D8BFD8',
  tomato: 'FF6347',
  transparent: '00000000',
  turquoise: '40E0D0',
  violet: 'EE82EE',
  wheat: 'F5DEB3',
  white: 'FFFFFF',
  whitesmoke: 'F5F5F5',
  yellow: 'FFFF00',
  yellowgreen: '9ACD32',
} as const)

export type OpenScadNamedColor = keyof typeof OPENSCAD_NAMED_COLOR_HEX

function hexRgba(hex: string): OpenScadRgba {
  return Object.freeze([
    Number.parseInt(hex.slice(0, 2), 16) / 255,
    Number.parseInt(hex.slice(2, 4), 16) / 255,
    Number.parseInt(hex.slice(4, 6), 16) / 255,
    hex.length === 8 ? Number.parseInt(hex.slice(6, 8), 16) / 255 : 1,
  ]) as OpenScadRgba
}

function expandCssHex(value: string): string | null {
  const match = /^#([0-9a-f]{3}|[0-9a-f]{4}|[0-9a-f]{6}|[0-9a-f]{8})$/iu.exec(value)
  if (!match) return null
  const digits = match[1]
  return digits.length <= 4
    ? [...digits].map(digit => `${digit}${digit}`).join('')
    : digits
}

export interface OpenScadColorResolution {
  readonly rgba: OpenScadRgba
  /** False is OpenSCAD's sentinel color, used for a non-color value or unknown name. */
  readonly valid: boolean
  readonly source: 'vector' | 'named' | 'hex' | 'invalid'
}

const INVALID_COLOR = Object.freeze([-1, -1, -1, -1]) as OpenScadRgba

function warnColorRanges(
  rgba: OpenScadRgba,
  context: OpenScadStableModuleSemanticsContext,
): void {
  for (let index = 0; index < 3; index++) {
    const value = rgba[index]
    if (value < 0 || value > 1) context.warn({
      code: 'OPENSCAD_COLOR_CHANNEL_RANGE',
      message: `Color channel ${index} is outside the inclusive 0..1 range`,
      value,
    })
  }
  const alpha = rgba[3]
  if (alpha < 0 || alpha > 1) context.warn({
    code: 'OPENSCAD_COLOR_ALPHA_RANGE',
    message: 'The color vector alpha channel is outside the inclusive 0..1 range',
    value: alpha,
  })
}

/**
 * Resolve color(c, alpha) exactly at the language boundary.
 *
 * OpenSCAD does not clamp authored channels. Missing vector channels are 1,
 * non-number vector entries convert to 0, and only a numeric alpha argument
 * overrides the alpha embedded in a vector/name.
 */
export function resolveOpenScadColor(
  value: unknown,
  alpha: unknown = undefined,
  context: OpenScadStableModuleSemanticsContext = SILENT_CONTEXT,
): OpenScadColorResolution {
  let rgba: OpenScadRgba
  let source: OpenScadColorResolution['source']

  if (Array.isArray(value)) {
    rgba = Object.freeze([0, 1, 2, 3].map(index => {
      if (index >= value.length) return 1
      return typeof value[index] === 'number' ? value[index] : 0
    })) as unknown as OpenScadRgba
    source = 'vector'
  } else if (typeof value === 'string') {
    const expanded = expandCssHex(value)
    if (expanded !== null) {
      rgba = hexRgba(expanded)
      source = 'hex'
    } else {
      const named = OPENSCAD_NAMED_COLOR_HEX[value.toLowerCase() as OpenScadNamedColor]
      if (named === undefined) {
        context.warn({
          code: 'OPENSCAD_COLOR_UNKNOWN',
          message: `Unknown OpenSCAD color name: ${value}`,
          value,
        })
        return Object.freeze({ rgba: INVALID_COLOR, valid: false, source: 'invalid' })
      }
      rgba = hexRgba(named)
      source = 'named'
    }
  } else {
    return Object.freeze({ rgba: INVALID_COLOR, valid: false, source: 'invalid' })
  }

  // OpenSCAD 2021 validates the authored vector before applying the separate
  // alpha argument. The override itself is accepted without a range warning.
  if (source === 'vector') warnColorRanges(rgba, context)
  if (typeof alpha === 'number') rgba = Object.freeze([rgba[0], rgba[1], rgba[2], alpha])
  return Object.freeze({ rgba, valid: true, source })
}

export const OPENSCAD_DEFAULT_FN = 0
export const OPENSCAD_DEFAULT_FA = 12
export const OPENSCAD_DEFAULT_FS = 2
export const OPENSCAD_MIN_FA = 0.01
export const OPENSCAD_MIN_FS = 0.01
export const OPENSCAD_2021_GEOMETRY_EPSILON = 0.00000095367431640625
export const OPENSCAD_FULL_MAX_FRAGMENTS = 256
export const OPENSCAD_PREVIEW_MAX_FRAGMENTS = 48

export interface OpenScadFragmentResolutionInput {
  readonly radius: number
  readonly fn?: unknown
  readonly fa?: unknown
  readonly fs?: unknown
  readonly quality?: 'preview' | 'full'
  /** An engine safety limit. The upstream language itself has no such small cap. */
  readonly maximum?: number
}

export interface OpenScadFragmentResolution {
  readonly fragments: number
  /** Count requested by OpenSCAD's formula before this engine's safety bound. */
  readonly unboundedFragments: number
  readonly source: '$fn' | '$fa/$fs' | 'geometry-epsilon'
  readonly effectiveFn: number
  readonly effectiveFa: number
  readonly effectiveFs: number
  readonly maximum: number
  readonly reduced: boolean
}

function numericOrZero(value: unknown): number {
  return typeof value === 'number' ? value : 0
}

function stableSpecial(
  input: OpenScadFragmentResolutionInput,
  name: 'fn' | 'fa' | 'fs',
  fallback: number,
): number {
  return Object.prototype.hasOwnProperty.call(input, name) ? numericOrZero(input[name]) : fallback
}

/** OpenSCAD's full-circle fragment formula with explicit host safety bounds. */
export function resolveOpenScadFragments(
  input: OpenScadFragmentResolutionInput,
  context: OpenScadStableModuleSemanticsContext = SILENT_CONTEXT,
): OpenScadFragmentResolution {
  const maximum = input.maximum
    ?? (input.quality === 'preview' ? OPENSCAD_PREVIEW_MAX_FRAGMENTS : OPENSCAD_FULL_MAX_FRAGMENTS)
  if (!Number.isSafeInteger(maximum) || maximum < 3) {
    throw new RangeError('OpenSCAD fragment maximum must be a safe integer of at least 3')
  }

  const fn = stableSpecial(input, 'fn', OPENSCAD_DEFAULT_FN)
  const nonFiniteFn = !Number.isFinite(fn)
  let effectiveFn = fn
  if (nonFiniteFn) {
    context.warn({
      code: 'OPENSCAD_FRAGMENT_NON_FINITE',
      message: 'Non-finite $fn selected the minimum fragment count',
      value: fn,
    })
  } else if (effectiveFn < 0) {
    context.warn({
      code: 'OPENSCAD_FN_NEGATIVE',
      message: 'Negative $fn was replaced with the automatic fragment mode',
      value: effectiveFn,
    })
    effectiveFn = 0
  }

  let fa = stableSpecial(input, 'fa', OPENSCAD_DEFAULT_FA)
  let fs = stableSpecial(input, 'fs', OPENSCAD_DEFAULT_FS)
  if (Number.isNaN(fa)) {
    context.warn({
      code: 'OPENSCAD_FRAGMENT_NON_FINITE',
      message: 'Non-finite $fa was replaced with its default',
      value: fa,
    })
    fa = OPENSCAD_DEFAULT_FA
  }
  if (Number.isNaN(fs)) {
    context.warn({
      code: 'OPENSCAD_FRAGMENT_NON_FINITE',
      message: 'Non-finite $fs was replaced with its default',
      value: fs,
    })
    fs = OPENSCAD_DEFAULT_FS
  }
  if (fa < OPENSCAD_MIN_FA) {
    context.warn({
      code: 'OPENSCAD_FA_TOO_SMALL',
      message: `$fa was raised to the OpenSCAD minimum ${OPENSCAD_MIN_FA}`,
      value: fa,
      limit: OPENSCAD_MIN_FA,
    })
    fa = OPENSCAD_MIN_FA
  }
  if (fs < OPENSCAD_MIN_FS) {
    context.warn({
      code: 'OPENSCAD_FS_TOO_SMALL',
      message: `$fs was raised to the OpenSCAD minimum ${OPENSCAD_MIN_FS}`,
      value: fs,
      limit: OPENSCAD_MIN_FS,
    })
    fs = OPENSCAD_MIN_FS
  }

  const radius = Number.isNaN(input.radius) ? 0 : Math.abs(input.radius)
  let source: OpenScadFragmentResolution['source']
  let unboundedFragments: number
  if (radius < OPENSCAD_2021_GEOMETRY_EPSILON || nonFiniteFn) {
    source = 'geometry-epsilon'
    unboundedFragments = 3
  } else if (effectiveFn > 0) {
    source = '$fn'
    unboundedFragments = Math.trunc(Math.max(effectiveFn, 3))
  } else {
    source = '$fa/$fs'
    const calculated = Math.max(Math.min(360 / fa, radius * 2 * Math.PI / fs), 5)
    unboundedFragments = Math.ceil(calculated)
    if (!Number.isFinite(unboundedFragments)) {
      context.warn({
        code: 'OPENSCAD_FRAGMENT_NON_FINITE',
        message: 'Non-finite fragment resolution was bounded by the engine safety limit',
        value: unboundedFragments,
        limit: maximum,
      })
      unboundedFragments = maximum
    }
  }

  const fragments = Math.min(unboundedFragments, maximum)
  const reduced = fragments !== unboundedFragments
  if (reduced) context.warn({
    code: 'OPENSCAD_FRAGMENTS_CLAMPED',
    message: `Fragment count was clamped to the engine safety limit ${maximum}`,
    value: unboundedFragments,
    limit: maximum,
  })
  return Object.freeze({ fragments, unboundedFragments, source, effectiveFn, effectiveFa: fa, effectiveFs: fs, maximum, reduced })
}

export interface OpenScadSweepFragmentResolution extends OpenScadFragmentResolution {
  readonly sweepDegrees: number
  readonly sweepFragments: number
}

/** Scale a full-circle resolution for rotate_extrude's bounded 0..360 degree sweep. */
export function resolveOpenScadSweepFragments(
  input: OpenScadFragmentResolutionInput & { readonly sweepDegrees: number },
  context: OpenScadStableModuleSemanticsContext = SILENT_CONTEXT,
): OpenScadSweepFragmentResolution {
  const full = resolveOpenScadFragments(input, context)
  const authoredSweep = Number.isFinite(input.sweepDegrees) ? Math.abs(input.sweepDegrees) : 360
  const sweepDegrees = Math.min(authoredSweep, 360)
  const sweepFragments = sweepDegrees === 0
    ? 0
    : sweepDegrees === 360
      ? full.fragments
      : Math.max(1, Math.floor(full.fragments * sweepDegrees / 360))
  return Object.freeze({ ...full, sweepDegrees, sweepFragments })
}

export interface OpenScadOffsetResolutionInput {
  readonly r?: unknown
  readonly delta?: unknown
  readonly chamfer?: unknown
}

export interface OpenScadOffsetResolution {
  readonly mode: 'radius' | 'delta'
  readonly distance: number
  readonly joinType: 'Round' | 'Miter' | 'Square'
  readonly chamfer: boolean
}

/** Resolve offset()'s mutually exclusive r/delta modes and Manifold join mode. */
export function resolveOpenScadOffset(
  input: OpenScadOffsetResolutionInput,
  _context: OpenScadStableModuleSemanticsContext = SILENT_CONTEXT,
): OpenScadOffsetResolution {
  const hasR = typeof input.r === 'number'
  const hasDelta = typeof input.delta === 'number'
  if (hasR) {
    return Object.freeze({
      mode: 'radius',
      distance: input.r as number,
      joinType: 'Round',
      chamfer: false,
    })
  }
  if (hasDelta) {
    const chamfer = input.chamfer === true
    return Object.freeze({
      mode: 'delta',
      distance: input.delta as number,
      joinType: chamfer ? 'Square' : 'Miter',
      chamfer,
    })
  }
  return Object.freeze({ mode: 'radius', distance: 1, joinType: 'Round', chamfer: false })
}
