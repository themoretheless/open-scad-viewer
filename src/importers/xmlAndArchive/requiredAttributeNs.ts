// xmlAndArchive: requiredAttributeNs.
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
import { XmlElement, attributeNs, attribute, parseXml, childElements, childElement } from './XmlElement'
import { decodeOpenScadAmfText } from './decodeOpenScadAmfText'


export function requiredAttributeNs(
  element: XmlElement,
  key: string,
  namespaceUri: string | null,
  format: OpenScadImportFormat,
): string {
  const value = attributeNs(element, key, namespaceUri)
  if (value === undefined || value.trim() === '') {
    return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} <${element.localName}> is missing ${key}.`, format)
  }
  return value.trim()
}


export function requiredAttribute(
  element: XmlElement,
  key: string,
  format: OpenScadImportFormat,
): string {
  const value = attribute(element, key)
  if (value === undefined || value.trim() === '') {
    return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} <${element.localName}> is missing ${key}.`, format)
  }
  return value.trim()
}


export function numericAttribute(element: XmlElement, key: string, format: OpenScadImportFormat): number {
  return finiteNumber(requiredAttribute(element, key, format), `${format.toUpperCase()} ${key}`, format)
}


export function numericText(element: XmlElement | null, label: string, format: OpenScadImportFormat): number {
  if (element === null) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} is missing ${label}.`, format)
  return finiteNumber(element.text.trim(), label, format)
}


export function unitScale(unit: string | undefined, format: 'amf' | '3mf'): number {
  const normalized = (unit ?? 'millimeter').toLowerCase()
  const scales: Record<string, number> = {
    micron: 0.001,
    micrometer: 0.001,
    millimeter: 1,
    centimeter: 10,
    meter: 1_000,
    inch: 25.4,
    foot: 304.8,
    feet: 304.8,
  }
  const scale = scales[normalized]
  if (scale === undefined) return dataError('E_IMPORT_UNSUPPORTED_FEATURE', `${format.toUpperCase()} unit ${JSON.stringify(unit)} is unsupported.`, format)
  return scale
}


export type Matrix4 = readonly [
  number, number, number, number,
  number, number, number, number,
  number, number, number, number,
  number, number, number, number,
]


export const IDENTITY_4: Matrix4 = [
  1, 0, 0, 0,
  0, 1, 0, 0,
  0, 0, 1, 0,
  0, 0, 0, 1,
]


export function multiply4(left: Matrix4, right: Matrix4): Matrix4 {
  const output = new Array<number>(16).fill(0)
  for (let row = 0; row < 4; row++) {
    for (let column = 0; column < 4; column++) {
      for (let index = 0; index < 4; index++) {
        output[row * 4 + index] += left[row * 4 + index] * right[index * 4 + column]
      }
    }
  }
  return output as unknown as Matrix4
}


export interface RawMesh3D {
  readonly vertices: readonly number[]
  readonly triangles: readonly number[]
}


export function appendRawMesh(
  targetVertices: number[],
  targetTriangles: number[],
  mesh: RawMesh3D,
  matrix: Matrix4,
  format: OpenScadImportFormat,
): void {
  const vertexOffset = targetVertices.length / 3
  const nextVertices = vertexOffset + mesh.vertices.length / 3
  const nextTriangles = targetTriangles.length / 3 + mesh.triangles.length / 3
  boundedCount(nextVertices, OPENSCAD_IMPORT_MAX_VERTICES, `${format.toUpperCase()} expanded vertex count`, format)
  boundedCount(nextTriangles, OPENSCAD_IMPORT_MAX_TRIANGLES, `${format.toUpperCase()} expanded triangle count`, format)
  const m0 = matrix[0], m1 = matrix[1], m2 = matrix[2], m3 = matrix[3]
  const m4 = matrix[4], m5 = matrix[5], m6 = matrix[6], m7 = matrix[7]
  const m8 = matrix[8], m9 = matrix[9], m10 = matrix[10], m11 = matrix[11]
  const srcVertices = mesh.vertices
  for (let offset = 0; offset < srcVertices.length; offset += 3) {
    const x = srcVertices[offset], y = srcVertices[offset + 1], z = srcVertices[offset + 2]
    targetVertices.push(
      m0 * x + m1 * y + m2 * z + m3,
      m4 * x + m5 * y + m6 * z + m7,
      m8 * x + m9 * y + m10 * z + m11,
    )
  }
  const determinant = m0 * (m5 * m10 - m6 * m9)
    - m1 * (m4 * m10 - m6 * m8)
    + m2 * (m4 * m9 - m5 * m8)
  if (determinant === 0 || !Number.isFinite(determinant)) {
    return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} transform is singular or non-finite.`, format)
  }
  const srcTriangles = mesh.triangles
  if (determinant < 0) {
    for (let offset = 0; offset < srcTriangles.length; offset += 3) {
      targetTriangles.push(
        vertexOffset + srcTriangles[offset],
        vertexOffset + srcTriangles[offset + 2],
        vertexOffset + srcTriangles[offset + 1],
      )
    }
  } else {
    for (let offset = 0; offset < srcTriangles.length; offset += 3) {
      targetTriangles.push(
        vertexOffset + srcTriangles[offset],
        vertexOffset + srcTriangles[offset + 1],
        vertexOffset + srcTriangles[offset + 2],
      )
    }
  }
}


