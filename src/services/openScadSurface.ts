import type { CallNode } from './openscadCompiler'
import { OpenSCADParseError } from './openscadErrors'
import {
  OpenScadProject,
  resolveOpenScadProjectPath,
  type OpenScadProjectFile,
} from './openScadProject'
import { u32 } from '../core/bytes'
export { parseOpenScadSurfaceDat } from './openScadSurface/parseOpenScadSurfaceDat'
export { decodeOpenScadSurfacePng, prepareOpenScadSurfaceAssets, loadOpenScadSurfaceHeightMap, triangulateOpenScadSurface } from './openScadSurface/decodeOpenScadSurfacePng'

export const OPENSCAD_SURFACE_MAX_DIMENSION = 4_096
export const OPENSCAD_SURFACE_MAX_PIXELS = 250_000
export const OPENSCAD_SURFACE_MAX_PREPARED_PIXELS = 1_000_000
export const OPENSCAD_SURFACE_MAX_TRIANGLES = 750_000
export const PNG_SIGNATURE = Object.freeze([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a])

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

export interface DecodedPngLuminance {
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

export const UTF8_FATAL = new TextDecoder('utf-8', { fatal: true })
export const NUMBER_TOKEN = /^[+-]?(?:(?:\d+(?:\.\d*)?)|(?:\.\d+))(?:[eE][+-]?\d+)?$/u

export const CRC_TABLE = (() => {
  const table = new Uint32Array(256)
  for (let index = 0; index < table.length; index++) {
    let value = index
    for (let bit = 0; bit < 8; bit++) value = (value & 1) === 0 ? value >>> 1 : 0xedb88320 ^ (value >>> 1)
    table[index] = value >>> 0
  }
  return table
})()

export const ADAM7 = Object.freeze([
  Object.freeze([0, 0, 8, 8]),
  Object.freeze([4, 0, 8, 8]),
  Object.freeze([0, 4, 4, 8]),
  Object.freeze([2, 0, 4, 4]),
  Object.freeze([0, 2, 2, 4]),
  Object.freeze([1, 0, 2, 2]),
  Object.freeze([0, 1, 1, 2]),
] as const)
