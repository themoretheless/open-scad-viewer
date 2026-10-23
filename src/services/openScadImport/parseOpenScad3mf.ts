import { OPENSCAD_IMPORT_DEFAULT_CURVE_SEGMENTS, OPENSCAD_IMPORT_MAX_2D_POINTS, OPENSCAD_IMPORT_MAX_CONTOURS, OPENSCAD_IMPORT_MAX_TRIANGLES, OPENSCAD_IMPORT_MAX_VERTICES, OPENSCAD_IMPORT_MAX_XML_DEPTH, OPENSCAD_IMPORT_MAX_XML_ELEMENTS, OpenScadImportDataError, THREE_MF_CORE_NAMESPACE, UTF8_FATAL } from '../openScadImport'
import type { DxfEntity, DxfPair, DxfSegment, Matrix2, Matrix4, OpenScadImportGeometry2D, OpenScadImportGeometry3D, OpenScadImportPoint2, OpenScadImportPreparationBudget, OpenScadImportRegion2D, ThreeMfObject, XmlElement } from '../openScadImport'
import type { OpenScadProjectFile } from '../openScadProject'
import { readSvgDocument } from '../svgDocument'
import type { SvgOptions } from '../svgDocument'
import { appendRawMesh, attributeNs, childElementNs, childElementsNs, multiply4, parseXml, unitScale } from './parseXml'
import { boundedCount, bytesOf, dataError, finiteNumber, geometry3D, textOf } from './parseOpenScadStl'
import { modelPathFromPackage, parse3mfTransform, threeMfNumberAttribute, threeMfResourceId, threeMfResourceIdAttribute, threeMfResourceIndexAttribute, unzip3mf } from './unzip3mf'

