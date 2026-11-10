import {
  OpenScadProject,
  resolveOpenScadProjectPath,
  type OpenScadProjectBlobFile,
} from './openScadProject'
import type { CallNode } from './openscadCompiler'
import { OpenSCADParseError } from './openscadErrors'
import { isWellFormedUnicode } from '../core/unicode'
import { u32 } from '../core/bytes'
import { textError } from './openScadText/prepareOpenScadTextAssets'
export { prepareOpenScadTextAssets, calculateOpenScadTextSegments } from './openScadText/prepareOpenScadTextAssets'
export { renderOpenScadText } from './openScadText/renderOpenScadText'

export const OPENSCAD_TEXT_MAX_FONTS = 32
export const OPENSCAD_TEXT_MAX_FACES_PER_COLLECTION = 32
export const OPENSCAD_TEXT_MAX_NAME_RECORDS = 512
export const OPENSCAD_TEXT_MAX_NAME_TABLE_BYTES = 512 * 1024
export const OPENSCAD_TEXT_MAX_CODE_POINTS = 4_096
export const OPENSCAD_TEXT_MAX_GLYPHS = 8_192
export const OPENSCAD_TEXT_MAX_PATH_COMMANDS = 100_000
export const OPENSCAD_TEXT_MAX_VERTICES = 250_000
export const OPENSCAD_TEXT_MAX_CURVE_SEGMENTS = 256
export const OPENSCAD_TEXT_MAX_ABS_SIZE = 1_000_000
export const OPENSCAD_TEXT_MAX_ABS_SPACING = 1_000_000
export const OPENSCAD_2021_GRID_FINE = 0.00000095367431640625

/**
 * FreeType 2.10 rounds OpenSCAD's fixed 100000/64 point size at 100 DPI to
 * 2170 pixels. OpenSCAD then divides outline coordinates by 100000. This is
 * 2170 * 64 / 100000 = 1.3888 em units before the authored `size` multiplier.
 */
export const OPENSCAD_2021_TEXT_EM_SCALE = 1.3888

export const OPENSCAD_TEXT_ERROR_CODES = Object.freeze([
  'E_TEXT_PROJECT_REQUIRED',
  'E_TEXT_PROJECT_MISMATCH',
  'E_TEXT_PARAMETER_INVALID',
  'E_TEXT_PARAMETER_LIMIT',
  'E_TEXT_FONT_LIMIT',
  'E_TEXT_FONT_NOT_FOUND',
  'E_TEXT_FONT_INVALID',
  'E_TEXT_FONT_UNSUPPORTED',
  'E_TEXT_RUNTIME_UNAVAILABLE',
  'E_TEXT_GLYPH_LIMIT',
  'E_TEXT_PATH_LIMIT',
  'E_TEXT_VERTEX_LIMIT',
  'E_TEXT_SHAPING_FAILED',
] as const)

export type OpenScadTextErrorCode = typeof OPENSCAD_TEXT_ERROR_CODES[number]

export interface OpenScadTextErrorDetails {
  readonly sourcePath?: string
  readonly font?: string
  readonly fontPath?: string
  readonly faceIndex?: number
  readonly limit?: number
  readonly actual?: number
}

export class OpenScadTextError extends Error {
  readonly details: Readonly<OpenScadTextErrorDetails>

  constructor(
    readonly code: OpenScadTextErrorCode,
    message: string,
    details: OpenScadTextErrorDetails = {},
    options: ErrorOptions = {},
  ) {
    super(message, options)
    this.name = 'OpenScadTextError'
    this.details = Object.freeze({ ...details })
  }
}

export interface OpenScadTextPositionedErrorDetails extends OpenScadTextErrorDetails {
  readonly sourcePath: string
}

/** Project-positioned text diagnostic suitable for one authored call. */
export class OpenScadTextPositionedError extends OpenSCADParseError {
  readonly details: Readonly<OpenScadTextPositionedErrorDetails>
  readonly sourcePath: string
  readonly font?: string
  readonly fontPath?: string
  readonly faceIndex?: number
  readonly limit?: number
  readonly actual?: number

  constructor(
    source: string,
    node: Pick<CallNode, 'p' | 'end'>,
    readonly textCode: OpenScadTextErrorCode,
    message: string,
    details: OpenScadTextPositionedErrorDetails,
  ) {
    super(source, node.p, message, textCode, node.end)
    this.name = 'OpenScadTextPositionedError'
    this.details = Object.freeze({ ...details })
    this.sourcePath = details.sourcePath
    this.font = details.font
    this.fontPath = details.fontPath
    this.faceIndex = details.faceIndex
    this.limit = details.limit
    this.actual = details.actual
  }
}

export interface OpenScadTextFont {
  readonly path: string
  readonly sha256: string
  readonly faceIndex: number
  readonly family: string
  readonly style: string
  readonly fullName: string
  readonly postscriptName: string
  readonly unitsPerEm: number
}

export interface RejectedOpenScadTextFont {
  readonly path: string
  readonly code: 'E_TEXT_FONT_INVALID' | 'E_TEXT_FONT_UNSUPPORTED'
  readonly message: string
}

