import type { CallNode } from './openscadCompiler'
import { OpenSCADParseError } from './openscadErrors'
import {
  OpenScadProject,
  resolveOpenScadProjectPath,
  type OpenScadProjectFile,
} from './openScadProject'

export const OPENSCAD_SURFACE_MAX_DIMENSION = 4_096
export const OPENSCAD_SURFACE_MAX_PIXELS = 250_000
export const OPENSCAD_SURFACE_MAX_PREPARED_PIXELS = 1_000_000
export const OPENSCAD_SURFACE_MAX_TRIANGLES = 750_000
const PNG_SIGNATURE = Object.freeze([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])

export const OPENSCAD_SURFACE_ERROR_CODES = Object.freeze([
  'E_SURFACE_PROJECT_REQUIRED',
  'E_SURFACE_FILE_REQUIRED',
  'E_SURFACE_PATH_INVALID',
  'E_SURFACE_FILE_MISSING',
  'E_SURFACE_DAT_ENCODING',
  'E_SURFACE_DAT_INVALID',
  'E_SURFACE_PNG_INVALID',
  'E_SURFACE_PNG_UNSUPPORTED',
  'E_SURFACE_DIMENSION_LIMIT',
  'E_SURFACE_TRIANGLE_LIMIT',
  'E_SURFACE_NON_MANIFOLD',
] as const)

export type OpenScadSurfaceErrorCode = typeof OPENSCAD_SURFACE_ERROR_CODES[number]

export interface OpenScadSurfaceErrorDetails {
  readonly sourcePath: string
  readonly specifier?: string
  readonly assetPath?: string
  readonly limit?: number
  readonly actual?: number
}

/** Positioned, project-aware diagnostic for one authored surface() call. */
export class OpenScadSurfaceError extends OpenSCADParseError {
  readonly details: Readonly<OpenScadSurfaceErrorDetails>

  constructor(
    source: string,
    node: Pick<CallNode, 'p' | 'end'>,
    readonly code: OpenScadSurfaceErrorCode,
    message: string,
    details: OpenScadSurfaceErrorDetails,
  ) {
    super(source, node.p, message, code, node.end)
    this.name = 'OpenScadSurfaceError'
    this.details = Object.freeze({ ...details })
  }
}

export class OpenScadSurfaceDataError extends Error {
  constructor(
    readonly code: Exclude<OpenScadSurfaceErrorCode, 'E_SURFACE_PROJECT_REQUIRED' | 'E_SURFACE_FILE_REQUIRED' | 'E_SURFACE_PATH_INVALID' | 'E_SURFACE_NON_MANIFOLD'>,
    message: string,
    readonly limit?: number,
    readonly actual?: number,
    readonly assetPath?: string,
  ) {
    super(message)
    this.name = 'OpenScadSurfaceDataError'
  }
}

export interface OpenScadSurfaceHeightMap {
  readonly rows: number
  readonly columns: number
  /** Row-major values, with row zero mapped to the positive-image Y origin. */
  readonly values: Float64Array
  readonly minimum: number
  readonly base: number
  readonly format: 'dat' | 'png'
}

interface DecodedPngLuminance {
  readonly width: number
  readonly height: number
  /** Top-to-bottom PNG scanline order, one linear-sRGB luminance value per pixel. */
  readonly pixels: Float64Array
}

export interface PreparedOpenScadSurfaceAssets {
  readonly png: ReadonlyMap<string, DecodedPngLuminance | OpenScadSurfaceDataError>
}

export interface OpenScadSurfaceMesh {
  readonly vertices: Float32Array
  readonly triangles: Uint32Array
  readonly triangleCount: number
}

const UTF8_FATAL = new TextDecoder('utf-8', { fatal: true })
const NUMBER_TOKEN = /^[+-]?(?:(?:\d+(?:\.\d*)?)|(?:\.\d+))(?:[eE][+-]?\d+)?$/u

