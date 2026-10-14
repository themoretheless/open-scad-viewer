import { CRC_TABLE, IDENTITY_4, OPENSCAD_IMPORT_MAX_TRIANGLES, OPENSCAD_IMPORT_MAX_VERTICES, OPENSCAD_IMPORT_MAX_XML_DEPTH, OPENSCAD_IMPORT_MAX_XML_ELEMENTS, OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES, OpenScadImportDataError } from '../openScadImport'
import type { AmfNode, Matrix4, OpenScadImportGeometry3D, OpenScadImportPreparationBudget, Zip64EntryValues, ZipDirectoryInfo } from '../openScadImport'
import type { OpenScadProjectFile } from '../openScadProject'
import { appendRawMesh, attribute, childElement, childElements, multiply4, numericText, parseXml, requiredAttribute, rotation4, unitScale } from './parseXml'
import { boundedCount, dataError, geometry3D } from './parseOpenScadStl'
import { decodeOpenScadAmfText } from './unzip3mf'

export function parseOpenScadAmfXml(text: string): OpenScadImportGeometry3D {
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
        transform: rotation4(
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
  projectBudget?: OpenScadImportPreparationBudget,
): Promise<OpenScadImportGeometry3D> {
  return parseOpenScadAmfXml(await decodeOpenScadAmfText(file, projectBudget))
}

export async function parseOpenScadAmf(file: OpenScadProjectFile): Promise<OpenScadImportGeometry3D> {
  return parseOpenScadAmfWithBudget(file)
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

export async function inflateRaw(compressed: Uint8Array, expected: number): Promise<Uint8Array> {
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

export function boundedZipUint64(view: DataView, offset: number, label: string): number {
  if (offset < 0 || offset + 8 > view.byteLength) {
    return dataError('E_IMPORT_INVALID_DATA', `3MF ZIP ${label} is truncated.`, '3mf')
  }
  const value = view.getBigUint64(offset, true)
  if (value > BigInt(Number.MAX_SAFE_INTEGER)) {
    return dataError('E_IMPORT_LIMIT', `3MF ZIP ${label} exceeds the safe integer range.`, '3mf', Number.MAX_SAFE_INTEGER)
  }
  return Number(value)
}

export function zipDirectoryInfo(view: DataView): ZipDirectoryInfo {
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

export function zip64EntryValues(
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
