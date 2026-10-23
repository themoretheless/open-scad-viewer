import type { OpenScadProjectFile } from '../services/openScadProject'
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
} from './types'

export interface XmlElement {
  readonly name: string
  readonly localName: string
  readonly namespaceUri: string | null
  readonly attributes: Readonly<Record<string, string>>
  readonly attributeNamespaces: Readonly<Record<string, string | null>>
  readonly children: readonly XmlElement[]
  readonly text: string
}

interface MutableXmlElement {
  name: string
  localName: string
  namespaceUri: string | null
  attributes: Record<string, string>
  attributeNamespaces: Record<string, string | null>
  namespaces: Record<string, string>
  children: MutableXmlElement[]
  text: string
}

function xmlNameCharacter(value: string, first: boolean): boolean {
  return first ? /[A-Za-z_:]/u.test(value) : /[A-Za-z0-9_.:-]/u.test(value)
}

function decodeXmlEntities(value: string, format: OpenScadImportFormat): string {
  if (/&(?!(?:amp|lt|gt|quot|apos|#[0-9]+|#x[0-9a-f]+);)/iu.test(value)) {
    return dataError('E_IMPORT_INVALID_DATA', 'XML contains a malformed or unsupported entity reference.', format)
  }
  return value.replace(/&([^;]{1,32});/gu, (_whole, entity: string) => {
    if (entity === 'amp') return '&'
    if (entity === 'lt') return '<'
    if (entity === 'gt') return '>'
    if (entity === 'quot') return '"'
    if (entity === 'apos') return "'"
    let code = -1
    if (/^#[0-9]+$/u.test(entity)) code = Number(entity.slice(1))
    else if (/^#x[0-9a-f]+$/iu.test(entity)) code = Number.parseInt(entity.slice(2), 16)
    if (Number.isSafeInteger(code) && code >= 0 && code <= 0x10ffff
      && !(code >= 0xd800 && code <= 0xdfff)) return String.fromCodePoint(code)
    return dataError('E_IMPORT_INVALID_DATA', `XML contains unsupported entity &${entity};.`, format)
  })
}

/** Strict, entity-safe XML subset shared by SVG, AMF, and 3MF. */
export function parseXml(text: string, format: OpenScadImportFormat): XmlElement {
  if (/<!DOCTYPE|<!ENTITY/iu.test(text)) {
    return dataError('E_IMPORT_UNSUPPORTED_FEATURE', `${format.toUpperCase()} XML declarations cannot define entities or a DTD.`, format)
  }
  const document: MutableXmlElement = {
    name: '#document',
    localName: '#document',
    namespaceUri: null,
    attributes: {},
    attributeNamespaces: {},
    namespaces: { xml: 'http://www.w3.org/XML/1998/namespace' },
    children: [],
    text: '',
  }
  const stack: MutableXmlElement[] = [document]
  let cursor = 0
  let elements = 0
  const skipWhitespace = (): void => {
    while (cursor < text.length && /\s/u.test(text[cursor])) cursor++
  }
  const name = (): string => {
    const start = cursor
    if (!xmlNameCharacter(text[cursor] ?? '', true)) {
      return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML contains an invalid element or attribute name.`, format)
    }
    cursor++
    while (cursor < text.length && xmlNameCharacter(text[cursor], false)) cursor++
    return text.slice(start, cursor)
  }
  while (cursor < text.length) {
    const open = text.indexOf('<', cursor)
    if (open < 0) {
      stack.at(-1)!.text += decodeXmlEntities(text.slice(cursor), format)
      cursor = text.length
      break
    }
    stack.at(-1)!.text += decodeXmlEntities(text.slice(cursor, open), format)
    cursor = open
    if (text.startsWith('<!--', cursor)) {
      const end = text.indexOf('-->', cursor + 4)
      if (end < 0) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML comment is unterminated.`, format)
      cursor = end + 3
      continue
    }
    if (text.startsWith('<?', cursor)) {
      const end = text.indexOf('?>', cursor + 2)
      if (end < 0) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML processing instruction is unterminated.`, format)
      cursor = end + 2
      continue
    }
    if (text.startsWith('<![CDATA[', cursor)) {
      const end = text.indexOf(']]>', cursor + 9)
      if (end < 0) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML CDATA is unterminated.`, format)
      stack.at(-1)!.text += text.slice(cursor + 9, end)
      cursor = end + 3
      continue
    }
    if (text.startsWith('</', cursor)) {
      cursor += 2
      skipWhitespace()
      const closing = name().toLowerCase()
      skipWhitespace()
      if (text[cursor] !== '>') return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML closing tag is malformed.`, format)
      cursor++
      const current = stack.pop()
      if (!current || current === document || current.name.toLowerCase() !== closing) {
        return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML closing tag does not match.`, format)
      }
      continue
    }
    if (text.startsWith('<!', cursor)) {
      return dataError('E_IMPORT_UNSUPPORTED_FEATURE', `${format.toUpperCase()} XML declaration is unsupported.`, format)
    }

    cursor++
    skipWhitespace()
    const elementName = name()
    const attributes: Record<string, string> = {}
    let selfClosing = false
    let terminated = false
    while (cursor < text.length) {
      skipWhitespace()
      if (text.startsWith('/>', cursor)) {
        selfClosing = true
        cursor += 2
        terminated = true
        break
      }
      if (text[cursor] === '>') {
        cursor++
        terminated = true
        break
      }
      const attributeName = name().toLowerCase()
      if (Object.hasOwn(attributes, attributeName)) {
        return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML contains a duplicate attribute.`, format)
      }
      skipWhitespace()
      if (text[cursor++] !== '=') return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML attribute is missing '='.`, format)
      skipWhitespace()
      const quote = text[cursor++]
      if (quote !== '"' && quote !== "'") {
        return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML attribute values must be quoted.`, format)
      }
      const end = text.indexOf(quote, cursor)
      if (end < 0) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML attribute is unterminated.`, format)
      attributes[attributeName] = decodeXmlEntities(text.slice(cursor, end), format)
      cursor = end + 1
      if (Object.keys(attributes).length > 64) {
        return dataError('E_IMPORT_LIMIT', `${format.toUpperCase()} XML element exceeds 64 attributes.`, format, 64, Object.keys(attributes).length)
      }
    }
    if (!terminated) return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML start tag is unterminated.`, format)
    const namespaces = { ...stack.at(-1)!.namespaces }
    for (const [attributeName, value] of Object.entries(attributes)) {
      if (attributeName === 'xmlns') {
        if (value === '') delete namespaces['']
        else namespaces[''] = value
      } else if (attributeName.startsWith('xmlns:')) {
        const prefix = attributeName.slice(6)
        if (!prefix || value === '') return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML namespace declaration is invalid.`, format)
        namespaces[prefix] = value
      }
    }
    const qname = (qualified: string): { prefix: string; localName: string } => {
      const parts = qualified.toLowerCase().split(':')
      if (parts.length > 2 || parts.some(part => part === '')) {
        return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML qualified name is invalid.`, format)
      }
      return { prefix: parts.length === 2 ? parts[0] : '', localName: parts.at(-1)! }
    }
    const elementQName = qname(elementName)
    const namespaceUri = namespaces[elementQName.prefix] ?? null
    if (elementQName.prefix && namespaceUri === null) {
      return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML uses an undeclared namespace prefix.`, format)
    }
    const attributeNamespaces: Record<string, string | null> = {}
    for (const attributeName of Object.keys(attributes)) {
      if (attributeName === 'xmlns' || attributeName.startsWith('xmlns:')) {
        attributeNamespaces[attributeName] = 'http://www.w3.org/2000/xmlns/'
        continue
      }
      const attributeQName = qname(attributeName)
      const attributeNamespace = attributeQName.prefix ? namespaces[attributeQName.prefix] ?? null : null
      if (attributeQName.prefix && attributeNamespace === null) {
        return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML attribute uses an undeclared namespace prefix.`, format)
      }
      attributeNamespaces[attributeName] = attributeNamespace
    }
    elements++
    if (elements > OPENSCAD_IMPORT_MAX_XML_ELEMENTS) {
      return dataError(
        'E_IMPORT_LIMIT',
        `${format.toUpperCase()} XML exceeds ${OPENSCAD_IMPORT_MAX_XML_ELEMENTS.toLocaleString()} elements.`,
        format,
        OPENSCAD_IMPORT_MAX_XML_ELEMENTS,
        elements,
      )
    }
    const element: MutableXmlElement = {
      name: elementName,
      localName: elementQName.localName,
      namespaceUri,
      attributes,
      attributeNamespaces,
      namespaces,
      children: [],
      text: '',
    }
    stack.at(-1)!.children.push(element)
    if (!selfClosing) {
      stack.push(element)
      if (stack.length - 1 > OPENSCAD_IMPORT_MAX_XML_DEPTH) {
        return dataError(
          'E_IMPORT_LIMIT',
          `${format.toUpperCase()} XML exceeds nesting depth ${OPENSCAD_IMPORT_MAX_XML_DEPTH}.`,
          format,
          OPENSCAD_IMPORT_MAX_XML_DEPTH,
          stack.length - 1,
        )
      }
    }
  }
  if (stack.length !== 1 || document.children.length !== 1 || document.text.trim() !== '') {
    return dataError('E_IMPORT_INVALID_DATA', `${format.toUpperCase()} XML must contain exactly one root element.`, format)
  }
  return document.children[0]
}

export function childElements(element: XmlElement, localName: string): readonly XmlElement[] {
  return element.children.filter(child => child.localName === localName)
}

export function childElement(element: XmlElement, localName: string): XmlElement | null {
  return element.children.find(child => child.localName === localName) ?? null
}

export function descendants(element: XmlElement, localName: string): XmlElement[] {
  const output: XmlElement[] = []
  const visit = (node: XmlElement): void => {
    if (node.localName === localName) output.push(node)
    for (const child of node.children) visit(child)
  }
  visit(element)
  return output
}

export function childElementsNs(element: XmlElement, localName: string, namespaceUri: string): readonly XmlElement[] {
  return element.children.filter(child => child.localName === localName && child.namespaceUri === namespaceUri)
}

export function childElementNs(element: XmlElement, localName: string, namespaceUri: string): XmlElement | null {
  return element.children.find(child => child.localName === localName && child.namespaceUri === namespaceUri) ?? null
}

export function descendantsNs(element: XmlElement, localName: string, namespaceUri: string): XmlElement[] {
  const output: XmlElement[] = []
  const visit = (node: XmlElement): void => {
    if (node.localName === localName && node.namespaceUri === namespaceUri) output.push(node)
    for (const child of node.children) visit(child)
  }
  visit(element)
  return output
}

export function attribute(element: XmlElement, key: string): string | undefined {
  const normalized = key.toLowerCase()
  const exact = element.attributes[normalized]
  if (exact !== undefined) return exact
  const suffix = `:${normalized}`
  for (const name in element.attributes) {
    if (name.endsWith(suffix)) return element.attributes[name]
  }
  return undefined
}

export function attributeNs(element: XmlElement, key: string, namespaceUri: string | null): string | undefined {
  const normalized = key.toLowerCase()
  if (namespaceUri === null && element.attributeNamespaces[normalized] === null) {
    const exact = element.attributes[normalized]
    if (exact !== undefined) return exact
  }
  const suffix = `:${normalized}`
  for (const name in element.attributes) {
    if (element.attributeNamespaces[name] === namespaceUri && (name === normalized || name.endsWith(suffix))) {
      return element.attributes[name]
    }
  }
  return undefined
}

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

interface AmfInstance {
  readonly id: string
  readonly transform: Matrix4
}

type AmfNode =
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

const CRC_TABLE = new Uint32Array(256)
for (let index = 0; index < CRC_TABLE.length; index++) {
  let value = index
  for (let bit = 0; bit < 8; bit++) value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1
  CRC_TABLE[index] = value >>> 0
}

function crc32(bytes: Uint8Array): number {
  let value = 0xffffffff
  for (const byte of bytes) value = CRC_TABLE[(value ^ byte) & 0xff] ^ (value >>> 8)
  return (value ^ 0xffffffff) >>> 0
}

function zipPath(value: string): string {
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

async function inflateRaw(compressed: Uint8Array, expected: number): Promise<Uint8Array> {
  let stream: ReadableStream<Uint8Array>
  try {
    const input = new Uint8Array(compressed)
    stream = new Blob([input.buffer]).stream().pipeThrough(new DecompressionStream('deflate-raw'))
  } catch {
    return dataError('E_IMPORT_UNSUPPORTED_FEATURE', 'This runtime cannot inflate compressed 3MF ZIP entries.', '3mf')
  }
  const reader = stream.getReader()
  const chunks: Uint8Array[] = []
  let length = 0
  try {
    while (true) {
      const { value, done } = await reader.read()
      if (done) break
      length += value.byteLength
      if (length > expected || length > OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES) {
        await reader.cancel()
        return dataError(
          'E_IMPORT_LIMIT',
          '3MF ZIP entry expands beyond its declared or permitted size.',
          '3mf',
          Math.min(expected, OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES),
          length,
        )
      }
      chunks.push(value)
    }
  } catch (error) {
    if (error instanceof OpenScadImportDataError) throw error
    return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP contains invalid deflate data.', '3mf')
  }
  if (length !== expected) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP entry size does not match its directory record.', '3mf')
  const output = new Uint8Array(length)
  let offset = 0
  for (const chunk of chunks) {
    output.set(chunk, offset)
    offset += chunk.byteLength
  }
  return output
}

interface ZipCentralEntry {
  readonly path: string
  readonly directory: boolean
  readonly flags: number
  readonly method: number
  readonly crc: number
  readonly compressedSize: number
  readonly uncompressedSize: number
  readonly localOffset: number
}

interface ZipDirectoryInfo {
  readonly endOffset: number
  readonly directoryEnd: number
  readonly entryCount: number
  readonly directorySize: number
  readonly directoryOffset: number
}

function boundedZipUint64(view: DataView, offset: number, label: string): number {
  if (offset < 0 || offset + 8 > view.byteLength) {
    return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP ${label} is truncated.`, '3mf')
  }
  const value = view.getBigUint64(offset, true)
  if (value > BigInt(Number.MAX_SAFE_INTEGER)) {
    return dataError('E_IMPORT_LIMIT', `3MF ZIP ${label} exceeds the safe integer range.`, '3mf', Number.MAX_SAFE_INTEGER)
  }
  return Number(value)
}