function dataError(
  code: OpenScadSurfaceDataError['code'],
  message: string,
  limit?: number,
  actual?: number,
): never {
  throw new OpenScadSurfaceDataError(code, message, limit, actual)
}

function isPng(bytes: Uint8Array): boolean {
  return bytes.byteLength >= PNG_SIGNATURE.length
    && PNG_SIGNATURE.every((value, index) => bytes[index] === value)
}

function surfaceTriangleCount(rows: number, columns: number): number {
  if (rows < 2 || columns < 2) return 0
  const cells = (rows - 1) * (columns - 1)
  const perimeterSegments = 2 * (rows - 1) + 2 * (columns - 1)
  return 4 * cells + 3 * perimeterSegments
}

function enforceDimensions(rows: number, columns: number): void {
  const dimension = Math.max(rows, columns)
  if (dimension > OPENSCAD_SURFACE_MAX_DIMENSION) {
    dataError(
      'E_SURFACE_DIMENSION_LIMIT',
      `surface() heightmap dimension exceeds ${OPENSCAD_SURFACE_MAX_DIMENSION.toLocaleString()} samples.`,
      OPENSCAD_SURFACE_MAX_DIMENSION,
      dimension,
    )
  }
  const pixels = rows * columns
  if (!Number.isSafeInteger(pixels) || pixels > OPENSCAD_SURFACE_MAX_PIXELS) {
    dataError(
      'E_SURFACE_DIMENSION_LIMIT',
      `surface() heightmap exceeds ${OPENSCAD_SURFACE_MAX_PIXELS.toLocaleString()} samples.`,
      OPENSCAD_SURFACE_MAX_PIXELS,
      pixels,
    )
  }
  const triangles = surfaceTriangleCount(rows, columns)
  if (!Number.isSafeInteger(triangles) || triangles > OPENSCAD_SURFACE_MAX_TRIANGLES) {
    dataError(
      'E_SURFACE_TRIANGLE_LIMIT',
      `surface() heightmap would exceed ${OPENSCAD_SURFACE_MAX_TRIANGLES.toLocaleString()} triangles.`,
      OPENSCAD_SURFACE_MAX_TRIANGLES,
      triangles,
    )
  }
}

function heightMap(
  rows: number,
  columns: number,
  values: Float64Array,
  format: OpenScadSurfaceHeightMap['format'],
): OpenScadSurfaceHeightMap {
  if (rows < 2 || columns < 2) {
    dataError(
      format === 'png' ? 'E_SURFACE_PNG_INVALID' : 'E_SURFACE_DAT_INVALID',
      'surface() heightmaps require at least two rows and two columns.',
    )
  }
  enforceDimensions(rows, columns)
  let minimum = Infinity
  for (const value of values) minimum = Math.min(minimum, value)
  const base = Math.min(0, minimum - 1)
  return Object.freeze({ rows, columns, values, minimum, base, format })
}

