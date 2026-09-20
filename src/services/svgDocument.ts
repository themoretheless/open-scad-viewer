import { callGeometryRust, warmGeometryKernel } from './geometry/kernel'

/** Shared browser/worker/Node SVG boundary. All document interpretation stays in Rust. */
import { SVG_MAX_BYTES, SVG_MAX_FONT_BYTES, SVG_MAX_TOTAL_FONT_BYTES, SVG_MAX_FONTS } from './svgLimits'
export { SVG_MAX_BYTES, SVG_MAX_FONT_BYTES, SVG_MAX_TOTAL_FONT_BYTES, SVG_MAX_FONTS } from './svgLimits'

export interface SvgOptions {
  /** CSS pixels per inch. Standalone documents use 96; OpenSCAD supplies its own DPI. */
  dpi?: number
  /** Maximum curve flattening error in millimeters. */
  tolerance?: number
  fonts?: readonly Uint8Array[]
  geometryMode?: 'vector' | 'silhouette'
  /** Longest raster edge, used only for the explicitly selected silhouette mode. */
  rasterSize?: number
  alphaThreshold?: number
}

export interface SvgDocumentResult {
  regions: { contours: [number, number][][]; fillRule: 'nonzero' | 'evenodd' }[]
  widthMm: number
  heightMm: number
  normalizedSvg: string
  warnings: string[]
}

function fontBase64(bytes: Uint8Array): string {
  let binary = ''
  for (let offset = 0; offset < bytes.length; offset += 8192) {
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 8192))
  }
  return btoa(binary)
}

function prepareSvgDocument(
  source: string,
  options: SvgOptions = {},
  action: 'parse' | 'preview' = 'parse',
  legacyDpi = false,
) {
  if (new TextEncoder().encode(source).length > SVG_MAX_BYTES) throw new Error('SVG exceeds 4 MiB.')
  const dpi = options.dpi ?? 96
  const tolerance = options.tolerance ?? 0.02
  const rasterSize = options.rasterSize ?? 512
  const alphaThreshold = options.alphaThreshold ?? 0.5
  if (!Number.isFinite(dpi) || dpi <= 0 || dpi > 100000) throw new Error('SVG DPI must be positive and at most 100000.')
  if (!Number.isFinite(tolerance) || tolerance < 0.0001 || tolerance > 10) throw new Error('SVG tolerance must be between 0.0001 and 10 mm.')
  if (!Number.isInteger(rasterSize) || rasterSize < 128 || rasterSize > 2048) throw new Error('SVG silhouette resolution must be between 128 and 2048 pixels.')
  if (!Number.isFinite(alphaThreshold) || alphaThreshold < 0.01 || alphaThreshold > 1) throw new Error('SVG alpha threshold must be between 0.01 and 1.')
  if (options.geometryMode !== undefined && options.geometryMode !== 'vector' && options.geometryMode !== 'silhouette') throw new Error('SVG geometry mode must be vector or silhouette.')
  const fonts = options.fonts ?? []
  if (fonts.length > SVG_MAX_FONTS) throw new Error('SVG supports at most 16 custom fonts.')
  if (fonts.some(font => !(font instanceof Uint8Array) || !font.length || font.length > SVG_MAX_FONT_BYTES)) throw new Error('Each SVG font must contain 1 byte to 4 MiB.')
  if (fonts.reduce((sum, font) => sum + font.length, 0) > SVG_MAX_TOTAL_FONT_BYTES) throw new Error('SVG fonts exceed 8 MiB in total.')
  return {
    action, source, dpi, tolerance, legacyDpi,
    geometryMode: options.geometryMode ?? 'vector', rasterSize, alphaThreshold,
    fonts: fonts.map(fontBase64),
  }
}

export function readSvgDocument(
  source: string,
  options: SvgOptions = {},
  action: 'parse' | 'preview' = 'parse',
  legacyDpi = false,
): SvgDocumentResult {
  return callGeometryRust<SvgDocumentResult>('svg', prepareSvgDocument(source, options, action, legacyDpi))
}

export async function readSvgDocumentAsync(
  source: string,
  options: SvgOptions = {},
  action: 'parse' | 'preview' = 'parse',
  legacyDpi = false,
): Promise<SvgDocumentResult> {
  // Validate and capture mutable options/fonts before yielding to compilation.
  const payload = prepareSvgDocument(source, options, action, legacyDpi)
  await warmGeometryKernel()
  return callGeometryRust<SvgDocumentResult>('svg', payload)
}
