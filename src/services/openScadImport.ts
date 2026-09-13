import {
  OpenScadProject,
  OpenScadProjectError,
  OPENSCAD_PROJECT_MAX_FILES,
  normalizeOpenScadProjectPath,
  resolveOpenScadProjectPath,
  type OpenScadProjectFile,
} from './openScadProject'
import type { CallNode } from './openscadCompiler'
import { OpenSCADParseError } from './openscadErrors'
import { readSvgDocument, type SvgOptions } from './svgDocument'

export const OPENSCAD_IMPORT_MAX_TRIANGLES = 250_000
export const OPENSCAD_IMPORT_MAX_VERTICES = 750_000
export const OPENSCAD_IMPORT_MAX_2D_POINTS = 500_000
export const OPENSCAD_IMPORT_MAX_CONTOURS = 50_000
export const OPENSCAD_IMPORT_MAX_XML_ELEMENTS = 250_000
export const OPENSCAD_IMPORT_MAX_XML_DEPTH = 64
export const OPENSCAD_IMPORT_MAX_ZIP_ENTRIES = 128
export const OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES = 12 * 1024 * 1024
export const OPENSCAD_IMPORT_MAX_PROJECT_TRIANGLES = OPENSCAD_IMPORT_MAX_TRIANGLES
export const OPENSCAD_IMPORT_MAX_PROJECT_VERTICES = OPENSCAD_IMPORT_MAX_VERTICES
export const OPENSCAD_IMPORT_MAX_PROJECT_2D_POINTS = OPENSCAD_IMPORT_MAX_2D_POINTS
export const OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES = OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES
export const OPENSCAD_IMPORT_DEFAULT_CURVE_SEGMENTS = 40
export const OPENSCAD_IMPORT_MAX_DXF_QUERY_RECORDS = OPENSCAD_IMPORT_MAX_XML_ELEMENTS
export const OPENSCAD_IMPORT_MAX_PROJECT_DXF_QUERY_RECORDS = OPENSCAD_IMPORT_MAX_DXF_QUERY_RECORDS

export const OPENSCAD_IMPORT_FORMATS = Object.freeze([
  'svg',
  'dxf',
  'stl',
  'off',
  'amf',
  '3mf',
] as const)

/**
 * Stable 2021.01 import profile. NEF3 is deliberately excluded: upstream only
 * enables it in CGAL builds, while the independent engine is Manifold-only.
 */
export const OPENSCAD_IMPORT_CAPABILITIES = Object.freeze({
  profile: 'OpenSCAD-2021.01' as const,
  formats: OPENSCAD_IMPORT_FORMATS,
  conditionalExclusions: Object.freeze([
    Object.freeze({
      format: 'nef3' as const,
      requires: 'CGAL' as const,
      diagnostic: 'E_IMPORT_NEF3_UNAVAILABLE' as const,
    }),
  ]),
})

export type OpenScadImportFormat = typeof OPENSCAD_IMPORT_FORMATS[number]

export const OPENSCAD_2021_LEGACY_IMPORT_FORMATS = Object.freeze([
  'stl',
  'off',
  'dxf',
] as const)

export type OpenScad2021LegacyImportFormat = typeof OPENSCAD_2021_LEGACY_IMPORT_FORMATS[number]

export const OPENSCAD_IMPORT_ERROR_CODES = Object.freeze([
  'E_IMPORT_PROJECT_REQUIRED',
  'E_IMPORT_FILE_REQUIRED',
  'E_IMPORT_PATH_INVALID',
  'E_IMPORT_FILE_MISSING',
  'E_IMPORT_FORMAT_UNSUPPORTED',
  'E_IMPORT_ENCODING',
  'E_IMPORT_INVALID_DATA',
  'E_IMPORT_UNSUPPORTED_FEATURE',
  'E_IMPORT_ARGUMENT_INVALID',
  'E_IMPORT_LIMIT',
  'E_IMPORT_EMPTY',
  'E_IMPORT_NEF3_UNAVAILABLE',
  'E_IMPORT_NON_MANIFOLD',
] as const)

export type OpenScadImportErrorCode = typeof OPENSCAD_IMPORT_ERROR_CODES[number]

export interface OpenScadImportErrorDetails {
  readonly sourcePath: string
  readonly specifier: string
  readonly assetPath?: string
  readonly format?: OpenScadImportFormat
  readonly limit?: number
  readonly actual?: number
}

/** Project-positioned import diagnostic, suitable for wrapping at an authored call. */
export class OpenScadImportError extends Error {
  readonly details: Readonly<OpenScadImportErrorDetails>
  readonly sourcePath: string
  readonly specifier: string
  readonly assetPath?: string
  readonly format?: OpenScadImportFormat
  readonly limit?: number
  readonly actual?: number

  constructor(
    readonly code: OpenScadImportErrorCode,
    message: string,
    details: OpenScadImportErrorDetails,
  ) {
    super(message)
    this.name = 'OpenScadImportError'
    this.details = Object.freeze({ ...details })
    this.sourcePath = details.sourcePath
    this.specifier = details.specifier
    this.assetPath = details.assetPath
    this.format = details.format
    this.limit = details.limit
    this.actual = details.actual
  }
}

export class OpenScadImportDataError extends Error {
  constructor(
    readonly code: Exclude<OpenScadImportErrorCode,
      | 'E_IMPORT_PROJECT_REQUIRED'
      | 'E_IMPORT_FILE_REQUIRED'
      | 'E_IMPORT_PATH_INVALID'
      | 'E_IMPORT_FILE_MISSING'
      | 'E_IMPORT_NEF3_UNAVAILABLE'
      | 'E_IMPORT_ARGUMENT_INVALID'
      | 'E_IMPORT_NON_MANIFOLD'>,
    message: string,
    readonly format?: OpenScadImportFormat,
    readonly limit?: number,
    readonly actual?: number,
  ) {
    super(message)
    this.name = 'OpenScadImportDataError'
  }
}

/** Source-positioned diagnostic emitted by an authored import() module call. */
export class OpenScadImportPositionedError extends OpenSCADParseError {
  readonly details: Readonly<OpenScadImportErrorDetails>
  readonly sourcePath: string
  readonly specifier: string
  readonly assetPath?: string
  readonly format?: OpenScadImportFormat
  readonly limit?: number
  readonly actual?: number

  constructor(
    source: string,
    node: Pick<CallNode, 'p' | 'end'>,
    readonly importCode: OpenScadImportErrorCode,
    message: string,
    details: OpenScadImportErrorDetails,
  ) {
    super(source, node.p, message, importCode, node.end)
    this.name = 'OpenScadImportPositionedError'
    this.details = Object.freeze({ ...details })
    this.sourcePath = details.sourcePath
    this.specifier = details.specifier
    this.assetPath = details.assetPath
    this.format = details.format
    this.limit = details.limit
    this.actual = details.actual
  }
}

export type OpenScadImportFillRule = 'evenodd' | 'nonzero'
export type OpenScadImportPoint2 = readonly [number, number]

export interface OpenScadImportRegion2D {
  /** One or more contours evaluated together using the declared fill rule. */
  readonly contours: readonly (readonly OpenScadImportPoint2[])[]
  readonly fillRule: OpenScadImportFillRule
  /** DXF layer; absent for SVG. */
  readonly layer?: string
}

export interface OpenScadImportGeometry2D {
  readonly dimension: 2
  readonly format: 'svg' | 'dxf'
  readonly regions: readonly OpenScadImportRegion2D[]
  readonly pointCount: number
}

export interface OpenScadImportGeometry3D {
  readonly dimension: 3
  readonly format: 'stl' | 'off' | 'amf' | '3mf'
  /** XYZ triples. */
  readonly vertices: Float32Array
  /** Indexed triangles. */
  readonly triangles: Uint32Array
  readonly triangleCount: number
}

export type OpenScadImportGeometry = OpenScadImportGeometry2D | OpenScadImportGeometry3D

export interface OpenScadImportLoadOptions {
  /** Empty string selects every DXF layer, matching import()'s default. */
  readonly layer?: string
  /** DXF/SVG origin, subtracted before scale. */
  readonly origin?: readonly [number, number]
  /** Uniform import scale. */
  readonly scale?: number
  /** Center the imported result on its axis-aligned bounding box. */
  readonly center?: boolean
  /** SVG pixel density; OpenSCAD 2021.01 defaults to 72. */
  readonly dpi?: number
  /**
   * Pin the decoder used by the 2021.01 import_stl/import_off/import_dxf
   * compatibility modules. Unlike import(), those modules do not infer their
   * decoder from the filename extension.
   */
  readonly forcedFormat?: OpenScad2021LegacyImportFormat
}

export interface OpenScadImportForcedAsset {
  /** Canonical project-relative asset path. */
  readonly path: string
  readonly format: OpenScad2021LegacyImportFormat
}

export interface OpenScadImportPreparationOptions {
  /**
   * Additional format-pinned decodes discovered from legacy import aliases.
   * The list is bounded by the project's own file-count limit and is deduped
   * by (path, format).
   */
  readonly forcedAssets?: readonly OpenScadImportForcedAsset[]
}

export interface OpenScadDxfDimensionMetadata {
  readonly layer: string
  readonly name: string
  readonly type: number
  readonly angle: number
  readonly coordinates: readonly OpenScadImportPoint2[]
  /** DIMENSION records inside BLOCKS are not transformed by the 2021 parser. */
  readonly modelSpace: boolean
}

export interface OpenScadDxfLineSegmentMetadata {
  readonly layer: string
  readonly start: OpenScadImportPoint2
  readonly end: OpenScadImportPoint2
}

/** The bounded DXF data retained solely for the legacy query functions. */
export interface OpenScadDxfQueryMetadata {
  readonly dimensions: readonly OpenScadDxfDimensionMetadata[]
  /** Expanded, source-ordered edges; paths are built after dxf_cross layer filtering. */
  readonly lineSegments: readonly OpenScadDxfLineSegmentMetadata[]
  readonly recordCount: number
}

export const OPENSCAD_DXF_QUERY_DIAGNOSTIC_CODES = Object.freeze([
  'OPENSCAD_DXF_QUERY_INVALID_ORIGIN',
  'OPENSCAD_DXF_QUERY_FILE_UNAVAILABLE',
  'OPENSCAD_DXF_QUERY_NOT_PREPARED',
  'OPENSCAD_DXF_QUERY_INVALID_DATA',
  'OPENSCAD_DXF_DIMENSION_NOT_FOUND',
  'OPENSCAD_DXF_DIMENSION_TYPE_UNSUPPORTED',
  'OPENSCAD_DXF_CROSS_NOT_FOUND',
] as const)

export type OpenScadDxfQueryDiagnosticCode = typeof OPENSCAD_DXF_QUERY_DIAGNOSTIC_CODES[number]

export interface OpenScadDxfQueryDiagnostic {
  readonly severity: 'warning'
  readonly code: OpenScadDxfQueryDiagnosticCode
  readonly message: string
  readonly sourcePath: string
  readonly specifier: string
  readonly assetPath?: string
  readonly layer?: string
  readonly name?: string
  readonly importCode?: OpenScadImportErrorCode
}

export interface OpenScadDxfQueryResult<T> {
  readonly value: T | undefined
  readonly diagnostics: readonly OpenScadDxfQueryDiagnostic[]
}

export interface OpenScadDxfQueryOptions {
  readonly layer?: string
  readonly origin?: readonly [number, number]
  readonly scale?: number
}

export interface OpenScadDxfDimensionQueryOptions extends OpenScadDxfQueryOptions {
  readonly name?: string
}

export interface PreparedOpenScadImportAssets {
  readonly assets: ReadonlyMap<string, OpenScadImportGeometry | OpenScadImportDataError>
  readonly forcedAssets: ReadonlyMap<string, OpenScadImportGeometry | OpenScadImportDataError>
  readonly dxfQueryMetadata: ReadonlyMap<string, OpenScadDxfQueryMetadata | OpenScadImportDataError>
}

interface OpenScadImportPreparationBudget {
  vertices: number
  triangles: number
  points: number
  zipExpandedBytes: number
  dxfQueryRecords: number
}

const UTF8_FATAL = new TextDecoder('utf-8', { fatal: true })
const UTF8 = new TextEncoder()
const NUMBER = /^[+-]?(?:(?:\d+(?:\.\d*)?)|(?:\.\d+))(?:[eE][+-]?\d+)?$/u
const FORMAT_SET = new Set<string>(OPENSCAD_IMPORT_FORMATS)
const LEGACY_FORMAT_SET = new Set<string>(OPENSCAD_2021_LEGACY_IMPORT_FORMATS)

function dataError(
  code: OpenScadImportDataError['code'],
  message: string,
  format?: OpenScadImportFormat,
  limit?: number,
  actual?: number,
): never {
  throw new OpenScadImportDataError(code, message, format, limit, actual)
}

function finiteNumber(token: string, label: string, format: OpenScadImportFormat): number {
  if (!NUMBER.test(token)) return dataError('E_IMPORT_INVALID_DATA', `${label} is not a number.`, format)
  const value = Number(token)
  if (!Number.isFinite(value)) return dataError('E_IMPORT_INVALID_DATA', `${label} must be finite.`, format)
  return value
}

function boundedCount(
  value: number,
  limit: number,
  label: string,
  format: OpenScadImportFormat,
): number {
  if (!Number.isSafeInteger(value) || value < 0) {
    return dataError('E_IMPORT_INVALID_DATA', `${label} must be a non-negative integer.`, format)
  }
  if (value > limit) {
    return dataError('E_IMPORT_LIMIT', `${label} exceeds the ${limit.toLocaleString()} limit.`, format, limit, value)
  }
  return value
}

function bytesOf(file: OpenScadProjectFile): Uint8Array {
  return file.kind === 'blob' ? file.data : UTF8.encode(file.source)
}

function textOf(file: OpenScadProjectFile, format: OpenScadImportFormat): string {
  if (file.kind === 'source') return file.source
  try {
    return UTF8_FATAL.decode(file.data)
  } catch {
    return dataError('E_IMPORT_ENCODING', `${format.toUpperCase()} input must be valid UTF-8 text.`, format)
  }
}

function formatOfPath(path: string): OpenScadImportFormat | null {
  const dot = path.lastIndexOf('.')
  if (dot < 0) return null
  const format = path.slice(dot + 1).toLowerCase()
  return FORMAT_SET.has(format) ? format as OpenScadImportFormat : null
}

function samePoint3(vertices: ArrayLike<number>, a: number, b: number): boolean {
  return vertices[a * 3] === vertices[b * 3]
    && vertices[a * 3 + 1] === vertices[b * 3 + 1]
    && vertices[a * 3 + 2] === vertices[b * 3 + 2]
}

function triangleDegenerate(vertices: ArrayLike<number>, a: number, b: number, c: number): boolean {
  if (a === b || b === c || a === c || samePoint3(vertices, a, b)
    || samePoint3(vertices, b, c) || samePoint3(vertices, a, c)) return true
  const ax = vertices[a * 3], ay = vertices[a * 3 + 1], az = vertices[a * 3 + 2]
  const abx = vertices[b * 3] - ax, aby = vertices[b * 3 + 1] - ay, abz = vertices[b * 3 + 2] - az
  const acx = vertices[c * 3] - ax, acy = vertices[c * 3 + 1] - ay, acz = vertices[c * 3 + 2] - az
  const nx = aby * acz - abz * acy
  const ny = abz * acx - abx * acz
  const nz = abx * acy - aby * acx
  return nx === 0 && ny === 0 && nz === 0
}