export interface AmfInstance {
  readonly id: string
  readonly transform: Matrix4
}


export type AmfNode =
  | { readonly kind: 'object'; readonly mesh: RawMesh3D }
  | { readonly kind: 'constellation'; readonly instances: readonly AmfInstance[] }


export type Rotation4Builder = (
  rx: number,
  ry: number,
  rz: number,
  translation: readonly [number, number, number],
) => Matrix4


export function parseOpenScadAmfXml(text: string, buildRotation4: Rotation4Builder): OpenScadImportGeometry3D {
  const root = parseXml(text, 'amf')
  if (root.localName !== 'amf') return dataError('E_IMPORT_INVALID_DATA', 'AMF root element must be <amf>.', 'amf')
  const scale = unitScale(attribute(root, 'unit'), 'amf')
  const nodes = new Map<string, AmfNode>()
  for (const object of childElements(root, 'object')) {
    const id = requiredAttribute(object, 'id', 'amf')
    if (nodes.has(id)) return dataError('E_IMPORT_INVALID_DATA', `AMF node id ${id} is duplicated.`, 'amf')
    const mesh = childElement(object, 'mesh')
    if (mesh === null) return dataError('E_IMPORT_INVALID_DATA', `AMF object ${id} is missing its mesh.`, 'amf')
    const verticesContainer = childElement(mesh, 'vertices')
    if (verticesContainer === null) return dataError('E_IMPORT_INVALID_DATA', `AMF object ${id} is missing vertices.`, 'amf')
    const vertices: number[] = []
    for (const [index, vertex] of childElements(verticesContainer, 'vertex').entries()) {
      const coordinates = childElement(vertex, 'coordinates')
      if (coordinates === null) return dataError('E_IMPORT_INVALID_DATA', `AMF vertex ${index} is missing coordinates.`, 'amf')
      vertices.push(
        numericText(childElement(coordinates, 'x'), 'AMF x', 'amf') * scale,
        numericText(childElement(coordinates, 'y'), 'AMF y', 'amf') * scale,
        numericText(childElement(coordinates, 'z'), 'AMF z', 'amf') * scale,
      )
      boundedCount(vertices.length / 3, OPENSCAD_IMPORT_MAX_VERTICES, 'AMF vertex count', 'amf')
    }
    const triangles: number[] = []
    for (const volume of childElements(mesh, 'volume')) {
      for (const triangle of childElements(volume, 'triangle')) {
        for (const name of ['v1', 'v2', 'v3'] as const) {
          const index = numericText(childElement(triangle, name), `AMF ${name}`, 'amf')
          if (!Number.isSafeInteger(index) || index < 0 || index >= vertices.length / 3) {
            return dataError('E_IMPORT_INVALID_DATA', `AMF ${name} index is out of bounds.`, 'amf')
          }
          triangles.push(index)
        }
        boundedCount(triangles.length / 3, OPENSCAD_IMPORT_MAX_TRIANGLES, 'AMF triangle count', 'amf')
      }
    }
    nodes.set(id, { kind: 'object', mesh: Object.freeze({ vertices, triangles }) })
  }
  const referenced = new Set<string>()
  for (const constellation of childElements(root, 'constellation')) {
    const id = requiredAttribute(constellation, 'id', 'amf')
    if (nodes.has(id)) return dataError('E_IMPORT_INVALID_DATA', `AMF node id ${id} is duplicated.`, 'amf')
    const instances = childElements(constellation, 'instance').map(instance => {
      const objectId = requiredAttribute(instance, 'objectid', 'amf')
      referenced.add(objectId)
      const read = (name: string): number => {
        const element = childElement(instance, name)
        return element === null ? 0 : numericText(element, `AMF ${name}`, 'amf') * (name.startsWith('delta') ? scale : 1)
      }
      return Object.freeze({
        id: objectId,
        transform: buildRotation4(
          read('rx'),
          read('ry'),
          read('rz'),
          [read('deltax'), read('deltay'), read('deltaz')],
        ),
      })
    })
    nodes.set(id, { kind: 'constellation', instances: Object.freeze(instances) })
  }
  const vertices: number[] = []
  const triangles: number[] = []
  let expansions = 0
  const expand = (id: string, matrix: Matrix4, stack: readonly string[]): void => {
    const node = nodes.get(id)
    if (!node) return dataError('E_IMPORT_INVALID_DATA', `AMF instance refers to missing node ${id}.`, 'amf')
    if (stack.includes(id)) return dataError('E_IMPORT_INVALID_DATA', `AMF constellation cycle includes ${id}.`, 'amf')
    expansions++
    if (expansions > OPENSCAD_IMPORT_MAX_XML_ELEMENTS) {
      return dataError('E_IMPORT_LIMIT', 'AMF instance expansion limit exceeded.', 'amf', OPENSCAD_IMPORT_MAX_XML_ELEMENTS, expansions)
    }
    if (node.kind === 'object') {
      appendRawMesh(vertices, triangles, node.mesh, matrix, 'amf')
      return
    }
    if (stack.length >= OPENSCAD_IMPORT_MAX_XML_DEPTH) {
      return dataError('E_IMPORT_LIMIT', 'AMF constellation nesting limit exceeded.', 'amf', OPENSCAD_IMPORT_MAX_XML_DEPTH, stack.length + 1)
    }
    for (const instance of node.instances) {
      expand(instance.id, multiply4(matrix, instance.transform), [...stack, id])
    }
  }
  const roots = [...nodes.keys()].filter(id => !referenced.has(id))
  if (nodes.size > 0 && roots.length === 0) {
    // A graph without a root is necessarily cyclic or self-referential; expand
    // one node to produce the stable cycle diagnostic.
    expand(nodes.keys().next().value!, IDENTITY_4, [])
  } else {
    for (const id of roots) {
      expand(id, IDENTITY_4, [])
    }
  }
  return geometry3D('amf', vertices, triangles)
}