export interface PreparedOpenScadTextAssets {
  readonly projectSha256: string
  readonly fonts: readonly OpenScadTextFont[]
  readonly rejected: readonly RejectedOpenScadTextFont[]
}

export type OpenScadTextDirection = 'ltr' | 'rtl' | 'ttb' | 'btt'
export type OpenScadTextHorizontalAlignment = 'left' | 'center' | 'right'
export type OpenScadTextVerticalAlignment = 'baseline' | 'bottom' | 'center' | 'top'
export type OpenScadTextPoint = readonly [number, number]
export type OpenScadTextContour = readonly OpenScadTextPoint[]

export interface OpenScadTextParameters {
  readonly text?: string
  readonly size?: number
  readonly font?: string
  readonly halign?: string
  readonly valign?: string
  readonly spacing?: number
  readonly direction?: string
  readonly language?: string
  readonly script?: string
  readonly $fn?: number
  readonly $fa?: number
  readonly $fs?: number
}

export interface OpenScadTextGlyph {
  readonly glyphId: number
  /** UTF-16 source index, matching the HarfBuzz/OpenSCAD cluster contract. */
  readonly cluster: number
  readonly flags: number
  readonly offset: OpenScadTextPoint
  readonly advance: OpenScadTextPoint
  /** All contours for this glyph; consume this one glyph with EvenOdd fill. */
  readonly contours: readonly OpenScadTextContour[]
}

export interface OpenScadTextLayout {
  readonly font: OpenScadTextFont
  readonly direction: OpenScadTextDirection
  readonly script: string
  readonly language: string
  readonly halign: OpenScadTextHorizontalAlignment
  readonly valign: OpenScadTextVerticalAlignment
  readonly spacing: number
  readonly size: number
  readonly curveSegments: number
  readonly glyphs: readonly OpenScadTextGlyph[]
  readonly advance: OpenScadTextPoint
  readonly width: number
  readonly ascent: number
  readonly descent: number
  readonly bounds: Readonly<{ min: OpenScadTextPoint; max: OpenScadTextPoint }> | null
  readonly polygonCount: number
  readonly vertexCount: number
}

export interface HarfBuzzBlob {
  readonly ptr: number
  destroy(): void
}

export interface HarfBuzzFace {
  readonly ptr: number
  readonly upem: number
  listNames(): Array<{ nameId: number; language: string }>
  getName(nameId: number, language: string): string
  destroy(): void
}

interface HarfBuzzGlyphExtents {
  readonly xBearing: number
  readonly yBearing: number
  readonly width: number
  readonly height: number
}

export interface HarfBuzzPathCommand {
  readonly type: 'M' | 'L' | 'Q' | 'C' | 'Z'
  readonly values: readonly number[]
}

export interface HarfBuzzFont {
  readonly ptr: number
  setScale(xScale: number, yScale: number): void
  glyphExtents(glyphId: number): HarfBuzzGlyphExtents | null
  glyphToJson(glyphId: number): HarfBuzzPathCommand[]
  destroy(): void
}

interface HarfBuzzGlyphInfo {
  readonly g: number
  readonly cl: number
  readonly dx: number
  readonly dy: number
  readonly ax: number
  readonly ay: number
  readonly flags: number
}

interface HarfBuzzBuffer {
  readonly ptr: number
  addText(text: string): void
  guessSegmentProperties(): void
  setDirection(direction: OpenScadTextDirection): void
  setLanguage(language: string): void
  setScript(script: string): void
  json(): HarfBuzzGlyphInfo[]
  destroy(): void
}

export interface HarfBuzzApi {
  createBlob(bytes: Uint8Array): HarfBuzzBlob
  createFace(blob: HarfBuzzBlob, faceIndex: number): HarfBuzzFace
  createFont(face: HarfBuzzFace): HarfBuzzFont
  createBuffer(): HarfBuzzBuffer
  shape(font: HarfBuzzFont, buffer: HarfBuzzBuffer, features?: string): void
}

export interface SfntFace {
  readonly offset: number
  readonly unitsPerEm: number
}

export interface ParsedFontQuery {
  readonly original: string
  readonly family: string
  readonly style?: string
}

export interface RawContour {
  readonly points: readonly (readonly [number, number])[]
}

export const preparedRuntimes = new WeakMap<PreparedOpenScadTextAssets, HarfBuzzApi>()
let harfBuzzPromise: Promise<HarfBuzzApi> | undefined

export async function loadHarfBuzz(): Promise<HarfBuzzApi> {
  if (!harfBuzzPromise) {
    const attempt = (async () => {
      const { createTextRuntime } = await import('./openScadTextRuntime')
      return await createTextRuntime() as HarfBuzzApi
    })()
      .catch(error => textError(
        'E_TEXT_RUNTIME_UNAVAILABLE',
        `The independent text shaping runtime could not initialize: ${error instanceof Error ? error.message : String(error)}`,
        {},
        error,
      ))
    harfBuzzPromise = attempt
    attempt.catch(() => {
      if (harfBuzzPromise === attempt) harfBuzzPromise = undefined
    })
  }
  return harfBuzzPromise
}