function geometry3D(
  format: OpenScadImportGeometry3D['format'],
  sourceVertices: readonly number[],
  sourceTriangles: readonly number[],
): OpenScadImportGeometry3D {
  if (sourceVertices.length % 3 !== 0 || sourceTriangles.length % 3 !== 0) {
    return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} mesh storage is inconsistent.`, format)
  }
  const vertexCount = boundedCount(
    sourceVertices.length / 3,
    OPENSCAD_IMPORT_MAX_VERTICES,
    `${format.toUpperCase()} vertex count`,
    format,
  )
  boundedCount(
    sourceTriangles.length / 3,
    OPENSCAD_IMPORT_MAX_TRIANGLES,
    `${format.toUpperCase()} triangle count`,
    format,
  )
  const vertices = new Float32Array(sourceVertices.length)
  for (let index = 0; index < sourceVertices.length; index++) {
    const value = sourceVertices[index]
    const stored = Math.fround(value)
    if (!Number.isFinite(value) || !Number.isFinite(stored)) {
      return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} contains a non-finite or out-of-range coordinate.`, format)
    }
    vertices[index] = stored
  }
  const triangles: number[] = []
  for (let offset = 0; offset < sourceTriangles.length; offset += 3) {
    const a = sourceTriangles[offset], b = sourceTriangles[offset + 1], c = sourceTriangles[offset + 2]
    if (![a, b, c].every(index => Number.isSafeInteger(index) && index >= 0 && index < vertexCount)) {
      return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} triangle index is out of bounds.`, format)
    }
    if (!triangleDegenerate(vertices, a, b, c)) triangles.push(a, b, c)
  }
  if (triangles.length === 0) {
    return dataError('E_IMPORT_EMPTY', `${format.toUpperCase()} contains no non-degenerate triangles.`, format)
  }
  return Object.freeze({
    dimension: 3 as const,
    format,
    vertices,
    triangles: Uint32Array.from(triangles),
    triangleCount: triangles.length / 3,
  })
}

function cloneGeometry(geometry: OpenScadImportGeometry): OpenScadImportGeometry {
  if (geometry.dimension === 3) {
    return Object.freeze({
      ...geometry,
      vertices: new Float32Array(geometry.vertices),
      triangles: new Uint32Array(geometry.triangles),
    })
  }
  const regions = geometry.regions.map(region => Object.freeze({
    ...region,
    contours: Object.freeze(region.contours.map(contour => Object.freeze(
      contour.map(point => Object.freeze([point[0], point[1]] as const)),
    ))),
  }))
  return Object.freeze({ ...geometry, regions: Object.freeze(regions) })
}

function startsWithSolid(bytes: Uint8Array): boolean {
  let offset = bytes.byteLength >= 3 && bytes[0] === 0xef && bytes[1] === 0xbb && bytes[2] === 0xbf ? 3 : 0
  while (offset < bytes.byteLength && /\s/u.test(String.fromCharCode(bytes[offset]))) offset++
  return String.fromCharCode(...bytes.subarray(offset, offset + 5)).toLowerCase() === 'solid'
}

/** Decode binary or ASCII STL into bounded indexed-neutral triangle geometry. */
export function parseOpenScadStl(file: OpenScadProjectFile): OpenScadImportGeometry3D {
  const bytes = bytesOf(file)
  const binarySizeMatches = bytes.byteLength >= 84
    && 84 + new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(80, true) * 50 === bytes.byteLength
  if (binarySizeMatches) return parseBinaryOpenScadStl(bytes)
  if (startsWithSolid(bytes)) return parseAsciiOpenScadStl(textOf(file, 'stl'))
  return dataError('E_IMPORT_INVALID_DATA', 'STL is neither a size-consistent binary file nor an ASCII solid.', 'stl')
}

function parseBinaryOpenScadStl(bytes: Uint8Array): OpenScadImportGeometry3D {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const count = boundedCount(view.getUint32(80, true), OPENSCAD_IMPORT_MAX_TRIANGLES, 'STL triangle count', 'stl')
  if (84 + count * 50 !== bytes.byteLength) {
    return dataError('E_IMPORT_INVALID_DATA', 'Binary STL size does not match its declared triangle count.', 'stl')
  }
  const vertices: number[] = []
  const triangles: number[] = []
  let record = 84
  for (let triangle = 0; triangle < count; triangle++, record += 50) {
    const base = vertices.length / 3
    for (let channel = 0; channel < 9; channel++) {
      const value = view.getFloat32(record + 12 + channel * 4, true)
      if (!Number.isFinite(value)) {
        return dataError('E_IMPORT_INVALID_DATA', `STL triangle ${triangle + 1} contains a non-finite coordinate.`, 'stl')
      }
      vertices.push(value)
    }
    triangles.push(base, base + 1, base + 2)
  }
  return geometry3D('stl', vertices, triangles)
}

function parseAsciiOpenScadStl(text: string): OpenScadImportGeometry3D {
  const vertices: number[] = []
  const triangles: number[] = []
  let facetVertices: number[] = []
  let state: 'before' | 'solid' | 'facet' | 'loop' | 'after-loop' | 'ended' = 'before'
  for (const raw of text.replace(/^\uFEFF/u, '').split(/\r\n|\n|\r/u)) {
    const line = raw.trim()
    if (!line) continue
    const parts = line.split(/\s+/u)
    const keyword = parts[0].toLowerCase()
    if (keyword === 'solid') {
      if (state !== 'before') return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL contains a misplaced solid declaration.', 'stl')
      state = 'solid'
    } else if (keyword === 'endsolid') {
      if (state !== 'solid') return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL ended inside a facet or outside a solid.', 'stl')
      state = 'ended'
    } else if (keyword === 'facet') {
      if (state !== 'solid' || parts[1]?.toLowerCase() !== 'normal' || parts.length !== 5) {
        return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL contains an invalid facet declaration.', 'stl')
      }
      for (const token of parts.slice(2)) finiteNumber(token, 'STL facet normal', 'stl')
      state = 'facet'
    } else if (keyword === 'outer') {
      if (state !== 'facet' || parts.length !== 2 || parts[1].toLowerCase() !== 'loop') {
        return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL contains an invalid outer loop.', 'stl')
      }
      state = 'loop'
    } else if (keyword === 'vertex') {
      if (state !== 'loop' || parts.length !== 4 || facetVertices.length >= 9) {
        return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL facet must contain exactly three vertices.', 'stl')
      }
      facetVertices.push(
        finiteNumber(parts[1], 'STL vertex', 'stl'),
        finiteNumber(parts[2], 'STL vertex', 'stl'),
        finiteNumber(parts[3], 'STL vertex', 'stl'),
      )
    } else if (keyword === 'endloop') {
      if (state !== 'loop' || parts.length !== 1 || facetVertices.length !== 9) {
        return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL loop must contain exactly three vertices.', 'stl')
      }
      state = 'after-loop'
    } else if (keyword === 'endfacet') {
      if (state !== 'after-loop' || parts.length !== 1 || facetVertices.length !== 9) {
        return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL facet must contain exactly three vertices.', 'stl')
      }
      boundedCount(triangles.length / 3 + 1, OPENSCAD_IMPORT_MAX_TRIANGLES, 'STL triangle count', 'stl')
      const base = vertices.length / 3
      vertices.push(...facetVertices)
      triangles.push(base, base + 1, base + 2)
      facetVertices = []
      state = 'solid'
    } else {
      return dataError('E_IMPORT_INVALID_DATA', `ASCII STL contains unsupported statement ${JSON.stringify(parts[0])}.`, 'stl')
    }
  }
  if (state !== 'ended' || facetVertices.length !== 0) {
    return dataError('E_IMPORT_INVALID_DATA', 'ASCII STL is incomplete.', 'stl')
  }
  return geometry3D('stl', vertices, triangles)
}

function offLines(text: string): string[][] {
  const lines: string[][] = []
  for (const raw of text.replace(/^\uFEFF/u, '').split(/\r\n|\n|\r/u)) {
    const line = raw.replace(/#.*/u, '').trim()
    if (line) lines.push(line.split(/\s+/u))
  }
  return lines
}

function polygonNormal(vertices: readonly number[], face: readonly number[]): [number, number, number] {
  let nx = 0, ny = 0, nz = 0
  for (let index = 0; index < face.length; index++) {
    const current = face[index] * 3
    const next = face[(index + 1) % face.length] * 3
    const x = vertices[current], y = vertices[current + 1], z = vertices[current + 2]
    const xx = vertices[next], yy = vertices[next + 1], zz = vertices[next + 2]
    nx += (y - yy) * (z + zz)
    ny += (z - zz) * (x + xx)
    nz += (x - xx) * (y + yy)
  }
  return [nx, ny, nz]
}

function triangulateFace3D(vertices: readonly number[], face: readonly number[], format: OpenScadImportFormat): number[] {
  if (face.length === 3) return [...face]
  const normal = polygonNormal(vertices, face)
  const axis = Math.abs(normal[0]) >= Math.abs(normal[1]) && Math.abs(normal[0]) >= Math.abs(normal[2])
    ? 0
    : Math.abs(normal[1]) >= Math.abs(normal[2]) ? 1 : 2
  const project = (index: number): [number, number] => {
    const offset = index * 3
    return axis === 0
      ? [vertices[offset + 1], vertices[offset + 2]]
      : axis === 1 ? [vertices[offset], vertices[offset + 2]] : [vertices[offset], vertices[offset + 1]]
  }
  const points = face.map(project)
  let area = 0
  for (let index = 0; index < points.length; index++) {
    const a = points[index], b = points[(index + 1) % points.length]
    area += a[0] * b[1] - b[0] * a[1]
  }
  if (area === 0) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} contains a degenerate polygon face.`, format)
  const orientation = Math.sign(area)
  const remaining = [...face.keys()]
  const output: number[] = []
  const cross = (a: readonly number[], b: readonly number[], c: readonly number[]) => (
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0])
  )
  const inside = (point: readonly number[], a: readonly number[], b: readonly number[], c: readonly number[]) => {
    const ab = cross(a, b, point) * orientation
    const bc = cross(b, c, point) * orientation
    const ca = cross(c, a, point) * orientation
    return ab >= 0 && bc >= 0 && ca >= 0
  }
  while (remaining.length > 3) {
    let clipped = false
    for (let cursor = 0; cursor < remaining.length; cursor++) {
      const before = remaining[(cursor + remaining.length - 1) % remaining.length]
      const current = remaining[cursor]
      const after = remaining[(cursor + 1) % remaining.length]
      const a = points[before], b = points[current], c = points[after]
      if (cross(a, b, c) * orientation <= 0) continue
      if (remaining.some(candidate => candidate !== before && candidate !== current && candidate !== after
        && inside(points[candidate], a, b, c))) continue
      output.push(face[before], face[current], face[after])
      remaining.splice(cursor, 1)
      clipped = true
      break
    }
    if (!clipped) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} contains a self-intersecting polygon face.`, format)
  }
  output.push(face[remaining[0]], face[remaining[1]], face[remaining[2]])
  return output
}

/** Parse ASCII OFF/COFF geometry, including bounded concave polygon faces. */
export function parseOpenScadOff(file: OpenScadProjectFile): OpenScadImportGeometry3D {
  const lines = offLines(textOf(file, 'off'))
  let lineIndex = 0
  const headerLine = lines[lineIndex++] ?? []
  const header = headerLine.shift()?.toUpperCase()
  if (header !== 'OFF' && header !== 'COFF') {
    return dataError('E_IMPORT_INVALID_DATA', 'OFF input must begin with OFF or COFF.', 'off')
  }
  const counts = headerLine.length ? headerLine : lines[lineIndex++] ?? []
  if (counts.length < 3) return dataError('E_IMPORT_INVALID_DATA', 'OFF input is missing its counts line.', 'off')
  const vertexCount = boundedCount(
    finiteNumber(counts[0], 'OFF vertex count', 'off'),
    OPENSCAD_IMPORT_MAX_VERTICES,
    'OFF vertex count',
    'off',
  )
  const faceCount = boundedCount(
    finiteNumber(counts[1], 'OFF face count', 'off'),
    OPENSCAD_IMPORT_MAX_TRIANGLES,
    'OFF face count',
    'off',
  )
  boundedCount(finiteNumber(counts[2], 'OFF edge count', 'off'), Number.MAX_SAFE_INTEGER, 'OFF edge count', 'off')
  const vertices: number[] = []
  for (let vertex = 0; vertex < vertexCount; vertex++) {
    const fields = lines[lineIndex++] ?? []
    if (fields.length < 3) return dataError('E_IMPORT_INVALID_DATA', `OFF vertex ${vertex + 1} is incomplete.`, 'off')
    vertices.push(
      finiteNumber(fields[0], `OFF vertex ${vertex + 1} x`, 'off'),
      finiteNumber(fields[1], `OFF vertex ${vertex + 1} y`, 'off'),
      finiteNumber(fields[2], `OFF vertex ${vertex + 1} z`, 'off'),
    )
    // OFF variants append normals/colors/texture coordinates per vertex. They do not affect imported geometry.
    for (const token of fields.slice(3)) finiteNumber(token, `${header} vertex attribute`, 'off')
  }
  const triangles: number[] = []
  for (let faceIndex = 0; faceIndex < faceCount; faceIndex++) {
    const fields = lines[lineIndex++] ?? []
    const size = boundedCount(
      finiteNumber(fields[0] ?? '', `OFF face ${faceIndex + 1} size`, 'off'),
      OPENSCAD_IMPORT_MAX_VERTICES,
      `OFF face ${faceIndex + 1} size`,
      'off',
    )
    if (size < 3) return dataError('E_IMPORT_INVALID_DATA', 'OFF faces require at least three vertices.', 'off')
    if (fields.length < size + 1) return dataError('E_IMPORT_INVALID_DATA', `OFF face ${faceIndex + 1} is incomplete.`, 'off')
    const face: number[] = []
    for (let index = 0; index < size; index++) {
      const value = finiteNumber(fields[index + 1], 'OFF face index', 'off')
      if (!Number.isSafeInteger(value) || value < 0 || value >= vertexCount) {
        return dataError('E_IMPORT_INVALID_DATA', 'OFF face index is out of bounds.', 'off')
      }
      face.push(value)
    }
    for (const token of fields.slice(size + 1)) finiteNumber(token, 'OFF face attribute', 'off')
    triangles.push(...triangulateFace3D(vertices, face, 'off'))
    boundedCount(triangles.length / 3, OPENSCAD_IMPORT_MAX_TRIANGLES, 'OFF triangle count', 'off')
  }
  // OFF permits optional colors after faces. They are intentionally ignored.
  return geometry3D('off', vertices, triangles)
}

interface XmlElement {
  readonly name: string
  readonly localName: string
  readonly namespaceUri: string | null
  readonly attributes: Readonly<Record<string, string>>
  readonly attributeNamespaces: Readonly<Record<string, string | null>>
  readonly children: readonly XmlElement[]
  readonly text: string
}

interface MutableXmlElement {
  name: string
  localName: string
  namespaceUri: string | null
  attributes: Record<string, string>
  attributeNamespaces: Record<string, string | null>
  namespaces: Record<string, string>
  children: MutableXmlElement[]
  text: string
}

function xmlNameCharacter(value: string, first: boolean): boolean {
  return first ? /[A-Za-z_:]/u.test(value) : /[A-Za-z0-9_.:-]/u.test(value)
}

function decodeXmlEntities(value: string, format: OpenScadImportFormat): string {
  if (/&(?!(?:amp|lt|gt|quot|apos|#[0-9]+|#x[0-9a-f]+);)/iu.test(value)) {
    return dataError('E_IMPORT_INVALID_DATA', 'XML contains a malformed or unsupported entity reference.', format)
  }
  return value.replace(/&([^;]{1,32});/gu, (_whole, entity: string) => {
    if (entity === 'amp') return '&'
    if (entity === 'lt') return '<'
    if (entity === 'gt') return '>'
    if (entity === 'quot') return '"'
    if (entity === 'apos') return "'"
    let code = -1
    if (/^#[0-9]+$/u.test(entity)) code = Number(entity.slice(1))
    else if (/^#x[0-9a-f]+$/iu.test(entity)) code = Number.parseInt(entity.slice(2), 16)
    if (Number.isSafeInteger(code) && code >= 0 && code <= 0x10ffff
      && !(code >= 0xd800 && code <= 0xdfff)) return String.fromCodePoint(code)
    return dataError('E_IMPORT_INVALID_DATA', `XML contains unsupported entity &${entity};.`, format)
  })
}

/** Strict, entity-safe XML subset shared by SVG, AMF, and 3MF. */
function parseXml(text: string, format: OpenScadImportFormat): XmlElement {
  if (/<!DOCTYPE|<!ENTITY/iu.test(text)) {
    return dataError('E_IMPORT_UNSUPPORTED_FEATURE', `${format.toUpperCase()} XML declarations cannot define entities or a DTD.`, format)
  }
  const document: MutableXmlElement = {
    name: '#document',
    localName: '#document',
    namespaceUri: null,
    attributes: {},
    attributeNamespaces: {},
    namespaces: { xml: 'http://www.w3.org/XML/1998/namespace' },
    children: [],
    text: '',
  }
  const stack: MutableXmlElement[] = [document]
  let cursor = 0
  let elements = 0
  const skipWhitespace = (): void => {
    while (cursor < text.length && /\s/u.test(text[cursor])) cursor++
  }
  const name = (): string => {
    const start = cursor
    if (!xmlNameCharacter(text[cursor] ?? '', true)) {
      return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML contains an invalid element or attribute name.`, format)
    }
    cursor++
    while (cursor < text.length && xmlNameCharacter(text[cursor], false)) cursor++
    return text.slice(start, cursor)
  }
  while (cursor < text.length) {
    const open = text.indexOf('<', cursor)
    if (open < 0) {
      stack.at(-1)!.text += decodeXmlEntities(text.slice(cursor), format)
      cursor = text.length
      break
    }
    stack.at(-1)!.text += decodeXmlEntities(text.slice(cursor, open), format)
    cursor = open
    if (text.startsWith('<!--', cursor)) {
      const end = text.indexOf('-->', cursor + 4)
      if (end < 0) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML comment is unterminated.`, format)
      cursor = end + 3
      continue
    }
    if (text.startsWith('<?', cursor)) {
      const end = text.indexOf('?>', cursor + 2)
      if (end < 0) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML processing instruction is unterminated.`, format)
      cursor = end + 2
      continue
    }
    if (text.startsWith('<![CDATA[', cursor)) {
      const end = text.indexOf(']]>', cursor + 9)
      if (end < 0) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML CDATA is unterminated.`, format)
      stack.at(-1)!.text += text.slice(cursor + 9, end)
      cursor = end + 3
      continue
    }
    if (text.startsWith('</', cursor)) {
      cursor += 2
      skipWhitespace()
      const closing = name().toLowerCase()
      skipWhitespace()
      if (text[cursor] !== '>') return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML closing tag is malformed.`, format)
      cursor++
      const current = stack.pop()
      if (!current || current === document || current.name.toLowerCase() !== closing) {
        return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML closing tag does not match.`, format)
      }
      continue
    }
    if (text.startsWith('<!', cursor)) {
      return dataError('E_IMPORT_UNSUPPORTED_FEATURE', `${format.toUpperCase()} XML declaration is unsupported.`, format)
    }

    cursor++
    skipWhitespace()
    const elementName = name()
    const attributes: Record<string, string> = {}
    let selfClosing = false
    let terminated = false
    while (cursor < text.length) {
      skipWhitespace()
      if (text.startsWith('/>', cursor)) {
        selfClosing = true
        cursor += 2
        terminated = true
        break
      }
      if (text[cursor] === '>') {
        cursor++
        terminated = true
        break
      }
      const attributeName = name().toLowerCase()
      if (Object.hasOwn(attributes, attributeName)) {
        return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML contains a duplicate attribute.`, format)
      }
      skipWhitespace()
      if (text[cursor++] !== '=') return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML attribute is missing '='.`, format)
      skipWhitespace()
      const quote = text[cursor++]
      if (quote !== '"' && quote !== "'") {
        return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML attribute values must be quoted.`, format)
      }
      const end = text.indexOf(quote, cursor)
      if (end < 0) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML attribute is unterminated.`, format)
      attributes[attributeName] = decodeXmlEntities(text.slice(cursor, end), format)
      cursor = end + 1
      if (Object.keys(attributes).length > 64) {
        return dataError('E_IMPORT_LIMIT', `${format.toUpperCase()} XML element exceeds 64 attributes.`, format, 64, Object.keys(attributes).length)
      }
    }
    if (!terminated) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML start tag is unterminated.`, format)
    const namespaces = { ...stack.at(-1)!.namespaces }
    for (const [attributeName, value] of Object.entries(attributes)) {
      if (attributeName === 'xmlns') {
        if (value === '') delete namespaces['']
        else namespaces[''] = value
      } else if (attributeName.startsWith('xmlns:')) {
        const prefix = attributeName.slice(6)
        if (!prefix || value === '') return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML namespace declaration is invalid.`, format)
        namespaces[prefix] = value
      }
    }
    const qname = (qualified: string): { prefix: string; localName: string } => {
      const parts = qualified.toLowerCase().split(':')
      if (parts.length > 2 || parts.some(part => part === '')) {
        return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML qualified name is invalid.`, format)
      }
      return { prefix: parts.length === 2 ? parts[0] : '', localName: parts.at(-1)! }
    }
    const elementQName = qname(elementName)
    const namespaceUri = namespaces[elementQName.prefix] ?? null
    if (elementQName.prefix && namespaceUri === null) {
      return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML uses an undeclared namespace prefix.`, format)
    }
    const attributeNamespaces: Record<string, string | null> = {}
    for (const attributeName of Object.keys(attributes)) {
      if (attributeName === 'xmlns' || attributeName.startsWith('xmlns:')) {
        attributeNamespaces[attributeName] = 'http://www.w3.org/2000/xmlns/'
        continue
      }
      const attributeQName = qname(attributeName)
      const attributeNamespace = attributeQName.prefix ? namespaces[attributeQName.prefix] ?? null : null
      if (attributeQName.prefix && attributeNamespace === null) {
        return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML attribute uses an undeclared namespace prefix.`, format)
      }
      attributeNamespaces[attributeName] = attributeNamespace
    }
    elements++
    if (elements > OPENSCAD_IMPORT_MAX_XML_ELEMENTS) {
      return dataError(
        'E_IMPORT_LIMIT',
        `${format.toUpperCase()} XML exceeds ${OPENSCAD_IMPORT_MAX_XML_ELEMENTS.toLocaleString()} elements.`,
        format,
        OPENSCAD_IMPORT_MAX_XML_ELEMENTS,
        elements,
      )
    }
    const element: MutableXmlElement = {
      name: elementName,
      localName: elementQName.localName,
      namespaceUri,
      attributes,
      attributeNamespaces,
      namespaces,
      children: [],
      text: '',
    }
    stack.at(-1)!.children.push(element)
    if (!selfClosing) {
      stack.push(element)
      if (stack.length - 1 > OPENSCAD_IMPORT_MAX_XML_DEPTH) {
        return dataError(
          'E_IMPORT_LIMIT',
          `${format.toUpperCase()} XML exceeds nesting depth ${OPENSCAD_IMPORT_MAX_XML_DEPTH}.`,
          format,
          OPENSCAD_IMPORT_MAX_XML_DEPTH,
          stack.length - 1,
        )
      }
    }
  }
  if (stack.length !== 1 || document.children.length !== 1 || document.text.trim() !== '') {
    return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML must contain exactly one root element.`, format)
  }
  return document.children[0]
}

function childElements(element: XmlElement, localName: string): readonly XmlElement[] {
  return element.children.filter(child => child.localName === localName)
}

function childElement(element: XmlElement, localName: string): XmlElement | null {
  return element.children.find(child => child.localName === localName) ?? null
}

function descendants(element: XmlElement, localName: string): XmlElement[] {
  const output: XmlElement[] = []
  const visit = (node: XmlElement): void => {
    if (node.localName === localName) output.push(node)
    for (const child of node.children) visit(child)
  }
  visit(element)
  return output
}

function childElementsNs(element: XmlElement, localName: string, namespaceUri: string): readonly XmlElement[] {
  return element.children.filter(child => child.localName === localName && child.namespaceUri === namespaceUri)
}

function childElementNs(element: XmlElement, localName: string, namespaceUri: string): XmlElement | null {
  return element.children.find(child => child.localName === localName && child.namespaceUri === namespaceUri) ?? null
}

function descendantsNs(element: XmlElement, localName: string, namespaceUri: string): XmlElement[] {
  const output: XmlElement[] = []
  const visit = (node: XmlElement): void => {
    if (node.localName === localName && node.namespaceUri === namespaceUri) output.push(node)
    for (const child of node.children) visit(child)
  }
  visit(element)
  return output
}

function attribute(element: XmlElement, key: string): string | undefined {
  const exact = element.attributes[key.toLowerCase()]
  if (exact !== undefined) return exact
  const suffix = `:${key.toLowerCase()}`
  const found = Object.entries(element.attributes).find(([name]) => name.endsWith(suffix))
  return found?.[1]
}

function attributeNs(element: XmlElement, key: string, namespaceUri: string | null): string | undefined {
  const normalized = key.toLowerCase()
  const found = Object.entries(element.attributes).find(([name]) => (
    name.split(':').at(-1) === normalized && element.attributeNamespaces[name] === namespaceUri
  ))
  return found?.[1]
}

function requiredAttributeNs(
  element: XmlElement,
  key: string,
  namespaceUri: string | null,
  format: OpenScadImportFormat,
): string {
  const value = attributeNs(element, key, namespaceUri)
  if (value === undefined || value.trim() === '') {
    return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} <${element.localName}> is missing ${key}.`, format)
  }
  return value.trim()
}

