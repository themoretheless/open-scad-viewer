/**
 * Unified mesh file import. Decodes STL (ASCII/binary), OBJ, PLY (ASCII/binary),
 * OFF, AMF and 3MF into one welded, indexed triangle mesh that the mesh
 * modeler, the Solid workbench and the format converter can all consume.
 *
 * STL/OFF/AMF/3MF reuse the bounded OpenSCAD `import()` decoders. OBJ and PLY
 * are decoded here with the same limits. File normals, colours, UVs, textures
 * and per-object identities are deliberately dropped: only positions and
 * triangle connectivity cross this boundary.
 */
import { sha256Hex } from '../core/sha256'
import {
  OPENSCAD_IMPORT_MAX_TRIANGLES,
  OPENSCAD_IMPORT_MAX_VERTICES,
  OpenScadImportDataError,
  parseOpenScad3mf,
  parseOpenScadAmf,
  parseOpenScadOff,
  parseOpenScadStl,
  type OpenScadImportGeometry3D,
} from './openScadImport'
import type { OpenScadProjectBlobFile } from './openScadProject'
import type { PolygonMesh } from './geometry/polygon'
import { asciiPrefix, detectMeshImportFormat, MESH_IMPORT_FORMATS, MESH_IMPORT_MAX_BYTES, type MeshImportFormat } from './meshFormats'

export { detectMeshImportFormat, MESH_IMPORT_ACCEPT, MESH_IMPORT_FORMATS, MESH_IMPORT_MAX_BYTES, stripMeshExtension, type MeshImportFormat } from './meshFormats'

export const MESH_IMPORT_MAX_TRIANGLES = OPENSCAD_IMPORT_MAX_TRIANGLES
export const MESH_IMPORT_MAX_VERTICES = OPENSCAD_IMPORT_MAX_VERTICES

export type MeshImportErrorCode =
  | 'unsupported-format'
  | 'too-large'
  | 'encoding'
  | 'invalid-data'
  | 'limit'
  | 'empty'

export class MeshImportError extends Error {
  constructor(readonly code: MeshImportErrorCode, message: string, readonly format?: MeshImportFormat) {
    super(message)
    this.name = 'MeshImportError'
  }
}

export interface ImportedMesh extends PolygonMesh {
  readonly format: MeshImportFormat
  readonly triangleCount: number
  readonly vertexCount: number
  /** Vertex count before welding; equal to `vertexCount` for already indexed files. */
  readonly sourceVertexCount: number
  /** Triangles dropped because welding collapsed them. */
  readonly degenerateTriangles: number
}

export interface MeshImportOptions {
  /** Override extension-based detection. */
  readonly format?: MeshImportFormat
  /**
   * Absolute welding tolerance. `0` (default) merges only bit-identical
   * positions, which is enough to close well-formed STL/PLY soups without
   * distorting geometry. `false` keeps the file's own indexing.
   */
  readonly weld?: number | false
  readonly maxBytes?: number
}

/** Decode a mesh file into one welded indexed triangle mesh. */
export async function importMeshFile(
  fileName: string,
  input: Uint8Array | ArrayBuffer,
  options: MeshImportOptions = {},
): Promise<ImportedMesh> {
  const bytes = input instanceof Uint8Array ? input : new Uint8Array(input)
  const maxBytes = options.maxBytes ?? MESH_IMPORT_MAX_BYTES
  if (bytes.byteLength > maxBytes) {
    throw new MeshImportError('too-large', `${fileName} exceeds the ${(maxBytes / 1_000_000).toFixed(0)} MB import limit.`)
  }
  const format = options.format ?? detectMeshImportFormat(fileName, bytes)
  if (!format) {
    throw new MeshImportError('unsupported-format', `Cannot detect a supported mesh format for ${fileName}. Supported: ${MESH_IMPORT_FORMATS.map(f => f.toUpperCase()).join(', ')}.`)
  }
  const raw = await decode(format, fileName, bytes)
  return finalize(format, raw.positions, raw.indices, options.weld ?? 0)
}

/** Convenience wrapper for browser `File` objects. */
export async function importMeshFromFile(file: Blob & { name?: string }, options: MeshImportOptions = {}): Promise<ImportedMesh> {
  return importMeshFile(file.name ?? 'mesh', await file.arrayBuffer(), options)
}