/** Parse raw, gzip-compressed, or ZIP-compressed AMF into millimetres. */
export async function parseOpenScadAmfWithBudget(
  file: OpenScadProjectFile,
  buildRotation4: Rotation4Builder,
  projectBudget?: OpenScadImportPreparationBudget,
): Promise<OpenScadImportGeometry3D> {
  return parseOpenScadAmfXml(await decodeOpenScadAmfText(file, projectBudget), buildRotation4)
}


export const CRC_TABLE = new Uint32Array(256)

for (let index = 0; index < CRC_TABLE.length; index++) {
  let value = index
  for (let bit = 0; bit < 8; bit++) value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1
  CRC_TABLE[index] = value >>> 0
}


export function crc32(bytes: Uint8Array): number {
  let value = 0xffffffff
  for (const byte of bytes) value = CRC_TABLE[(value ^ byte) & 0xff] ^ (value >>> 8)
  return (value ^ 0xffffffff) >>> 0
}


export function zipPath(value: string): string {
  const path = value.replace(/^\/+/, '').replace(/\/$/u, '')
  if (!path || path.includes('\\') || path.includes('\0')) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP contains an invalid entry path.', '3mf')
  }
  const segments = path.split('/')
  if (segments.some(segment => segment === '' || segment === '.' || segment === '..')) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP entry path escapes or is not canonical.', '3mf')
  }
  return segments.join('/')
}
