import { DXF_QUERY_GRID_RESOLUTION } from '../openScadImport'
import type { DxfQueryPathLine, OpenScadDxfDimensionQueryOptions, OpenScadDxfLineSegmentMetadata, OpenScadDxfQueryOptions, OpenScadDxfQueryResult, OpenScadImportForcedAsset, OpenScadImportGeometry, OpenScadImportLoadOptions, OpenScadImportPoint2, PreparedOpenScadImportAssets } from '../openScadImport'
import { OpenScadProject, resolveOpenScadProjectPath } from '../openScadProject'
import { dxfQueryDiagnostic, dxfQueryResult, loadPreparedOpenScadImport, prepareOpenScadImportAssets, resolveDxfQueryTransform, resolvePreparedDxfQueryMetadata, transformDxfDimensionCoordinates } from './prepareOpenScadImportAssets'

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

export function transformDxfQueryPoint(
  point: OpenScadImportPoint2,
  origin: readonly [number, number],
  scale: number,
): OpenScadImportPoint2 {
  return [(point[0] - origin[0]) * scale, (point[1] - origin[1]) * scale]
}

export function roundLikeCpp(value: number): number {
  if (!Number.isFinite(value)) return value
  return value < 0 ? -Math.floor(-value + 0.5) : Math.floor(value + 0.5)
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