function zipDirectoryInfo(view: DataView): ZipDirectoryInfo {
  const length = view.byteLength
  const minimum = Math.max(0, length - 65_557)
  // Scan forward: the real EOCD precedes all bytes in its own comment, so a
  // comment containing a second plausible EOCD signature cannot shadow it.
  for (let end = minimum; end <= length - 22; end++) {
    if (view.getUint32(end, true) !== 0x06054b50
      || end + 22 + view.getUint16(end + 20, true) !== length) continue
    const disk = view.getUint16(end + 4, true)
    const directoryDisk = view.getUint16(end + 6, true)
    const diskEntries = view.getUint16(end + 8, true)
    const classicCount = view.getUint16(end + 10, true)
    const classicSize = view.getUint32(end + 12, true)
    const classicOffset = view.getUint32(end + 16, true)
    const zip64 = diskEntries === 0xffff || classicCount === 0xffff
      || classicSize === 0xffffffff || classicOffset === 0xffffffff
    if (!zip64) {
      if (disk !== 0 || directoryDisk !== 0 || diskEntries !== classicCount || classicCount === 0
        || classicOffset + classicSize !== end) continue
      return {
        endOffset: end,
        directoryEnd: end,
        entryCount: classicCount,
        directorySize: classicSize,
        directoryOffset: classicOffset,
      }
    }
    const locator = end - 20
    if (locator < 0 || view.getUint32(locator, true) !== 0x07064b50
      || view.getUint32(locator + 4, true) !== 0 || view.getUint32(locator + 16, true) !== 1) continue
    const zip64End = boundedZipUint64(view, locator + 8, 'ZIP64 end-record offset')
    if (zip64End + 56 > locator || view.getUint32(zip64End, true) !== 0x06064b50) continue
    const recordSize = boundedZipUint64(view, zip64End + 4, 'ZIP64 end-record size')
    if (recordSize < 44 || zip64End + 12 + recordSize !== locator) continue
    if (view.getUint32(zip64End + 16, true) !== 0 || view.getUint32(zip64End + 20, true) !== 0) continue
    const diskCount = boundedZipUint64(view, zip64End + 24, 'ZIP64 disk entry count')
    const entryCount = boundedZipUint64(view, zip64End + 32, 'ZIP64 total entry count')
    const directorySize = boundedZipUint64(view, zip64End + 40, 'ZIP64 central-directory size')
    const directoryOffset = boundedZipUint64(view, zip64End + 48, 'ZIP64 central-directory offset')
    if (diskCount !== entryCount || entryCount === 0 || directoryOffset + directorySize !== zip64End) continue
    return {
      endOffset: end,
      directoryEnd: zip64End,
      entryCount,
      directorySize,
      directoryOffset,
    }
  }
  return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP end record is missing or inconsistent.', '3mf')
}

