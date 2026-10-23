import { OPENSCAD_IMPORT_MAX_TRIANGLES, OPENSCAD_IMPORT_MAX_VERTICES, OPENSCAD_IMPORT_MAX_XML_DEPTH, OPENSCAD_IMPORT_MAX_XML_ELEMENTS } from '../openScadImport'
import type { Matrix4, MutableXmlElement, OpenScadImportFormat, RawMesh3D, XmlElement } from '../openScadImport'
import { boundedCount, dataError, decodeXmlEntities, finiteNumber, xmlNameCharacter } from './parseOpenScadStl'

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

export function multiply4(left: Matrix4, right: Matrix4): Matrix4 {
  const output = new Array<number>(16).fill(0)
  for (let row = 0; row < 4; row++) {
    for (let column = 0; column < 4; column++) {
      for (let index = 0; index < 4; index++) {
        output[row * 4 + column] += left[row * 4 + index] * right[index * 4 + column]
      }
    }
  }
  return output as unknown as Matrix4
}

export function transform3(matrix: Matrix4, x: number, y: number, z: number): [number, number, number] {
  return [
    matrix[0] * x + matrix[1] * y + matrix[2] * z + matrix[3],
    matrix[4] * x + matrix[5] * y + matrix[6] * z + matrix[7],
    matrix[8] * x + matrix[9] * y + matrix[10] * z + matrix[11],
  ]
}

export function rotation4(rx: number, ry: number, rz: number, translation: readonly [number, number, number]): Matrix4 {
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