export function parseOpenScadSurfaceDat(file: OpenScadProjectFile): OpenScadSurfaceHeightMap {
  let text: string
  if (file.kind === 'source') {
    text = file.source
  } else {
    try {
      text = UTF8_FATAL.decode(file.data)
    } catch {
      return dataError('E_SURFACE_DAT_ENCODING', 'surface() DAT input must be valid UTF-8 text.')
    }
  }

  const rows: number[][] = []
  for (const rawLine of text.split(/\r\n|\n|\r/u)) {
    const line = rawLine.trim()
    if (line.length === 0 || line.startsWith('#')) continue
    if (rows.length >= OPENSCAD_SURFACE_MAX_DIMENSION) {
      return dataError(
        'E_SURFACE_DIMENSION_LIMIT',
        `surface() heightmap dimension exceeds ${OPENSCAD_SURFACE_MAX_DIMENSION.toLocaleString()} samples.`,
        OPENSCAD_SURFACE_MAX_DIMENSION,
        rows.length + 1,
      )
    }
    const row: number[] = []
    for (const match of line.matchAll(/\S+/gu)) {
      const token = match[0]
      if (row.length >= OPENSCAD_SURFACE_MAX_DIMENSION) {
        return dataError(
          'E_SURFACE_DIMENSION_LIMIT',
          `surface() heightmap dimension exceeds ${OPENSCAD_SURFACE_MAX_DIMENSION.toLocaleString()} samples.`,
          OPENSCAD_SURFACE_MAX_DIMENSION,
          row.length + 1,
        )
      }
      if (!NUMBER_TOKEN.test(token)) {
        return dataError('E_SURFACE_DAT_INVALID', `surface() DAT input contains illegal value ${JSON.stringify(token)}.`)
      }
      const value = Number(token)
      if (!Number.isFinite(value)) {
        return dataError('E_SURFACE_DAT_INVALID', 'surface() DAT input contains a non-finite height.')
      }
      row.push(value)
    }
    if ((rows.length + 1) * row.length > OPENSCAD_SURFACE_MAX_PIXELS) {
      return dataError(
        'E_SURFACE_DIMENSION_LIMIT',
        `surface() heightmap exceeds ${OPENSCAD_SURFACE_MAX_PIXELS.toLocaleString()} samples.`,
        OPENSCAD_SURFACE_MAX_PIXELS,
        (rows.length + 1) * row.length,
      )
    }
    rows.push(row)
  }
  if (rows.length < 2 || (rows[0]?.length ?? 0) < 2) {
    return dataError('E_SURFACE_DAT_INVALID', 'surface() DAT input requires at least two non-empty rows and columns.')
  }
  const columns = rows[0].length
  if (rows.some(row => row.length !== columns)) {
    return dataError('E_SURFACE_DAT_INVALID', 'surface() DAT rows must contain the same number of heights.')
  }
  enforceDimensions(rows.length, columns)
  return heightMap(rows.length, columns, Float64Array.from(rows.flat()), 'dat')
}

function u32(bytes: Uint8Array, offset: number): number {
  return ((bytes[offset] * 0x1000000)
    + (bytes[offset + 1] << 16)
    + (bytes[offset + 2] << 8)
    + bytes[offset + 3]) >>> 0
}

const CRC_TABLE = (() => {
  const table = new Uint32Array(256)
  for (let index = 0; index < table.length; index++) {
    let value = index
    for (let bit = 0; bit < 8; bit++) value = (value & 1) === 0 ? value >>> 1 : 0xedb88320 ^ (value >>> 1)
    table[index] = value >>> 0
  }
  return table
})()

function pngCrc(bytes: Uint8Array, start: number, end: number): number {
  let crc = 0xffffffff
  for (let index = start; index < end; index++) crc = CRC_TABLE[(crc ^ bytes[index]) & 0xff] ^ (crc >>> 8)
  return (crc ^ 0xffffffff) >>> 0
}

function channelsForColorType(colorType: number): number {
  switch (colorType) {
    case 0: return 1
    case 2: return 3
    case 3: return 1
    case 4: return 2
    case 6: return 4
    default: return dataError('E_SURFACE_PNG_UNSUPPORTED', `surface() PNG color type ${colorType} is unsupported.`)
  }
}

function validateBitDepth(colorType: number, bitDepth: number): void {
  const valid = colorType === 0
    ? [1, 2, 4, 8, 16]
    : colorType === 3
      ? [1, 2, 4, 8]
      : [8, 16]
  if (!valid.includes(bitDepth)) {
    dataError(
      'E_SURFACE_PNG_UNSUPPORTED',
      `surface() PNG bit depth ${bitDepth} is invalid for color type ${colorType}.`,
    )
  }
}

function passSize(total: number, start: number, step: number): number {
  return total <= start ? 0 : Math.ceil((total - start) / step)
}

