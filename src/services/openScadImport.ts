import { readSvgDocument, type SvgOptions } from './svgDocument'
import {
  normalizeOpenScadProjectPath,
  OPENSCAD_PROJECT_MAX_FILES,
  OpenScadProjectError,
  resolveOpenScadProjectPath,
  type OpenScadProject,
  type OpenScadProjectFile,
} from './openScadProject'
import {
  boundedCount,
  cloneGeometry,
  dataError,
  formatOfPath,
  geometry2D,
  geometry3D,
  LEGACY_FORMAT_SET,
  OPENSCAD_2021_LEGACY_IMPORT_FORMATS,
  OPENSCAD_DXF_QUERY_DIAGNOSTIC_CODES,
  OPENSCAD_IMPORT_CAPABILITIES,
  OPENSCAD_IMPORT_DEFAULT_CURVE_SEGMENTS,
  OPENSCAD_IMPORT_ERROR_CODES,
  OPENSCAD_IMPORT_FORMATS,
  OPENSCAD_IMPORT_MAX_2D_POINTS,
  OPENSCAD_IMPORT_MAX_CONTOURS,
  OPENSCAD_IMPORT_MAX_DXF_QUERY_RECORDS,
  OPENSCAD_IMPORT_MAX_PROJECT_2D_POINTS,
  OPENSCAD_IMPORT_MAX_PROJECT_DXF_QUERY_RECORDS,
  OPENSCAD_IMPORT_MAX_PROJECT_TRIANGLES,
  OPENSCAD_IMPORT_MAX_PROJECT_VERTICES,
  OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES,
  OPENSCAD_IMPORT_MAX_TRIANGLES,
  OPENSCAD_IMPORT_MAX_VERTICES,
  OPENSCAD_IMPORT_MAX_XML_DEPTH,
  OPENSCAD_IMPORT_MAX_XML_ELEMENTS,
  OPENSCAD_IMPORT_MAX_ZIP_ENTRIES,
  OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES,
  OpenScadImportDataError,
  OpenScadImportError,
  OpenScadImportPositionedError,
  textOf,
  type OpenScad2021LegacyImportFormat,
  type OpenScadDxfDimensionMetadata,
  type OpenScadDxfDimensionQueryOptions,
  type OpenScadDxfLineSegmentMetadata,
  type OpenScadDxfQueryDiagnostic,
  type OpenScadDxfQueryDiagnosticCode,
  type OpenScadDxfQueryMetadata,
  type OpenScadDxfQueryOptions,
  type OpenScadDxfQueryResult,
  type OpenScadImportErrorCode,
  type OpenScadImportErrorDetails,
  type OpenScadImportFillRule,
  type OpenScadImportForcedAsset,
  type OpenScadImportFormat,
  type OpenScadImportGeometry,
  type OpenScadImportGeometry2D,
  type OpenScadImportGeometry3D,
  type OpenScadImportLoadOptions,
  type OpenScadImportPoint2,
  type OpenScadImportPreparationBudget,
  type OpenScadImportPreparationOptions,
  type OpenScadImportRegion2D,
  type PreparedOpenScadImportAssets,
} from '../importers/types'
import {
  parseOpenScadOff,
  parseOpenScadStl,
} from '../importers/stlOffImporter'
import {
  parseOpenScad3mf,
  parseOpenScad3mfWithBudget,
  parseOpenScadAmfWithBudget,
  type Matrix4,
} from '../importers/xmlAndArchive'
import {
  buildDxfQueryPaths,
  dxfDimensionMetadata,
  dxfInteger,
  dxfLwPolyline,
  dxfNumber,
  dxfPairs,
  dxfPolyline,
  dxfQueryDiagnostic,
  dxfQueryResult,
  dxfSections,
  dxfValue,
  entityLayer,
  IDENTITY_2,
  multiply2,
  resolveDxfQueryTransform,
  resolvePreparedDxfQueryMetadata,
  stitchDxfSegments,
  transform2,
  transformDxfDimensionCoordinates,
  type DxfBlock,
  type DxfEntity,
  type DxfSegment,
  type Matrix2,
  type OpenScadDxfIntermediate,
} from '../importers/dxfImporter'