/** Parse a bounded core 3MF package, including build items and component graphs. */
export async function parseOpenScad3mfWithBudget(
  file: OpenScadProjectFile,
  projectBudget?: OpenScadImportPreparationBudget,
): Promise<OpenScadImportGeometry3D> {
  const files = await unzip3mf(bytesOf(file), projectBudget)
  const modelPath = modelPathFromPackage(files)
  const modelBytes = files.get(modelPath)!
  let root: XmlElement
  try {
    root = parseXml(UTF8_FATAL.decode(modelBytes), '3mf')
  } catch (error) {
    if (error instanceof OpenScadImportDataError) throw error
    return dataError('E_IMPORT_ENCODING', '3MF model must be UTF-8 XML.', '3mf')
  }
  if (root.localName !== 'model' || root.namespaceUri !== THREE_MF_CORE_NAMESPACE) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF model root must use the official core namespace.', '3mf')
  }
  const requiredExtensions = attributeNs(root, 'requiredextensions', null)?.trim()
  if (requiredExtensions) {
    return dataError('E_IMPORT_UNSUPPORTED_FEATURE', `3MF requires unsupported extensions: ${requiredExtensions}.`, '3mf')
  }
  const scale = unitScale(attributeNs(root, 'unit', null), '3mf')
  const resourceElements = childElementsNs(root, 'resources', THREE_MF_CORE_NAMESPACE)
  const buildElements = childElementsNs(root, 'build', THREE_MF_CORE_NAMESPACE)
  if (resourceElements.length !== 1 || buildElements.length !== 1) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF model requires exactly one core resources and build element.', '3mf')
  }
  const resources = resourceElements[0], build = buildElements[0]
  const objects = new Map<string, ThreeMfObject>()
  const resourceIds = new Set<string>()
  for (const resource of resources.children.filter(child => child.namespaceUri === THREE_MF_CORE_NAMESPACE)) {
    const rawId = attributeNs(resource, 'id', null)
    if (rawId === undefined) continue
    const id = threeMfResourceId(rawId, `3MF ${resource.localName} id`)
    if (resourceIds.has(id)) return dataError('E_IMPORT_INVALID_DATA', `3MF resource id ${id} is duplicated.`, '3mf')
    resourceIds.add(id)
  }
  for (const object of childElementsNs(resources, 'object', THREE_MF_CORE_NAMESPACE)) {
    const id = threeMfResourceIdAttribute(object, 'id')
    const meshElement = childElementNs(object, 'mesh', THREE_MF_CORE_NAMESPACE)
    const componentsElement = childElementNs(object, 'components', THREE_MF_CORE_NAMESPACE)
    if ((meshElement === null) === (componentsElement === null)) {
      return dataError('E_IMPORT_INVALID_DATA', `3MF object ${id} must contain exactly one mesh or components element.`, '3mf')
    }
    if (meshElement !== null) {
      const verticesElement = childElementNs(meshElement, 'vertices', THREE_MF_CORE_NAMESPACE)
      const trianglesElement = childElementNs(meshElement, 'triangles', THREE_MF_CORE_NAMESPACE)
      if (verticesElement === null || trianglesElement === null) {
        return dataError('E_IMPORT_INVALID_DATA', `3MF mesh ${id} requires vertices and triangles.`, '3mf')
      }
      const vertices: number[] = []
      for (const vertex of childElementsNs(verticesElement, 'vertex', THREE_MF_CORE_NAMESPACE)) {
        vertices.push(
          threeMfNumberAttribute(vertex, 'x') * scale,
          threeMfNumberAttribute(vertex, 'y') * scale,
          threeMfNumberAttribute(vertex, 'z') * scale,
        )
        boundedCount(vertices.length / 3, OPENSCAD_IMPORT_MAX_VERTICES, '3MF object vertex count', '3mf')
      }
      const triangles: number[] = []
      for (const triangle of childElementsNs(trianglesElement, 'triangle', THREE_MF_CORE_NAMESPACE)) {
        for (const name of ['v1', 'v2', 'v3'] as const) {
          const index = threeMfResourceIndexAttribute(triangle, name)
          if (index >= vertices.length / 3) {
            return dataError('E_IMPORT_INVALID_DATA', `3MF ${name} index is out of bounds.`, '3mf')
          }
          triangles.push(index)
        }
        boundedCount(triangles.length / 3, OPENSCAD_IMPORT_MAX_TRIANGLES, '3MF object triangle count', '3mf')
      }
      objects.set(id, { kind: 'mesh', mesh: Object.freeze({ vertices, triangles }) })
    } else {
      const components = childElementsNs(componentsElement!, 'component', THREE_MF_CORE_NAMESPACE).map(component => {
        return Object.freeze({
          objectId: threeMfResourceIdAttribute(component, 'objectid'),
          transform: parse3mfTransform(attributeNs(component, 'transform', null), scale),
        })
      })
      objects.set(id, { kind: 'components', components: Object.freeze(components) })
    }
  }

  const vertices: number[] = []
  const triangles: number[] = []
  let expansions = 0
  const expand = (id: string, matrix: Matrix4, stack: readonly string[]): void => {
    const object = objects.get(id)
    if (!object) return dataError('E_IMPORT_INVALID_DATA', `3MF refers to missing object ${id}.`, '3mf')
    if (stack.includes(id)) return dataError('E_IMPORT_INVALID_DATA', `3MF component cycle includes object ${id}.`, '3mf')
    expansions++
    if (expansions > OPENSCAD_IMPORT_MAX_XML_ELEMENTS) {
      return dataError('E_IMPORT_LIMIT', '3MF component expansion limit exceeded.', '3mf', OPENSCAD_IMPORT_MAX_XML_ELEMENTS, expansions)
    }
    if (object.kind === 'mesh') {
      appendRawMesh(vertices, triangles, object.mesh, matrix, '3mf')
      return
    }
    if (stack.length >= OPENSCAD_IMPORT_MAX_XML_DEPTH) {
      return dataError('E_IMPORT_LIMIT', '3MF component nesting limit exceeded.', '3mf', OPENSCAD_IMPORT_MAX_XML_DEPTH, stack.length + 1)
    }
    for (const component of object.components) {
      expand(component.objectId, multiply4(matrix, component.transform), [...stack, id])
    }
  }
  const items = childElementsNs(build, 'item', THREE_MF_CORE_NAMESPACE)
  if (items.length === 0) return dataError('E_IMPORT_EMPTY', '3MF build contains no items.', '3mf')
  for (const item of items) {
    expand(
      threeMfResourceIdAttribute(item, 'objectid'),
      parse3mfTransform(attributeNs(item, 'transform', null), scale),
      [],
    )
  }
  return geometry3D('3mf', vertices, triangles)
}

export async function parseOpenScad3mf(file: OpenScadProjectFile): Promise<OpenScadImportGeometry3D> {
  return parseOpenScad3mfWithBudget(file)
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

export function curveSteps(sweep: number, segments = OPENSCAD_IMPORT_DEFAULT_CURVE_SEGMENTS): number {
  return Math.max(2, Math.min(512, Math.ceil(Math.abs(sweep) / (Math.PI * 2) * segments)))
}

export function arcPoints(
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

export function bulgePoints(
  start: OpenScadImportPoint2,
  end: OpenScadImportPoint2,
  bulge: number,
): OpenScadImportPoint2[] {
  if (bulge === 0) return [start, end]
  const dx = end[0] - start[0], dy = end[1] - start[1]
  const chord = Math.hypot(dx, dy)
  if (!(chord > 0)) return [start]
  const sweep = 4 * Math.atan(bulge)
  const radius = chord / (2 * Math.sin(Math.abs(sweep) / 2))
  const midpoint: OpenScadImportPoint2 = [(start[0] + end[0]) / 2, (start[1] + end[1]) / 2]
  const distance = chord / (2 * Math.tan(sweep / 2))
  const center: OpenScadImportPoint2 = [midpoint[0] - dy / chord * distance, midpoint[1] + dx / chord * distance]
  const startAngle = Math.atan2(start[1] - center[1], start[0] - center[0])
  return arcPoints(center, radius, radius, startAngle, sweep)
}

export function pointNear(left: OpenScadImportPoint2, right: OpenScadImportPoint2): boolean {
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
