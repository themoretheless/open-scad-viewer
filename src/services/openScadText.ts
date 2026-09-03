import {
  OpenScadProject,
  resolveOpenScadProjectPath,
  type OpenScadProjectBlobFile,
} from './openScadProject'
import type { CallNode } from './openscadCompiler'
import { OpenSCADParseError } from './openscadErrors'

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
const OPENSCAD_2021_GRID_FINE = 0.00000095367431640625

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

interface HarfBuzzBlob {
  readonly ptr: number
  destroy(): void
}

interface HarfBuzzFace {
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

interface HarfBuzzPathCommand {
  readonly type: 'M' | 'L' | 'Q' | 'C' | 'Z'
  readonly values: readonly number[]
}

interface HarfBuzzFont {
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

interface HarfBuzzApi {
  createBlob(bytes: Uint8Array): HarfBuzzBlob
  createFace(blob: HarfBuzzBlob, faceIndex: number): HarfBuzzFace
  createFont(face: HarfBuzzFace): HarfBuzzFont
  createBuffer(): HarfBuzzBuffer
  shape(font: HarfBuzzFont, buffer: HarfBuzzBuffer, features?: string): void
}

interface SfntFace {
  readonly offset: number
  readonly unitsPerEm: number
}

interface ParsedFontQuery {
  readonly original: string
  readonly family: string
  readonly style?: string
}

interface RawContour {
  readonly points: readonly (readonly [number, number])[]
}

const preparedRuntimes = new WeakMap<PreparedOpenScadTextAssets, HarfBuzzApi>()
let harfBuzzPromise: Promise<HarfBuzzApi> | undefined

function textError(
  code: OpenScadTextErrorCode,
  message: string,
  details: OpenScadTextErrorDetails = {},
  cause?: unknown,
): never {
  throw new OpenScadTextError(code, message, details, cause === undefined ? {} : { cause })
}

function runtimeIsNode(): boolean {
  const processValue = (globalThis as { process?: { versions?: { node?: string }; type?: string } }).process
  return typeof processValue?.versions?.node === 'string' && processValue.type !== 'renderer'
}

async function loadHarfBuzz(): Promise<HarfBuzzApi> {
  if (!harfBuzzPromise) {
    const attempt = (async () => {
      const [{ default: createHarfBuzzModule }, { default: bindHarfBuzz }] = await Promise.all([
        import('harfbuzzjs/hb.js'),
        import('harfbuzzjs/hbjs.js'),
      ])
      const module = runtimeIsNode()
        ? await createHarfBuzzModule()
        : await (async () => {
            const wasmUrl = (await import('harfbuzzjs/hb.wasm?url')).default
            return createHarfBuzzModule({
              locateFile: path => path === 'hb.wasm' ? wasmUrl : path,
            })
          })()
      return bindHarfBuzz(module) as HarfBuzzApi
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

function u16(bytes: Uint8Array, offset: number): number {
  return bytes[offset] * 0x100 + bytes[offset + 1]
}

function u32(bytes: Uint8Array, offset: number): number {
  return ((bytes[offset] * 0x1000000)
    + (bytes[offset + 1] << 16)
    + (bytes[offset + 2] << 8)
    + bytes[offset + 3]) >>> 0
}

function asciiTag(bytes: Uint8Array, offset: number): string {
  return String.fromCharCode(bytes[offset], bytes[offset + 1], bytes[offset + 2], bytes[offset + 3])
}

function invalidFont(
  path: string,
  message: string,
  code: 'E_TEXT_FONT_INVALID' | 'E_TEXT_FONT_UNSUPPORTED' = 'E_TEXT_FONT_INVALID',
): never {
  return textError(code, `Font ${path} ${message}`, { fontPath: path })
}

function validateSfntFace(bytes: Uint8Array, path: string, offset: number): SfntFace {
  if (offset < 0 || offset + 12 > bytes.byteLength) invalidFont(path, 'has a truncated SFNT header.')
  const signature = asciiTag(bytes, offset)
  if (u32(bytes, offset) !== 0x00010000 && signature !== 'OTTO' && signature !== 'true') {
    invalidFont(path, 'uses an unsupported SFNT face signature.', 'E_TEXT_FONT_UNSUPPORTED')
  }
  const tableCount = u16(bytes, offset + 4)
  if (tableCount === 0 || tableCount > 256) invalidFont(path, 'has an invalid SFNT table count.')
  const directoryEnd = offset + 12 + tableCount * 16
  if (!Number.isSafeInteger(directoryEnd) || directoryEnd > bytes.byteLength) {
    invalidFont(path, 'has a truncated SFNT table directory.')
  }

  const tables = new Map<string, { offset: number; length: number }>()
  const occupied: Array<{ start: number; end: number; tag: string }> = []
  for (let index = 0; index < tableCount; index++) {
    const entry = offset + 12 + index * 16
    const tag = asciiTag(bytes, entry)
    const tableOffset = u32(bytes, entry + 8)
    const length = u32(bytes, entry + 12)
    const end = tableOffset + length
    if (!Number.isSafeInteger(end) || tableOffset > bytes.byteLength || end > bytes.byteLength) {
      invalidFont(path, `has an out-of-bounds ${JSON.stringify(tag)} table.`)
    }
    if (tables.has(tag)) invalidFont(path, `has duplicate ${JSON.stringify(tag)} tables.`)
    tables.set(tag, { offset: tableOffset, length })
    if (length > 0) occupied.push({ start: tableOffset, end, tag })
  }
  occupied.sort((left, right) => left.start - right.start || left.end - right.end)
  for (let index = 1; index < occupied.length; index++) {
    const previous = occupied[index - 1]
    const current = occupied[index]
    if (current.start < previous.end) {
      invalidFont(path, `has overlapping ${JSON.stringify(previous.tag)} and ${JSON.stringify(current.tag)} tables.`)
    }
  }

  const head = tables.get('head')
  const maxp = tables.get('maxp')
  const cmap = tables.get('cmap')
  const name = tables.get('name')
  if (!head || head.length < 54 || !maxp || maxp.length < 6 || !cmap || cmap.length < 4) {
    invalidFont(path, 'is missing a required head, maxp, or cmap table.')
  }
  const hasTrueTypeOutlines = tables.has('glyf') && tables.has('loca')
  const hasCffOutlines = tables.has('CFF ') || tables.has('CFF2')
  if (!hasTrueTypeOutlines && !hasCffOutlines) {
    invalidFont(path, 'has no supported TrueType or CFF outline tables.', 'E_TEXT_FONT_UNSUPPORTED')
  }
  if (name && name.length > OPENSCAD_TEXT_MAX_NAME_TABLE_BYTES) {
    textError(
      'E_TEXT_FONT_LIMIT',
      `Font ${path} name table exceeds ${OPENSCAD_TEXT_MAX_NAME_TABLE_BYTES.toLocaleString()} bytes.`,
      { fontPath: path, limit: OPENSCAD_TEXT_MAX_NAME_TABLE_BYTES, actual: name.length },
    )
  }
  const unitsPerEm = u16(bytes, head.offset + 18)
  if (unitsPerEm < 16 || unitsPerEm > 16_384) invalidFont(path, 'has an invalid units-per-em value.')
  const glyphCount = u16(bytes, maxp.offset + 4)
  if (glyphCount === 0) invalidFont(path, 'contains no glyphs.')
  if (hasTrueTypeOutlines) {
    const loca = tables.get('loca')!
    const indexToLocFormat = u16(bytes, head.offset + 50)
    if (indexToLocFormat !== 0 && indexToLocFormat !== 1) invalidFont(path, 'has an invalid loca format.')
    const requiredLocaBytes = (glyphCount + 1) * (indexToLocFormat === 0 ? 2 : 4)
    if (loca.length < requiredLocaBytes) invalidFont(path, 'has a truncated loca table.')
  }
  return Object.freeze({ offset, unitsPerEm })
}

function validateFont(bytes: Uint8Array, path: string): readonly SfntFace[] {
  if (bytes.byteLength < 12) invalidFont(path, 'is truncated.')
  if (asciiTag(bytes, 0) !== 'ttcf') return Object.freeze([validateSfntFace(bytes, path, 0)])

  const version = u32(bytes, 4)
  if (version !== 0x00010000 && version !== 0x00020000) {
    invalidFont(path, 'uses an unsupported TrueType collection version.', 'E_TEXT_FONT_UNSUPPORTED')
  }
  const count = u32(bytes, 8)
  if (count === 0 || count > OPENSCAD_TEXT_MAX_FACES_PER_COLLECTION) {
    textError(
      'E_TEXT_FONT_LIMIT',
      `Font collection ${path} exceeds ${OPENSCAD_TEXT_MAX_FACES_PER_COLLECTION} faces.`,
      { fontPath: path, limit: OPENSCAD_TEXT_MAX_FACES_PER_COLLECTION, actual: count },
    )
  }
  const directoryEnd = 12 + count * 4
  if (!Number.isSafeInteger(directoryEnd) || directoryEnd > bytes.byteLength) {
    invalidFont(path, 'has a truncated TrueType collection directory.')
  }
  const offsets = new Set<number>()
  const faces: SfntFace[] = []
  for (let index = 0; index < count; index++) {
    const offset = u32(bytes, 12 + index * 4)
    if (offsets.has(offset)) invalidFont(path, 'contains duplicate face offsets.')
    offsets.add(offset)
    faces.push(validateSfntFace(bytes, path, offset))
  }
  return Object.freeze(faces)
}

function fontExtension(path: string): boolean {
  return /\.(?:ttf|otf|ttc|otc)$/iu.test(path)
}

function canonicalName(value: string): string {
  return value.normalize('NFKC').trim().replace(/\s+/gu, ' ').toLocaleLowerCase('en-US')
}

function safeFontName(
  face: HarfBuzzFace,
  entries: readonly { nameId: number; language: string }[],
  nameId: number,
): string | undefined {
  const matches = entries.filter(entry => entry.nameId === nameId)
  const selected = matches.find(entry => /^en(?:-|$)/iu.test(entry.language)) ?? matches[0]
  if (!selected) return undefined
  const value = face.getName(nameId, selected.language).replace(/\u0000/gu, '').trim()
  if (value.length > 1_024) throw new Error(`name ID ${nameId} exceeds 1024 characters`)
  return value || undefined
}

function describeFace(
  hb: HarfBuzzApi,
  file: OpenScadProjectBlobFile,
  faceIndex: number,
  validated: SfntFace,
): OpenScadTextFont {
  const blob = hb.createBlob(file.data)
  let face: HarfBuzzFace | undefined
  try {
    face = hb.createFace(blob, faceIndex)
    if (face.upem !== validated.unitsPerEm) throw new Error('HarfBuzz reported inconsistent units-per-em')
    const entries = face.listNames()
    if (entries.length > OPENSCAD_TEXT_MAX_NAME_RECORDS) {
      textError(
        'E_TEXT_FONT_LIMIT',
        `Font ${file.path} exceeds ${OPENSCAD_TEXT_MAX_NAME_RECORDS} name records.`,
        {
          fontPath: file.path,
          faceIndex,
          limit: OPENSCAD_TEXT_MAX_NAME_RECORDS,
          actual: entries.length,
        },
      )
    }
    const filename = file.path.slice(file.path.lastIndexOf('/') + 1).replace(/\.[^.]+$/u, '')
    const family = safeFontName(face, entries, 16) ?? safeFontName(face, entries, 1) ?? filename
    const style = safeFontName(face, entries, 17) ?? safeFontName(face, entries, 2) ?? 'Regular'
    return Object.freeze({
      path: file.path,
      sha256: file.sha256,
      faceIndex,
      family,
      style,
      fullName: safeFontName(face, entries, 4) ?? `${family} ${style}`.trim(),
      postscriptName: safeFontName(face, entries, 6) ?? `${family}-${style}`.replace(/\s+/gu, ''),
      unitsPerEm: face.upem,
    })
  } finally {
    face?.destroy()
    blob.destroy()
  }
}

/** Validate and index every bounded VFS font before the synchronous evaluator. */
export async function prepareOpenScadTextAssets(
  project: OpenScadProject,
): Promise<PreparedOpenScadTextAssets> {
  if (!(project instanceof OpenScadProject)) {
    return textError('E_TEXT_PROJECT_REQUIRED', 'text() requires an immutable OpenScadProject VFS.')
  }
  const fontPaths = project.list()
    .filter(file => file.kind === 'blob' && fontExtension(file.path))
    .map(file => file.path)
  if (fontPaths.length > OPENSCAD_TEXT_MAX_FONTS) {
    return textError(
      'E_TEXT_FONT_LIMIT',
      `OpenSCAD project exceeds ${OPENSCAD_TEXT_MAX_FONTS} font files.`,
      { limit: OPENSCAD_TEXT_MAX_FONTS, actual: fontPaths.length },
    )
  }

  const fonts: OpenScadTextFont[] = []
  const rejected: RejectedOpenScadTextFont[] = []
  const hb = fontPaths.length === 0 ? undefined : await loadHarfBuzz()
  for (const path of fontPaths) {
    const file = project.read(path)
    if (file?.kind !== 'blob') continue
    try {
      const faces = validateFont(file.data, path)
      for (let faceIndex = 0; faceIndex < faces.length; faceIndex++) {
        fonts.push(describeFace(hb!, file, faceIndex, faces[faceIndex]))
      }
    } catch (error) {
      const typed = error instanceof OpenScadTextError
        ? error
        : new OpenScadTextError(
          'E_TEXT_FONT_INVALID',
          `Font ${path} could not be read safely: ${error instanceof Error ? error.message : String(error)}`,
          { fontPath: path },
          { cause: error },
        )
      rejected.push(Object.freeze({
        path,
        code: typed.code === 'E_TEXT_FONT_UNSUPPORTED' ? typed.code : 'E_TEXT_FONT_INVALID',
        message: typed.message,
      }))
    }
  }
  fonts.sort((left, right) => left.path.localeCompare(right.path) || left.faceIndex - right.faceIndex)
  rejected.sort((left, right) => left.path.localeCompare(right.path))
  const prepared = Object.freeze({
    projectSha256: project.sha256,
    fonts: Object.freeze(fonts),
    rejected: Object.freeze(rejected),
  })
  if (hb) preparedRuntimes.set(prepared, hb)
  return prepared
}

function parseFontQuery(value: string): ParsedFontQuery {
  const fields = value.split(':')
  const family = fields.shift()?.trim() ?? ''
  let style: string | undefined
  for (const field of fields) {
    const match = /^\s*style\s*=\s*(.*?)\s*$/iu.exec(field)
    if (match) style = match[1]
  }
  return Object.freeze({ original: value, family, ...(style ? { style } : {}) })
}

function exactFontPaths(project: OpenScadProject, sourcePath: string, query: string): readonly string[] {
  if (!fontExtension(query) || query.includes(':')) return []
  const paths: string[] = []
  try {
    paths.push(resolveOpenScadProjectPath(sourcePath, query))
  } catch {
    // The family-name resolver below will produce the stable public error.
  }
  if (!query.startsWith('.')) {
    try {
      if (project.read(query)?.kind === 'blob') paths.push(query)
    } catch {
      // Reject absolute/traversing candidates through the typed font resolver.
    }
  }
  return Object.freeze([...new Set(paths)])
}

function resolveFont(
  project: OpenScadProject,
  prepared: PreparedOpenScadTextAssets,
  sourcePath: string,
  value: string,
): OpenScadTextFont {
  const exactPaths = exactFontPaths(project, sourcePath, value.trim())
  for (const path of exactPaths) {
    const match = prepared.fonts.find(font => font.path === path)
    if (match) return match
    const rejected = prepared.rejected.find(font => font.path === path)
    if (rejected) return textError(rejected.code, rejected.message, { sourcePath, font: value, fontPath: path })
  }

  const query = parseFontQuery(value)
  if (query.family.length === 0) {
    const regular = prepared.fonts.find(font => /^(?:regular|normal|book)$/iu.test(font.style))
    const selected = regular ?? prepared.fonts[0]
    if (selected) return selected
  } else {
    const family = canonicalName(query.family)
    const style = query.style === undefined ? undefined : canonicalName(query.style)
    const scored = prepared.fonts.flatMap(font => {
      const filename = font.path.slice(font.path.lastIndexOf('/') + 1).replace(/\.[^.]+$/u, '')
      const names = [font.family, font.fullName, font.postscriptName, filename].map(canonicalName)
      const familyScore = names[0] === family ? 4
        : names[1] === family || names[2] === family ? 3
          : names[3] === family ? 2
            : names.some(name => name.startsWith(`${family} `) || name.startsWith(`${family}-`)) ? 1 : 0
      if (familyScore === 0) return []
      const styleMatches = style === undefined || canonicalName(font.style) === style
        || canonicalName(font.fullName).endsWith(` ${style}`)
      if (!styleMatches) return []
      const regularBonus = style === undefined && /^(?:regular|normal|book)$/iu.test(font.style) ? 1 : 0
      return [{ font, score: familyScore * 2 + regularBonus }]
    }).sort((left, right) => right.score - left.score
      || left.font.path.localeCompare(right.font.path)
      || left.font.faceIndex - right.font.faceIndex)
    if (scored[0]) return scored[0].font
  }

  if (prepared.fonts.length === 0 && prepared.rejected[0]) {
    const rejected = prepared.rejected[0]
    return textError(rejected.code, rejected.message, {
      sourcePath,
      font: value,
      fontPath: rejected.path,
    })
  }
  return textError(
    'E_TEXT_FONT_NOT_FOUND',
    value.trim().length === 0
      ? 'text() could not resolve a default font from the bounded project VFS.'
      : `text() could not resolve font ${JSON.stringify(value)} from the bounded project VFS.`,
    {
      sourcePath,
      font: value,
      ...(exactPaths[0] === undefined ? {} : { fontPath: exactPaths[0] }),
    },
  )
}

function finiteParameter(value: number, name: string): number {
  if (typeof value !== 'number' || !Number.isFinite(value)) {
    return textError('E_TEXT_PARAMETER_INVALID', `text() ${name} must be a finite number.`)
  }
  return value
}

function boundedParameter(value: number, name: string, limit: number): number {
  finiteParameter(value, name)
  if (Math.abs(value) > limit) {
    return textError(
      'E_TEXT_PARAMETER_LIMIT',
      `text() ${name} exceeds the absolute limit ${limit.toLocaleString()}.`,
      { limit, actual: Math.abs(value) },
    )
  }
  return value
}

/** OpenSCAD 2021.01 text curve subdivision derived from $fn/$fa/$fs. */
export function calculateOpenScadTextSegments(
  size: number,
  fn = 0,
  fs = 2,
  fa = 12,
): number {
  boundedParameter(size, 'size', OPENSCAD_TEXT_MAX_ABS_SIZE)
  finiteParameter(fn, '$fn')
  finiteParameter(fs, '$fs')
  finiteParameter(fa, '$fa')
  let fragments: number
  if (size < OPENSCAD_2021_GRID_FINE) fragments = 3
  else if (fn > 0) fragments = Math.trunc(Math.max(fn, 3))
  else {
    if (fs <= 0 || fa <= 0) {
      return textError('E_TEXT_PARAMETER_INVALID', 'text() $fa and $fs must be positive when $fn is zero.')
    }
    fragments = Math.ceil(Math.max(Math.min(360 / fa, size * 2 * Math.PI / fs), 5))
  }
  const segments = Math.max(Math.floor(fragments / 8) + 1, 2)
  if (segments > OPENSCAD_TEXT_MAX_CURVE_SEGMENTS) {
    return textError(
      'E_TEXT_PARAMETER_LIMIT',
      `text() curve subdivision exceeds ${OPENSCAD_TEXT_MAX_CURVE_SEGMENTS} segments.`,
      { limit: OPENSCAD_TEXT_MAX_CURVE_SEGMENTS, actual: segments },
    )
  }
  return segments
}

function codePointCount(text: string): number {
  let count = 0
  for (const _character of text) {
    count++
    if (count > OPENSCAD_TEXT_MAX_CODE_POINTS) return count
  }
  return count
}

function isWellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false
    } else if (unit >= 0xdc00 && unit <= 0xdfff) return false
  }
  return true
}

function normalizeDirection(value: string): OpenScadTextDirection | undefined {
  const normalized = value.toLocaleLowerCase('en-US')
  return normalized === 'ltr' || normalized === 'rtl' || normalized === 'ttb' || normalized === 'btt'
    ? normalized
    : undefined
}

function detectedScript(text: string): string {
  for (const character of text) {
    const codePoint = character.codePointAt(0)!
    if ((codePoint >= 0x0590 && codePoint <= 0x05ff)
      || (codePoint >= 0xfb1d && codePoint <= 0xfb4f)) return 'Hebr'
    if ((codePoint >= 0x0600 && codePoint <= 0x08ff)
      || (codePoint >= 0xfb50 && codePoint <= 0xfdff)
      || (codePoint >= 0xfe70 && codePoint <= 0xfeff)) return 'Arab'
    if (codePoint >= 0x0900 && codePoint <= 0x097f) return 'Deva'
    if (codePoint >= 0x0980 && codePoint <= 0x09ff) return 'Beng'
    if (codePoint >= 0x0e00 && codePoint <= 0x0e7f) return 'Thai'
    if (codePoint >= 0x3040 && codePoint <= 0x309f) return 'Hira'
    if (codePoint >= 0x30a0 && codePoint <= 0x30ff) return 'Kana'
    if ((codePoint >= 0x3400 && codePoint <= 0x9fff)
      || (codePoint >= 0x20000 && codePoint <= 0x3134f)) return 'Hani'
    if ((codePoint >= 0xac00 && codePoint <= 0xd7af)
      || (codePoint >= 0x1100 && codePoint <= 0x11ff)) return 'Hang'
    if (codePoint >= 0x0370 && codePoint <= 0x03ff) return 'Grek'
    if (codePoint >= 0x0400 && codePoint <= 0x052f) return 'Cyrl'
    if ((codePoint >= 0x0041 && codePoint <= 0x005a)
      || (codePoint >= 0x0061 && codePoint <= 0x007a)
      || (codePoint >= 0x00c0 && codePoint <= 0x02af)) return 'Latn'
  }
  return 'Zyyy'
}

function effectiveScript(text: string, requested: string): string {
  const trimmed = requested.trim()
  return /^[A-Za-z]{4,32}$/u.test(trimmed) ? trimmed : detectedScript(text)
}

function directionForScript(script: string): OpenScadTextDirection {
  const tag = script.slice(0, 4).toLocaleLowerCase('en-US')
  return [
    'adlm', 'arab', 'hebr', 'mand', 'mani', 'mend', 'merc', 'mero', 'nkoo',
    'orkh', 'phli', 'phlp', 'phnx', 'prti', 'rohg', 'samr', 'sarb', 'sogd',
    'sogo', 'syrc', 'thaa', 'yezi',
  ].includes(tag) ? 'rtl' : 'ltr'
}

function horizontalAlignment(value: string): OpenScadTextHorizontalAlignment {
  return value === 'center' || value === 'right' ? value : 'left'
}

function verticalAlignment(value: string): OpenScadTextVerticalAlignment {
  return value === 'top' || value === 'center' || value === 'bottom' ? value : 'baseline'
}

function samePoint(
  left: readonly [number, number],
  right: readonly [number, number],
): boolean {
  return left[0] === right[0] && left[1] === right[1]
}

function flattenGlyph(
  commands: readonly HarfBuzzPathCommand[],
  curveSegments: number,
  glyphId: number,
): readonly RawContour[] {
  if (commands.length > OPENSCAD_TEXT_MAX_PATH_COMMANDS) {
    return textError(
      'E_TEXT_PATH_LIMIT',
      `text() glyph ${glyphId} exceeds ${OPENSCAD_TEXT_MAX_PATH_COMMANDS.toLocaleString()} path commands.`,
      { limit: OPENSCAD_TEXT_MAX_PATH_COMMANDS, actual: commands.length },
    )
  }
  const contours: RawContour[] = []
  let points: Array<readonly [number, number]> = []
  let pen: readonly [number, number] = [0, 0]
  let commandBudget = 0

  const finish = (): void => {
    while (points.length > 1 && samePoint(points[0], points.at(-1)!)) points.pop()
    if (points.length >= 3) contours.push(Object.freeze({ points: Object.freeze(points) }))
    points = []
  }
  const add = (x: number, y: number): void => {
    if (!Number.isFinite(x) || !Number.isFinite(y)) {
      return textError('E_TEXT_FONT_INVALID', `text() glyph ${glyphId} contains non-finite outline coordinates.`)
    }
    points.push(Object.freeze([x, y] as const))
    commandBudget++
    if (commandBudget > OPENSCAD_TEXT_MAX_VERTICES) {
      return textError(
        'E_TEXT_VERTEX_LIMIT',
        `text() glyph ${glyphId} exceeds ${OPENSCAD_TEXT_MAX_VERTICES.toLocaleString()} flattened vertices.`,
        { limit: OPENSCAD_TEXT_MAX_VERTICES, actual: commandBudget },
      )
    }
  }

  for (const command of commands) {
    const values = command.values
    switch (command.type) {
      case 'M': {
        if (values.length !== 2) return textError('E_TEXT_FONT_INVALID', `text() glyph ${glyphId} has an invalid move command.`)
        finish()
        pen = [values[0], values[1]]
        add(pen[0], pen[1])
        break
      }
      case 'L': {
        if (values.length !== 2) return textError('E_TEXT_FONT_INVALID', `text() glyph ${glyphId} has an invalid line command.`)
        pen = [values[0], values[1]]
        add(pen[0], pen[1])
        break
      }
      case 'Q': {
        if (values.length !== 4) return textError('E_TEXT_FONT_INVALID', `text() glyph ${glyphId} has an invalid quadratic command.`)
        const from = pen
        const control: readonly [number, number] = [values[0], values[1]]
        const to: readonly [number, number] = [values[2], values[3]]
        for (let step = 1; step <= curveSegments; step++) {
          const t = step / curveSegments
          const inverse = 1 - t
          add(
            from[0] * inverse * inverse + control[0] * 2 * inverse * t + to[0] * t * t,
            from[1] * inverse * inverse + control[1] * 2 * inverse * t + to[1] * t * t,
          )
        }
        pen = to
        break
      }
      case 'C': {
        if (values.length !== 6) return textError('E_TEXT_FONT_INVALID', `text() glyph ${glyphId} has an invalid cubic command.`)
        const from = pen
        const control1: readonly [number, number] = [values[0], values[1]]
        const control2: readonly [number, number] = [values[2], values[3]]
        const to: readonly [number, number] = [values[4], values[5]]
        for (let step = 1; step <= curveSegments; step++) {
          const t = step / curveSegments
          const inverse = 1 - t
          add(
            from[0] * inverse ** 3 + control1[0] * 3 * inverse ** 2 * t
              + control2[0] * 3 * inverse * t ** 2 + to[0] * t ** 3,
            from[1] * inverse ** 3 + control1[1] * 3 * inverse ** 2 * t
              + control2[1] * 3 * inverse * t ** 2 + to[1] * t ** 3,
          )
        }
        pen = to
        break
      }
      case 'Z':
        if (values.length !== 0) return textError('E_TEXT_FONT_INVALID', `text() glyph ${glyphId} has an invalid close command.`)
        finish()
        break
    }
  }
  finish()
  return Object.freeze(contours)
}

function freezePoint(x: number, y: number): OpenScadTextPoint {
  return Object.freeze([x, y] as const)
}

function fontForLayout(
  hb: HarfBuzzApi,
  project: OpenScadProject,
  descriptor: OpenScadTextFont,
): { blob: HarfBuzzBlob; face: HarfBuzzFace; font: HarfBuzzFont } {
  const file = project.read(descriptor.path)
  if (file?.kind !== 'blob' || file.sha256 !== descriptor.sha256) {
    return textError(
      'E_TEXT_PROJECT_MISMATCH',
      `Prepared text font ${descriptor.path} no longer matches the project snapshot.`,
      { fontPath: descriptor.path, faceIndex: descriptor.faceIndex },
    )
  }
  const blob = hb.createBlob(file.data)
  let face: HarfBuzzFace | undefined
  let font: HarfBuzzFont | undefined
  try {
    face = hb.createFace(blob, descriptor.faceIndex)
    if (face.upem !== descriptor.unitsPerEm) throw new Error('units-per-em changed')
    font = hb.createFont(face)
    font.setScale(face.upem, face.upem)
    return { blob, face, font }
  } catch (error) {
    font?.destroy()
    face?.destroy()
    blob.destroy()
    return textError(
      'E_TEXT_FONT_INVALID',
      `Font ${descriptor.path} could not create a shaping face.`,
      { fontPath: descriptor.path, faceIndex: descriptor.faceIndex },
      error,
    )
  }
}

/**
 * Shape Unicode and emit real per-glyph outline polygons. This function is
 * synchronous after prepareOpenScadTextAssets(), matching the geometry evaluator.
 */
export function renderOpenScadText(
  project: OpenScadProject,
  prepared: PreparedOpenScadTextAssets,
  parameters: OpenScadTextParameters = {},
  sourcePath = project.entrypoint,
): OpenScadTextLayout {
  if (!(project instanceof OpenScadProject)) {
    return textError('E_TEXT_PROJECT_REQUIRED', 'text() requires an immutable OpenScadProject VFS.')
  }
  if (prepared.projectSha256 !== project.sha256) {
    return textError(
      'E_TEXT_PROJECT_MISMATCH',
      'Prepared text assets belong to a different OpenScadProject snapshot.',
      { sourcePath },
    )
  }
  const text = parameters.text ?? ''
  const fontQuery = parameters.font ?? ''
  if (typeof text !== 'string' || !isWellFormedUnicode(text) || text.includes('\u0000')) {
    return textError('E_TEXT_PARAMETER_INVALID', 'text() text must be a well-formed Unicode string without NUL.')
  }
  if (typeof fontQuery !== 'string' || !isWellFormedUnicode(fontQuery)) {
    return textError('E_TEXT_PARAMETER_INVALID', 'text() font must be a well-formed Unicode string.')
  }
  const points = codePointCount(text)
  if (points > OPENSCAD_TEXT_MAX_CODE_POINTS) {
    return textError(
      'E_TEXT_PARAMETER_LIMIT',
      `text() exceeds ${OPENSCAD_TEXT_MAX_CODE_POINTS.toLocaleString()} Unicode code points.`,
      { limit: OPENSCAD_TEXT_MAX_CODE_POINTS, actual: points },
    )
  }
  const size = boundedParameter(parameters.size ?? 10, 'size', OPENSCAD_TEXT_MAX_ABS_SIZE)
  const spacing = boundedParameter(parameters.spacing ?? 1, 'spacing', OPENSCAD_TEXT_MAX_ABS_SPACING)
  const language = parameters.language ?? 'en'
  const requestedDirection = parameters.direction ?? ''
  const requestedScript = parameters.script ?? ''
  if (typeof language !== 'string' || !/^[A-Za-z0-9-]{1,64}$/u.test(language)) {
    return textError('E_TEXT_PARAMETER_INVALID', 'text() language must be a bounded ASCII BCP-47 tag.')
  }
  if (typeof requestedDirection !== 'string' || typeof requestedScript !== 'string'
    || !isWellFormedUnicode(requestedDirection) || !isWellFormedUnicode(requestedScript)) {
    return textError('E_TEXT_PARAMETER_INVALID', 'text() direction and script must be Unicode strings.')
  }
  const curveSegments = calculateOpenScadTextSegments(
    size,
    parameters.$fn ?? 0,
    parameters.$fs ?? 2,
    parameters.$fa ?? 12,
  )
  const descriptor = resolveFont(project, prepared, sourcePath, fontQuery)
  const hb = preparedRuntimes.get(prepared)
  if (!hb) return textError('E_TEXT_RUNTIME_UNAVAILABLE', 'Prepared text assets have no shaping runtime.')

  const script = effectiveScript(text, requestedScript)
  const explicitDirection = normalizeDirection(requestedDirection)
  const direction = explicitDirection ?? directionForScript(script)
  const halign = horizontalAlignment(parameters.halign ?? 'left')
  const valign = verticalAlignment(parameters.valign ?? 'baseline')
  const unitScale = OPENSCAD_2021_TEXT_EM_SCALE / descriptor.unitsPerEm
  const { blob, face, font } = fontForLayout(hb, project, descriptor)
  const buffer = hb.createBuffer()
  try {
    buffer.addText(text)
    buffer.guessSegmentProperties()
    // OpenSCAD resolves direction from the effective script before shaping,
    // including when `script` was authored but `direction` was omitted.
    // Applying both resolved properties also keeps the returned layout and
    // HarfBuzz glyph order consistent for that case.
    buffer.setDirection(direction)
    if (script !== 'Zyyy') buffer.setScript(script)
    buffer.setLanguage(language)
    hb.shape(font, buffer)
    const shaped = buffer.json()
    if (shaped.length > OPENSCAD_TEXT_MAX_GLYPHS) {
      return textError(
        'E_TEXT_GLYPH_LIMIT',
        `text() shaping exceeds ${OPENSCAD_TEXT_MAX_GLYPHS.toLocaleString()} glyphs.`,
        { limit: OPENSCAD_TEXT_MAX_GLYPHS, actual: shaped.length },
      )
    }
    if (shaped.some(glyph => !Number.isSafeInteger(glyph.g) || glyph.g < 0
      || !Number.isSafeInteger(glyph.cl) || glyph.cl < 0
      || ![glyph.dx, glyph.dy, glyph.ax, glyph.ay].every(Number.isFinite))) {
      return textError('E_TEXT_SHAPING_FAILED', `Font ${descriptor.path} returned invalid shaping positions.`, {
        fontPath: descriptor.path,
        faceIndex: descriptor.faceIndex,
      })
    }

    const horizontal = direction === 'ltr' || direction === 'rtl'
    let width = 0
    let ascent = 0
    let descent = 0
    for (const glyph of shaped) {
      const extents = font.glyphExtents(glyph.g)
      if (horizontal) {
        const yMaximum = extents?.yBearing ?? 0
        const yMinimum = extents === null ? 0 : extents.yBearing + extents.height
        ascent = Math.max(ascent, Math.max(0, yMaximum * unitScale))
        descent = Math.max(descent, Math.max(0, -yMinimum * unitScale))
        width += glyph.ax * unitScale * spacing
      } else {
        const glyphWidth = extents === null ? 0 : Math.abs(extents.width) * unitScale
        width = Math.max(width, glyphWidth)
        ascent += glyph.ay * unitScale * spacing
      }
    }
    const xAlignment = halign === 'right' ? -width : halign === 'center' ? -width / 2 : 0
    const yAlignment = valign === 'top' ? -ascent
      : valign === 'center' ? descent / 2 - ascent / 2
        : valign === 'bottom' ? descent : 0

    const outlineCache = new Map<number, readonly RawContour[]>()
    const glyphs: OpenScadTextGlyph[] = []
    let advanceX = 0
    let advanceY = 0
    let polygonCount = 0
    let vertexCount = 0
    let minX = Infinity
    let minY = Infinity
    let maxX = -Infinity
    let maxY = -Infinity
    for (const glyph of shaped) {
      let rawContours = outlineCache.get(glyph.g)
      if (!rawContours) {
        rawContours = flattenGlyph(font.glyphToJson(glyph.g), curveSegments, glyph.g)
        outlineCache.set(glyph.g, rawContours)
      }
      const glyphOffsetX = xAlignment + glyph.dx * unitScale + advanceX
      const glyphOffsetY = yAlignment + glyph.dy * unitScale + advanceY
      const contours: OpenScadTextContour[] = []
      if (size !== 0) {
        for (const rawContour of rawContours) {
          const contour = rawContour.points.map(point => {
            const x = size * (point[0] * unitScale + glyphOffsetX)
            const y = size * (point[1] * unitScale + glyphOffsetY)
            if (!Number.isFinite(x) || !Number.isFinite(y)) {
              return textError('E_TEXT_PARAMETER_LIMIT', 'text() produced out-of-range outline coordinates.')
            }
            minX = Math.min(minX, x)
            minY = Math.min(minY, y)
            maxX = Math.max(maxX, x)
            maxY = Math.max(maxY, y)
            return freezePoint(x, y)
          })
          vertexCount += contour.length
          if (vertexCount > OPENSCAD_TEXT_MAX_VERTICES) {
            return textError(
              'E_TEXT_VERTEX_LIMIT',
              `text() exceeds ${OPENSCAD_TEXT_MAX_VERTICES.toLocaleString()} outline vertices.`,
              { limit: OPENSCAD_TEXT_MAX_VERTICES, actual: vertexCount },
            )
          }
          contours.push(Object.freeze(contour))
        }
      }
      polygonCount += contours.length
      glyphs.push(Object.freeze({
        glyphId: glyph.g,
        cluster: glyph.cl,
        flags: glyph.flags,
        offset: freezePoint(size * glyphOffsetX, size * glyphOffsetY),
        advance: freezePoint(size * glyph.ax * unitScale * spacing, size * glyph.ay * unitScale * spacing),
        contours: Object.freeze(contours),
      }))
      advanceX += glyph.ax * unitScale * spacing
      advanceY += glyph.ay * unitScale * spacing
    }

    return Object.freeze({
      font: descriptor,
      direction,
      script,
      language,
      halign,
      valign,
      spacing,
      size,
      curveSegments,
      glyphs: Object.freeze(glyphs),
      advance: freezePoint(size * advanceX, size * advanceY),
      width: size * width,
      ascent: size * ascent,
      descent: size * descent,
      bounds: vertexCount === 0 ? null : Object.freeze({
        min: freezePoint(minX, minY),
        max: freezePoint(maxX, maxY),
      }),
      polygonCount,
      vertexCount,
    })
  } catch (error) {
    if (error instanceof OpenScadTextError) throw error
    return textError(
      'E_TEXT_SHAPING_FAILED',
      `text() shaping failed for ${descriptor.path}: ${error instanceof Error ? error.message : String(error)}`,
      { fontPath: descriptor.path, faceIndex: descriptor.faceIndex },
      error,
    )
  } finally {
    buffer.destroy()
    font.destroy()
    face.destroy()
    blob.destroy()
  }
}