interface RawMesh { positions: ArrayLike<number>; indices: ArrayLike<number> }

async function decode(format: MeshImportFormat, fileName: string, bytes: Uint8Array): Promise<RawMesh> {
  try {
    switch (format) {
      case 'stl': return fromOpenScad(parseOpenScadStl(blobFile(fileName, bytes)))
      case 'off': return fromOpenScad(parseOpenScadOff(blobFile(fileName, bytes)))
      case 'amf': return fromOpenScad(await parseOpenScadAmf(blobFile(fileName, bytes)))
      case '3mf': return fromOpenScad(await parseOpenScad3mf(blobFile(fileName, bytes)))
      case 'obj': return parseObj(utf8(bytes, 'obj'))
      case 'ply': return parsePly(bytes)
    }
  } catch (error) {
    if (error instanceof MeshImportError) throw error
    if (error instanceof OpenScadImportDataError) {
      const code: MeshImportErrorCode = error.code === 'E_IMPORT_LIMIT' ? 'limit'
        : error.code === 'E_IMPORT_EMPTY' ? 'empty'
        : error.code === 'E_IMPORT_ENCODING' ? 'encoding'
        : 'invalid-data'
      throw new MeshImportError(code, error.message, format)
    }
    throw new MeshImportError('invalid-data', error instanceof Error ? error.message : String(error), format)
  }
}

function blobFile(path: string, data: Uint8Array): OpenScadProjectBlobFile {
  return { kind: 'blob', path, data, byteLength: data.byteLength, sha256: sha256Hex(data) }
}

function fromOpenScad(geometry: OpenScadImportGeometry3D): RawMesh {
  return { positions: geometry.vertices, indices: geometry.triangles }
}

const UTF8_FATAL = new TextDecoder('utf-8', { fatal: true })

function utf8(bytes: Uint8Array, format: MeshImportFormat): string {
  try {
    return UTF8_FATAL.decode(bytes).replace(/^\uFEFF/u, '')
  } catch {
    throw new MeshImportError('encoding', `${format.toUpperCase()} input must be valid UTF-8 text.`, format)
  }
}

function invalid(format: MeshImportFormat, message: string): never {
  throw new MeshImportError('invalid-data', `${format.toUpperCase()}: ${message}`, format)
}

function checkLimit(format: MeshImportFormat, value: number, limit: number, label: string): void {
  if (value > limit) {
    throw new MeshImportError('limit', `${format.toUpperCase()} ${label} ${value.toLocaleString()} exceeds the ${limit.toLocaleString()} limit.`, format)
  }
}

const NUMBER = /^[+-]?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?$/u

function number(format: MeshImportFormat, token: string | undefined, label: string): number {
  if (token === undefined || !NUMBER.test(token)) return invalid(format, `${label} is not a number.`)
  const value = Number(token)
  if (!Number.isFinite(value)) return invalid(format, `${label} must be finite.`)
  return value
}

function integer(format: MeshImportFormat, token: string | undefined, label: string): number {
  if (token === undefined || !/^[+-]?\d+$/u.test(token)) return invalid(format, `${label} is not an integer.`)
  return Number(token)
}

/** Fan-triangulate a polygon given as vertex indices; caller validates ranges. */
function fan(indices: number[], face: readonly number[]): void {
  for (let i = 1; i + 1 < face.length; i++) indices.push(face[0], face[i], face[i + 1])
}

// ---------------------------------------------------------------------------
// OBJ

