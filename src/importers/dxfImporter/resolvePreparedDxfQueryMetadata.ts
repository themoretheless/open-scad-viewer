// dxfImporter: resolvePreparedDxfQueryMetadata.
import {
  boundedCount,
  dataError,
  finiteNumber,
  OPENSCAD_IMPORT_MAX_DXF_QUERY_RECORDS,
  OPENSCAD_IMPORT_MAX_XML_ELEMENTS,
  type OpenScadDxfDimensionMetadata,
  type OpenScadDxfLineSegmentMetadata,
  type OpenScadDxfQueryDiagnostic,
  type OpenScadDxfQueryDiagnosticCode,
  type OpenScadDxfQueryMetadata,
  type OpenScadDxfQueryOptions,
  type OpenScadDxfQueryResult,
  type OpenScadImportDataError,
  type OpenScadImportPoint2,
  type OpenScadImportRegion2D,
  type PreparedOpenScadImportAssets,
} from '.././types'

import {
  resolveOpenScadProjectPath,
  type OpenScadProject,
} from '../../services/openScadProject'
import { ResolvedDxfQueryMetadata, dxfQueryDiagnostic } from './Matrix2'


export function resolvePreparedDxfQueryMetadata(
  project: OpenScadProject,
  prepared: PreparedOpenScadImportAssets,
  sourcePath: string,
  specifier: string,
  isImportDataError: (value: unknown) => value is OpenScadImportDataError,
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
  if (isImportDataError(metadata)) {
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


export interface ResolvedDxfQueryTransform {
  readonly layer: string
  readonly origin: readonly [number, number]
  readonly scale: number
  readonly diagnostics: readonly OpenScadDxfQueryDiagnostic[]
}


export function resolveDxfQueryTransform(
  options: OpenScadDxfQueryOptions,
  sourcePath: string,
  specifier: string,
): ResolvedDxfQueryTransform {
  const diagnostics: OpenScadDxfQueryDiagnostic[] = []
  let origin: readonly [number, number] = [0, 0]
  if (options.origin !== undefined) {
    const supplied: unknown = options.origin
    if (Array.isArray(supplied) && supplied.length === 2
      && typeof supplied[0] === 'number' && typeof supplied[1] === 'number') {
      origin = [supplied[0], supplied[1]]
      if (!Number.isFinite(origin[0]) || !Number.isFinite(origin[1])) {
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


export function transformDxfDimensionCoordinates(
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


export function transformDxfQueryPoint(
  point: OpenScadImportPoint2,
  origin: readonly [number, number],
  scale: number,
): OpenScadImportPoint2 {
  return [(point[0] - origin[0]) * scale, (point[1] - origin[1]) * scale]
}


export const DXF_QUERY_GRID_RESOLUTION = 1 / 1024


export function roundLikeCpp(value: number): number {
  if (!Number.isFinite(value)) return value
  return value < 0 ? -Math.floor(-value + 0.5) : Math.floor(value + 0.5)
}


export interface DxfQueryPathLine {
  readonly points: readonly [OpenScadImportPoint2, OpenScadImportPoint2]
  readonly keys: readonly [string, string]
  disabled: boolean
}


/**
 * Rebuild DxfData paths after applying the requested layer and transform.
 * The 2021.01 reader aligns endpoints to a 1/1024 grid, extracts open paths
 * in line-index order, and only then extracts closed paths.
 */
export function buildDxfQueryPaths(
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
    const startKey = gridKeyForPoint(start)
    const endKey = gridKeyForPoint(end)
    const index = lines.length
    grid.get(startKey)!.push(index)
    grid.get(endKey)!.push(index)
    lines.push({ points: [start, end], keys: [startKey, endKey], disabled: false })
  }

  const enabled = new Set<number>()
  for (let i = 0; i < lines.length; i++) enabled.add(i)
  const paths: OpenScadImportPoint2[][] = []
  const follow = (initialLine: number, initialPoint: 0 | 1): OpenScadImportPoint2[] => {
    let currentLine = initialLine
    let currentPoint = initialPoint
    const path: OpenScadImportPoint2[] = [lines[currentLine].points[currentPoint]]
    while (true) {
      const otherPoint = currentPoint === 0 ? 1 : 0
      const line = lines[currentLine]
      const reference = line.points[otherPoint]
      const referenceKey = line.keys[otherPoint]
      path.push(reference)
      line.disabled = true
      enabled.delete(currentLine)
      let nextLine: number | undefined
      let nextPoint: 0 | 1 = 0
      for (const candidate of grid.get(referenceKey) ?? []) {
        const candLine = lines[candidate]
        if (candLine.disabled) continue
        if (candLine.keys[0] === referenceKey) {
          nextLine = candidate
          nextPoint = 0
          break
        }
        if (candLine.keys[1] === referenceKey) {
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
      const lineKeys = lines[lineIndex].keys
      for (const endpoint of [0, 1] as const) {
        const cell = grid.get(lineKeys[endpoint])
        let hasNeighbor = false
        if (cell) {
          for (let i = 0; i < cell.length; i++) {
            const candidate = cell[i]
            if (candidate !== lineIndex && !lines[candidate].disabled) {
              hasNeighbor = true
              break
            }
          }
        }
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
