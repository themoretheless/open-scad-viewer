// xmlAndArchive: inflateRaw.
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
import { zipPath, crc32 } from './requiredAttributeNs'


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


export interface ZipCentralEntry {
  readonly path: string
  readonly directory: boolean
  readonly flags: number
  readonly method: number
  readonly crc: number
  readonly compressedSize: number
  readonly uncompressedSize: number
  readonly localOffset: number
}


export interface ZipDirectoryInfo {
  readonly endOffset: number
  readonly directoryEnd: number
  readonly entryCount: number
  readonly directorySize: number
  readonly directoryOffset: number
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


export interface Zip64EntryValues {
  readonly uncompressedSize?: number
  readonly compressedSize?: number
  readonly localOffset?: number
  readonly diskStart?: number
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


export function asAmfArchiveError(error: OpenScadImportDataError): OpenScadImportDataError {
  return new OpenScadImportDataError(
    error.code,
    error.message.replaceAll('3MF', 'AMF'),
    'amf',
    error.limit,
    error.actual,
  )
}