const ADAM7 = Object.freeze([
  Object.freeze([0, 0, 8, 8]),
  Object.freeze([4, 0, 8, 8]),
  Object.freeze([0, 4, 4, 8]),
  Object.freeze([2, 0, 4, 4]),
  Object.freeze([0, 2, 2, 4]),
  Object.freeze([1, 0, 2, 2]),
  Object.freeze([0, 1, 1, 2]),
] as const)

function expectedInflatedBytes(
  width: number,
  height: number,
  bitsPerPixel: number,
  interlace: number,
): number {
  const passes = interlace === 0 ? [[0, 0, 1, 1] as const] : ADAM7
  let total = 0
  for (const [x0, y0, dx, dy] of passes) {
    const passWidth = passSize(width, x0, dx)
    const passHeight = passSize(height, y0, dy)
    if (passWidth === 0 || passHeight === 0) continue
    total += passHeight * (1 + Math.ceil(passWidth * bitsPerPixel / 8))
  }
  if (!Number.isSafeInteger(total) || total > 16 * 1024 * 1024) {
    dataError('E_SURFACE_DIMENSION_LIMIT', 'surface() PNG inflated representation is too large.', 16 * 1024 * 1024, total)
  }
  return total
}

async function inflatePng(bytes: Uint8Array, expectedBytes: number): Promise<Uint8Array> {
  let stream: ReadableStream<Uint8Array>
  try {
    const input = new Uint8Array(bytes.byteLength)
    input.set(bytes)
    stream = new Blob([input.buffer]).stream().pipeThrough(new DecompressionStream('deflate'))
  } catch {
    return dataError('E_SURFACE_PNG_UNSUPPORTED', 'This runtime cannot decode PNG deflate streams.')
  }
  const reader = stream.getReader()
  const chunks: Uint8Array[] = []
  let total = 0
  try {
    while (true) {
      const { done, value } = await reader.read()
      if (done) break
      total += value.byteLength
      if (total > expectedBytes) {
        await reader.cancel()
        return dataError('E_SURFACE_PNG_INVALID', 'surface() PNG expands beyond its declared dimensions.')
      }
      chunks.push(value)
    }
  } catch {
    return dataError('E_SURFACE_PNG_INVALID', 'surface() PNG contains an invalid compressed stream.')
  }
  if (total !== expectedBytes) {
    return dataError('E_SURFACE_PNG_INVALID', 'surface() PNG scanline payload has the wrong length.')
  }
  const output = new Uint8Array(total)
  let offset = 0
  for (const chunk of chunks) {
    output.set(chunk, offset)
    offset += chunk.byteLength
  }
  return output
}

function paeth(left: number, up: number, upperLeft: number): number {
  const estimate = left + up - upperLeft
  const dl = Math.abs(estimate - left)
  const du = Math.abs(estimate - up)
  const dul = Math.abs(estimate - upperLeft)
  return dl <= du && dl <= dul ? left : du <= dul ? up : upperLeft
}

function unfilter(
  target: Uint8Array,
  encoded: Uint8Array,
  previous: Uint8Array,
  filter: number,
  bytesPerPixel: number,
): void {
  for (let index = 0; index < encoded.length; index++) {
    const left = index >= bytesPerPixel ? target[index - bytesPerPixel] : 0
    const up = previous[index] ?? 0
    const upperLeft = index >= bytesPerPixel ? previous[index - bytesPerPixel] : 0
    const predictor = filter === 0 ? 0
      : filter === 1 ? left
        : filter === 2 ? up
          : filter === 3 ? Math.floor((left + up) / 2)
            : filter === 4 ? paeth(left, up, upperLeft)
              : dataError('E_SURFACE_PNG_INVALID', `surface() PNG uses invalid scanline filter ${filter}.`)
    target[index] = (encoded[index] + predictor) & 0xff
  }
}

