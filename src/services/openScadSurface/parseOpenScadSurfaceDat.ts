import { ADAM7, CRC_TABLE, NUMBER_TOKEN, OPENSCAD_SURFACE_MAX_DIMENSION, OPENSCAD_SURFACE_MAX_PIXELS, OPENSCAD_SURFACE_MAX_TRIANGLES, OpenScadSurfaceDataError, PNG_SIGNATURE, UTF8_FATAL } from '../openScadSurface'
import type { OpenScadSurfaceHeightMap } from '../openScadSurface'
import type { OpenScadProjectFile } from '../openScadProject'

export function dataError(
  code: OpenScadSurfaceDataError['code'],
  message: string,
  limit?: number,
  actual?: number,
): never {
  throw new OpenScadSurfaceDataError(code, message, limit, actual)
}

export function isPng(bytes: Uint8Array): boolean {
  return bytes.byteLength >= PNG_SIGNATURE.length
    && PNG_SIGNATURE.every((value, index) => bytes[index] === value)
}

export function surfaceTriangleCount(rows: number, columns: number): number {
  if (rows < 2 || columns < 2) return 0
  const cells = (rows - 1) * (columns - 1)
  const perimeterSegments = 2 * (rows - 1) + 2 * (columns - 1)
  return 4 * cells + 3 * perimeterSegments
}

export function enforceDimensions(rows: number, columns: number): void {
  const dimension = Math.max(rows, columns)
  if (dimension > OPENSCAD_SURFACE_MAX_DIMENSION) {
    dataError(
      'E_SURFACE_DIMENSION_LIMIT',
      `surface() heightmap dimension exceeds ${OPENSCAD_SURFACE_MAX_DIMENSION.toLocaleString('en-US')} samples.`,
      OPENSCAD_SURFACE_MAX_DIMENSION,
      dimension,
    )
  }
  const pixels = rows * columns
  if (!Number.isSafeInteger(pixels) || pixels > OPENSCAD_SURFACE_MAX_PIXELS) {
    dataError(
      'E_SURFACE_DIMENSION_LIMIT',
      `surface() heightmap exceeds ${OPENSCAD_SURFACE_MAX_PIXELS.toLocaleString('en-US')} samples.`,
      OPENSCAD_SURFACE_MAX_PIXELS,
      pixels,
    )
  }
  const triangles = surfaceTriangleCount(rows, columns)
  if (!Number.isSafeInteger(triangles) || triangles > OPENSCAD_SURFACE_MAX_TRIANGLES) {
    dataError(
      'E_SURFACE_TRIANGLE_LIMIT',
      `surface() heightmap would exceed ${OPENSCAD_SURFACE_MAX_TRIANGLES.toLocaleString('en-US')} triangles.`,
      OPENSCAD_SURFACE_MAX_TRIANGLES,
      triangles,
    )
  }
}

export function heightMap(
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
        `surface() heightmap dimension exceeds ${OPENSCAD_SURFACE_MAX_DIMENSION.toLocaleString('en-US')} samples.`,
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
          `surface() heightmap dimension exceeds ${OPENSCAD_SURFACE_MAX_DIMENSION.toLocaleString('en-US')} samples.`,
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
        `surface() heightmap exceeds ${OPENSCAD_SURFACE_MAX_PIXELS.toLocaleString('en-US')} samples.`,
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

export function pngCrc(bytes: Uint8Array, start: number, end: number): number {
  let crc = 0xffffffff
  for (let index = start; index < end; index++) crc = CRC_TABLE[(crc ^ bytes[index]) & 0xff] ^ (crc >>> 8)
  return (crc ^ 0xffffffff) >>> 0
}

export function channelsForColorType(colorType: number): number {
  switch (colorType) {
    case 0: return 1
    case 2: return 3
    case 3: return 1
    case 4: return 2
    case 6: return 4
    default: return dataError('E_SURFACE_PNG_UNSUPPORTED', `surface() PNG color type ${colorType} is unsupported.`)
  }
}

export function validateBitDepth(colorType: number, bitDepth: number): void {
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

export function passSize(total: number, start: number, step: number): number {
  return total <= start ? 0 : Math.ceil((total - start) / step)
}

export function expectedInflatedBytes(
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

export async function inflatePng(bytes: Uint8Array, expectedBytes: number): Promise<Uint8Array> {
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

export function paeth(left: number, up: number, upperLeft: number): number {
  const estimate = left + up - upperLeft
  const dl = Math.abs(estimate - left)
  const du = Math.abs(estimate - up)
  const dul = Math.abs(estimate - upperLeft)
  return dl <= du && dl <= dul ? left : du <= dul ? up : upperLeft
}

export function unfilter(
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

export function sample(row: Uint8Array, index: number, bitDepth: number): number {
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

export function pixelLuminance(
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
