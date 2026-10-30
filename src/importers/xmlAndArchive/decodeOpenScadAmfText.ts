// xmlAndArchive: decodeOpenScadAmfText.
import type { OpenScadProjectFile } from '../../services/openScadProject'

import {
  boundedCount,
  bytesOf,
  dataError,
  finiteNumber,
  geometry3D,
  OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES,
  OPENSCAD_IMPORT_MAX_TRIANGLES,
  OPENSCAD_IMPORT_MAX_VERTICES,
  OPENSCAD_IMPORT_MAX_XML_DEPTH,
  OPENSCAD_IMPORT_MAX_XML_ELEMENTS,
  OPENSCAD_IMPORT_MAX_ZIP_ENTRIES,
  OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES,
  OpenScadImportDataError,
  UTF8_FATAL,
  type OpenScadImportFormat,
  type OpenScadImportGeometry3D,
  type OpenScadImportPreparationBudget,
} from '.././types'
import { unzip3mf, asAmfArchiveError } from './inflateRaw'
import { zipPath, requiredAttributeNs, Matrix4, IDENTITY_4, RawMesh3D, unitScale, appendRawMesh, multiply4 } from './requiredAttributeNs'
import { XmlElement, parseXml, childElementsNs, attributeNs, childElementNs } from './XmlElement'


export async function gunzipAmf(
  bytes: Uint8Array,
  projectBudget?: OpenScadImportPreparationBudget,
): Promise<Uint8Array> {
  let stream: ReadableStream<Uint8Array>
  try {
    const input = new Uint8Array(bytes)
    stream = new Blob([input.buffer]).stream().pipeThrough(new DecompressionStream('gzip'))
  } catch {
    return dataError('E_IMPORT_UNSUPPORTED_FEATURE', 'This runtime cannot inflate gzip-compressed AMF.', 'amf')
  }
  const reader = stream.getReader()
  const chunks: Uint8Array[] = []
  let length = 0
  try {
    while (true) {
      const { value, done } = await reader.read()
      if (done) break
      length += value.byteLength
      const projectLength = (projectBudget?.zipExpandedBytes ?? 0) + length
      if (length > OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES
        || projectLength > OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES) {
        await reader.cancel()
        return dataError(
          'E_IMPORT_LIMIT',
          'Gzip-compressed AMF expands beyond its per-file or prepared-project limit.',
          'amf',
          Math.min(
            OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES,
            OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES - (projectBudget?.zipExpandedBytes ?? 0),
          ),
          length,
        )
      }
      chunks.push(value)
    }
  } catch (error) {
    if (error instanceof OpenScadImportDataError) throw error
    return dataError('E_IMPORT_INVALID_DATA', 'AMF contains invalid gzip data.', 'amf')
  }
  if (projectBudget !== undefined) projectBudget.zipExpandedBytes += length
  const output = new Uint8Array(length)
  let offset = 0
  for (const chunk of chunks) {
    output.set(chunk, offset)
    offset += chunk.byteLength
  }
  return output
}


export function decodeAmfUtf8(bytes: Uint8Array): string {
  try {
    return UTF8_FATAL.decode(bytes)
  } catch {
    return dataError('E_IMPORT_ENCODING', 'AMF XML must be valid UTF-8.', 'amf')
  }
}


export async function decodeOpenScadAmfText(
  file: OpenScadProjectFile,
  projectBudget?: OpenScadImportPreparationBudget,
): Promise<string> {
  if (file.kind === 'source') return file.source
  const bytes = file.data
  if (bytes.byteLength >= 2 && bytes[0] === 0x1f && bytes[1] === 0x8b) {
    return decodeAmfUtf8(await gunzipAmf(bytes, projectBudget))
  }
  if (bytes.byteLength >= 4 && bytes[0] === 0x50 && bytes[1] === 0x4b
    && bytes[2] === 0x03 && bytes[3] === 0x04) {
    let files: ReadonlyMap<string, Uint8Array>
    try {
      files = await unzip3mf(bytes, projectBudget)
    } catch (error) {
      if (!(error instanceof OpenScadImportDataError)) throw error
      throw asAmfArchiveError(error)
    }
    const candidates = [...files.entries()].filter(([path]) => path.toLowerCase().endsWith('.amf'))
    if (candidates.length !== 1) {
      return dataError('E_IMPORT_INVALID_DATA', 'Compressed AMF ZIP must contain exactly one .amf document.', 'amf')
    }
    return decodeAmfUtf8(candidates[0][1])
  }
  return decodeAmfUtf8(bytes)
}