export {
  OPENSCAD_2021_LEGACY_IMPORT_FORMATS,
  OPENSCAD_DXF_QUERY_DIAGNOSTIC_CODES,
  OPENSCAD_IMPORT_CAPABILITIES,
  OPENSCAD_IMPORT_DEFAULT_CURVE_SEGMENTS,
  OPENSCAD_IMPORT_ERROR_CODES,
  OPENSCAD_IMPORT_FORMATS,
  OPENSCAD_IMPORT_MAX_2D_POINTS,
  OPENSCAD_IMPORT_MAX_CONTOURS,
  OPENSCAD_IMPORT_MAX_DXF_QUERY_RECORDS,
  OPENSCAD_IMPORT_MAX_PROJECT_2D_POINTS,
  OPENSCAD_IMPORT_MAX_PROJECT_DXF_QUERY_RECORDS,
  OPENSCAD_IMPORT_MAX_PROJECT_TRIANGLES,
  OPENSCAD_IMPORT_MAX_PROJECT_VERTICES,
  OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES,
  OPENSCAD_IMPORT_MAX_TRIANGLES,
  OPENSCAD_IMPORT_MAX_VERTICES,
  OPENSCAD_IMPORT_MAX_XML_DEPTH,
  OPENSCAD_IMPORT_MAX_XML_ELEMENTS,
  OPENSCAD_IMPORT_MAX_ZIP_ENTRIES,
  OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES,
  OpenScadImportDataError,
  OpenScadImportError,
  OpenScadImportPositionedError,
  parseOpenScad3mf,
  parseOpenScadOff,
  parseOpenScadStl,
  type OpenScad2021LegacyImportFormat,
  type OpenScadDxfDimensionMetadata,
  type OpenScadDxfDimensionQueryOptions,
  type OpenScadDxfLineSegmentMetadata,
  type OpenScadDxfQueryDiagnostic,
  type OpenScadDxfQueryDiagnosticCode,
  type OpenScadDxfQueryMetadata,
  type OpenScadDxfQueryOptions,
  type OpenScadDxfQueryResult,
  type OpenScadImportErrorCode,
  type OpenScadImportErrorDetails,
  type OpenScadImportFillRule,
  type OpenScadImportForcedAsset,
  type OpenScadImportFormat,
  type OpenScadImportGeometry,
  type OpenScadImportGeometry2D,
  type OpenScadImportGeometry3D,
  type OpenScadImportLoadOptions,
  type OpenScadImportPoint2,
  type OpenScadImportPreparationOptions,
  type OpenScadImportRegion2D,
  type PreparedOpenScadImportAssets,
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

export async function parseOpenScadAmf(file: OpenScadProjectFile): Promise<OpenScadImportGeometry3D> {
  return parseOpenScadAmfWithBudget(file, rotation4)
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
    case 'amf': return parseOpenScadAmfWithBudget(file, rotation4, projectBudget)
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
  if (!Array.isArray(origin) || origin.length !== 2 || !Number.isFinite(origin[0]) || !Number.isFinite(origin[1])) {
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
    if (options.center) {
      const centerX = (minX + maxX) / 2
      const centerY = (minY + maxY) / 2
      return geometry2D(source.format, regions.map(region => ({
        ...region,
        contours: region.contours.map(contour => contour.map(point => [point[0] - centerX, point[1] - centerY] as const)),
      })))
    }
    return geometry2D(source.format, regions)
  }
  if (!options.center) {
    return cloneGeometry(source)
  }
  const srcVertices = source.vertices
  let minX = Infinity, maxX = -Infinity, minY = Infinity, maxY = -Infinity, minZ = Infinity, maxZ = -Infinity
  for (let offset = 0; offset < srcVertices.length; offset += 3) {
    const x = srcVertices[offset], y = srcVertices[offset + 1], z = srcVertices[offset + 2]
    if (x < minX) minX = x; if (x > maxX) maxX = x
    if (y < minY) minY = y; if (y > maxY) maxY = y
    if (z < minZ) minZ = z; if (z > maxZ) maxZ = z
  }
  const centerX = (minX + maxX) / 2, centerY = (minY + maxY) / 2, centerZ = (minZ + maxZ) / 2
  const vertices = new Float64Array(srcVertices.length)
  for (let offset = 0; offset < srcVertices.length; offset += 3) {
    vertices[offset] = srcVertices[offset] - centerX
    vertices[offset + 1] = srcVertices[offset + 1] - centerY
    vertices[offset + 2] = srcVertices[offset + 2] - centerZ
  }
  return geometry3D(source.format, vertices, source.triangles)
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

const isImportDataError = (value: unknown): value is OpenScadImportDataError => (
  value instanceof OpenScadImportDataError
)

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
  const resolved = resolvePreparedDxfQueryMetadata(project, prepared, sourcePath, specifier, isImportDataError)
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
  const resolved = resolvePreparedDxfQueryMetadata(project, prepared, sourcePath, specifier, isImportDataError)
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
