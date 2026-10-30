// xmlAndArchive: XmlElement.
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



export interface XmlElement {
  readonly name: string
  readonly localName: string
  readonly namespaceUri: string | null
  readonly attributes: Readonly<Record<string, string>>
  readonly attributeNamespaces: Readonly<Record<string, string | null>>
  readonly children: readonly XmlElement[]
  readonly text: string
}


export interface MutableXmlElement {
  name: string
  localName: string
  namespaceUri: string | null
  attributes: Record<string, string>
  attributeNamespaces: Record<string, string | null>
  namespaces: Record<string, string>
  children: MutableXmlElement[]
  text: string
}


export function xmlNameCharacter(value: string, first: boolean): boolean {
  return first ? /[A-Za-z_:]/u.test(value) : /[A-Za-z0-9_.:-]/u.test(value)
}


export function decodeXmlEntities(value: string, format: OpenScadImportFormat): string {
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