interface Zip64EntryValues {
  readonly uncompressedSize?: number
  readonly compressedSize?: number
  readonly localOffset?: number
  readonly diskStart?: number
}

function zip64EntryValues(
  bytes: Uint8Array,
  extraOffset: number,
  extraLength: number,
  needs: { readonly uncompressedSize: boolean; readonly compressedSize: boolean; readonly localOffset: boolean; readonly diskStart: boolean },
): Zip64EntryValues {
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const end = extraOffset + extraLength
  if (extraOffset < 0 || end > bytes.byteLength) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP extra field is truncated.', '3mf')
  let cursor = extraOffset
  while (cursor + 4 <= end) {
    const id = view.getUint16(cursor, true), size = view.getUint16(cursor + 2, true)
    cursor += 4
    if (cursor + size > end) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP extra field is malformed.', '3mf')
    if (id !== 0x0001) {
      cursor += size
      continue
    }
    const fieldEnd = cursor + size
    const output: { uncompressedSize?: number; compressedSize?: number; localOffset?: number; diskStart?: number } = {}
    const read64 = (label: string): number => {
      if (cursor + 8 > fieldEnd) return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP64 ${label} is missing.`, '3mf')
      const value = boundedZipUint64(view, cursor, label)
      cursor += 8
      return value
    }
    if (needs.uncompressedSize) output.uncompressedSize = read64('uncompressed size')
    if (needs.compressedSize) output.compressedSize = read64('compressed size')
    if (needs.localOffset) output.localOffset = read64('local-header offset')
    if (needs.diskStart) {
      if (cursor + 4 > fieldEnd) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP64 disk start is missing.', '3mf')
      output.diskStart = view.getUint32(cursor, true)
    }
    return output
  }
  if (Object.values(needs).some(Boolean)) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP64 entry is missing its 0x0001 extra field.', '3mf')
  return {}
}

async function unzip3mf(
  bytes: Uint8Array,
  projectBudget?: OpenScadImportPreparationBudget,
): Promise<ReadonlyMap<string, Uint8Array>> {
  if (bytes.byteLength < 22) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP is shorter than its end record.', '3mf')
  const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
  const directory = zipDirectoryInfo(view)
  const { entryCount, directorySize, directoryOffset, directoryEnd } = directory
  boundedCount(entryCount, OPENSCAD_IMPORT_MAX_ZIP_ENTRIES, '3MF ZIP entry count', '3mf')
  if (directoryOffset + directorySize !== directoryEnd || directoryOffset > bytes.byteLength) {
    return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP central directory is inconsistent.', '3mf')
  }
  const entries: ZipCentralEntry[] = []
  const seen = new Set<string>()
  let expanded = 0
  let cursor = directoryOffset
  for (let index = 0; index < entryCount; index++) {
    if (cursor + 46 > directoryEnd || view.getUint32(cursor, true) !== 0x02014b50) {
      return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP central directory entry is truncated.', '3mf')
    }
    const flags = view.getUint16(cursor + 8, true)
    const method = view.getUint16(cursor + 10, true)
    const crc = view.getUint32(cursor + 16, true)
    const compressedSize32 = view.getUint32(cursor + 20, true)
    const uncompressedSize32 = view.getUint32(cursor + 24, true)
    const nameLength = view.getUint16(cursor + 28, true)
    const extraLength = view.getUint16(cursor + 30, true)
    const entryComment = view.getUint16(cursor + 32, true)
    const diskStart16 = view.getUint16(cursor + 34, true)
    const localOffset32 = view.getUint32(cursor + 42, true)
    const next = cursor + 46 + nameLength + extraLength + entryComment
    if (next > directoryEnd) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP central directory entry is truncated.', '3mf')
    const zip64Values = zip64EntryValues(
      bytes,
      cursor + 46 + nameLength,
      extraLength,
      {
        uncompressedSize: uncompressedSize32 === 0xffffffff,
        compressedSize: compressedSize32 === 0xffffffff,
        localOffset: localOffset32 === 0xffffffff,
        diskStart: diskStart16 === 0xffff,
      },
    )
    const compressedSize = compressedSize32 === 0xffffffff ? zip64Values.compressedSize! : compressedSize32
    const uncompressedSize = uncompressedSize32 === 0xffffffff ? zip64Values.uncompressedSize! : uncompressedSize32
    const localOffset = localOffset32 === 0xffffffff ? zip64Values.localOffset! : localOffset32
    const diskStart = diskStart16 === 0xffff ? zip64Values.diskStart! : diskStart16
    if (diskStart !== 0) return dataError('E_IMPORT_UNSUPPORTED_FEATURE', 'Multi-disk 3MF ZIP entries are unsupported.', '3mf')
    if ((flags & 1) !== 0 || (flags & ~0x080e) !== 0 || (method !== 0 && method !== 8)) {
      return dataError('E_IMPORT_UNSUPPORTED_FEATURE', '3MF ZIP entry uses encryption, flags, or compression not permitted by this importer.', '3mf')
    }
    const encodedName = bytes.subarray(cursor + 46, cursor + 46 + nameLength)
    let decodedName: string
    try {
      decodedName = UTF8_FATAL.decode(encodedName)
    } catch {
      return dataError('E_IMPORT_ENCODING', '3MF ZIP entry names must be UTF-8.', '3mf')
    }
    if ((flags & 0x0800) === 0 && encodedName.some(byte => byte >= 0x80)) {
      return dataError('E_IMPORT_ENCODING', 'Non-ASCII 3MF ZIP entry names must set the UTF-8 flag.', '3mf')
    }
    const directory = decodedName.endsWith('/')
    const path = zipPath(decodedName)
    if (directory && (compressedSize !== 0 || uncompressedSize !== 0)) {
      return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP directory ${path} must be empty.`, '3mf')
    }
    if (seen.has(path)) return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP contains duplicate entry ${path}.`, '3mf')
    seen.add(path)
    expanded += uncompressedSize
    if (!Number.isSafeInteger(expanded) || expanded > OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES) {
      return dataError(
        'E_IMPORT_LIMIT',
        `3MF ZIP expands beyond ${OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES.toLocaleString()} bytes.`,
        '3mf',
        OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES,
        expanded,
      )
    }
    entries.push({ path, directory, flags, method, crc, compressedSize, uncompressedSize, localOffset })
    cursor = next
  }
  if (cursor !== directoryEnd) return dataError('E_IMPORT_INVALID_DATA', '3MF ZIP central directory has trailing data.', '3mf')
  if (projectBudget !== undefined) {
    const total = projectBudget.zipExpandedBytes + expanded
    if (!Number.isSafeInteger(total) || total > OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES) {
      return dataError(
        'E_IMPORT_LIMIT',
        `Prepared project 3MF data expands beyond ${OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES.toLocaleString()} bytes.`,
        '3mf',
        OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES,
        total,
      )
    }
    projectBudget.zipExpandedBytes = total
  }

  const files = new Map<string, Uint8Array>()
  for (const entry of entries) {
    if (entry.localOffset + 30 > directoryOffset || view.getUint32(entry.localOffset, true) !== 0x04034b50) {
      return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP local entry ${entry.path} is invalid.`, '3mf')
    }
    const localFlags = view.getUint16(entry.localOffset + 6, true)
    const localMethod = view.getUint16(entry.localOffset + 8, true)
    const nameLength = view.getUint16(entry.localOffset + 26, true)
    const extraLength = view.getUint16(entry.localOffset + 28, true)
    const dataStart = entry.localOffset + 30 + nameLength + extraLength
    const dataEnd = dataStart + entry.compressedSize
    if (localFlags !== entry.flags || localMethod !== entry.method || dataEnd > directoryOffset) {
      return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP local entry ${entry.path} disagrees with its directory record.`, '3mf')
    }
    let localName: string
    try {
      localName = UTF8_FATAL.decode(bytes.subarray(entry.localOffset + 30, entry.localOffset + 30 + nameLength))
    } catch {
      return dataError('E_IMPORT_ENCODING', '3MF ZIP local entry name must be UTF-8.', '3mf')
    }
    if (zipPath(localName) !== entry.path || localName.endsWith('/') !== entry.directory) {
      return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP local entry name does not match ${entry.path}.`, '3mf')
    }
    const compressed = bytes.subarray(dataStart, dataEnd)
    const data = entry.method === 0
      ? new Uint8Array(compressed)
      : await inflateRaw(compressed, entry.uncompressedSize)
    if (data.byteLength !== entry.uncompressedSize || crc32(data) !== entry.crc) {
      return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP entry ${entry.path} failed size or CRC verification.`, '3mf')
    }
    if (!entry.directory) files.set(entry.path, data)
  }
  return files
}

function asAmfArchiveError(error: OpenScadImportDataError): OpenScadImportDataError {
  return new OpenScadImportDataError(
    error.code,
    error.message.replaceAll('3MF', 'AMF'),
    'amf',
    error.limit,
    error.actual,
  )
}

async function gunzipAmf(
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

function decodeAmfUtf8(bytes: Uint8Array): string {
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

function packageTarget(value: string): string {
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

const THREE_MF_CORE_NAMESPACE = 'http://schemas.microsoft.com/3dmanufacturing/core/2015/02'
const OPC_RELATIONSHIPS_NAMESPACE = 'http://schemas.openxmlformats.org/package/2006/relationships'
const THREE_MF_START_PART_RELATIONSHIP = 'http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel'

function modelPathFromPackage(files: ReadonlyMap<string, Uint8Array>): string {
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

function threeMfResourceId(value: string, label: string): string {
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

function threeMfResourceIdAttribute(element: XmlElement, name: string): string {
  return threeMfResourceId(requiredAttributeNs(element, name, null, '3mf'), `3MF ${name}`)
}

function threeMfResourceIndexAttribute(element: XmlElement, name: string): number {
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

function threeMfNumberAttribute(element: XmlElement, name: string): number {
  return finiteNumber(requiredAttributeNs(element, name, null, '3mf'), `3MF ${name}`, '3mf')
}

function parse3mfTransform(value: string | undefined, scale: number): Matrix4 {
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

type ThreeMfObject =
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
