import type { OpenScadProjectFile } from '../services/openScadProject'
import type { CallNode } from '../services/openscadCompiler'
import { OpenSCADParseError } from '../services/openscadErrors'

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

export interface OpenScadImportPreparationBudget {
  vertices: number
  triangles: number
  points: number
  zipExpandedBytes: number
  dxfQueryRecords: number
}

export const UTF8_FATAL = new TextDecoder('utf-8', { fatal: true })
export const UTF8 = new TextEncoder()
export const NUMBER = /^[+-]?(?:(?:\d+(?:\.\d*)?)|(?:\.\d+))(?:[eE][+-]?\d+)?$/u
export const FORMAT_SET = new Set<string>(OPENSCAD_IMPORT_FORMATS)
export const LEGACY_FORMAT_SET = new Set<string>(OPENSCAD_2021_LEGACY_IMPORT_FORMATS)

export function dataError(
  code: OpenScadImportDataError['code'],
  message: string,
  format?: OpenScadImportFormat,
  limit?: number,
  actual?: number,
): never {
  throw new OpenScadImportDataError(code, message, format, limit, actual)
}

export function finiteNumber(token: string, label: string, format: OpenScadImportFormat): number {
  if (!NUMBER.test(token)) return dataError('E_IMPORT_INVALID_DATA', `${label} is not a number.`, format)
  const value = Number(token)
  if (!Number.isFinite(value)) return dataError('E_IMPORT_INVALID_DATA', `${label} must be finite.`, format)
  return value
}

export function boundedCount(
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

export function bytesOf(file: OpenScadProjectFile): Uint8Array {
  return file.kind === 'blob' ? file.data : UTF8.encode(file.source)
}

export function textOf(file: OpenScadProjectFile, format: OpenScadImportFormat): string {
  if (file.kind === 'source') return file.source
  try {
    return UTF8_FATAL.decode(file.data)
  } catch {
    return dataError('E_IMPORT_ENCODING', `${format.toUpperCase()} input must be valid UTF-8 text.`, format)
  }
}

export function formatOfPath(path: string): OpenScadImportFormat | null {
  const dot = path.lastIndexOf('.')
  if (dot < 0) return null
  const format = path.slice(dot + 1).toLowerCase()
  return FORMAT_SET.has(format) ? format as OpenScadImportFormat : null
}

export function samePoint3(vertices: ArrayLike<number>, a: number, b: number): boolean {
  return vertices[a * 3] === vertices[b * 3]
    && vertices[a * 3 + 1] === vertices[b * 3 + 1]
    && vertices[a * 3 + 2] === vertices[b * 3 + 2]
}

export function triangleDegenerate(vertices: ArrayLike<number>, a: number, b: number, c: number): boolean {
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

export function geometry3D(
  format: OpenScadImportGeometry3D['format'],
  sourceVertices: ArrayLike<number>,
  sourceTriangles: ArrayLike<number>,
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
  const validIndex = (index: number): boolean => Number.isSafeInteger(index) && index >= 0 && index < vertexCount
  for (let offset = 0; offset < sourceTriangles.length; offset += 3) {
    const a = sourceTriangles[offset], b = sourceTriangles[offset + 1], c = sourceTriangles[offset + 2]
    if (!validIndex(a) || !validIndex(b) || !validIndex(c)) {
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
    triangles: new Uint32Array(triangles),
    triangleCount: triangles.length / 3,
  })
}

export function cloneGeometry(geometry: OpenScadImportGeometry): OpenScadImportGeometry {
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

export function cleanContour(points: readonly OpenScadImportPoint2[]): OpenScadImportPoint2[] {
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

export function geometry2D(
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