function requiredAttribute(
  element: XmlElement,
  key: string,
  format: OpenScadImportFormat,
): string {
  const value = attribute(element, key)
  if (value === undefined || value.trim() === '') {
    return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} <${element.localName}> is missing ${key}.`, format)
  }
  return value.trim()
}

function numericAttribute(element: XmlElement, key: string, format: OpenScadImportFormat): number {
  return finiteNumber(requiredAttribute(element, key, format), `${format.toUpperCase()} ${key}`, format)
}

function numericText(element: XmlElement | null, label: string, format: OpenScadImportFormat): number {
  if (element === null) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} is missing ${label}.`, format)
  return finiteNumber(element.text.trim(), label, format)
}

function unitScale(unit: string | undefined, format: 'amf' | '3mf'): number {
  const normalized = (unit ?? 'millimeter').toLowerCase()
  const scales: Record<string, number> = {
    micron: 0.001,
    micrometer: 0.001,
    millimeter: 1,
    centimeter: 10,
    meter: 1_000,
    inch: 25.4,
    foot: 304.8,
    feet: 304.8,
  }
  const scale = scales[normalized]
  if (scale === undefined) return dataError('E_IMPORT_UNSUPPORTED_FEATURE', `${format.toUpperCase()} unit ${JSON.stringify(unit)} is unsupported.`, format)
  return scale
}

type Matrix4 = readonly [
  number, number, number, number,
  number, number, number, number,
  number, number, number, number,
  number, number, number, number,
]

const IDENTITY_4: Matrix4 = [
  1, 0, 0, 0,
  0, 1, 0, 0,
  0, 0, 1, 0,
  0, 0, 0, 1,
]

function multiply4(left: Matrix4, right: Matrix4): Matrix4 {
  const output = new Array<number>(16).fill(0)
  for (let row = 0; row < 4; row++) {
    for (let column = 0; column < 4; column++) {
      for (let index = 0; index < 4; index++) {
        output[row * 4 + column] += left[row * 4 + index] * right[index * 4 + column]
      }
    }
  }
  return output as unknown as Matrix4
}

function transform3(matrix: Matrix4, x: number, y: number, z: number): [number, number, number] {
  return [
    matrix[0] * x + matrix[1] * y + matrix[2] * z + matrix[3],
    matrix[4] * x + matrix[5] * y + matrix[6] * z + matrix[7],
    matrix[8] * x + matrix[9] * y + matrix[10] * z + matrix[11],
  ]
}

function rotation4(rx: number, ry: number, rz: number, translation: readonly [number, number, number]): Matrix4 {
  const x = rx * Math.PI / 180, y = ry * Math.PI / 180, z = rz * Math.PI / 180
  const cx = Math.cos(x), sx = Math.sin(x), cy = Math.cos(y), sy = Math.sin(y), cz = Math.cos(z), sz = Math.sin(z)
  // OpenSCAD/AMF rotate in X, then Y, then Z order.
  return [
    cz * cy, cz * sy * sx - sz * cx, cz * sy * cx + sz * sx, translation[0],
    sz * cy, sz * sy * sx + cz * cx, sz * sy * cx - cz * sx, translation[1],
    -sy, cy * sx, cy * cx, translation[2],
    0, 0, 0, 1,
  ]
}

interface RawMesh3D {
  readonly vertices: readonly number[]
  readonly triangles: readonly number[]
}

function appendRawMesh(
  targetVertices: number[],
  targetTriangles: number[],
  mesh: RawMesh3D,
  matrix: Matrix4,
  format: OpenScadImportFormat,
): void {
  const vertexOffset = targetVertices.length / 3
  const nextVertices = vertexOffset + mesh.vertices.length / 3
  const nextTriangles = targetTriangles.length / 3 + mesh.triangles.length / 3
  boundedCount(nextVertices, OPENSCAD_IMPORT_MAX_VERTICES, `${format.toUpperCase()} expanded vertex count`, format)
  boundedCount(nextTriangles, OPENSCAD_IMPORT_MAX_TRIANGLES, `${format.toUpperCase()} expanded triangle count`, format)
  for (let offset = 0; offset < mesh.vertices.length; offset += 3) {
    targetVertices.push(...transform3(matrix, mesh.vertices[offset], mesh.vertices[offset + 1], mesh.vertices[offset + 2]))
  }
  const determinant = matrix[0] * (matrix[5] * matrix[10] - matrix[6] * matrix[9])
    - matrix[1] * (matrix[4] * matrix[10] - matrix[6] * matrix[8])
    + matrix[2] * (matrix[4] * matrix[9] - matrix[5] * matrix[8])
  if (determinant === 0 || !Number.isFinite(determinant)) {
    return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} transform is singular or non-finite.`, format)
  }
  for (let offset = 0; offset < mesh.triangles.length; offset += 3) {
    const a = vertexOffset + mesh.triangles[offset]
    const b = vertexOffset + mesh.triangles[offset + 1]
    const c = vertexOffset + mesh.triangles[offset + 2]
    targetTriangles.push(a, ...(determinant < 0 ? [c, b] : [b, c]))
  }
}

interface AmfInstance {
  readonly id: string
  readonly transform: Matrix4
}

type AmfNode =
  | { readonly kind: 'object'; readonly mesh: RawMesh3D }
  | { readonly kind: 'constellation'; readonly instances: readonly AmfInstance[] }

function parseOpenScadAmfXml(text: string): OpenScadImportGeometry3D {
  const root = parseXml(text, 'amf')
  if (root.localName !== 'amf') return dataError('E_IMPORT_INVALID_DATA', 'AMF root element must be <amf>.', 'amf')
  const scale = unitScale(attribute(root, 'unit'), 'amf')
  const nodes = new Map<string, AmfNode>()
  for (const object of childElements(root, 'object')) {
    const id = requiredAttribute(object, 'id', 'amf')
    if (nodes.has(id)) return dataError('E_IMPORT_INVALID_DATA', `AMF node id ${id} is duplicated.`, 'amf')
    const mesh = childElement(object, 'mesh')
    if (mesh === null) return dataError('E_IMPORT_INVALID_DATA', `AMF object ${id} is missing its mesh.`, 'amf')
    const verticesContainer = childElement(mesh, 'vertices')
    if (verticesContainer === null) return dataError('E_IMPORT_INVALID_DATA', `AMF object ${id} is missing vertices.`, 'amf')
    const vertices: number[] = []
    for (const [index, vertex] of childElements(verticesContainer, 'vertex').entries()) {
      const coordinates = childElement(vertex, 'coordinates')
      if (coordinates === null) return dataError('E_IMPORT_INVALID_DATA', `AMF vertex ${index} is missing coordinates.`, 'amf')
      vertices.push(
        numericText(childElement(coordinates, 'x'), 'AMF x', 'amf') * scale,
        numericText(childElement(coordinates, 'y'), 'AMF y', 'amf') * scale,
        numericText(childElement(coordinates, 'z'), 'AMF z', 'amf') * scale,
      )
      boundedCount(vertices.length / 3, OPENSCAD_IMPORT_MAX_VERTICES, 'AMF vertex count', 'amf')
    }
    const triangles: number[] = []
    for (const volume of childElements(mesh, 'volume')) {
      for (const triangle of childElements(volume, 'triangle')) {
        for (const name of ['v1', 'v2', 'v3'] as const) {
          const index = numericText(childElement(triangle, name), `AMF ${name}`, 'amf')
          if (!Number.isSafeInteger(index) || index < 0 || index >= vertices.length / 3) {
            return dataError('E_IMPORT_INVALID_DATA', `AMF ${name} index is out of bounds.`, 'amf')
          }
          triangles.push(index)
        }
        boundedCount(triangles.length / 3, OPENSCAD_IMPORT_MAX_TRIANGLES, 'AMF triangle count', 'amf')
      }
    }
    nodes.set(id, { kind: 'object', mesh: Object.freeze({ vertices, triangles }) })
  }
  const referenced = new Set<string>()
  for (const constellation of childElements(root, 'constellation')) {
    const id = requiredAttribute(constellation, 'id', 'amf')
    if (nodes.has(id)) return dataError('E_IMPORT_INVALID_DATA', `AMF node id ${id} is duplicated.`, 'amf')
    const instances = childElements(constellation, 'instance').map(instance => {
      const objectId = requiredAttribute(instance, 'objectid', 'amf')
      referenced.add(objectId)
      const read = (name: string): number => {
        const element = childElement(instance, name)
        return element === null ? 0 : numericText(element, `AMF ${name}`, 'amf') * (name.startsWith('delta') ? scale : 1)
      }
      return Object.freeze({
        id: objectId,
        transform: rotation4(
          read('rx'),
          read('ry'),
          read('rz'),
          [read('deltax'), read('deltay'), read('deltaz')],
        ),
      })
    })
    nodes.set(id, { kind: 'constellation', instances: Object.freeze(instances) })
  }
  const vertices: number[] = []
  const triangles: number[] = []
  let expansions = 0
  const expand = (id: string, matrix: Matrix4, stack: readonly string[]): void => {
    const node = nodes.get(id)
    if (!node) return dataError('E_IMPORT_INVALID_DATA', `AMF instance refers to missing node ${id}.`, 'amf')
    if (stack.includes(id)) return dataError('E_IMPORT_INVALID_DATA', `AMF constellation cycle includes ${id}.`, 'amf')
    expansions++
    if (expansions > OPENSCAD_IMPORT_MAX_XML_ELEMENTS) {
      return dataError('E_IMPORT_LIMIT', 'AMF instance expansion limit exceeded.', 'amf', OPENSCAD_IMPORT_MAX_XML_ELEMENTS, expansions)
    }
    if (node.kind === 'object') {
      appendRawMesh(vertices, triangles, node.mesh, matrix, 'amf')
      return
    }
    if (stack.length >= OPENSCAD_IMPORT_MAX_XML_DEPTH) {
      return dataError('E_IMPORT_LIMIT', 'AMF constellation nesting limit exceeded.', 'amf', OPENSCAD_IMPORT_MAX_XML_DEPTH, stack.length + 1)
    }
    for (const instance of node.instances) {
      expand(instance.id, multiply4(matrix, instance.transform), [...stack, id])
    }
  }
  const roots = [...nodes.keys()].filter(id => !referenced.has(id))
  if (nodes.size > 0 && roots.length === 0) {
    // A graph without a root is necessarily cyclic or self-referential; expand
    // one node to produce the stable cycle diagnostic.
    expand(nodes.keys().next().value!, IDENTITY_4, [])
  } else {
    for (const id of roots) {
      expand(id, IDENTITY_4, [])
    }
  }
  return geometry3D('amf', vertices, triangles)
}

/** Parse raw, gzip-compressed, or ZIP-compressed AMF into millimetres. */
async function parseOpenScadAmfWithBudget(
  file: OpenScadProjectFile,
  projectBudget?: OpenScadImportPreparationBudget,
): Promise<OpenScadImportGeometry3D> {
  return parseOpenScadAmfXml(await decodeOpenScadAmfText(file, projectBudget))
}

export async function parseOpenScadAmf(file: OpenScadProjectFile): Promise<OpenScadImportGeometry3D> {
  return parseOpenScadAmfWithBudget(file)
}

const CRC_TABLE = new Uint32Array(256)
for (let index = 0; index < CRC_TABLE.length; index++) {
  let value = index
  for (let bit = 0; bit < 8; bit++) value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1
  CRC_TABLE[index] = value >>> 0
}

function crc32(bytes: Uint8Array): number {
  let value = 0xffffffff
  for (const byte of bytes) value = CRC_TABLE[(value ^ byte) & 0xff] ^ (value >>> 8)
  return (value ^ 0xffffffff) >>> 0
}

function zipPath(value: string): string {
  const path = value.replace(/^\/+/, '').replace(/\/$/u, '')
  if (!path || path.includes('\\') || path.includes('\0')) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP contains an invalid entry path.', '3mf')
  }
  const segments = path.split('/')
  if (segments.some(segment => segment === '' || segment === '.' || segment === '..')) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP entry path escapes or is not canonical.', '3mf')
  }
  return segments.join('/')
}

async function inflateRaw(compressed: Uint8Array, expected: number): Promise<Uint8Array> {
  let stream: ReadableStream<Uint8Array>
  try {
    const input = new Uint8Array(compressed)
    stream = new Blob([input.buffer]).stream().pipeThrough(new DecompressionStream('deflate-raw'))
  } catch {
    return dataError('E_IMPORT_UNSUPPORTED_FEATURE', 'This runtime cannot inflate compressed 3MF ZIP entries.', '3mf')
  }
  const reader = stream.getReader()
  const chunks: Uint8Array[] = []
  let length = 0
  try {
    while (true) {
      const { value, done } = await reader.read()
      if (done) break
      length += value.byteLength
      if (length > expected || length > OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES) {
        await reader.cancel()
        return dataError(
          'E_IMPORT_LIMIT',
          '3MF ZIP entry expands beyond its declared or permitted size.',
          '3mf',
          Math.min(expected, OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES),
          length,
        )
      }
      chunks.push(value)
    }
  } catch (error) {
    if (error instanceof OpenScadImportDataError) throw error
    return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP contains invalid deflate data.', '3mf')
  }
  if (length !== expected) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP entry size does not match its directory record.', '3mf')
  const output = new Uint8Array(length)
  let offset = 0
  for (const chunk of chunks) {
    output.set(chunk, offset)
    offset += chunk.byteLength
  }
  return output
}

interface ZipCentralEntry {
  readonly path: string
  readonly directory: boolean
  readonly flags: number
  readonly method: number
  readonly crc: number
  readonly compressedSize: number
  readonly uncompressedSize: number
  readonly localOffset: number
}

interface ZipDirectoryInfo {
  readonly endOffset: number
  readonly directoryEnd: number
  readonly entryCount: number
  readonly directorySize: number
  readonly directoryOffset: number
}

function boundedZipUint64(view: DataView, offset: number, label: string): number {
  if (offset < 0 || offset + 8 > view.byteLength) {
    return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP ${label} is truncated.`, '3mf')
  }
  const value = view.getBigUint64(offset, true)
  if (value > BigInt(Number.MAX_SAFE_INTEGER)) {
    return dataError('E_IMPORT_LIMIT', `3MF ZIP ${label} exceeds the safe integer range.`, '3mf', Number.MAX_SAFE_INTEGER)
  }
  return Number(value)
}

