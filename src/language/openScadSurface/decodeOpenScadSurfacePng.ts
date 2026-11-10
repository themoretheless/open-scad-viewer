import { ADAM7, OPENSCAD_SURFACE_MAX_PREPARED_PIXELS, OPENSCAD_SURFACE_MAX_TRIANGLES, OpenScadSurfaceDataError, PNG_SIGNATURE } from '../openScadSurface'
import type { DecodedPngLuminance, OpenScadSurfaceHeightMap, OpenScadSurfaceMesh, PreparedOpenScadSurfaceAssets } from '../openScadSurface'
import { OpenScadProject, resolveOpenScadProjectPath } from '../openScadProject'
import { channelsForColorType, dataError, enforceDimensions, expectedInflatedBytes, heightMap, inflatePng, isPng, parseOpenScadSurfaceDat, passSize, pixelLuminance, pngCrc, surfaceTriangleCount, unfilter, validateBitDepth } from './parseOpenScadSurfaceDat'
import { u32 } from '../../core/bytes'

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
          `surface() project PNG decode budget exceeds ${OPENSCAD_SURFACE_MAX_PREPARED_PIXELS.toLocaleString('en-US')} pixels.`,
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

export function dataErrorAtPath(error: OpenScadSurfaceDataError, path: string): OpenScadSurfaceDataError {
  if (error.assetPath === path) return error
  return new OpenScadSurfaceDataError(error.code, error.message, error.limit, error.actual, path)
}

export function pngHeightMap(decoded: DecodedPngLuminance, invert: boolean): OpenScadSurfaceHeightMap {
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
      `surface() heightmap would exceed ${OPENSCAD_SURFACE_MAX_TRIANGLES.toLocaleString('en-US')} triangles.`,
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
