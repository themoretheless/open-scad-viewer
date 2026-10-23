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
} from './types'
import {
  resolveOpenScadProjectPath,
  type OpenScadProject,
} from '../services/openScadProject'

export type Matrix2 = readonly [number, number, number, number, number, number]
export const IDENTITY_2: Matrix2 = [1, 0, 0, 1, 0, 0]

export function multiply2(left: Matrix2, right: Matrix2): Matrix2 {
  return [
    left[0] * right[0] + left[2] * right[1],
    left[1] * right[0] + left[3] * right[1],
    left[0] * right[2] + left[2] * right[3],
    left[1] * right[2] + left[3] * right[3],
    left[0] * right[4] + left[2] * right[5] + left[4],
    left[1] * right[4] + left[3] * right[5] + left[5],
  ]
}

export function transform2(matrix: Matrix2, point: OpenScadImportPoint2): OpenScadImportPoint2 {
  return [
    matrix[0] * point[0] + matrix[2] * point[1] + matrix[4],
    matrix[1] * point[0] + matrix[3] * point[1] + matrix[5],
  ]
}

export interface DxfPair { readonly code: number; readonly value: string }
export interface DxfEntity { readonly type: string; readonly pairs: readonly DxfPair[] }

export function dxfPairs(text: string): DxfPair[] {
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

export function dxfSections(pairs: readonly DxfPair[]): ReadonlyMap<string, readonly DxfEntity[]> {
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

export function dxfValues(entity: DxfEntity, code: number): string[] {
  return entity.pairs.filter(pair => pair.code === code).map(pair => pair.value)
}

export function dxfValue(entity: DxfEntity, code: number, fallback?: string): string {
  const value = entity.pairs.find(pair => pair.code === code)?.value ?? fallback
  if (value === undefined) return dataError('E_IMPORT_INVALID_DATA', `DXF ${entity.type} is missing group ${code}.`, 'dxf')
  return value
}

export function dxfNumber(entity: DxfEntity, code: number, fallback?: number): number {
  const value = dxfValues(entity, code)[0]
  return value === undefined && fallback !== undefined
    ? fallback
    : finiteNumber(value ?? '', `DXF ${entity.type} group ${code}`, 'dxf')
}

export function dxfInteger(entity: DxfEntity, code: number, fallback = 0): number {
  const value = dxfNumber(entity, code, fallback)
  if (!Number.isSafeInteger(value)) return dataError('E_IMPORT_INVALID_DATA', `DXF ${entity.type} group ${code} must be an integer.`, 'dxf')
  return value
}

export interface DxfSegment {
  readonly layer: string
  readonly points: readonly OpenScadImportPoint2[]
}

function pointNear(left: OpenScadImportPoint2, right: OpenScadImportPoint2): boolean {
  return Math.round(left[0] * 1024) === Math.round(right[0] * 1024)
    && Math.round(left[1] * 1024) === Math.round(right[1] * 1024)
}

export function stitchDxfSegments(segments: readonly DxfSegment[]): OpenScadImportRegion2D[] {
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

export function dxfPolyline(
  points: readonly OpenScadImportPoint2[],
  closed: boolean,
): OpenScadImportPoint2[] {
  return closed ? [...points, points[0]] : [...points]
}

export function dxfLwPolyline(entity: DxfEntity): OpenScadImportPoint2[] {
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

export function entityLayer(entity: DxfEntity, inherited = '0'): string {
  const layer = dxfValue(entity, 8, inherited)
  return layer === '0' ? inherited : layer
}

export interface DxfBlock { readonly base: OpenScadImportPoint2; readonly entities: readonly DxfEntity[] }

export interface OpenScadDxfIntermediate {
  readonly regions: readonly OpenScadImportRegion2D[]
  readonly queryMetadata: OpenScadDxfQueryMetadata
}

export function dxfDimensionMetadata(
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
      const coordinates = new Array<readonly [number, number]>(7)
      for (let index = 0; index < 7; index++) {
        coordinates[index] = Object.freeze([
          dxfNumber(entity, 10 + index, 0),
          dxfNumber(entity, 20 + index, 0),
        ] as const)
      }
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

export interface ResolvedDxfQueryMetadata {
  readonly metadata: OpenScadDxfQueryMetadata
  readonly assetPath: string
}

export function dxfQueryDiagnostic(
  code: OpenScadDxfQueryDiagnosticCode,
  message: string,
  sourcePath: string,
  specifier: string,
  details: Omit<OpenScadDxfQueryDiagnostic, 'severity' | 'code' | 'message' | 'sourcePath' | 'specifier'> = {},
): OpenScadDxfQueryDiagnostic {
  return Object.freeze({ severity: 'warning', code, message, sourcePath, specifier, ...details })
}

export function dxfQueryResult<T>(
  value: T | undefined,
  diagnostics: readonly OpenScadDxfQueryDiagnostic[],
): OpenScadDxfQueryResult<T> {
  return Object.freeze({ value, diagnostics: Object.freeze([...diagnostics]) })
}

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