function zipDirectoryInfo(view: DataView): ZipDirectoryInfo {
  const length = view.byteLength
  const minimum = Math.max(0, length - 65_557)
  // Scan forward: the real EOCD precedes all bytes in its own comment, so a
  // comment containing a second plausible EOCD signature cannot shadow it.
  for (let end = minimum; end <= length - 22; end++) {
    if (view.getUint32(end, true) !== 0x06054b50
      || end + 22 + view.getUint16(end + 20, true) !== length) continue
    const disk = view.getUint16(end + 4, true)
    const directoryDisk = view.getUint16(end + 6, true)
    const diskEntries = view.getUint16(end + 8, true)
    const classicCount = view.getUint16(end + 10, true)
    const classicSize = view.getUint32(end + 12, true)
    const classicOffset = view.getUint32(end + 16, true)
    const zip64 = diskEntries === 0xffff || classicCount === 0xffff
      || classicSize === 0xffffffff || classicOffset === 0xffffffff
    if (!zip64) {
      if (disk !== 0 || directoryDisk !== 0 || diskEntries !== classicCount || classicCount === 0
        || classicOffset + classicSize !== end) continue
      return {
        endOffset: end,
        directoryEnd: end,
        entryCount: classicCount,
        directorySize: classicSize,
        directoryOffset: classicOffset,
      }
    }
    const locator = end - 20
    if (locator < 0 || view.getUint32(locator, true) !== 0x07064b50
      || view.getUint32(locator + 4, true) !== 0 || view.getUint32(locator + 16, true) !== 1) continue
    const zip64End = boundedZipUint64(view, locator + 8, 'ZIP64 end-record offset')
    if (zip64End + 56 > locator || view.getUint32(zip64End, true) !== 0x06064b50) continue
    const recordSize = boundedZipUint64(view, zip64End + 4, 'ZIP64 end-record size')
    if (recordSize < 44 || zip64End + 12 + recordSize !== locator) continue
    if (view.getUint32(zip64End + 16, true) !== 0 || view.getUint32(zip64End + 20, true) !== 0) continue
    const diskCount = boundedZipUint64(view, zip64End + 24, 'ZIP64 disk entry count')
    const entryCount = boundedZipUint64(view, zip64End + 32, 'ZIP64 total entry count')
    const directorySize = boundedZipUint64(view, zip64End + 40, 'ZIP64 central-directory size')
    const directoryOffset = boundedZipUint64(view, zip64End + 48, 'ZIP64 central-directory offset')
    if (diskCount !== entryCount || entryCount === 0 || directoryOffset + directorySize !== zip64End) continue
    return {
      endOffset: end,
      directoryEnd: zip64End,
      entryCount,
      directorySize,
      directoryOffset,
    }
  }
  return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP end record is missing or inconsistent.', '3mf')
}

interface Zip64EntryValues {
  readonly uncompressedSize?: number
  readonly compressedSize?: number
  readonly localOffset?: number
  readonly diskStart?: number
}

