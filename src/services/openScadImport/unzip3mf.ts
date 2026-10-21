import { IDENTITY_4, OPC_RELATIONSHIPS_NAMESPACE, OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES, OPENSCAD_IMPORT_MAX_ZIP_ENTRIES, OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES, OpenScadImportDataError, THREE_MF_START_PART_RELATIONSHIP, UTF8_FATAL } from '../openScadImport'
import type { Matrix4, OpenScadImportPreparationBudget, XmlElement, ZipCentralEntry } from '../openScadImport'
import type { OpenScadProjectFile } from '../openScadProject'
import { attributeNs, childElementsNs, parseXml, requiredAttributeNs } from './parseXml'
import { boundedCount, dataError, finiteNumber } from './parseOpenScadStl'
import { crc32, inflateRaw, zip64EntryValues, zipDirectoryInfo, zipPath } from './parseOpenScadAmf'

export async function unzip3mf(
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
        `3MF ZIP expands beyond ${OPENSCAD_IMPORT_MAX_ZIP_EXPANDED_BYTES.toLocaleString('en-US')} bytes.`,
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
        `Prepared project 3MF data expands beyond ${OPENSCAD_IMPORT_MAX_PROJECT_ZIP_EXPANDED_BYTES.toLocaleString('en-US')} bytes.`,
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

export function asAmfArchiveError(error: OpenScadImportDataError): OpenScadImportDataError {
  return new OpenScadImportDataError(
    error.code,
    error.message.replaceAll('3MF', 'AMF'),
    'amf',
    error.limit,
    error.actual,
  )
}

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
