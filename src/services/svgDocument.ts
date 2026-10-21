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
  contours?: [number, number][][]
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

export type SvgMatrix2D = [number, number, number, number, number, number]

function multiplySvgMatrices(m1: SvgMatrix2D, m2: SvgMatrix2D): SvgMatrix2D {
  return [
    m1[0] * m2[0] + m1[2] * m2[1],
    m1[1] * m2[0] + m1[3] * m2[1],
    m1[0] * m2[2] + m1[2] * m2[3],
    m1[1] * m2[2] + m1[3] * m2[3],
    m1[0] * m2[4] + m1[2] * m2[5] + m1[4],
    m1[1] * m2[4] + m1[3] * m2[5] + m1[5],
  ]
}

export function parseSvgTransformMatrix(transformAttr: string): SvgMatrix2D {
  if (typeof transformAttr !== 'string' || transformAttr.length > 4096) {
    throw new Error('Invalid or oversized SVG transform attribute.')
  }
  const trimmed = transformAttr.trim()
  if (!trimmed || trimmed === 'none') return [1, 0, 0, 1, 0, 0]
  const tokenRegex = /\s*(matrix|translate|scale|rotate|skewX|skewY)\s*\(([^)]*)\)/gy
  let cursor = 0
  let current: SvgMatrix2D = [1, 0, 0, 1, 0, 0]
  while (cursor < trimmed.length) {
    tokenRegex.lastIndex = cursor
    const match = tokenRegex.exec(trimmed)
    if (!match) throw new Error(`Unsupported or malformed SVG transform near "${trimmed.slice(cursor, cursor + 24)}".`)
    cursor = tokenRegex.lastIndex
    const kind = match[1]
    const rawArgs = match[2].trim()
    if (!rawArgs) throw new Error(`Empty SVG transform arguments in ${kind}().`)
    const args = rawArgs.split(/[\s,]+/).filter(Boolean).map(Number)
    if (!args.length || args.some(v => !Number.isFinite(v) || Math.abs(v) > 1e9)) {
      throw new Error(`Non-finite or out-of-range SVG transform arguments in ${kind}().`)
    }
    let next: SvgMatrix2D
    if (kind === 'matrix') {
      if (args.length !== 6) throw new Error('SVG matrix() requires 6 finite numbers.')
      next = [args[0], args[1], args[2], args[3], args[4], args[5]]
    } else if (kind === 'translate') {
      if (args.length !== 1 && args.length !== 2) throw new Error('SVG translate() requires 1 or 2 arguments.')
      next = [1, 0, 0, 1, args[0], args[1] ?? 0]
    } else if (kind === 'scale') {
      if (args.length !== 1 && args.length !== 2) throw new Error('SVG scale() requires 1 or 2 arguments.')
      const sx = args[0], sy = args[1] ?? sx
      next = [sx, 0, 0, sy, 0, 0]
    } else if (kind === 'rotate') {
      if (args.length !== 1 && args.length !== 3) throw new Error('SVG rotate() requires 1 or 3 arguments.')
      const rad = (args[0] * Math.PI) / 180
      const c = Math.cos(rad), s = Math.sin(rad)
      if (args.length === 1) {
        next = [c, s, -s, c, 0, 0]
      } else {
        const cx = args[1], cy = args[2]
        next = multiplySvgMatrices(
          multiplySvgMatrices([1, 0, 0, 1, cx, cy], [c, s, -s, c, 0, 0]),
          [1, 0, 0, 1, -cx, -cy],
        )
      }
    } else if (kind === 'skewX') {
      if (args.length !== 1) throw new Error('SVG skewX() requires 1 argument.')
      const t = Math.tan((args[0] * Math.PI) / 180)
      if (!Number.isFinite(t) || Math.abs(t) > 1e6) throw new Error('Degenerate SVG skewX() angle.')
      next = [1, 0, t, 1, 0, 0]
    } else {
      if (args.length !== 1) throw new Error('SVG skewY() requires 1 argument.')
      const t = Math.tan((args[0] * Math.PI) / 180)
      if (!Number.isFinite(t) || Math.abs(t) > 1e6) throw new Error('Degenerate SVG skewY() angle.')
      next = [1, t, 0, 1, 0, 0]
    }
    current = multiplySvgMatrices(current, next)
    while (cursor < trimmed.length && /[\s,]/.test(trimmed[cursor])) cursor++
  }
  const det = current[0] * current[3] - current[1] * current[2]
  if (!Number.isFinite(det) || Math.abs(det) <= 1e-12) {
    throw new Error('Degenerate singular SVG transform matrix.')
  }
  return current
}

function prepareSvgDocument(
  source: string,
  options: SvgOptions = {},
  action: 'parse' | 'preview' | 'profile' = 'parse',
  legacyDpi = false,
) {
  if (source.length > SVG_MAX_BYTES || (source.length * 3 > SVG_MAX_BYTES && new TextEncoder().encode(source).length > SVG_MAX_BYTES)) throw new Error('SVG exceeds 4 MiB.')
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
  action: 'parse' | 'preview' | 'profile' = 'parse',
  legacyDpi = false,
): SvgDocumentResult {
  return callGeometryRust<SvgDocumentResult>('svg', prepareSvgDocument(source, options, action, legacyDpi))
}

export async function readSvgDocumentAsync(
  source: string,
  options: SvgOptions = {},
  action: 'parse' | 'preview' | 'profile' = 'parse',
  legacyDpi = false,
): Promise<SvgDocumentResult> {
  // Validate and capture mutable options/fonts before yielding to compilation.
  const payload = prepareSvgDocument(source, options, action, legacyDpi)
  await warmGeometryKernel()
  return callGeometryRust<SvgDocumentResult>('svg', payload)
}