function parseObj(text: string): RawMesh {
  const positions: number[] = []
  const indices: number[] = []
  let vertexCount = 0
  for (const raw of text.split(/\r\n|\n|\r/u)) {
    const hash = raw.indexOf('#')
    const line = (hash >= 0 ? raw.slice(0, hash) : raw).trim()
    if (!line) continue
    const parts = line.split(/\s+/u)
    const keyword = parts[0]
    if (keyword === 'v') {
      if (parts.length < 4) invalid('obj', 'vertex needs three coordinates.')
      checkLimit('obj', vertexCount + 1, MESH_IMPORT_MAX_VERTICES, 'vertex count')
      const w = parts.length >= 5 && parts.length !== 7 ? number('obj', parts[4], 'vertex w') : 1
      if (w === 0) invalid('obj', 'vertex w must be non-zero.')
      positions.push(
        number('obj', parts[1], 'vertex x') / w,
        number('obj', parts[2], 'vertex y') / w,
        number('obj', parts[3], 'vertex z') / w,
      )
      vertexCount++
    } else if (keyword === 'f') {
      if (parts.length < 4) invalid('obj', 'face needs at least three vertices.')
      const face: number[] = []
      for (const token of parts.slice(1)) {
        const index = integer('obj', token.split('/')[0], 'face vertex index')
        if (index === 0) invalid('obj', 'face vertex index 0 is invalid.')
        const resolved = index > 0 ? index - 1 : vertexCount + index
        if (resolved < 0 || resolved >= vertexCount) invalid('obj', `face references vertex ${index} before it is defined.`)
        face.push(resolved)
      }
      checkLimit('obj', indices.length / 3 + face.length - 2, MESH_IMPORT_MAX_TRIANGLES, 'triangle count')
      fan(indices, face)
    }
    // vn, vt, vp, o, g, s, mtllib, usemtl, l, p, curve/surface statements are ignored.
  }
  if (indices.length === 0) throw new MeshImportError('empty', 'OBJ contains no faces.', 'obj')
  return { positions, indices }
}

// ---------------------------------------------------------------------------
// PLY

type PlyScalar = 'char' | 'uchar' | 'short' | 'ushort' | 'int' | 'uint' | 'float' | 'double'
interface PlyProperty { name: string; type: PlyScalar; list?: { count: PlyScalar; item: PlyScalar } }
interface PlyElement { name: string; count: number; properties: PlyProperty[] }
type PlyFormat = 'ascii' | 'binary_little_endian' | 'binary_big_endian'

const PLY_SCALARS: Readonly<Record<string, PlyScalar>> = Object.freeze({
  char: 'char', int8: 'char',
  uchar: 'uchar', uint8: 'uchar',
  short: 'short', int16: 'short',
  ushort: 'ushort', uint16: 'ushort',
  int: 'int', int32: 'int',
  uint: 'uint', uint32: 'uint',
  float: 'float', float32: 'float',
  double: 'double', float64: 'double',
})

const PLY_SIZES: Readonly<Record<PlyScalar, number>> = Object.freeze({
  char: 1, uchar: 1, short: 2, ushort: 2, int: 4, uint: 4, float: 4, double: 8,
})

function plyScalar(token: string | undefined): PlyScalar {
  const type = token === undefined ? undefined : PLY_SCALARS[token]
  return type ?? invalid('ply', `unknown property type ${JSON.stringify(token)}.`)
}

function parsePlyHeader(bytes: Uint8Array): { format: PlyFormat; elements: PlyElement[]; bodyOffset: number } {
  // The header is ASCII and ends with "end_header" followed by a newline.
  const limit = Math.min(bytes.byteLength, 1 << 20)
  let end = -1
  for (let i = 0; i + 10 <= limit; i++) {
    if (bytes[i] === 0x65 && asciiPrefix(bytes.subarray(i, i + 10), 10) === 'end_header') {
      end = i + 10
      break
    }
  }
  if (end < 0) invalid('ply', 'header is missing end_header.')
  // Consume the line terminator after end_header (\n, \r\n or \r).
  let bodyOffset = end
  if (bytes[bodyOffset] === 0x0d) bodyOffset++
  if (bytes[bodyOffset] === 0x0a) bodyOffset++
  const headerText = asciiPrefix(bytes.subarray(0, end), end)
  const lines = headerText.split(/\r\n|\n|\r/u).map(line => line.trim()).filter(Boolean)
  if (lines[0] !== 'ply') invalid('ply', 'file must start with "ply".')
  let format: PlyFormat | null = null
  const elements: PlyElement[] = []
  for (const line of lines.slice(1)) {
    const parts = line.split(/\s+/u)
    switch (parts[0]) {
      case 'format': {
        if (parts[1] !== 'ascii' && parts[1] !== 'binary_little_endian' && parts[1] !== 'binary_big_endian') {
          invalid('ply', `unsupported format ${JSON.stringify(parts[1])}.`)
        }
        if (parts[2] !== '1.0') invalid('ply', `unsupported version ${JSON.stringify(parts[2])}.`)
        format = parts[1] as PlyFormat
        break
      }
      case 'comment':
      case 'obj_info':
        break
      case 'element': {
        const count = integer('ply', parts[2], `element ${parts[1]} count`)
        if (count < 0) invalid('ply', 'element count must be non-negative.')
        elements.push({ name: parts[1], count, properties: [] })
        break
      }
      case 'property': {
        const element = elements[elements.length - 1] ?? invalid('ply', 'property declared before any element.')
        if (parts[1] === 'list') {
          element.properties.push({ name: parts[4], type: 'int', list: { count: plyScalar(parts[2]), item: plyScalar(parts[3]) } })
        } else {
          element.properties.push({ name: parts[2], type: plyScalar(parts[1]) })
        }
        break
      }
      case 'end_header':
        break
      default:
        invalid('ply', `unknown header statement ${JSON.stringify(parts[0])}.`)
    }
  }
  if (!format) invalid('ply', 'header is missing a format line.')
  return { format, elements, bodyOffset }
}

