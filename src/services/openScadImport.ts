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
export { parseOpenScadStl, parseOpenScadOff } from './openScadImport/parseOpenScadStl'
export { parseOpenScadAmf } from './openScadImport/parseOpenScadAmf'
export { parseOpenScad3mf, parseOpenScadSvg } from './openScadImport/parseOpenScad3mf'
export { parseOpenScadDxfQueryMetadata, parseOpenScadDxf } from './openScadImport/parseOpenScadDxfQueryMetadata'
export { prepareOpenScadImportAssets, loadPreparedOpenScadImport } from './openScadImport/prepareOpenScadImportAssets'
export { queryPreparedOpenScadDxfDimension, queryPreparedOpenScadDxfCross, loadOpenScadImport } from './openScadImport/queryPreparedOpenScadDxfDimension'

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

export interface XmlElement {
  readonly name: string
  readonly localName: string
  readonly namespaceUri: string | null
  readonly attributes: Readonly<Record<string, string>>
  readonly attributeNamespaces: Readonly<Record<string, string | null>>
  readonly children: readonly XmlElement[]
  readonly text: string
}

export interface MutableXmlElement {
  name: string
  localName: string
  namespaceUri: string | null
  attributes: Record<string, string>
  attributeNamespaces: Record<string, string | null>
  namespaces: Record<string, string>
  children: MutableXmlElement[]
  text: string
}

export type Matrix4 = readonly [
  number, number, number, number,
  number, number, number, number,
  number, number, number, number,
  number, number, number, number,
]

export const IDENTITY_4: Matrix4 = [
  1, 0, 0, 0,
  0, 1, 0, 0,
  0, 0, 1, 0,
  0, 0, 0, 1,
]

export interface RawMesh3D {
  readonly vertices: readonly number[]
  readonly triangles: readonly number[]
}

interface AmfInstance {
  readonly id: string
  readonly transform: Matrix4
}

export type AmfNode =
  | { readonly kind: 'object'; readonly mesh: RawMesh3D }
  | { readonly kind: 'constellation'; readonly instances: readonly AmfInstance[] }

export const CRC_TABLE = new Uint32Array(256)
for (let index = 0; index < CRC_TABLE.length; index++) {
  let value = index
  for (let bit = 0; bit < 8; bit++) value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1
  CRC_TABLE[index] = value >>> 0
}

export interface ZipCentralEntry {
  readonly path: string
  readonly directory: boolean
  readonly flags: number
  readonly method: number
  readonly crc: number
  readonly compressedSize: number
  readonly uncompressedSize: number
  readonly localOffset: number
}

export interface ZipDirectoryInfo {
  readonly endOffset: number
  readonly directoryEnd: number
  readonly entryCount: number
  readonly directorySize: number
  readonly directoryOffset: number
}

export interface Zip64EntryValues {
  readonly uncompressedSize?: number
  readonly compressedSize?: number
  readonly localOffset?: number
  readonly diskStart?: number
}

export const THREE_MF_CORE_NAMESPACE = 'http://schemas.microsoft.com/3dmanufacturing/core/2015/02'
export const OPC_RELATIONSHIPS_NAMESPACE = 'http://schemas.openxmlformats.org/package/2006/relationships'
export const THREE_MF_START_PART_RELATIONSHIP = 'http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel'

export type ThreeMfObject =
  | { readonly kind: 'mesh'; readonly mesh: RawMesh3D }
  | { readonly kind: 'components'; readonly components: readonly { readonly objectId: string; readonly transform: Matrix4 }[] }

export type Matrix2 = readonly [number, number, number, number, number, number]
export const IDENTITY_2: Matrix2 = [1, 0, 0, 1, 0, 0]

export interface DxfPair { readonly code: number; readonly value: string }
export interface DxfEntity { readonly type: string; readonly pairs: readonly DxfPair[] }

export interface DxfSegment {
  readonly layer: string
  readonly points: readonly OpenScadImportPoint2[]
}

export interface DxfBlock { readonly base: OpenScadImportPoint2; readonly entities: readonly DxfEntity[] }

export interface OpenScadDxfIntermediate {
  readonly regions: readonly OpenScadImportRegion2D[]
  readonly queryMetadata: OpenScadDxfQueryMetadata
}

export interface PreparedImportDecode {
  readonly asset: OpenScadImportGeometry | OpenScadImportDataError
  readonly dxfQueryMetadata?: OpenScadDxfQueryMetadata | OpenScadImportDataError
}

export interface ResolvedDxfQueryMetadata {
  readonly metadata: OpenScadDxfQueryMetadata
  readonly assetPath: string
}

export interface ResolvedDxfQueryTransform {
  readonly layer: string
  readonly origin: readonly [number, number]
  readonly scale: number
  readonly diagnostics: readonly OpenScadDxfQueryDiagnostic[]
}

export const DXF_QUERY_GRID_RESOLUTION = 1 / 1024

export interface DxfQueryPathLine {
  readonly keys: readonly [string,string]
  readonly points: readonly [OpenScadImportPoint2, OpenScadImportPoint2]
  disabled: boolean
}