function sample(row: Uint8Array, index: number, bitDepth: number): number {
  if (bitDepth === 8) return row[index]
  if (bitDepth === 16) {
    // lodepng's default OpenSCAD 2021 decode target is RGBA8: 16-bit
    // channels are narrowed by taking their most-significant byte.
    return row[index * 2]
  }
  const bitOffset = index * bitDepth
  const shift = 8 - bitDepth - (bitOffset & 7)
  const mask = (1 << bitDepth) - 1
  const value = (row[bitOffset >>> 3] >>> shift) & mask
  return Math.round(value * 255 / mask)
}

function pixelLuminance(
  row: Uint8Array,
  x: number,
  colorType: number,
  bitDepth: number,
  palette: Uint8Array | null,
): number {
  const channels = channelsForColorType(colorType)
  const first = x * channels
  if (colorType === 0 || colorType === 4) return sample(row, first, bitDepth)
  if (colorType === 3) {
    const index = sample(row, first, bitDepth) * ((1 << bitDepth) - 1) / 255
    const paletteOffset = Math.round(index) * 3
    if (palette === null || paletteOffset + 2 >= palette.length) {
      return dataError('E_SURFACE_PNG_INVALID', 'surface() PNG references a missing palette entry.')
    }
    return 0.2126 * palette[paletteOffset]
      + 0.7152 * palette[paletteOffset + 1]
      + 0.0722 * palette[paletteOffset + 2]
  }
  const red = sample(row, first, bitDepth)
  const green = sample(row, first + 1, bitDepth)
  const blue = sample(row, first + 2, bitDepth)
  return 0.2126 * red + 0.7152 * green + 0.0722 * blue
}