function parsePly(bytes: Uint8Array): RawMesh {
  const { format, elements, bodyOffset } = parsePlyHeader(bytes)
  const vertexElement = elements.find(e => e.name === 'vertex') ?? invalid('ply', 'header declares no vertex element.')
  const faceElement = elements.find(e => e.name === 'face') ?? invalid('ply', 'header declares no face element.')
  checkLimit('ply', vertexElement.count, MESH_IMPORT_MAX_VERTICES, 'vertex count')
  checkLimit('ply', faceElement.count, MESH_IMPORT_MAX_TRIANGLES, 'face count')
  const axis = ['x', 'y', 'z'].map(name => {
    const index = vertexElement.properties.findIndex(p => p.name === name)
    if (index < 0 || vertexElement.properties[index].list) invalid('ply', `vertex element lacks scalar property ${name}.`)
    return index
  })
  const faceListIndex = faceElement.properties.findIndex(p => p.list && (p.name === 'vertex_indices' || p.name === 'vertex_index'))
  if (faceListIndex < 0) invalid('ply', 'face element lacks a vertex_indices list.')

  const positions: number[] = []
  const indices: number[] = []
  const reader = format === 'ascii' ? asciiPlyReader(bytes, bodyOffset) : binaryPlyReader(bytes, bodyOffset, format === 'binary_little_endian')

  for (const element of elements) {
    const isVertex = element === vertexElement
    const isFace = element === faceElement
    for (let i = 0; i < element.count; i++) {
      const scalars: number[] = []
      let face: number[] | null = null
      for (let p = 0; p < element.properties.length; p++) {
        const property = element.properties[p]
        if (property.list) {
          const count = reader.scalar(property.list.count)
          if (!Number.isSafeInteger(count) || count < 0) invalid('ply', 'list length must be a non-negative integer.')
          const items: number[] = []
          for (let k = 0; k < count; k++) items.push(reader.scalar(property.list.item))
          if (isFace && p === faceListIndex) face = items
          scalars.push(Number.NaN)
        } else {
          scalars.push(reader.scalar(property.type))
        }
      }
      if (isVertex) {
        for (const a of axis) {
          const value = scalars[a]
          if (!Number.isFinite(value)) invalid('ply', `vertex ${i} has a non-finite coordinate.`)
          positions.push(value)
        }
      } else if (isFace && face) {
        if (face.length < 3) invalid('ply', `face ${i} has fewer than three vertices.`)
        for (const index of face) {
          if (!Number.isSafeInteger(index) || index < 0 || index >= vertexElement.count) invalid('ply', `face ${i} index ${index} is out of range.`)
        }
        checkLimit('ply', indices.length / 3 + face.length - 2, MESH_IMPORT_MAX_TRIANGLES, 'triangle count')
        fan(indices, face)
      }
    }
  }
  if (indices.length === 0) throw new MeshImportError('empty', 'PLY contains no faces.', 'ply')
  return { positions, indices }
}