function zip64EntryValues(
  bytes: Uint8Array,
  extraOffset: number,
  extraLength: number,
  needs: { readonly uncompressedSize: boolean; readonly compressedSize: boolean; readonly localOffset: boolean; readonly diskStart: boolean },
): Zip64EntryValues {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const end = extraOffset + extraLength
  if (extraOffset < 0 || end > bytes.byteLength) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP extra field is truncated.', '3mf')
  let cursor = extraOffset
  while (cursor + 4 <= end) {
    const id = view.getUint16(cursor, true), size = view.getUint16(cursor + 2, true)
    cursor += 4
    if (cursor + size > end) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP extra field is malformed.', '3mf')
    if (id !== 0x0001) {
      cursor += size
      continue
    }
    const fieldEnd = cursor + size
    const output: { uncompressedSize?: number; compressedSize?: number; localOffset?: number; diskStart?: number } = {}
    const read64 = (label: string): number => {
      if (cursor + 8 > fieldEnd) return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP64 ${label} is missing.`, '3mf')
      const value = boundedZipUint64(view, cursor, label)
      cursor += 8
      return value
    }
    if (needs.uncompressedSize) output.uncompressedSize = read64('uncompressed size')
    if (needs.compressedSize) output.compressedSize = read64('compressed size')
    if (needs.localOffset) output.localOffset = read64('local-header offset')
    if (needs.diskStart) {
      if (cursor + 4 > fieldEnd) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP64 disk start is missing.', '3mf')
      output.diskStart = view.getUint32(cursor, true)
    }
    return output
  }
  if (Object.values(needs).some(Boolean)) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP64 entry is missing its 0x0001 extra field.', '3mf')
  return {}
}

async function unzip3mf(
  bytes: Uint8Array,
  projectBudget?: OpenScadImportPreparationBudget,
): Promise<ReadonlyMap<string, Uint8Array>> {
  if (bytes.byteLength < 22) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP is shorter than its end record.', '3mf')
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const directory = zipDirectoryInfo(view)
  const { entryCount, directorySize, directoryOffset, directoryEnd } = directory
  boundedCount(entryCount, OPENSCAD_IMPORT_MAX_ZIP_ENTRIES, '3MF ZIP entry count', '3mf')
  if (directoryOffset + directorySize !== directoryEnd || directoryOffset > bytes.byteLength) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP central directory is inconsistent.', '3mf')
  }
  const entries: ZipCentralEntry[] = []
  const seen = new Set<string>()
  let expanded = 0
  let cursor = directoryOffset
  for (let index = 0; index < entryCount; index++) {
    if (cursor + 46 > directoryEnd || view.getUint32(cursor, true) !== 0x02014b50) {
      return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP central directory entry is truncated.', '3mf')
    }
    const flags = view.getUint16(cursor + 8, true)
    const method = view.getUint16(cursor + 10, true)
    const crc = view.getUint32(cursor + 16, true)
    const compressedSize32 = view.getUint32(cursor + 20, true)
    const uncompressedSize32 = view.getUint32(cursor + 24, true)
    const nameLength = view.getUint16(cursor + 28, true)
    const extraLength = view.getUint16(cursor + 30, true)
    const entryComment = view.getUint16(cursor + 32, true)
    const diskStart16 = view.getUint16(cursor + 34, true)
    const localOffset32 = view.getUint32(cursor + 42, true)
    const next = cursor + 46 + nameLength + extraLength + entryComment
    if (next > directoryEnd) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP central directory entry is truncated.', '3mf')
    const zip64Values = zip64EntryValues(
      bytes,
      cursor + 46 + nameLength,
      extraLength,
      {
        uncompressedSize: uncompressedSize32 === 0xffffffff,
        compressedSize: compressedSize32 === 0xffffffff,
        localOffset: localOffset32 === 0xffffffff,
        diskStart: diskStart16 === 0xffff,
      },
    )
    const compressedSize = compressedSize32 === 0xffffffff ? zip64Values.compressedSize! : compressedSize32
    const uncompressedSize = uncompressedSize32 === 0xffffffff ? zip64Values.uncompressedSize! : uncompressedSize32
    const localOffset = localOffset32 === 0xffffffff ? zip64Values.localOffset! : localOffset32
    const diskStart = diskStart16 === 0xffff ? zip64Values.diskStart! : diskStart16
    if (diskStart !== 0) return dataError('E_IMPORT_UNSUPPORTED_FEATURE', 'Multi-disk 3MF ZIP entries are unsupported.', '3mf')
    if ((flags & 1) !== 0 || (flags & ~0x080e) !== 0 || (method !== 0 && method !== 8)) {
      return dataError('E_IMPORT_UNSUPPORTED_FEATURE', '3MF ZIP entry uses encryption, flags, or compression not permitted by this importer.', '3mf')
    }
    const encodedName = bytes.subarray(cursor + 46, cursor + 46 + nameLength)
    let decodedName: string
    try {
      decodedName = UTF8_FATAL.decode(encodedName)
    } catch {
      return dataError('E_IMPORT_ENCODING', '3MF ZIP entry names must be UTF-8.', '3mf')
    }
    if ((flags & 0x0800) === 0 && encodedName.some(byte => byte >= 0x80)) {
      return dataError('E_IMPORT_ENCODING', 'Non-ASCII 3MF ZIP entry names must set the UTF-8 flag.', '3mf')
    }
    const directory = decodedName.endsWith('/')
    const path = zipPath(decodedName)
    if (directory && (compressedSize !== 0 || uncompressedSize !== 0)) {
      return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP directory ${path} must be empty.`, '3mf')
    }
    if (seen.has(path)) return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP contains duplicate entry ${path}.`, '3mf')
    seen.add(path)
    expanded += uncompressedSize
    if (!Number.isSafeInteger(expanded) || expanded > OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES) {
      return dataError(
        'E_IMPORT_LIMIT',
        `3MF ZIP expands beyond ${OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES.toLocaleString()} bytes.`,
        '3mf',
        OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES,
        expanded,
      )
    }
    entries.push({ path, directory, flags, method, crc, compressedSize, uncompressedSize, localOffset })
    cursor = next
  }
  if (cursor !== directoryEnd) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP central directory has trailing data.', '3mf')
  if (projectBudget !== undefined) {
    const total = projectBudget.zipExpandedBytes + expanded
    if (!Number.isSafeInteger(total) || total > OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES) {
      return dataError(
        'E_IMPORT_LIMIT',
        `Prepared project 3MF data expands beyond ${OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES.toLocaleString()} bytes.`,
        '3mf',
        OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES,
        total,
      )
    }
    projectBudget.zipExpandedBytes = total
  }

  const files = new Map<string, Uint8Array>()
  for (const entry of entries) {
    if (entry.localOffset + 30 > directoryOffset || view.getUint32(entry.localOffset, true) !== 0x04034b50) {
      return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP local entry ${entry.path} is invalid.`, '3mf')
    }
    const localFlags = view.getUint16(entry.localOffset + 6, true)
    const localMethod = view.getUint16(entry.localOffset + 8, true)
    const nameLength = view.getUint16(entry.localOffset + 26, true)
    const extraLength = view.getUint16(entry.localOffset + 28, true)
    const dataStart = entry.localOffset + 30 + nameLength + extraLength
    const dataEnd = dataStart + entry.compressedSize
    if (localFlags !== entry.flags || localMethod !== entry.method || dataEnd > directoryOffset) {
      return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP local entry ${entry.path} disagrees with its directory record.`, '3mf')
    }
    let localName: string
    try {
      localName = UTF8_FATAL.decode(bytes.subarray(entry.localOffset + 30, entry.localOffset + 30 + nameLength))
    } catch {
      return dataError('E_IMPORT_ENCODING', '3MF ZIP local entry name must be UTF-8.', '3mf')
    }
    if (zipPath(localName) !== entry.path || localName.endsWith('/') !== entry.directory) {
      return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP local entry name does not match ${entry.path}.`, '3mf')
    }
    const compressed = bytes.subarray(dataStart, dataEnd)
    const data = entry.method === 0
      ? new Uint8Array(compressed)
      : await inflateRaw(compressed, entry.uncompressedSize)
    if (data.byteLength !== entry.uncompressedSize || crc32(data) !== entry.crc) {
      return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP entry ${entry.path} failed size or CRC verification.`, '3mf')
    }
    if (!entry.directory) files.set(entry.path, data)
  }
  return files
}

function asAmfArchiveError(error: OpenScadImportDataError): OpenScadImportDataError {
  return new OpenScadImportDataError(
    error.code,
    error.message.replaceAll('3MF', 'AMF'),
    'amf',
    error.limit,
    error.actual,
  )
}

async function gunzipAmf(
  bytes: Uint8Array,
  projectBudget?: OpenScadImportPreparationBudget,
): Promise<Uint8Array> {
  let stream: ReadableStream<Uint8Array>
  try {
    const input = new Uint8Array(bytes)
    stream = new Blob([input.buffer]).stream().pipeThrough(new DecompressionStream('gzip'))
  } catch {
    return dataError('E_IMPORT_UNSUPPORTED_FEATURE', 'This runtime cannot inflate gzip-compressed AMF.', 'amf')
  }
  const reader = stream.getReader()
  const chunks: Uint8Array[] = []
  let length = 0
  try {
    while (true) {
      const { value, done } = await reader.read()
      if (done) break
      length += value.byteLength
      const projectLength = (projectBudget?.zipExpandedBytes ?? 0) + length
      if (length > OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES
        || projectLength > OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES) {
        await reader.cancel()
        return dataError(
          'E_IMPORT_LIMIT',
          'Gzip-compressed AMF expands beyond its per-file or prepared-project limit.',
          'amf',
          Math.min(
            OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES,
            OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES - (projectBudget?.zipExpandedBytes ?? 0),
          ),
          length,
        )
      }
      chunks.push(value)
    }
  } catch (error) {
    if (error instanceof OpenScadImportDataError) throw error
    return dataError('E_IMPORT_INVALID_DATA', 'AMF contains invalid gzip data.', 'amf')
  }
  if (projectBudget !== undefined) projectBudget.zipExpandedBytes += length
  const output = new Uint8Array(length)
  let offset = 0
  for (const chunk of chunks) {
    output.set(chunk, offset)
    offset += chunk.byteLength
  }
  return output
}

function decodeAmfUtf8(bytes: Uint8Array): string {
  try {
    return UTF8_FATAL.decode(bytes)
  } catch {
    return dataError('E_IMPORT_ENCODING', 'AMF XML must be valid UTF-8.', 'amf')
  }
}

async function decodeOpenScadAmfText(
  file: OpenScadProjectFile,
  projectBudget?: OpenScadImportPreparationBudget,
): Promise<string> {
  if (file.kind === 'source') return file.source
  const bytes = file.data
  if (bytes.byteLength >= 2 && bytes[0] === 0x1f && bytes[1] === 0x8b) {
    return decodeAmfUtf8(await gunzipAmf(bytes, projectBudget))
  }
  if (bytes.byteLength >= 4 && bytes[0] === 0x50 && bytes[1] === 0x4b
    && bytes[2] === 0x03 && bytes[3] === 0x04) {
    let files: ReadonlyMap<string, Uint8Array>
    try {
      files = await unzip3mf(bytes, projectBudget)
    } catch (error) {
      if (!(error instanceof OpenScadImportDataError)) throw error
      throw asAmfArchiveError(error)
    }
    const candidates = [...files.entries()].filter(([path]) => path.toLowerCase().endsWith('.amf'))
    if (candidates.length !== 1) {
      return dataError('E_IMPORT_INVALID_DATA', 'Compressed AMF ZIP must contain exactly one .amf document.', 'amf')
    }
    return decodeAmfUtf8(candidates[0][1])
  }
  return decodeAmfUtf8(bytes)
}

function packageTarget(value: string): string {
  if (/[?#]/u.test(value) || /^[A-Za-z][A-Za-z0-9+.-]*:/u.test(value) || /%2f|%5c/iu.test(value)) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF StartPart Target must be an internal OPC part URI.', '3mf')
  }
  let decoded: string
  try {
    decoded = decodeURIComponent(value)
  } catch {
    return dataError('E_IMPORT_INVALID_DATA', '3MF StartPart Target contains invalid percent encoding.', '3mf')
  }
  return zipPath(decoded)
}

const THREE_MF_CORE_NAMESPACE = 'http://schemas.microsoft.com/3dmanufacturing/core/2015/02'
const OPC_RELATIONSHIPS_NAMESPACE = 'http://schemas.openxmlformats.org/package/2006/relationships'
const THREE_MF_START_PART_RELATIONSHIP = 'http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel'

function modelPathFromPackage(files: ReadonlyMap<string, Uint8Array>): string {
  const relationships = files.get('_rels/.rels')
  if (relationships === undefined) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF package is missing the root _rels/.rels part.', '3mf')
  }
  let root: XmlElement
  try {
    root = parseXml(UTF8_FATAL.decode(relationships), '3mf')
  } catch (error) {
    if (error instanceof OpenScadImportDataError) throw error
    return dataError('E_IMPORT_ENCODING', '3MF package relationships must be UTF-8 XML.', '3mf')
  }
  if (root.localName !== 'relationships' || root.namespaceUri !== OPC_RELATIONSHIPS_NAMESPACE) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF root relationships part has the wrong XML namespace.', '3mf')
  }
  const startParts = childElementsNs(root, 'relationship', OPC_RELATIONSHIPS_NAMESPACE).filter(relationship => (
    attributeNs(relationship, 'type', null) === THREE_MF_START_PART_RELATIONSHIP
  ))
  if (startParts.length !== 1) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF package must contain exactly one official StartPart relationship.', '3mf')
  }
  const relationship = startParts[0]
  const targetMode = attributeNs(relationship, 'targetmode', null)?.trim().toLowerCase()
  if (targetMode !== undefined && targetMode !== 'internal') {
    return dataError('E_IMPORT_UNSUPPORTED_FEATURE', '3MF StartPart cannot target an external resource.', '3mf')
  }
  const path = packageTarget(requiredAttributeNs(relationship, 'target', null, '3mf'))
  if (!files.has(path)) return dataError('E_IMPORT_INVALID_DATA', `3MF primary model ${path} is missing.`, '3mf')
  return path
}

function threeMfResourceId(value: string, label: string): string {
  const normalized = value.trim()
  if (!/^\+?[0-9]+$/u.test(normalized)) {
    return dataError('E_IMPORT_INVALID_DATA', `${label} must be an ST_ResourceID positive integer.`, '3mf')
  }
  const numeric = BigInt(normalized)
  if (numeric < 1n || numeric >= 2_147_483_648n) {
    return dataError('E_IMPORT_INVALID_DATA', `${label} must be between 1 and 2147483647.`, '3mf')
  }
  return numeric.toString()
}

function threeMfResourceIdAttribute(element: XmlElement, name: string): string {
  return threeMfResourceId(requiredAttributeNs(element, name, null, '3mf'), `3MF ${name}`)
}

function threeMfResourceIndexAttribute(element: XmlElement, name: string): number {
  const value = requiredAttributeNs(element, name, null, '3mf')
  if (!/^\+?[0-9]+$/u.test(value)) {
    return dataError('E_IMPORT_INVALID_DATA', `3MF ${name} must be an ST_ResourceIndex integer.`, '3mf')
  }
  const numeric = BigInt(value)
  if (numeric < 0n || numeric >= 2_147_483_648n) {
    return dataError('E_IMPORT_INVALID_DATA', `3MF ${name} must be between 0 and 2147483647.`, '3mf')
  }
  return Number(numeric)
}

function threeMfNumberAttribute(element: XmlElement, name: string): number {
  return finiteNumber(requiredAttributeNs(element, name, null, '3mf'), `3MF ${name}`, '3mf')
}

function parse3mfTransform(value: string | undefined, scale: number): Matrix4 {
  if (value === undefined || value.trim() === '') return IDENTITY_4
  const tokens = value.trim().split(/\s+/u)
  if (tokens.length !== 12) return dataError('E_IMPORT_INVALID_DATA', '3MF transform must contain 12 numbers.', '3mf')
  const numbers = tokens.map(token => finiteNumber(token, '3MF transform', '3mf'))
  // 3MF serializes a row-vector 4x3 affine matrix; convert to our column-vector 4x4 form.
  return [
    numbers[0], numbers[3], numbers[6], numbers[9] * scale,
    numbers[1], numbers[4], numbers[7], numbers[10] * scale,
    numbers[2], numbers[5], numbers[8], numbers[11] * scale,
    0, 0, 0, 1,
  ]
}

type ThreeMfObject =
  | { readonly kind: 'mesh'; readonly mesh: RawMesh3D }
  | { readonly kind: 'components'; readonly components: readonly { readonly objectId: string; readonly transform: Matrix4 }[] }

/** Parse a bounded core 3MF package, including build items and component graphs. */
async function parseOpenScad3mfWithBudget(
  file: OpenScadProjectFile,
  projectBudget?: OpenScadImportPreparationBudget,
): Promise<OpenScadImportGeometry3D> {
  const files = await unzip3mf(bytesOf(file), projectBudget)
  const modelPath = modelPathFromPackage(files)
  const modelBytes = files.get(modelPath)!
  let root: XmlElement
  try {
    root = parseXml(UTF8_FATAL.decode(modelBytes), '3mf')
  } catch (error) {
    if (error instanceof OpenScadImportDataError) throw error
    return dataError('E_IMPORT_ENCODING', '3MF model must be UTF-8 XML.', '3mf')
  }
  if (root.localName !== 'model' || root.namespaceUri !== THREE_MF_CORE_NAMESPACE) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF model root must use the official core namespace.', '3mf')
  }
  const requiredExtensions = attributeNs(root, 'requiredextensions', null)?.trim()
  if (requiredExtensions) {
    return dataError('E_IMPORT_UNSUPPORTED_FEATURE', `3MF requires unsupported extensions: ${requiredExtensions}.`, '3mf')
  }
  const scale = unitScale(attributeNs(root, 'unit', null), '3mf')
  const resourceElements = childElementsNs(root, 'resources', THREE_MF_CORE_NAMESPACE)
  const buildElements = childElementsNs(root, 'build', THREE_MF_CORE_NAMESPACE)
  if (resourceElements.length !== 1 || buildElements.length !== 1) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF model requires exactly one core resources and build element.', '3mf')
  }
  const resources = resourceElements[0], build = buildElements[0]
  const objects = new Map<string, ThreeMfObject>()
  const resourceIds = new Set<string>()
  for (const resource of resources.children.filter(child => child.namespaceUri === THREE_MF_CORE_NAMESPACE)) {
    const rawId = attributeNs(resource, 'id', null)
    if (rawId === undefined) continue
    const id = threeMfResourceId(rawId, `3MF ${resource.localName} id`)
    if (resourceIds.has(id)) return dataError('E_IMPORT_INVALID_DATA', `3MF resource id ${id} is duplicated.`, '3mf')
    resourceIds.add(id)
  }
  for (const object of childElementsNs(resources, 'object', THREE_MF_CORE_NAMESPACE)) {
    const id = threeMfResourceIdAttribute(object, 'id')
    const meshElement = childElementNs(object, 'mesh', THREE_MF_CORE_NAMESPACE)
    const componentsElement = childElementNs(object, 'components', THREE_MF_CORE_NAMESPACE)
    if ((meshElement === null) === (componentsElement === null)) {
      return dataError('E_IMPORT_INVALID_DATA', `3MF object ${id} must contain exactly one mesh or components element.`, '3mf')
    }
    if (meshElement !== null) {
      const verticesElement = childElementNs(meshElement, 'vertices', THREE_MF_CORE_NAMESPACE)
      const trianglesElement = childElementNs(meshElement, 'triangles', THREE_MF_CORE_NAMESPACE)
      if (verticesElement === null || trianglesElement === null) {
        return dataError('E_IMPORT_INVALID_DATA', `3MF mesh ${id} requires vertices and triangles.`, '3mf')
      }
      const vertices: number[] = []
      for (const vertex of childElementsNs(verticesElement, 'vertex', THREE_MF_CORE_NAMESPACE)) {
        vertices.push(
          threeMfNumberAttribute(vertex, 'x') * scale,
          threeMfNumberAttribute(vertex, 'y') * scale,
          threeMfNumberAttribute(vertex, 'z') * scale,
        )
        boundedCount(vertices.length / 3, OPENSCAD_IMPORT_MAX_VERTICES, '3MF object vertex count', '3mf')
      }
      const triangles: number[] = []
      for (const triangle of childElementsNs(trianglesElement, 'triangle', THREE_MF_CORE_NAMESPACE)) {
        for (const name of ['v1', 'v2', 'v3'] as const) {
          const index = threeMfResourceIndexAttribute(triangle, name)
          if (index >= vertices.length / 3) {
            return dataError('E_IMPORT_INVALID_DATA', `3MF ${name} index is out of bounds.`, '3mf')
          }
          triangles.push(index)
        }
        boundedCount(triangles.length / 3, OPENSCAD_IMPORT_MAX_TRIANGLES, '3MF object triangle count', '3mf')
      }
      objects.set(id, { kind: 'mesh', mesh: Object.freeze({ vertices, triangles }) })
    } else {
      const components = childElementsNs(componentsElement!, 'component', THREE_MF_CORE_NAMESPACE).map(component => {
        return Object.freeze({
          objectId: threeMfResourceIdAttribute(component, 'objectid'),
          transform: parse3mfTransform(attributeNs(component, 'transform', null), scale),
        })
      })
      objects.set(id, { kind: 'components', components: Object.freeze(components) })
    }
  }

  const vertices: number[] = []
  const triangles: number[] = []
  let expansions = 0
  const expand = (id: string, matrix: Matrix4, stack: readonly string[]): void => {
    const object = objects.get(id)
    if (!object) return dataError('E_IMPORT_INVALID_DATA', `3MF refers to missing object ${id}.`, '3mf')
    if (stack.includes(id)) return dataError('E_IMPORT_INVALID_DATA', `3MF component cycle includes object ${id}.`, '3mf')
    expansions++
    if (expansions > OPENSCAD_IMPORT_MAX_XML_ELEMENTS) {
      return dataError('E_IMPORT_LIMIT', '3MF component expansion limit exceeded.', '3mf', OPENSCAD_IMPORT_MAX_XML_ELEMENTS, expansions)
    }
    if (object.kind === 'mesh') {
      appendRawMesh(vertices, triangles, object.mesh, matrix, '3mf')
      return
    }
    if (stack.length >= OPENSCAD_IMPORT_MAX_XML_DEPTH) {
      return dataError('E_IMPORT_LIMIT', '3MF component nesting limit exceeded.', '3mf', OPENSCAD_IMPORT_MAX_XML_DEPTH, stack.length + 1)
    }
    for (const component of object.components) {
      expand(component.objectId, multiply4(matrix, component.transform), [...stack, id])
    }
  }
  const items = childElementsNs(build, 'item', THREE_MF_CORE_NAMESPACE)
  if (items.length === 0) return dataError('E_IMPORT_EMPTY', '3MF build contains no items.', '3mf')
  for (const item of items) {
    expand(
      threeMfResourceIdAttribute(item, 'objectid'),
      parse3mfTransform(attributeNs(item, 'transform', null), scale),
      [],
    )
  }
  return geometry3D('3mf', vertices, triangles)
}

export async function parseOpenScad3mf(file: OpenScadProjectFile): Promise<OpenScadImportGeometry3D> {
  return parseOpenScad3mfWithBudget(file)
}

function cleanContour(points: readonly OpenScadImportPoint2[]): OpenScadImportPoint2[] {
  const output: OpenScadImportPoint2[] = []
  for (const point of points) {
    if (!Number.isFinite(point[0]) || !Number.isFinite(point[1])) {
      return dataError('E_IMPORT_INVALID_DATA', '2D import contains a non-finite coordinate.')
    }
    const previous = output.at(-1)
    if (!previous || previous[0] !== point[0] || previous[1] !== point[1]) output.push([point[0], point[1]])
  }
  if (output.length > 1 && output[0][0] === output.at(-1)![0] && output[0][1] === output.at(-1)![1]) output.pop()
  return output
}

function geometry2D(
  format: OpenScadImportGeometry2D['format'],
  sourceRegions: readonly OpenScadImportRegion2D[],
): OpenScadImportGeometry2D {
  const regions: OpenScadImportRegion2D[] = []
  let pointCount = 0
  let contourCount = 0
  for (const region of sourceRegions) {
    const contours: OpenScadImportPoint2[][] = []
    for (const sourceContour of region.contours) {
      const contour = cleanContour(sourceContour)
      if (contour.length < 3) continue
      let area = 0
      for (let index = 0; index < contour.length; index++) {
        const a = contour[index], b = contour[(index + 1) % contour.length]
        area += a[0] * b[1] - b[0] * a[1]
      }
      if (area === 0) continue
      pointCount += contour.length
      contourCount++
      boundedCount(pointCount, OPENSCAD_IMPORT_MAX_2D_POINTS, `${format.toUpperCase()} point count`, format)
      boundedCount(contourCount, OPENSCAD_IMPORT_MAX_CONTOURS, `${format.toUpperCase()} contour count`, format)
      contours.push(contour)
    }
    if (contours.length) regions.push(Object.freeze({
      contours: Object.freeze(contours.map(contour => Object.freeze(contour))),
      fillRule: region.fillRule,
      ...(region.layer === undefined ? {} : { layer: region.layer }),
    }))
  }
  if (regions.length === 0) return dataError('E_IMPORT_EMPTY', `${format.toUpperCase()} contains no filled 2D geometry.`, format)
  return Object.freeze({ dimension: 2 as const, format, regions: Object.freeze(regions), pointCount })
}

type Matrix2 = readonly [number, number, number, number, number, number]
const IDENTITY_2: Matrix2 = [1, 0, 0, 1, 0, 0]

function multiply2(left: Matrix2, right: Matrix2): Matrix2 {
  return [
    left[0] * right[0] + left[2] * right[1],
    left[1] * right[0] + left[3] * right[1],
    left[0] * right[2] + left[2] * right[3],
    left[1] * right[2] + left[3] * right[3],
    left[0] * right[4] + left[2] * right[5] + left[4],
    left[1] * right[4] + left[3] * right[5] + left[5],
  ]
}

function transform2(matrix: Matrix2, point: OpenScadImportPoint2): OpenScadImportPoint2 {
  return [
    matrix[0] * point[0] + matrix[2] * point[1] + matrix[4],
    matrix[1] * point[0] + matrix[3] * point[1] + matrix[5],
  ]
}

/** SVG geometry in the captured OpenSCAD millimeter/DPI coordinate convention. */
export function parseOpenScadSvg(file: OpenScadProjectFile, dpi = 72, options: SvgOptions = {}): OpenScadImportGeometry2D {
  try {
    const parsed = readSvgDocument(textOf(file, 'svg'), { ...options, dpi }, 'parse', true)
    return geometry2D('svg', parsed.regions)
  } catch (error) {
    if (error instanceof OpenScadImportDataError) throw error
    const code = error && typeof error === 'object' && 'code' in error ? String(error.code) : ''
    const message = error instanceof Error ? error.message : 'SVG conversion failed.'
    return dataError(
      ['E_IMPORT_FORMAT_UNSUPPORTED', 'E_IMPORT_ENCODING', 'E_IMPORT_INVALID_DATA', 'E_IMPORT_UNSUPPORTED_FEATURE', 'E_IMPORT_LIMIT', 'E_IMPORT_EMPTY'].includes(code)
        ? code as OpenScadImportDataError['code']
        : /limit|exceeds/i.test(message) ? 'E_IMPORT_LIMIT' : 'E_IMPORT_INVALID_DATA',
      message, 'svg',
    )
  }
}

interface DxfPair { readonly code: number; readonly value: string }
interface DxfEntity { readonly type: string; readonly pairs: readonly DxfPair[] }

function dxfPairs(text: string): DxfPair[] {
  if (text.replace(/^\uFEFF/u, '').startsWith('AutoCAD Binary DXF')) {
    return dataError('E_IMPORT_UNSUPPORTED_FEATURE', 'Binary DXF is unsupported by the OpenSCAD 2021.01 importer.', 'dxf')
  }
  const lines = text.replace(/^\uFEFF/u, '').split(/\r\n|\n|\r/u)
  while (lines.length && lines.at(-1)!.trim() === '') lines.pop()
  if (lines.length % 2 !== 0) return dataError('E_IMPORT_INVALID_DATA', 'DXF must contain group-code/value line pairs.', 'dxf')
  const pairs: DxfPair[] = []
  for (let line = 0; line < lines.length; line += 2) {
    const code = Number(lines[line].trim())
    if (!Number.isSafeInteger(code) || code < 0 || code > 1071) {
      return dataError('E_IMPORT_INVALID_DATA', `DXF group code on line ${line + 1} is invalid.`, 'dxf')
    }
    pairs.push({ code, value: lines[line + 1].trim() })
    if (pairs.length > OPENSCAD_IMPORT_MAX_XML_ELEMENTS * 4) {
      return dataError(
        'E_IMPORT_LIMIT',
        'DXF group pair limit exceeded.',
        'dxf',
        OPENSCAD_IMPORT_MAX_XML_ELEMENTS * 4,
        pairs.length,
      )
    }
  }
  return pairs
}

function dxfSections(pairs: readonly DxfPair[]): ReadonlyMap<string, readonly DxfEntity[]> {
  const sections = new Map<string, DxfEntity[]>()
  let section = ''
  let current: { type: string; pairs: DxfPair[] } | null = null
  const flush = (): void => {
    if (current && section) {
      const target = sections.get(section) ?? []
      target.push(Object.freeze({ type: current.type, pairs: Object.freeze(current.pairs) }))
      sections.set(section, target)
    }
    current = null
  }
  for (let index = 0; index < pairs.length; index++) {
    const pair = pairs[index]
    if (pair.code === 0 && pair.value.toUpperCase() === 'SECTION') {
      flush()
      const name = pairs[++index]
      if (!name || name.code !== 2) return dataError('E_IMPORT_INVALID_DATA', 'DXF SECTION is missing its name.', 'dxf')
      section = name.value.toUpperCase()
      continue
    }
    if (pair.code === 0 && pair.value.toUpperCase() === 'ENDSEC') {
      flush()
      section = ''
      continue
    }
    if (!section) continue
    if (pair.code === 0) {
      flush()
      current = { type: pair.value.toUpperCase(), pairs: [] }
    } else if (current) current.pairs.push(pair)
  }
  flush()
  return sections
}

function dxfValues(entity: DxfEntity, code: number): string[] {
  return entity.pairs.filter(pair => pair.code === code).map(pair => pair.value)
}

function dxfValue(entity: DxfEntity, code: number, fallback?: string): string {
  const value = entity.pairs.find(pair => pair.code === code)?.value ?? fallback
  if (value === undefined) return dataError('E_IMPORT_INVALID_DATA', `DXF ${entity.type} is missing group ${code}.`, 'dxf')
  return value
}

function dxfNumber(entity: DxfEntity, code: number, fallback?: number): number {
  const value = dxfValues(entity, code)[0]
  return value === undefined && fallback !== undefined
    ? fallback
    : finiteNumber(value ?? '', `DXF ${entity.type} group ${code}`, 'dxf')
}

function dxfInteger(entity: DxfEntity, code: number, fallback = 0): number {
  const value = dxfNumber(entity, code, fallback)
  if (!Number.isSafeInteger(value)) return dataError('E_IMPORT_INVALID_DATA', `DXF ${entity.type} group ${code} must be an integer.`, 'dxf')
  return value
}

function curveSteps(sweep: number, segments = OPENSCAD_IMPORT_DEFAULT_CURVE_SEGMENTS): number {
  return Math.max(2, Math.min(512, Math.ceil(Math.abs(sweep) / (Math.PI * 2) * segments)))
}

function arcPoints(
  center: OpenScadImportPoint2,
  radiusX: number,
  radiusY: number,
  start: number,
  sweep: number,
  axisAngle = 0,
): OpenScadImportPoint2[] {
  if (!(radiusX > 0) || !(radiusY > 0)) return []
  const steps = curveSteps(sweep)
  const cosine = Math.cos(axisAngle), sine = Math.sin(axisAngle)
  const points: OpenScadImportPoint2[] = []
  for (let index = 0; index <= steps; index++) {
    const angle = start + sweep * index / steps
    const x = radiusX * Math.cos(angle), y = radiusY * Math.sin(angle)
    points.push([center[0] + cosine * x - sine * y, center[1] + sine * x + cosine * y])
  }
  return points
}

function bulgePoints(
  start: OpenScadImportPoint2,
  end: OpenScadImportPoint2,
  bulge: number,
): OpenScadImportPoint2[] {
  if (bulge === 0) return [start, end]
  const dx = end[0] - start[0], dy = end[1] - start[1]
  const chord = Math.hypot(dx, dy)
  if (!(chord > 0)) return [start]
  const sweep = 4 * Math.atan(bulge)
  const radius = chord / (2 * Math.sin(Math.abs(sweep) / 2))
  const midpoint: OpenScadImportPoint2 = [(start[0] + end[0]) / 2, (start[1] + end[1]) / 2]
  const distance = chord / (2 * Math.tan(sweep / 2))
  const center: OpenScadImportPoint2 = [midpoint[0] - dy / chord * distance, midpoint[1] + dx / chord * distance]
  const startAngle = Math.atan2(start[1] - center[1], start[0] - center[0])
  return arcPoints(center, radius, radius, startAngle, sweep)
}

interface DxfSegment {
  readonly layer: string
  readonly points: readonly OpenScadImportPoint2[]
}

function pointNear(left: OpenScadImportPoint2, right: OpenScadImportPoint2): boolean {
  return Math.round(left[0] * 1024) === Math.round(right[0] * 1024)
    && Math.round(left[1] * 1024) === Math.round(right[1] * 1024)
}

function stitchDxfSegments(segments: readonly DxfSegment[]): OpenScadImportRegion2D[] {
  const regions: OpenScadImportRegion2D[] = []
  const layers = new Map<string, DxfSegment[]>()
  for (const segment of segments) {
    const target = layers.get(segment.layer) ?? []
    target.push(segment)
    layers.set(segment.layer, target)
  }
  for (const [layer, source] of layers) {
    const degree = new Map<string, number>()
    const endpointKey = (point: OpenScadImportPoint2): string => `${Math.round(point[0] * 1024)},${Math.round(point[1] * 1024)}`
    for (const segment of source) {
      for (const endpoint of [segment.points[0], segment.points.at(-1)!]) {
        const key = endpointKey(endpoint)
        const next = (degree.get(key) ?? 0) + 1
        if (next > 2) return dataError('E_IMPORT_INVALID_DATA', `DXF layer ${layer} contains a branch vertex.`, 'dxf')
        degree.set(key, next)
      }
    }
    const pending = [...source]
    while (pending.length) {
      const openIndex = pending.findIndex(segment => (
        degree.get(endpointKey(segment.points[0])) === 1
        || degree.get(endpointKey(segment.points.at(-1)!)) === 1
      ))
      const first = pending.splice(openIndex < 0 ? pending.length - 1 : openIndex, 1)[0]
      const path = [...first.points]
      let changed = true
      while (changed && !pointNear(path[0], path.at(-1)!)) {
        changed = false
        for (let index = 0; index < pending.length; index++) {
          const candidate = pending[index]
          const start = candidate.points[0], end = candidate.points.at(-1)!
          if (pointNear(path.at(-1)!, start)) path.push(...candidate.points.slice(1))
          else if (pointNear(path.at(-1)!, end)) path.push(...[...candidate.points].reverse().slice(1))
          else if (pointNear(path[0], end)) path.unshift(...candidate.points.slice(0, -1))
          else if (pointNear(path[0], start)) path.unshift(...[...candidate.points].reverse().slice(0, -1))
          else continue
          pending.splice(index, 1)
          changed = true
          break
        }
      }
      if (path.length >= 3) {
        if (pointNear(path[0], path.at(-1)!)) path.pop()
        regions.push({ contours: [path], fillRule: 'evenodd', layer })
      }
    }
  }
  return regions
}

function dxfPolyline(
  points: readonly OpenScadImportPoint2[],
  closed: boolean,
): OpenScadImportPoint2[] {
  return closed ? [...points, points[0]] : [...points]
}

function dxfLwPolyline(entity: DxfEntity): OpenScadImportPoint2[] {
  const points: [number, number | null][] = []
  for (const pair of entity.pairs) {
    if (pair.code === 10) points.push([finiteNumber(pair.value, 'DXF polyline x', 'dxf'), null])
    else if (pair.code === 20 && points.length) {
      if (points.at(-1)![1] !== null) return dataError('E_IMPORT_INVALID_DATA', 'DXF LWPOLYLINE repeats a y coordinate.', 'dxf')
      points.at(-1)![1] = finiteNumber(pair.value, 'DXF polyline y', 'dxf')
    }
    // OpenSCAD 2021.01 intentionally treats group-42 bulges as straight chords.
  }
  if (points.length < 2 || points.some(point => point[1] === null)) {
    return dataError('E_IMPORT_INVALID_DATA', 'DXF LWPOLYLINE has inconsistent vertices.', 'dxf')
  }
  return points.map(point => [point[0], point[1]!] as const)
}

function entityLayer(entity: DxfEntity, inherited = '0'): string {
  const layer = dxfValue(entity, 8, inherited)
  return layer === '0' ? inherited : layer
}

interface DxfBlock { readonly base: OpenScadImportPoint2; readonly entities: readonly DxfEntity[] }

interface OpenScadDxfIntermediate {
  readonly regions: readonly OpenScadImportRegion2D[]
  readonly queryMetadata: OpenScadDxfQueryMetadata
}

function dxfDimensionMetadata(
  sections: ReadonlyMap<string, readonly DxfEntity[]>,
): readonly OpenScadDxfDimensionMetadata[] {
  const dimensions: OpenScadDxfDimensionMetadata[] = []
  // Map iteration preserves section order, which is also the order in which
  // the 2021 stream parser encounters DIMENSION records.
  for (const [section, entities] of sections) {
    if (section !== 'BLOCKS' && section !== 'ENTITIES') continue
    for (const entity of entities) {
      if (entity.type !== 'DIMENSION') continue
      boundedCount(
        dimensions.length + 1,
        OPENSCAD_IMPORT_MAX_DXF_QUERY_RECORDS,
        'DXF query record count',
        'dxf',
      )
      const coordinates = Array.from({ length: 7 }, (_, index) => Object.freeze([
        dxfNumber(entity, 10 + index, 0),
        dxfNumber(entity, 20 + index, 0),
      ] as const))
      dimensions.push(Object.freeze({
        layer: dxfValue(entity, 8, ''),
        name: dxfValue(entity, 1, ''),
        type: dxfInteger(entity, 70, 0),
        angle: dxfNumber(entity, 50, 0),
        coordinates: Object.freeze(coordinates),
        modelSpace: section === 'ENTITIES',
      }))
    }
  }
  return Object.freeze(dimensions)
}

function parseOpenScadDxfIntermediate(file: OpenScadProjectFile): OpenScadDxfIntermediate {
  const sections = dxfSections(dxfPairs(textOf(file, 'dxf')))
  const entities = sections.get('ENTITIES') ?? []
  const blockEntities = sections.get('BLOCKS') ?? []
  const blocks = new Map<string, DxfBlock>()
  for (let cursor = 0; cursor < blockEntities.length;) {
    const header = blockEntities[cursor++]
    if (header.type !== 'BLOCK') continue
    const name = dxfValue(header, 2, dxfValue(header, 3, ''))
    const members: DxfEntity[] = []
    while (cursor < blockEntities.length && blockEntities[cursor].type !== 'ENDBLK') members.push(blockEntities[cursor++])
    if (cursor < blockEntities.length) cursor++
    if (name) blocks.set(name, {
      base: [dxfNumber(header, 10, 0), dxfNumber(header, 20, 0)],
      entities: Object.freeze(members),
    })
  }

  const regions: OpenScadImportRegion2D[] = []
  const segments: DxfSegment[] = []
  const querySegments: OpenScadDxfLineSegmentMetadata[] = []
  let expansions = 0
  const recordQuerySegments = (
    points: readonly OpenScadImportPoint2[],
    layer: string,
  ): void => {
    for (let index = 1; index < points.length; index++) {
      boundedCount(
        querySegments.length + 1,
        OPENSCAD_IMPORT_MAX_DXF_QUERY_RECORDS,
        'DXF query record count',
        'dxf',
      )
      querySegments.push({ layer, start: points[index - 1], end: points[index] })
    }
  }
  const addPolyline = (
    points: readonly OpenScadImportPoint2[],
    closed: boolean,
    layer: string,
    matrix: Matrix2,
  ): void => {
    const transformed = dxfPolyline(points, closed).map(point => transform2(matrix, point))
    recordQuerySegments(transformed, layer)
    if (closed && transformed.length >= 3) regions.push({ contours: [transformed], fillRule: 'evenodd', layer })
    else if (transformed.length >= 2) segments.push({ layer, points: transformed })
  }
  const visit = (items: readonly DxfEntity[], matrix: Matrix2, inheritedLayer: string, stack: readonly string[]): void => {
    for (let cursor = 0; cursor < items.length; cursor++) {
      const entity = items[cursor]
      const layer = entityLayer(entity, inheritedLayer)
      if (entity.type === 'LINE') {
        addPolyline([
          [dxfNumber(entity, 10), dxfNumber(entity, 20)],
          [dxfNumber(entity, 11), dxfNumber(entity, 21)],
        ], false, layer, matrix)
      } else if (entity.type === 'LWPOLYLINE') {
        addPolyline(dxfLwPolyline(entity), (dxfInteger(entity, 70) & 1) !== 0, layer, matrix)
      } else if (entity.type === 'POLYLINE') {
        const vertices: OpenScadImportPoint2[] = []
        while (cursor + 1 < items.length && items[cursor + 1].type === 'VERTEX') {
          const vertex = items[++cursor]
          vertices.push([dxfNumber(vertex, 10), dxfNumber(vertex, 20)])
        }
        if (cursor + 1 < items.length && items[cursor + 1].type === 'SEQEND') cursor++
        if (vertices.length >= 2) addPolyline(vertices, (dxfInteger(entity, 70) & 1) !== 0, layer, matrix)
      } else if (entity.type === 'CIRCLE') {
        const center: OpenScadImportPoint2 = [dxfNumber(entity, 10), dxfNumber(entity, 20)]
        const radius = dxfNumber(entity, 40)
        const contour = arcPoints(center, radius, radius, 0, Math.PI * 2).slice(0, -1).map(point => transform2(matrix, point))
        if (contour.length >= 2) recordQuerySegments([...contour, contour[0]], layer)
        regions.push({ contours: [contour], fillRule: 'evenodd', layer })
      } else if (entity.type === 'ARC') {
        const start = dxfNumber(entity, 50) * Math.PI / 180
        let sweep = (dxfNumber(entity, 51) - dxfNumber(entity, 50)) * Math.PI / 180
        while (sweep <= 0) sweep += Math.PI * 2
        addPolyline(arcPoints(
          [dxfNumber(entity, 10), dxfNumber(entity, 20)],
          dxfNumber(entity, 40),
          dxfNumber(entity, 40),
          start,
          sweep,
        ), false, layer, matrix)
      } else if (entity.type === 'ELLIPSE') {
        const major: OpenScadImportPoint2 = [dxfNumber(entity, 11), dxfNumber(entity, 21)]
        const radius = Math.hypot(...major)
        const ratio = dxfNumber(entity, 40)
        const start = dxfNumber(entity, 41, 0)
        let sweep = dxfNumber(entity, 42, Math.PI * 2) - start
        while (sweep <= 0) sweep += Math.PI * 2
        const curve = arcPoints(
          [dxfNumber(entity, 10), dxfNumber(entity, 20)],
          radius,
          radius * ratio,
          start,
          sweep,
          Math.atan2(major[1], major[0]),
        )
        if (Math.abs(sweep - Math.PI * 2) < 1e-8) {
          const contour = curve.slice(0, -1).map(point => transform2(matrix, point))
          if (contour.length >= 2) recordQuerySegments([...contour, contour[0]], layer)
          regions.push({ contours: [contour], fillRule: 'evenodd', layer })
        } else addPolyline(curve, false, layer, matrix)
      } else if (entity.type === 'INSERT') {
        const name = dxfValue(entity, 2)
        const block = blocks.get(name)
        if (!block) return dataError('E_IMPORT_INVALID_DATA', `DXF INSERT refers to missing block ${name}.`, 'dxf')
        if (stack.includes(name)) return dataError('E_IMPORT_INVALID_DATA', `DXF block cycle includes ${name}.`, 'dxf')
        expansions++
        if (expansions > OPENSCAD_IMPORT_MAX_XML_ELEMENTS) {
          return dataError('E_IMPORT_LIMIT', 'DXF block expansion limit exceeded.', 'dxf', OPENSCAD_IMPORT_MAX_XML_ELEMENTS, expansions)
        }
        const rotation = dxfNumber(entity, 50, 0) * Math.PI / 180
        const sx = dxfNumber(entity, 41, 1), sy = dxfNumber(entity, 42, 1)
        const insertion: OpenScadImportPoint2 = [dxfNumber(entity, 10, 0), dxfNumber(entity, 20, 0)]
        if (sx === 0 || sy === 0) return dataError('E_IMPORT_INVALID_DATA', 'DXF INSERT scale cannot be zero.', 'dxf')
        const a = Math.cos(rotation) * sx, b = Math.sin(rotation) * sx
        const c = -Math.sin(rotation) * sy, d = Math.cos(rotation) * sy
        const local: Matrix2 = [
          a, b, c, d,
          insertion[0] - a * block.base[0] - c * block.base[1],
          insertion[1] - b * block.base[0] - d * block.base[1],
        ]
        const columns = dxfInteger(entity, 70, 1)
        const rows = dxfInteger(entity, 71, 1)
        if (columns < 1 || rows < 1) return dataError('E_IMPORT_INVALID_DATA', 'DXF INSERT array dimensions must be positive.', 'dxf')
        boundedCount(columns * rows, OPENSCAD_IMPORT_MAX_XML_ELEMENTS, 'DXF INSERT array size', 'dxf')
        for (let row = 0; row < rows; row++) {
          for (let column = 0; column < columns; column++) {
            const offset: Matrix2 = [1, 0, 0, 1, column * dxfNumber(entity, 44, 0), row * dxfNumber(entity, 45, 0)]
            visit(block.entities, multiply2(matrix, multiply2(local, offset)), layer, [...stack, name])
          }
        }
      }
    }
  }
  visit(entities, IDENTITY_2, '0', [])
  regions.push(...stitchDxfSegments(segments))
  const dimensions = dxfDimensionMetadata(sections)
  const recordCount = dimensions.length + querySegments.length
  boundedCount(recordCount, OPENSCAD_IMPORT_MAX_DXF_QUERY_RECORDS, 'DXF query record count', 'dxf')
  return Object.freeze({
    regions: Object.freeze(regions),
    queryMetadata: Object.freeze({
      dimensions,
      lineSegments: Object.freeze(querySegments.map(segment => Object.freeze({
        layer: segment.layer,
        start: Object.freeze([segment.start[0], segment.start[1]] as const),
        end: Object.freeze([segment.end[0], segment.end[1]] as const),
      }))),
      recordCount,
    }),
  })
}

/** Parse the bounded metadata consumed by dxf_dim() and dxf_cross(). */
export function parseOpenScadDxfQueryMetadata(file: OpenScadProjectFile): OpenScadDxfQueryMetadata {
  return parseOpenScadDxfIntermediate(file).queryMetadata
}

/** Parse ASCII DXF geometry, layers, bulged polylines, curves, blocks and inserts. */
export function parseOpenScadDxf(file: OpenScadProjectFile): OpenScadImportGeometry2D {
  return geometry2D('dxf', parseOpenScadDxfIntermediate(file).regions)
}

async function parseOpenScadImportFile(
  file: OpenScadProjectFile,
  format: OpenScadImportFormat,
  dpi = 72,
  projectBudget?: OpenScadImportPreparationBudget,
): Promise<OpenScadImportGeometry> {
  switch (format) {
    case 'svg': return parseOpenScadSvg(file, dpi)
    case 'dxf': return parseOpenScadDxf(file)
    case 'stl': return parseOpenScadStl(file)
    case 'off': return parseOpenScadOff(file)
    case 'amf': return parseOpenScadAmfWithBudget(file, projectBudget)
    case '3mf': return parseOpenScad3mfWithBudget(file, projectBudget)
  }
}

function reservePreparedGeometry(
  budget: OpenScadImportPreparationBudget,
  geometry: OpenScadImportGeometry,
): void {
  if (geometry.dimension === 2) {
    const points = budget.points + geometry.pointCount
    if (!Number.isSafeInteger(points) || points > OPENSCAD_IMPORT_MAX_PROJECT_2D_POINTS) {
      return dataError(
        'E_IMPORT_LIMIT',
        `Prepared project 2D geometry exceeds ${OPENSCAD_IMPORT_MAX_PROJECT_2D_POINTS.toLocaleString()} points.`,
        geometry.format,
        OPENSCAD_IMPORT_MAX_PROJECT_2D_POINTS,
        points,
      )
    }
    budget.points = points
    return
  }
  const vertices = budget.vertices + geometry.vertices.length / 3
  const triangles = budget.triangles + geometry.triangleCount
  if (!Number.isSafeInteger(vertices) || vertices > OPENSCAD_IMPORT_MAX_PROJECT_VERTICES) {
    return dataError(
      'E_IMPORT_LIMIT',
      `Prepared project 3D geometry exceeds ${OPENSCAD_IMPORT_MAX_PROJECT_VERTICES.toLocaleString()} vertices.`,
      geometry.format,
      OPENSCAD_IMPORT_MAX_PROJECT_VERTICES,
      vertices,
    )
  }
  if (!Number.isSafeInteger(triangles) || triangles > OPENSCAD_IMPORT_MAX_PROJECT_TRIANGLES) {
    return dataError(
      'E_IMPORT_LIMIT',
      `Prepared project 3D geometry exceeds ${OPENSCAD_IMPORT_MAX_PROJECT_TRIANGLES.toLocaleString()} triangles.`,
      geometry.format,
      OPENSCAD_IMPORT_MAX_PROJECT_TRIANGLES,
      triangles,
    )
  }
  budget.vertices = vertices
  budget.triangles = triangles
}

function reservePreparedDxfQueryMetadata(
  budget: OpenScadImportPreparationBudget,
  metadata: OpenScadDxfQueryMetadata,
): void {
  const records = budget.dxfQueryRecords + metadata.recordCount
  if (!Number.isSafeInteger(records) || records > OPENSCAD_IMPORT_MAX_PROJECT_DXF_QUERY_RECORDS) {
    return dataError(
      'E_IMPORT_LIMIT',
      `Prepared project DXF query metadata exceeds ${OPENSCAD_IMPORT_MAX_PROJECT_DXF_QUERY_RECORDS.toLocaleString()} records.`,
      'dxf',
      OPENSCAD_IMPORT_MAX_PROJECT_DXF_QUERY_RECORDS,
      records,
    )
  }
  budget.dxfQueryRecords = records
}

interface PreparedImportDecode {
  readonly asset: OpenScadImportGeometry | OpenScadImportDataError
  readonly dxfQueryMetadata?: OpenScadDxfQueryMetadata | OpenScadImportDataError
}

async function prepareOpenScadImportAsset(
  file: OpenScadProjectFile,
  format: OpenScadImportFormat,
  budget: OpenScadImportPreparationBudget,
  svgOptions: SvgOptions = {},
): Promise<PreparedImportDecode> {
  if (format === 'dxf') {
    let intermediate: OpenScadDxfIntermediate
    try {
      intermediate = parseOpenScadDxfIntermediate(file)
    } catch (error) {
      if (!(error instanceof OpenScadImportDataError)) throw error
      return Object.freeze({ asset: error, dxfQueryMetadata: error })
    }

    let metadata: OpenScadDxfQueryMetadata | OpenScadImportDataError = intermediate.queryMetadata
    try {
      reservePreparedDxfQueryMetadata(budget, metadata)
    } catch (error) {
      if (!(error instanceof OpenScadImportDataError)) throw error
      metadata = error
    }

    let asset: OpenScadImportGeometry | OpenScadImportDataError
    try {
      asset = geometry2D('dxf', intermediate.regions)
      reservePreparedGeometry(budget, asset)
    } catch (error) {
      if (!(error instanceof OpenScadImportDataError)) throw error
      asset = error
    }
    return Object.freeze({ asset, dxfQueryMetadata: metadata })
  }

  try {
    const asset = format === 'svg'
      ? parseOpenScadSvg(file, 72, svgOptions)
      : await parseOpenScadImportFile(file, format, 72, budget)
    reservePreparedGeometry(budget, asset)
    return Object.freeze({ asset })
  } catch (error) {
    if (!(error instanceof OpenScadImportDataError)) throw error
    return Object.freeze({ asset: error })
  }
}

function forcedImportAssetKey(path: string, format: OpenScad2021LegacyImportFormat): string {
  return `${format}\0${path}`
}

/** SVG text uses only bundled or explicitly supplied project fonts. */
function svgProjectOptions(project: OpenScadProject): SvgOptions {
  const fonts: Uint8Array[] = []
  for (const summary of project.list()) {
    if (summary.kind !== 'blob' || !/\.(ttf|otf|ttc)$/i.test(summary.path)) continue
    const file = project.read(summary.path)
    if (file?.kind === 'blob') fonts.push(file.data)
  }
  return { fonts }
}

/** Decode every supported project asset once, without consulting the host filesystem. */
export async function prepareOpenScadImportAssets(
  project: OpenScadProject,
  options: OpenScadImportPreparationOptions = {},
): Promise<PreparedOpenScadImportAssets> {
  const assets = new Map<string, OpenScadImportGeometry | OpenScadImportDataError>()
  const forcedAssets = new Map<string, OpenScadImportGeometry | OpenScadImportDataError>()
  const dxfQueryMetadata = new Map<string, OpenScadDxfQueryMetadata | OpenScadImportDataError>()
  const svgOptions = project.list().some(file => /\.svg$/i.test(file.path)) ? svgProjectOptions(project) : {}
  const budget: OpenScadImportPreparationBudget = {
    vertices: 0,
    triangles: 0,
    points: 0,
    zipExpandedBytes: 0,
    dxfQueryRecords: 0,
  }
  for (const summary of project.list()) {
    const format = formatOfPath(summary.path)
    if (format === null) continue
    const file = project.read(summary.path)!
    const prepared = await prepareOpenScadImportAsset(file, format, budget, svgOptions)
    assets.set(summary.path, prepared.asset)
    if (prepared.dxfQueryMetadata !== undefined) {
      dxfQueryMetadata.set(summary.path, prepared.dxfQueryMetadata)
    }
  }

  const requests = options.forcedAssets ?? []
  if (requests.length > OPENSCAD_PROJECT_MAX_FILES * OPENSCAD_2021_LEGACY_IMPORT_FORMATS.length) {
    return dataError(
      'E_IMPORT_LIMIT',
      'Forced legacy import preparation request limit exceeded.',
      undefined,
      OPENSCAD_PROJECT_MAX_FILES * OPENSCAD_2021_LEGACY_IMPORT_FORMATS.length,
      requests.length,
    )
  }
  for (const request of requests) {
    if (!LEGACY_FORMAT_SET.has(request.format)) {
      throw new TypeError(`Unsupported forced legacy import format ${String(request.format)}`)
    }
    const path = normalizeOpenScadProjectPath(request.path)
    const key = forcedImportAssetKey(path, request.format)
    if (forcedAssets.has(key)) continue
    const inferredFormat = formatOfPath(path)
    if (inferredFormat === request.format && assets.has(path)) {
      forcedAssets.set(key, assets.get(path)!)
      continue
    }
    const file = project.read(path)
    if (file === null) continue
    const prepared = await prepareOpenScadImportAsset(file, request.format, budget)
    forcedAssets.set(key, prepared.asset)
    if (prepared.dxfQueryMetadata !== undefined && !dxfQueryMetadata.has(path)) {
      dxfQueryMetadata.set(path, prepared.dxfQueryMetadata)
    }
  }
  return Object.freeze({ assets, forcedAssets, dxfQueryMetadata })
}

function contextualImportError(
  error: OpenScadImportDataError,
  sourcePath: string,
  specifier: string,
  assetPath: string,
  format: OpenScadImportFormat,
): OpenScadImportError {
  return new OpenScadImportError(error.code, error.message, {
    sourcePath,
    specifier,
    assetPath,
    format,
    ...(error.limit === undefined ? {} : { limit: error.limit }),
    ...(error.actual === undefined ? {} : { actual: error.actual }),
  })
}

function validateImportOptions(
  options: OpenScadImportLoadOptions,
  sourcePath: string,
  specifier: string,
  assetPath: string,
): Required<Pick<OpenScadImportLoadOptions, 'origin' | 'scale' | 'center' | 'dpi'>> & Pick<OpenScadImportLoadOptions, 'layer'> {
  const origin = options.origin ?? [0, 0]
  const scale = options.scale ?? 1
  const dpi = options.dpi ?? 72
  if (!Array.isArray(origin) || origin.length !== 2 || !origin.every(Number.isFinite)) {
    throw new OpenScadImportError('E_IMPORT_ARGUMENT_INVALID', 'import() origin must contain two finite numbers.', {
      sourcePath, specifier, assetPath,
    })
  }
  if (typeof scale !== 'number' || !Number.isFinite(scale) || !(scale > 0)) {
    throw new OpenScadImportError('E_IMPORT_ARGUMENT_INVALID', 'import() scale must be a positive finite number.', {
      sourcePath, specifier, assetPath,
    })
  }
  if (typeof dpi !== 'number' || !Number.isFinite(dpi) || !(dpi > 0)) {
    throw new OpenScadImportError('E_IMPORT_ARGUMENT_INVALID', 'import() dpi must be a positive finite number.', {
      sourcePath, specifier, assetPath,
    })
  }
  if (options.layer !== undefined && typeof options.layer !== 'string') {
    throw new OpenScadImportError('E_IMPORT_ARGUMENT_INVALID', 'import() layer must be a string.', {
      sourcePath, specifier, assetPath,
    })
  }
  if (options.center !== undefined && typeof options.center !== 'boolean') {
    throw new OpenScadImportError('E_IMPORT_ARGUMENT_INVALID', 'import() center must be boolean.', {
      sourcePath, specifier, assetPath,
    })
  }
  return { origin: [origin[0], origin[1]], scale, center: options.center ?? false, dpi, layer: options.layer }
}

function transformLoadedGeometry(
  source: OpenScadImportGeometry,
  options: ReturnType<typeof validateImportOptions>,
): OpenScadImportGeometry {
  if (source.dimension === 2) {
    const selected = source.format === 'dxf' && options.layer
      ? source.regions.filter(region => region.layer === options.layer)
      : source.regions
    if (selected.length === 0) return dataError('E_IMPORT_EMPTY', `${source.format.toUpperCase()} layer contains no filled geometry.`, source.format)
    const applyDxfOptions = source.format === 'dxf'
    let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity
    const regions = selected.map(region => ({
      ...region,
      contours: region.contours.map(contour => contour.map(point => {
        const x = applyDxfOptions ? (point[0] - options.origin[0]) * options.scale : point[0]
        const y = applyDxfOptions ? (point[1] - options.origin[1]) * options.scale : point[1]
        minX = Math.min(minX, x); maxX = Math.max(maxX, x)
        minY = Math.min(minY, y); maxY = Math.max(maxY, y)
        return [x, y] as const
      })),
    }))
    const centerX = options.center ? (minX + maxX) / 2 : 0
    const centerY = options.center ? (minY + maxY) / 2 : 0
    return geometry2D(source.format, regions.map(region => ({
      ...region,
      contours: region.contours.map(contour => contour.map(point => [point[0] - centerX, point[1] - centerY] as const)),
    })))
  }
  const vertices = [...source.vertices]
  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity, minZ = Infinity, maxZ = -Infinity
  for (let offset = 0; offset < vertices.length; offset += 3) {
    minX = Math.min(minX, vertices[offset]); maxX = Math.max(maxX, vertices[offset])
    minY = Math.min(minY, vertices[offset + 1]); maxY = Math.max(maxY, vertices[offset + 1])
    minZ = Math.min(minZ, vertices[offset + 2]); maxZ = Math.max(maxZ, vertices[offset + 2])
  }
  if (options.center) {
    const centerX = (minX + maxX) / 2, centerY = (minY + maxY) / 2, centerZ = (minZ + maxZ) / 2
    for (let offset = 0; offset < vertices.length; offset += 3) {
      vertices[offset] -= centerX
      vertices[offset + 1] -= centerY
      vertices[offset + 2] -= centerZ
    }
  }
  return geometry3D(source.format, vertices, [...source.triangles])
}

function resolveImportAsset(
  project: OpenScadProject,
  sourcePath: string,
  specifier: string,
  forcedFormat?: OpenScad2021LegacyImportFormat,
): { readonly assetPath: string; readonly file: OpenScadProjectFile; readonly format: OpenScadImportFormat } {
  let assetPath: string
  try {
    assetPath = resolveOpenScadProjectPath(sourcePath, specifier)
  } catch (error) {
    if (!(error instanceof OpenScadProjectError)) throw error
    throw new OpenScadImportError('E_IMPORT_PATH_INVALID', error.message, { sourcePath, specifier })
  }
  const extension = assetPath.slice(assetPath.lastIndexOf('.') + 1).toLowerCase()
  if (forcedFormat === undefined && extension === 'nef3') {
    throw new OpenScadImportError(
      'E_IMPORT_NEF3_UNAVAILABLE',
      'NEF3 import is only available in CGAL-enabled OpenSCAD builds; this independent runtime uses Manifold.',
      { sourcePath, specifier, assetPath },
    )
  }
  if (forcedFormat !== undefined && !LEGACY_FORMAT_SET.has(forcedFormat)) {
    throw new OpenScadImportError(
      'E_IMPORT_FORMAT_UNSUPPORTED',
      `Unsupported forced legacy import format ${String(forcedFormat)}.`,
      { sourcePath, specifier, assetPath },
    )
  }
  const format = forcedFormat ?? formatOfPath(assetPath)
  if (format === null) {
    throw new OpenScadImportError(
      'E_IMPORT_FORMAT_UNSUPPORTED',
      `import() does not support the extension of ${assetPath}.`,
      { sourcePath, specifier, assetPath },
    )
  }
  const file = project.read(assetPath)
  if (file === null) {
    throw new OpenScadImportError('E_IMPORT_FILE_MISSING', `Imported project asset ${assetPath} is missing.`, {
      sourcePath, specifier, assetPath, format,
    })
  }
  return { assetPath, file, format }
}

/** Resolve and clone a pre-decoded import using only module-relative project paths. */
export function loadPreparedOpenScadImport(
  project: OpenScadProject,
  prepared: PreparedOpenScadImportAssets,
  sourcePath: string,
  specifier: string,
  options: OpenScadImportLoadOptions = {},
): OpenScadImportGeometry {
  const { assetPath, file, format } = resolveImportAsset(
    project,
    sourcePath,
    specifier,
    options.forcedFormat,
  )
  const validated = validateImportOptions(options, sourcePath, specifier, assetPath)
  let decoded = options.forcedFormat === undefined
    ? prepared.assets.get(assetPath)
    : prepared.forcedAssets.get(forcedImportAssetKey(assetPath, options.forcedFormat))
      ?? (formatOfPath(assetPath) === options.forcedFormat ? prepared.assets.get(assetPath) : undefined)
  if (format === 'svg' && validated.dpi !== 72) {
    try {
      decoded = parseOpenScadSvg(file, validated.dpi, svgProjectOptions(project))
    } catch (error) {
      if (!(error instanceof OpenScadImportDataError)) throw error
      throw contextualImportError(error, sourcePath, specifier, assetPath, format)
    }
  }
  if (decoded === undefined) {
    throw new OpenScadImportError('E_IMPORT_INVALID_DATA', `Imported project asset ${assetPath} was not prepared.`, {
      sourcePath, specifier, assetPath, format,
    })
  }
  if (decoded instanceof OpenScadImportDataError) {
    throw contextualImportError(decoded, sourcePath, specifier, assetPath, format)
  }
  try {
    return transformLoadedGeometry(cloneGeometry(decoded), validated)
  } catch (error) {
    if (!(error instanceof OpenScadImportDataError)) throw error
    throw contextualImportError(error, sourcePath, specifier, assetPath, format)
  }
}

interface ResolvedDxfQueryMetadata {
  readonly metadata: OpenScadDxfQueryMetadata
  readonly assetPath: string
}

function dxfQueryDiagnostic(
  code: OpenScadDxfQueryDiagnosticCode,
  message: string,
  sourcePath: string,
  specifier: string,
  details: Omit<OpenScadDxfQueryDiagnostic, 'severity' | 'code' | 'message' | 'sourcePath' | 'specifier'> = {},
): OpenScadDxfQueryDiagnostic {
  return Object.freeze({ severity: 'warning', code, message, sourcePath, specifier, ...details })
}

function dxfQueryResult<T>(
  value: T | undefined,
  diagnostics: readonly OpenScadDxfQueryDiagnostic[],
): OpenScadDxfQueryResult<T> {
  return Object.freeze({ value, diagnostics: Object.freeze([...diagnostics]) })
}

function resolvePreparedDxfQueryMetadata(
  project: OpenScadProject,
  prepared: PreparedOpenScadImportAssets,
  sourcePath: string,
  specifier: string,
): ResolvedDxfQueryMetadata | OpenScadDxfQueryDiagnostic {
  let assetPath: string
  try {
    assetPath = resolveOpenScadProjectPath(sourcePath, specifier)
  } catch (error) {
    const detail = error instanceof Error ? error.message : String(error)
    return dxfQueryDiagnostic(
      'OPENSCAD_DXF_QUERY_FILE_UNAVAILABLE',
      `DXF query path could not be resolved: ${detail}`,
      sourcePath,
      specifier,
    )
  }
  if (project.read(assetPath) === null) {
    return dxfQueryDiagnostic(
      'OPENSCAD_DXF_QUERY_FILE_UNAVAILABLE',
      `DXF query asset ${assetPath} is missing from the project.`,
      sourcePath,
      specifier,
      { assetPath },
    )
  }
  const metadata = prepared.dxfQueryMetadata.get(assetPath)
  if (metadata === undefined) {
    return dxfQueryDiagnostic(
      'OPENSCAD_DXF_QUERY_NOT_PREPARED',
      `DXF query asset ${assetPath} was not prepared as DXF.`,
      sourcePath,
      specifier,
      { assetPath },
    )
  }
  if (metadata instanceof OpenScadImportDataError) {
    return dxfQueryDiagnostic(
      'OPENSCAD_DXF_QUERY_INVALID_DATA',
      `DXF query asset ${assetPath} could not be decoded: ${metadata.message}`,
      sourcePath,
      specifier,
      { assetPath, importCode: metadata.code },
    )
  }
  return Object.freeze({ metadata, assetPath })
}

interface ResolvedDxfQueryTransform {
  readonly layer: string
  readonly origin: readonly [number, number]
  readonly scale: number
  readonly diagnostics: readonly OpenScadDxfQueryDiagnostic[]
}

function resolveDxfQueryTransform(
  options: OpenScadDxfQueryOptions,
  sourcePath: string,
  specifier: string,
): ResolvedDxfQueryTransform {
  const diagnostics: OpenScadDxfQueryDiagnostic[] = []
  let origin: readonly [number, number] = [0, 0]
  if (options.origin !== undefined) {
    const supplied: unknown = options.origin
    if (Array.isArray(supplied) && supplied.length === 2
      && supplied.every(value => typeof value === 'number')) {
      origin = [supplied[0], supplied[1]]
      if (!origin.every(Number.isFinite)) {
        diagnostics.push(dxfQueryDiagnostic(
          'OPENSCAD_DXF_QUERY_INVALID_ORIGIN',
          'DXF query origin contains a non-finite coordinate; 2021.01 arithmetic is preserved.',
          sourcePath,
          specifier,
        ))
      }
    } else {
      diagnostics.push(dxfQueryDiagnostic(
        'OPENSCAD_DXF_QUERY_INVALID_ORIGIN',
        'DXF query origin was not a two-number vector; [0, 0] is used.',
        sourcePath,
        specifier,
      ))
    }
  }
  // The 2021.01 implementation silently keeps the default when getDouble()
  // cannot read the value, and deliberately does not reject NaN or infinity.
  const scale = typeof options.scale === 'number' ? options.scale : 1
  return Object.freeze({
    layer: typeof options.layer === 'string' ? options.layer : '',
    origin: Object.freeze([origin[0], origin[1]] as const),
    scale,
    diagnostics: Object.freeze(diagnostics),
  })
}

function transformDxfDimensionCoordinates(
  dimension: OpenScadDxfDimensionMetadata,
  origin: readonly [number, number],
  scale: number,
): readonly OpenScadImportPoint2[] {
  if (!dimension.modelSpace) return dimension.coordinates
  return dimension.coordinates.map((point, index) => {
    // Groups 11/21, 12/22 and 16/26 are direction-like coordinates in the
    // upstream stream parser: they are scaled but do not receive origin.
    const subtractOrigin = index !== 1 && index !== 2 && index !== 6
    return [
      (point[0] - (subtractOrigin ? origin[0] : 0)) * scale,
      (point[1] - (subtractOrigin ? origin[1] : 0)) * scale,
    ] as const
  })
}

/** Evaluate OpenSCAD 2021.01 dxf_dim() against a prepared, bounded project asset. */
export function queryPreparedOpenScadDxfDimension(
  project: OpenScadProject,
  prepared: PreparedOpenScadImportAssets,
  sourcePath: string,
  specifier: string,
  options: OpenScadDxfDimensionQueryOptions = {},
): OpenScadDxfQueryResult<number> {
  const transform = resolveDxfQueryTransform(options, sourcePath, specifier)
  const diagnostics = [...transform.diagnostics]
  const resolved = resolvePreparedDxfQueryMetadata(project, prepared, sourcePath, specifier)
  if ('severity' in resolved) return dxfQueryResult<number>(undefined, [...diagnostics, resolved])
  const name = typeof options.name === 'string' ? options.name : ''
  const dimension = resolved.metadata.dimensions.find(candidate => (
    (transform.layer === '' || candidate.layer === transform.layer)
    && (name === '' || candidate.name === name)
  ))
  if (dimension === undefined) {
    diagnostics.push(dxfQueryDiagnostic(
      'OPENSCAD_DXF_DIMENSION_NOT_FOUND',
      `No matching DXF dimension was found in ${resolved.assetPath}.`,
      sourcePath,
      specifier,
      { assetPath: resolved.assetPath, layer: transform.layer, name },
    ))
    return dxfQueryResult<number>(undefined, diagnostics)
  }

  const points = transformDxfDimensionCoordinates(dimension, transform.origin, transform.scale)
  const type = dimension.type & 7
  let value: number | undefined
  if (type === 0) {
    const radians = dimension.angle * Math.PI / 180
    const x = points[4][0] - points[3][0]
    const y = points[4][1] - points[3][1]
    value = Math.abs(x * Math.cos(radians) + y * Math.sin(radians))
  } else if (type === 1) {
    value = Math.hypot(points[4][0] - points[3][0], points[4][1] - points[3][1])
  } else if (type === 2) {
    const first = Math.atan2(
      points[0][0] - points[5][0],
      points[0][1] - points[5][1],
    ) * 180 / Math.PI
    const second = Math.atan2(
      points[4][0] - points[3][0],
      points[4][1] - points[3][1],
    ) * 180 / Math.PI
    value = Math.abs(first - second)
  } else if (type === 3 || type === 4) {
    value = Math.hypot(points[5][0] - points[0][0], points[5][1] - points[0][1])
  } else if (type === 6) {
    value = (dimension.type & 64) !== 0 ? points[3][0] : points[3][1]
  }
  if (value === undefined) {
    diagnostics.push(dxfQueryDiagnostic(
      'OPENSCAD_DXF_DIMENSION_TYPE_UNSUPPORTED',
      `DXF dimension ${name || '<first>'} uses unsupported type ${type}.`,
      sourcePath,
      specifier,
      { assetPath: resolved.assetPath, layer: transform.layer, name },
    ))
  }
  return dxfQueryResult(value, diagnostics)
}

function transformDxfQueryPoint(
  point: OpenScadImportPoint2,
  origin: readonly [number, number],
  scale: number,
): OpenScadImportPoint2 {
  return [(point[0] - origin[0]) * scale, (point[1] - origin[1]) * scale]
}

const DXF_QUERY_GRID_RESOLUTION = 1 / 1024

function roundLikeCpp(value: number): number {
  if (!Number.isFinite(value)) return value
  return value < 0 ? -Math.floor(-value + 0.5) : Math.floor(value + 0.5)
}

interface DxfQueryPathLine {
  readonly points: readonly [OpenScadImportPoint2, OpenScadImportPoint2]
  disabled: boolean
}

/**
 * Rebuild DxfData paths after applying the requested layer and transform.
 * The 2021.01 reader aligns endpoints to a 1/1024 grid, extracts open paths
 * in line-index order, and only then extracts closed paths.
 */
function buildDxfQueryPaths(
  segments: readonly OpenScadDxfLineSegmentMetadata[],
  layer: string,
  origin: readonly [number, number],
  scale: number,
): readonly (readonly OpenScadImportPoint2[])[] {
  const grid = new Map<string, number[]>()
  const keyOf = (x: number, y: number): string => `${x},${y}`
  const gridKeyForPoint = (point: OpenScadImportPoint2): string => keyOf(
    roundLikeCpp(point[0] / DXF_QUERY_GRID_RESOLUTION),
    roundLikeCpp(point[1] / DXF_QUERY_GRID_RESOLUTION),
  )
  const align = (point: OpenScadImportPoint2): OpenScadImportPoint2 => {
    const transformed = transformDxfQueryPoint(point, origin, scale)
    let ix = roundLikeCpp(transformed[0] / DXF_QUERY_GRID_RESOLUTION)
    let iy = roundLikeCpp(transformed[1] / DXF_QUERY_GRID_RESOLUTION)
    if (Number.isFinite(ix) && Number.isFinite(iy) && !grid.has(keyOf(ix, iy))) {
      let distance = 10
      // Deliberately keep ix/iy mutable in the loop: Grid2d::align() in
      // OpenSCAD 2021.01 updates them as it chooses the nearest occupied cell.
      for (let jx = ix - 1; jx <= ix + 1; jx++) {
        for (let jy = iy - 1; jy <= iy + 1; jy++) {
          if (!grid.has(keyOf(jx, jy))) continue
          const candidateDistance = Math.abs(ix - jx) + Math.abs(iy - jy)
          if (candidateDistance < distance) {
            distance = candidateDistance
            ix = jx
            iy = jy
          }
        }
      }
    }
    // Converting a non-finite double to int64_t is implementation-defined in
    // the original C++; retaining the value gives this adapter deterministic
    // warning-free behavior for the same otherwise-degenerate query.
    const aligned: OpenScadImportPoint2 = Number.isFinite(ix) && Number.isFinite(iy)
      ? [ix * DXF_QUERY_GRID_RESOLUTION, iy * DXF_QUERY_GRID_RESOLUTION]
      : transformed
    const key = keyOf(ix, iy)
    if (!grid.has(key)) grid.set(key, [])
    return aligned
  }

  const lines: DxfQueryPathLine[] = []
  for (const segment of segments) {
    if (layer !== '' && segment.layer !== layer) continue
    const start = align(segment.start)
    const end = align(segment.end)
    const index = lines.length
    grid.get(gridKeyForPoint(start))!.push(index)
    grid.get(gridKeyForPoint(end))!.push(index)
    lines.push({ points: [start, end], disabled: false })
  }

  const enabled = new Set(lines.map((_, index) => index))
  const paths: OpenScadImportPoint2[][] = []
  const follow = (initialLine: number, initialPoint: 0 | 1): OpenScadImportPoint2[] => {
    let currentLine = initialLine
    let currentPoint = initialPoint
    const path: OpenScadImportPoint2[] = [lines[currentLine].points[currentPoint]]
    while (true) {
      const otherPoint = currentPoint === 0 ? 1 : 0
      const reference = lines[currentLine].points[otherPoint]
      path.push(reference)
      lines[currentLine].disabled = true
      enabled.delete(currentLine)
      let nextLine: number | undefined
      let nextPoint: 0 | 1 = 0
      for (const candidate of grid.get(gridKeyForPoint(reference)) ?? []) {
        if (lines[candidate].disabled) continue
        if (gridKeyForPoint(lines[candidate].points[0]) === gridKeyForPoint(reference)) {
          nextLine = candidate
          nextPoint = 0
          break
        }
        if (gridKeyForPoint(lines[candidate].points[1]) === gridKeyForPoint(reference)) {
          nextLine = candidate
          nextPoint = 1
          break
        }
      }
      if (nextLine === undefined) break
      currentLine = nextLine
      currentPoint = nextPoint
    }
    return path
  }

  while (enabled.size > 0) {
    let initial: readonly [number, 0 | 1] | undefined
    for (const lineIndex of enabled) {
      for (const endpoint of [0, 1] as const) {
        const point = lines[lineIndex].points[endpoint]
        const hasNeighbor = (grid.get(gridKeyForPoint(point)) ?? []).some(candidate => (
          candidate !== lineIndex && !lines[candidate].disabled
        ))
        if (!hasNeighbor) {
          initial = [lineIndex, endpoint]
          break
        }
      }
      if (initial !== undefined) break
    }
    if (initial === undefined) break
    paths.push(follow(initial[0], initial[1]))
  }

  while (enabled.size > 0) {
    const first = enabled.values().next().value as number
    paths.push(follow(first, 0))
  }
  return paths
}

/** Evaluate OpenSCAD 2021.01 dxf_cross() against a prepared, bounded project asset. */
export function queryPreparedOpenScadDxfCross(
  project: OpenScadProject,
  prepared: PreparedOpenScadImportAssets,
  sourcePath: string,
  specifier: string,
  options: OpenScadDxfQueryOptions = {},
): OpenScadDxfQueryResult<readonly [number, number]> {
  const transform = resolveDxfQueryTransform(options, sourcePath, specifier)
  const diagnostics = [...transform.diagnostics]
  const resolved = resolvePreparedDxfQueryMetadata(project, prepared, sourcePath, specifier)
  if ('severity' in resolved) {
    return dxfQueryResult<readonly [number, number]>(undefined, [...diagnostics, resolved])
  }
  const paths = buildDxfQueryPaths(
    resolved.metadata.lineSegments,
    transform.layer,
    transform.origin,
    transform.scale,
  ).filter(path => path.length === 2)
  if (paths.length >= 2) {
    const [firstStart, firstEnd] = paths[0]
    const [secondStart, secondEnd] = paths[1]
    const denominator = (secondEnd[1] - secondStart[1]) * (firstEnd[0] - firstStart[0])
      - (secondEnd[0] - secondStart[0]) * (firstEnd[1] - firstStart[1])
    if (denominator !== 0) {
      const amount = ((secondEnd[0] - secondStart[0]) * (firstStart[1] - secondStart[1])
        - (secondEnd[1] - secondStart[1]) * (firstStart[0] - secondStart[0])) / denominator
      return dxfQueryResult(Object.freeze([
        firstStart[0] + amount * (firstEnd[0] - firstStart[0]),
        firstStart[1] + amount * (firstEnd[1] - firstStart[1]),
      ] as const), diagnostics)
    }
  }
  diagnostics.push(dxfQueryDiagnostic(
    'OPENSCAD_DXF_CROSS_NOT_FOUND',
    `The first two eligible DXF paths in ${resolved.assetPath} do not define a cross.`,
    sourcePath,
    specifier,
    { assetPath: resolved.assetPath, layer: transform.layer },
  ))
  return dxfQueryResult<readonly [number, number]>(undefined, diagnostics)
}

/** Convenience import for callers that do not retain a prepared project cache. */
export async function loadOpenScadImport(
  project: OpenScadProject,
  sourcePath: string,
  specifier: string,
  options: OpenScadImportLoadOptions = {},
): Promise<OpenScadImportGeometry> {
  let forcedAssets: readonly OpenScadImportForcedAsset[] | undefined
  if (options.forcedFormat !== undefined) {
    try {
      forcedAssets = [{
        path: resolveOpenScadProjectPath(sourcePath, specifier),
        format: options.forcedFormat,
      }]
    } catch {
      // loadPreparedOpenScadImport owns the contextual path diagnostic.
    }
  }
  const prepared = await prepareOpenScadImportAssets(
    project,
    forcedAssets === undefined ? {} : { forcedAssets },
  )
  return loadPreparedOpenScadImport(project, prepared, sourcePath, specifier, options)
}