/** Dependency-free, bounded PNG decoder for OpenSCAD surface() luminance. */
export async function decodeOpenScadSurfacePng(bytes: Uint8Array): Promise<DecodedPngLuminance> {
  if (!isPng(bytes)) return dataError('E_SURFACE_PNG_INVALID', 'surface() asset is not a PNG image.')
  let cursor = PNG_SIGNATURE.length
  let width = 0
  let height = 0
  let bitDepth = 0
  let colorType = -1
  let interlace = -1
  let palette: Uint8Array | null = null
  let sawHeader = false
  let sawEnd = false
  const compressedChunks: Uint8Array[] = []
  let compressedLength = 0

  while (cursor < bytes.byteLength) {
    if (cursor + 12 > bytes.byteLength) return dataError('E_SURFACE_PNG_INVALID', 'surface() PNG contains a truncated chunk.')
    const length = u32(bytes, cursor)
    const typeOffset = cursor + 4
    const dataOffset = cursor + 8
    const end = dataOffset + length
    if (!Number.isSafeInteger(end) || end + 4 > bytes.byteLength) {
      return dataError('E_SURFACE_PNG_INVALID', 'surface() PNG chunk length exceeds the file.')
    }
    const type = String.fromCharCode(...bytes.subarray(typeOffset, typeOffset + 4))
    if (pngCrc(bytes, typeOffset, end) !== u32(bytes, end)) {
      return dataError('E_SURFACE_PNG_INVALID', `surface() PNG chunk ${type} has an invalid CRC.`)
    }
    const chunk = bytes.subarray(dataOffset, end)
    if (type === 'IHDR') {
      if (sawHeader || length !== 13 || cursor !== PNG_SIGNATURE.length) {
        return dataError('E_SURFACE_PNG_INVALID', 'surface() PNG has an invalid IHDR chunk.')
      }
      width = u32(chunk, 0)
      height = u32(chunk, 4)
      bitDepth = chunk[8]
      colorType = chunk[9]
      const compression = chunk[10]
      const filtering = chunk[11]
      interlace = chunk[12]
      if (width === 0 || height === 0 || compression !== 0 || filtering !== 0 || (interlace !== 0 && interlace !== 1)) {
        return dataError('E_SURFACE_PNG_UNSUPPORTED', 'surface() PNG has unsupported header settings.')
      }
      channelsForColorType(colorType)
      validateBitDepth(colorType, bitDepth)
      enforceDimensions(height, width)
      sawHeader = true
    } else if (!sawHeader) {
      return dataError('E_SURFACE_PNG_INVALID', 'surface() PNG must begin with IHDR.')
    } else if (type === 'PLTE') {
      if (chunk.length === 0 || chunk.length > 768 || chunk.length % 3 !== 0) {
        return dataError('E_SURFACE_PNG_INVALID', 'surface() PNG has an invalid palette.')
      }
      palette = new Uint8Array(chunk)
    } else if (type === 'IDAT') {
      compressedLength += chunk.byteLength
      if (compressedLength > bytes.byteLength) {
        return dataError('E_SURFACE_PNG_INVALID', 'surface() PNG compressed payload exceeds the file.')
      }
      compressedChunks.push(new Uint8Array(chunk))
    } else if (type === 'IEND') {
      if (length !== 0) return dataError('E_SURFACE_PNG_INVALID', 'surface() PNG has an invalid IEND chunk.')
      sawEnd = true
      cursor = end + 4
      break
    } else if ((bytes[typeOffset] & 0x20) === 0) {
      return dataError('E_SURFACE_PNG_UNSUPPORTED', `surface() PNG uses unsupported critical chunk ${type}.`)
    }
    cursor = end + 4
  }
  if (!sawHeader || !sawEnd || compressedChunks.length === 0 || cursor !== bytes.byteLength) {
    return dataError('E_SURFACE_PNG_INVALID', 'surface() PNG is incomplete or contains trailing bytes.')
  }
  if (colorType === 3 && palette === null) {
    return dataError('E_SURFACE_PNG_INVALID', 'surface() indexed PNG is missing its palette.')
  }

  const compressed = new Uint8Array(compressedLength)
  let compressedOffset = 0
  for (const chunk of compressedChunks) {
    compressed.set(chunk, compressedOffset)
    compressedOffset += chunk.byteLength
  }
  const bitsPerPixel = channelsForColorType(colorType) * bitDepth
  const expected = expectedInflatedBytes(width, height, bitsPerPixel, interlace)
  const inflated = await inflatePng(compressed, expected)
  const pixels = new Float64Array(width * height)
  const passes = interlace === 0 ? [[0, 0, 1, 1] as const] : ADAM7
  let inputOffset = 0
  for (const [x0, y0, dx, dy] of passes) {
    const passWidth = passSize(width, x0, dx)
    const passHeight = passSize(height, y0, dy)
    if (passWidth === 0 || passHeight === 0) continue
    const stride = Math.ceil(passWidth * bitsPerPixel / 8)
    const bytesPerPixel = Math.max(1, Math.ceil(bitsPerPixel / 8))
    let previous = new Uint8Array(stride)
    for (let passY = 0; passY < passHeight; passY++) {
      const filter = inflated[inputOffset++]
      const encoded = inflated.subarray(inputOffset, inputOffset + stride)
      inputOffset += stride
      const row = new Uint8Array(stride)
      unfilter(row, encoded, previous, filter, bytesPerPixel)
      for (let passX = 0; passX < passWidth; passX++) {
        const x = x0 + passX * dx
        const y = y0 + passY * dy
        pixels[y * width + x] = pixelLuminance(row, passX, colorType, bitDepth, palette)
      }
      previous = row
    }
  }
  if (inputOffset !== inflated.byteLength) {
    return dataError('E_SURFACE_PNG_INVALID', 'surface() PNG contains unconsumed scanline data.')
  }
  return Object.freeze({ width, height, pixels })
}