interface PlyReader { scalar(type: PlyScalar): number }

function asciiPlyReader(bytes: Uint8Array, offset: number): PlyReader {
  const tokens = utf8(bytes.subarray(offset), 'ply').split(/\s+/u).filter(Boolean)
  let cursor = 0
  return {
    scalar(type) {
      const token = tokens[cursor++]
      if (token === undefined) invalid('ply', 'body ended before all declared elements were read.')
      return type === 'float' || type === 'double' ? number('ply', token, 'value') : integer('ply', token, 'value')
    },
  }
}

function binaryPlyReader(bytes: Uint8Array, offset: number, littleEndian: boolean): PlyReader {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  let cursor = offset
  return {
    scalar(type) {
      const size = PLY_SIZES[type]
      if (cursor + size > bytes.byteLength) invalid('ply', 'body ended before all declared elements were read.')
      const at = cursor
      cursor += size
      switch (type) {
        case 'char': return view.getInt8(at)
        case 'uchar': return view.getUint8(at)
        case 'short': return view.getInt16(at, littleEndian)
        case 'ushort': return view.getUint16(at, littleEndian)
        case 'int': return view.getInt32(at, littleEndian)
        case 'uint': return view.getUint32(at, littleEndian)
        case 'float': return view.getFloat32(at, littleEndian)
        case 'double': return view.getFloat64(at, littleEndian)
      }
    },
  }
}

// ---------------------------------------------------------------------------
// Welding / finalisation

function finalize(format: MeshImportFormat, positions: ArrayLike<number>, indices: ArrayLike<number>, weld: number | false): ImportedMesh {
  const sourceVertexCount = positions.length / 3
  if (positions.length % 3 !== 0 || indices.length % 3 !== 0) invalid(format, 'mesh storage is inconsistent.')
  checkLimit(format, sourceVertexCount, MESH_IMPORT_MAX_VERTICES, 'vertex count')
  checkLimit(format, indices.length / 3, MESH_IMPORT_MAX_TRIANGLES, 'triangle count')

  const remap = new Uint32Array(sourceVertexCount)
  const outPositions: number[] = []
  if (weld === false) {
    for (let i = 0; i < sourceVertexCount; i++) remap[i] = i
    for (let i = 0; i < positions.length; i++) outPositions.push(positions[i])
  } else {
    const scale = weld > 0 ? 1 / weld : 0
    const seen = new Map<string, number>()
    for (let i = 0; i < sourceVertexCount; i++) {
      const x = positions[i * 3], y = positions[i * 3 + 1], z = positions[i * 3 + 2]
      if (!Number.isFinite(x) || !Number.isFinite(y) || !Number.isFinite(z)) invalid(format, `vertex ${i} has a non-finite coordinate.`)
      const key = scale > 0
        ? `${Math.round(x * scale)},${Math.round(y * scale)},${Math.round(z * scale)}`
        : `${Math.fround(x)},${Math.fround(y)},${Math.fround(z)}`
      let index = seen.get(key)
      if (index === undefined) {
        index = outPositions.length / 3
        seen.set(key, index)
        outPositions.push(x, y, z)
      }
      remap[i] = index
    }
  }

  const outIndices: number[] = []
  let degenerate = 0
  for (let t = 0; t < indices.length; t += 3) {
    const a = indices[t], b = indices[t + 1], c = indices[t + 2]
    if (![a, b, c].every(i => Number.isSafeInteger(i) && i >= 0 && i < sourceVertexCount)) invalid(format, 'triangle index is out of bounds.')
    const ra = remap[a], rb = remap[b], rc = remap[c]
    if (ra === rb || rb === rc || rc === ra) { degenerate++; continue }
    outIndices.push(ra, rb, rc)
  }
  if (outIndices.length === 0) throw new MeshImportError('empty', `${format.toUpperCase()} contains no usable triangles.`, format)

  return Object.freeze({
    format,
    positions: outPositions,
    indices: outIndices,
    triangleCount: outIndices.length / 3,
    vertexCount: outPositions.length / 3,
    sourceVertexCount,
    degenerateTriangles: degenerate,
  })
}
