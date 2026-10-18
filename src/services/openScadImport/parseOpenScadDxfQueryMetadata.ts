import { IDENTITY_2, OPENSCAD_IMPORT_MAX_DXF_QUERY_RECORDS, OPENSCAD_IMPORT_MAX_PROJECT_2D_POINTS, OPENSCAD_IMPORT_MAX_PROJECT_DXF_QUERY_RECORDS, OPENSCAD_IMPORT_MAX_PROJECT_TRIANGLES, OPENSCAD_IMPORT_MAX_PROJECT_VERTICES, OPENSCAD_IMPORT_MAX_XML_ELEMENTS, OpenScadImportDataError } from '../openScadImport'
import type { DxfBlock, DxfEntity, DxfSegment, Matrix2, OpenScad2021LegacyImportFormat, OpenScadDxfDimensionMetadata, OpenScadDxfIntermediate, OpenScadDxfLineSegmentMetadata, OpenScadDxfQueryMetadata, OpenScadImportFormat, OpenScadImportGeometry, OpenScadImportGeometry2D, OpenScadImportPoint2, OpenScadImportPreparationBudget, OpenScadImportRegion2D, PreparedImportDecode } from '../openScadImport'
import { OpenScadProject } from '../openScadProject'
import type { OpenScadProjectFile } from '../openScadProject'
import type { SvgOptions } from '../svgDocument'
import { arcPoints, dxfInteger, dxfNumber, dxfPairs, dxfPolyline, dxfSections, dxfValue, geometry2D, multiply2, parseOpenScad3mfWithBudget, parseOpenScadSvg, stitchDxfSegments, transform2 } from './parseOpenScad3mf'
import { boundedCount, dataError, finiteNumber, parseOpenScadOff, parseOpenScadStl, textOf } from './parseOpenScadStl'
import { parseOpenScadAmfWithBudget } from './parseOpenScadAmf'

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

export function parseOpenScadDxfIntermediate(file: OpenScadProjectFile): OpenScadDxfIntermediate {
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

export async function parseOpenScadImportFile(
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

export function reservePreparedGeometry(
  budget: OpenScadImportPreparationBudget,
  geometry: OpenScadImportGeometry,
): void {
  if (geometry.dimension === 2) {
    const points = budget.points + geometry.pointCount
    if (!Number.isSafeInteger(points) || points > OPENSCAD_IMPORT_MAX_PROJECT_2D_POINTS) {
      return dataError(
        'E_IMPORT_LIMIT',
        `Prepared project 2D geometry exceeds ${OPENSCAD_IMPORT_MAX_PROJECT_2D_POINTS.toLocaleString('en-US')} points.`,
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
      `Prepared project 3D geometry exceeds ${OPENSCAD_IMPORT_MAX_PROJECT_VERTICES.toLocaleString('en-US')} vertices.`,
      geometry.format,
      OPENSCAD_IMPORT_MAX_PROJECT_VERTICES,
      vertices,
    )
  }
  if (!Number.isSafeInteger(triangles) || triangles > OPENSCAD_IMPORT_MAX_PROJECT_TRIANGLES) {
    return dataError(
      'E_IMPORT_LIMIT',
      `Prepared project 3D geometry exceeds ${OPENSCAD_IMPORT_MAX_PROJECT_TRIANGLES.toLocaleString('en-US')} triangles.`,
      geometry.format,
      OPENSCAD_IMPORT_MAX_PROJECT_TRIANGLES,
      triangles,
    )
  }
  budget.vertices = vertices
  budget.triangles = triangles
}

export function reservePreparedDxfQueryMetadata(
  budget: OpenScadImportPreparationBudget,
  metadata: OpenScadDxfQueryMetadata,
): void {
  const records = budget.dxfQueryRecords + metadata.recordCount
  if (!Number.isSafeInteger(records) || records > OPENSCAD_IMPORT_MAX_PROJECT_DXF_QUERY_RECORDS) {
    return dataError(
      'E_IMPORT_LIMIT',
      `Prepared project DXF query metadata exceeds ${OPENSCAD_IMPORT_MAX_PROJECT_DXF_QUERY_RECORDS.toLocaleString('en-US')} records.`,
      'dxf',
      OPENSCAD_IMPORT_MAX_PROJECT_DXF_QUERY_RECORDS,
      records,
    )
  }
  budget.dxfQueryRecords = records
}

export async function prepareOpenScadImportAsset(
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

export function forcedImportAssetKey(path: string, format: OpenScad2021LegacyImportFormat): string {
  return `${format}\0${path}`
}

/** SVG text uses only bundled or explicitly supplied project fonts. */
export function svgProjectOptions(project: OpenScadProject): SvgOptions {
  const fonts: Uint8Array[] = []
  for (const summary of project.list()) {
    if (summary.kind !== 'blob' || !/\.(ttf|otf|ttc)$/i.test(summary.path)) continue
    const file = project.read(summary.path)
    if (file?.kind === 'blob') fonts.push(file.data)
  }
  return { fonts }
}