/** Decode bounded PNG blobs once before the synchronous geometry evaluator runs. */
export async function prepareOpenScadSurfaceAssets(
  project: OpenScadProject,
): Promise<PreparedOpenScadSurfaceAssets> {
  const png = new Map<string, DecodedPngLuminance | OpenScadSurfaceDataError>()
  let preparedPixels = 0
  for (const summary of project.list()) {
    const file = project.read(summary.path)
    if (file?.kind !== 'blob' || !isPng(file.data)) continue
    try {
      const decoded = await decodeOpenScadSurfacePng(file.data)
      const nextPreparedPixels = preparedPixels + decoded.width * decoded.height
      if (nextPreparedPixels > OPENSCAD_SURFACE_MAX_PREPARED_PIXELS) {
        png.set(file.path, new OpenScadSurfaceDataError(
          'E_SURFACE_DIMENSION_LIMIT',
          `surface() project PNG decode budget exceeds ${OPENSCAD_SURFACE_MAX_PREPARED_PIXELS.toLocaleString()} pixels.`,
          OPENSCAD_SURFACE_MAX_PREPARED_PIXELS,
          nextPreparedPixels,
          file.path,
        ))
        continue
      }
      preparedPixels = nextPreparedPixels
      png.set(file.path, decoded)
    } catch (error) {
      png.set(file.path, error instanceof OpenScadSurfaceDataError
        ? error
        : new OpenScadSurfaceDataError('E_SURFACE_PNG_INVALID', 'surface() PNG decoding failed.'))
    }
  }
  return Object.freeze({ png })
}

function dataErrorAtPath(error: OpenScadSurfaceDataError, path: string): OpenScadSurfaceDataError {
  if (error.assetPath === path) return error
  return new OpenScadSurfaceDataError(error.code, error.message, error.limit, error.actual, path)
}

function pngHeightMap(decoded: DecodedPngLuminance, invert: boolean): OpenScadSurfaceHeightMap {
  const values = new Float64Array(decoded.width * decoded.height)
  const scale = 100 / 255
  for (let y = 0; y < decoded.height; y++) {
    const targetY = decoded.height - 1 - y
    for (let x = 0; x < decoded.width; x++) {
      const pixel = decoded.pixels[y * decoded.width + x]
      // Preserve OpenSCAD 2021.01 exactly, including its historical
      // invert expression (1 - pixel), which places inverted PNGs below Z=0.
      values[targetY * decoded.width + x] = scale * (invert ? 1 - pixel : pixel)
    }
  }
  return heightMap(decoded.height, decoded.width, values, 'png')
}

export function loadOpenScadSurfaceHeightMap(
  project: OpenScadProject,
  prepared: PreparedOpenScadSurfaceAssets,
  importer: string,
  specifier: string,
  invert: boolean,
): { readonly path: string; readonly map: OpenScadSurfaceHeightMap } {
  const path = resolveOpenScadProjectPath(importer, specifier)
  const file = project.read(path)
  if (file === null) {
    throw new OpenScadSurfaceDataError(
      'E_SURFACE_FILE_MISSING',
      `surface() asset ${path} is missing from the project.`,
      undefined,
      undefined,
      path,
    )
  }
  if (file.kind === 'blob' && isPng(file.data)) {
    const decoded = prepared.png.get(path)
    if (decoded instanceof OpenScadSurfaceDataError) throw dataErrorAtPath(decoded, path)
    if (decoded === undefined) {
      return dataError('E_SURFACE_PNG_INVALID', 'surface() PNG was not prepared for evaluation.')
    }
    return Object.freeze({ path, map: pngHeightMap(decoded, invert) })
  }
  try {
    return Object.freeze({ path, map: parseOpenScadSurfaceDat(file) })
  } catch (error) {
    if (error instanceof OpenScadSurfaceDataError) throw dataErrorAtPath(error, path)
    throw error
  }
}