export function packageTarget(value: string): string {
  if (/[?#]/u.test(value) || /^[A-Za-z][A-Za-z0-9+.-]*:/u.test(value) || /%2f|%5c/iu.test(value)) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF StartPart Target must be an internal OPC part URI.', '3mf')
  }
  let decoded: string
  try {
    decoded = decodeURIComponent(value)
  } catch {
    return dataError('E_IMPORT_INVALID_DATA', '3MF StartPart Target contains invalid percent encoding.', '3mf')
  }
  return zipPath(decoded)
}


export const THREE_MF_CORE_NAMESPACE = 'http://schemas.microsoft.com/3dmanufacturing/core/2015/02'

export const OPC_RELATIONSHIPS_NAMESPACE = 'http://schemas.openxmlformats.org/package/2006/relationships'

export const THREE_MF_START_PART_RELATIONSHIP = 'http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel'


export function modelPathFromPackage(files: ReadonlyMap<string, Uint8Array>): string {
  const relationships = files.get('_rels/.rels')
  if (relationships === undefined) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF package is missing the root _rels/.rels part.', '3mf')
  }
  let root: XmlElement
  try {
    root = parseXml(UTF8_FATAL.decode(relationships), '3mf')
  } catch (error) {
    if (error instanceof OpenScadImportDataError) throw error
    return dataError('E_IMPORT_ENCODING', '3MF package relationships must be UTF-8 XML.', '3mf')
  }
  if (root.localName !== 'relationships' || root.namespaceUri !== OPC_RELATIONSHIPS_NAMESPACE) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF root relationships part has the wrong XML namespace.', '3mf')
  }
  const startParts = childElementsNs(root, 'relationship', OPC_RELATIONSHIPS_NAMESPACE).filter(relationship => (
    attributeNs(relationship, 'type', null) === THREE_MF_START_PART_RELATIONSHIP
  ))
  if (startParts.length !== 1) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF package must contain exactly one official StartPart relationship.', '3mf')
  }
  const relationship = startParts[0]
  const targetMode = attributeNs(relationship, 'targetmode', null)?.trim().toLowerCase()
  if (targetMode !== undefined && targetMode !== 'internal') {
    return dataError('E_IMPORT_UNSUPPORTED_FEATURE', '3MF StartPart cannot target an external resource.', '3mf')
  }
  const path = packageTarget(requiredAttributeNs(relationship, 'target', null, '3mf'))
  if (!files.has(path)) return dataError('E_IMPORT_INVALID_DATA', `3MF primary model ${path} is missing.`, '3mf')
  return path
}


export function threeMfResourceId(value: string, label: string): string {
  const normalized = value.trim()
  if (!/^\+?[0-9]+$/u.test(normalized)) {
    return dataError('E_IMPORT_INVALID_DATA', `${label} must be an ST_ResourceID positive integer.`, '3mf')
  }
  const numeric = BigInt(normalized)
  if (numeric < 1n || numeric >= 2_147_483_648n) {
    return dataError('E_IMPORT_INVALID_DATA', `${label} must be between 1 and 2147483647.`, '3mf')
  }
  return numeric.toString()
}


export function threeMfResourceIdAttribute(element: XmlElement, name: string): string {
  return threeMfResourceId(requiredAttributeNs(element, name, null, '3mf'), `3MF ${name}`)
}


export function threeMfResourceIndexAttribute(element: XmlElement, name: string): number {
  const value = requiredAttributeNs(element, name, null, '3mf')
  if (!/^\+?[0-9]+$/u.test(value)) {
    return dataError('E_IMPORT_INVALID_DATA', `3MF ${name} must be an ST_ResourceIndex integer.`, '3mf')
  }
  const numeric = BigInt(value)
  if (numeric < 0n || numeric >= 2_147_483_648n) {
    return dataError('E_IMPORT_INVALID_DATA', `3MF ${name} must be between 0 and 2147483647.`, '3mf')
  }
  return Number(numeric)
}


export function threeMfNumberAttribute(element: XmlElement, name: string): number {
  return finiteNumber(requiredAttributeNs(element, name, null, '3mf'), `3MF ${name}`, '3mf')
}


export function parse3mfTransform(value: string | undefined, scale: number): Matrix4 {
  if (value === undefined || value.trim() === '') return IDENTITY_4
  const tokens = value.trim().split(/\s+/u)
  if (tokens.length !== 12) return dataError('E_IMPORT_INVALID_DATA', '3MF transform must contain 12 numbers.', '3mf')
  const numbers = tokens.map(token => finiteNumber(token, '3MF transform', '3mf'))
  // 3MF serializes a row-vector 4x3 affine matrix; convert to our column-vector 4x4 form.
  return [
    numbers[0], numbers[3], numbers[6], numbers[9] * scale,
    numbers[1], numbers[4], numbers[7], numbers[10] * scale,
    numbers[2], numbers[5], numbers[8], numbers[11] * scale,
    0, 0, 0, 1,
  ]
}


export type ThreeMfObject =
  | { readonly kind: 'mesh'; readonly mesh: RawMesh3D }
  | { readonly kind: 'components'; readonly components: readonly { readonly objectId: string; readonly transform: Matrix4 }[] }


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