/** Triangulate the OpenSCAD 2021 surface solid with a midpoint fan per cell. */
export function triangulateOpenScadSurface(
  map: OpenScadSurfaceHeightMap,
  center: boolean,
): OpenScadSurfaceMesh {
  const { rows, columns, values, base } = map
  const triangleCount = surfaceTriangleCount(rows, columns)
  if (triangleCount > OPENSCAD_SURFACE_MAX_TRIANGLES) {
    return dataError(
      'E_SURFACE_TRIANGLE_LIMIT',
      `surface() heightmap would exceed ${OPENSCAD_SURFACE_MAX_TRIANGLES.toLocaleString()} triangles.`,
      OPENSCAD_SURFACE_MAX_TRIANGLES,
      triangleCount,
    )
  }
  const cells = (rows - 1) * (columns - 1)
  const perimeter: Array<readonly [number, number]> = []
  for (let row = 0; row < rows; row++) perimeter.push([row, 0])
  for (let column = 1; column < columns; column++) perimeter.push([rows - 1, column])
  for (let row = rows - 2; row >= 0; row--) perimeter.push([row, columns - 1])
  for (let column = columns - 2; column > 0; column--) perimeter.push([0, column])

  const vertexCount = rows * columns + cells + perimeter.length + 1
  const vertices = new Float32Array(vertexCount * 3)
  const triangles = new Uint32Array(triangleCount * 3)
  const ox = center ? -(columns - 1) / 2 : 0
  const oy = center ? -(rows - 1) / 2 : 0
  let vertex = 0
  const addVertex = (x: number, y: number, z: number): number => {
    const index = vertex++
    vertices[index * 3] = x
    vertices[index * 3 + 1] = y
    vertices[index * 3 + 2] = z
    return index
  }
  const gridIndex = (row: number, column: number) => row * columns + column
  for (let row = 0; row < rows; row++) {
    for (let column = 0; column < columns; column++) {
      addVertex(ox + column, oy + row, values[gridIndex(row, column)])
    }
  }
  const centerStart = vertex
  for (let row = 0; row < rows - 1; row++) {
    for (let column = 0; column < columns - 1; column++) {
      const average = (
        values[gridIndex(row, column)]
        + values[gridIndex(row, column + 1)]
        + values[gridIndex(row + 1, column)]
        + values[gridIndex(row + 1, column + 1)]
      ) / 4
      addVertex(ox + column + 0.5, oy + row + 0.5, average)
    }
  }
  const baseByGrid = new Map<number, number>()
  for (const [row, column] of perimeter) {
    baseByGrid.set(gridIndex(row, column), addVertex(ox + column, oy + row, base))
  }
  const baseCenter = addVertex(ox + (columns - 1) / 2, oy + (rows - 1) / 2, base)

  let triangle = 0
  const addTriangle = (a: number, b: number, c: number): void => {
    triangles[triangle * 3] = a
    triangles[triangle * 3 + 1] = b
    triangles[triangle * 3 + 2] = c
    triangle++
  }
  for (let row = 0; row < rows - 1; row++) {
    for (let column = 0; column < columns - 1; column++) {
      const topLeft = gridIndex(row, column)
      const topRight = gridIndex(row, column + 1)
      const bottomLeft = gridIndex(row + 1, column)
      const bottomRight = gridIndex(row + 1, column + 1)
      const middle = centerStart + row * (columns - 1) + column
      addTriangle(topLeft, topRight, middle)
      addTriangle(topRight, bottomRight, middle)
      addTriangle(bottomRight, bottomLeft, middle)
      addTriangle(bottomLeft, topLeft, middle)
    }
  }
  for (let index = 0; index < perimeter.length; index++) {
    const [rowA, columnA] = perimeter[index]
    const [rowB, columnB] = perimeter[(index + 1) % perimeter.length]
    const topA = gridIndex(rowA, columnA)
    const topB = gridIndex(rowB, columnB)
    const baseA = baseByGrid.get(topA)!
    const baseB = baseByGrid.get(topB)!
    // Perimeter order is clockwise from below (the bottom's outward view).
    // Side winding depends on that same order: base -> top -> next-top.
    addTriangle(baseA, topA, topB)
    addTriangle(baseA, topB, baseB)
    addTriangle(baseCenter, baseA, baseB)
  }
  if (triangle !== triangleCount || vertex !== vertexCount) {
    throw new Error('surface() triangulation accounting invariant failed')
  }
  return Object.freeze({ vertices, triangles, triangleCount })
}
