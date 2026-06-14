/**
 * 3MF export from MeshData[].
 * 3MF is a ZIP archive containing XML files that describe 3D geometry.
 * We build the ZIP manually (stored/uncompressed) to avoid dependencies.
 */
import type { MeshData } from './openscadParser'
import type { Mat4 } from './math3d'

/** Apply a row-major 4x4 transform to a 3D point. */
function transformPoint(t: Mat4, x: number, y: number, z: number): [number, number, number] {
  return [
    t[0] * x + t[1] * y + t[2] * z + t[3],
    t[4] * x + t[5] * y + t[6] * z + t[7],
    t[8] * x + t[9] * y + t[10] * z + t[11],
  ]
}

/* ── CRC-32 ──────────────────────────────────────── */

const crcTable = new Uint32Array(256)
for (let n = 0; n < 256; n++) {
  let c = n
  for (let k = 0; k < 8; k++) {
    c = c & 1 ? 0xedb88320 ^ (c >>> 1) : c >>> 1
  }
  crcTable[n] = c
}

function crc32(data: Uint8Array): number {
  let crc = 0xffffffff
  for (let i = 0; i < data.length; i++) {
    crc = crcTable[(crc ^ data[i]) & 0xff] ^ (crc >>> 8)
  }
  return (crc ^ 0xffffffff) >>> 0
}

/* ── ZIP builder (stored / no compression) ───────── */

interface ZipEntry {
  name: Uint8Array
  data: Uint8Array
  crc: number
  offset: number
}

function encodeUTF8(s: string): Uint8Array {
  return new TextEncoder().encode(s)
}

function buildZip(files: { name: string; data: string }[]): Uint8Array {
  const entries: ZipEntry[] = []

  // Calculate total size for pre-allocation
  let localHeadersSize = 0
  const encodedFiles = files.map(f => {
    const name = encodeUTF8(f.name)
    const data = encodeUTF8(f.data)
    localHeadersSize += 30 + name.length + data.length
    return { name, data }
  })

  let centralDirSize = 0
  for (const f of encodedFiles) {
    centralDirSize += 46 + f.name.length
  }

  const totalSize = localHeadersSize + centralDirSize + 22
  const buf = new Uint8Array(totalSize)
  const view = new DataView(buf.buffer)
  let offset = 0

  // Write local file headers + data
  for (const f of encodedFiles) {
    const crc = crc32(f.data)
    entries.push({ name: f.name, data: f.data, crc, offset })

    // Local file header signature
    view.setUint32(offset, 0x04034b50, true); offset += 4
    // Version needed to extract
    view.setUint16(offset, 20, true); offset += 2
    // General purpose bit flag
    view.setUint16(offset, 0, true); offset += 2
    // Compression method (0 = stored)
    view.setUint16(offset, 0, true); offset += 2
    // Last mod file time
    view.setUint16(offset, 0, true); offset += 2
    // Last mod file date
    view.setUint16(offset, 0, true); offset += 2
    // CRC-32
    view.setUint32(offset, crc, true); offset += 4
    // Compressed size
    view.setUint32(offset, f.data.length, true); offset += 4
    // Uncompressed size
    view.setUint32(offset, f.data.length, true); offset += 4
    // Filename length
    view.setUint16(offset, f.name.length, true); offset += 2
    // Extra field length
    view.setUint16(offset, 0, true); offset += 2
    // Filename
    buf.set(f.name, offset); offset += f.name.length
    // File data
    buf.set(f.data, offset); offset += f.data.length
  }

  // Central directory
  const centralDirOffset = offset
  for (const entry of entries) {
    // Central directory file header signature
    view.setUint32(offset, 0x02014b50, true); offset += 4
    // Version made by
    view.setUint16(offset, 20, true); offset += 2
    // Version needed to extract
    view.setUint16(offset, 20, true); offset += 2
    // General purpose bit flag
    view.setUint16(offset, 0, true); offset += 2
    // Compression method
    view.setUint16(offset, 0, true); offset += 2
    // Last mod file time
    view.setUint16(offset, 0, true); offset += 2
    // Last mod file date
    view.setUint16(offset, 0, true); offset += 2
    // CRC-32
    view.setUint32(offset, entry.crc, true); offset += 4
    // Compressed size
    view.setUint32(offset, entry.data.length, true); offset += 4
    // Uncompressed size
    view.setUint32(offset, entry.data.length, true); offset += 4
    // Filename length
    view.setUint16(offset, entry.name.length, true); offset += 2
    // Extra field length
    view.setUint16(offset, 0, true); offset += 2
    // File comment length
    view.setUint16(offset, 0, true); offset += 2
    // Disk number start
    view.setUint16(offset, 0, true); offset += 2
    // Internal file attributes
    view.setUint16(offset, 0, true); offset += 2
    // External file attributes
    view.setUint32(offset, 0, true); offset += 4
    // Relative offset of local header
    view.setUint32(offset, entry.offset, true); offset += 4
    // Filename
    buf.set(entry.name, offset); offset += entry.name.length
  }

  const centralDirLen = offset - centralDirOffset

  // End of central directory record
  view.setUint32(offset, 0x06054b50, true); offset += 4
  // Number of this disk
  view.setUint16(offset, 0, true); offset += 2
  // Disk where central directory starts
  view.setUint16(offset, 0, true); offset += 2
  // Number of central directory records on this disk
  view.setUint16(offset, entries.length, true); offset += 2
  // Total number of central directory records
  view.setUint16(offset, entries.length, true); offset += 2
  // Size of central directory
  view.setUint32(offset, centralDirLen, true); offset += 4
  // Offset of start of central directory
  view.setUint32(offset, centralDirOffset, true); offset += 4
  // Comment length
  view.setUint16(offset, 0, true); offset += 2

  return buf
}

/* ── XML helpers ─────────────────────────────────── */

function escapeXml(s: string): string {
  return s.replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;')
}

function formatNum(n: number): string {
  // Avoid -0 and excessive precision
  return (Object.is(n, -0) ? 0 : n).toFixed(6)
}

/* ── 3MF content generation ──────────────────────── */

function buildContentTypes(): string {
  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">',
    '  <Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml" />',
    '  <Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml" />',
    '</Types>',
  ].join('\n')
}

function buildRels(): string {
  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">',
    '  <Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel" />',
    '</Relationships>',
  ].join('\n')
}

function buildModel(meshes: MeshData[]): string {
  // Combine all meshes into a single vertex/triangle list
  const vertexLines: string[] = []
  const triangleLines: string[] = []
  let vertexOffset = 0

  for (const m of meshes) {
    const verts = m.vertices // interleaved: pos(3) + normal(3), stride 6
    const indices = m.indices
    const t = m.transform
    const vertCount = verts.length / 6

    // Write transformed vertices
    for (let i = 0; i < vertCount; i++) {
      const x = verts[i * 6], y = verts[i * 6 + 1], z = verts[i * 6 + 2]
      const p = transformPoint(t, x, y, z)
      vertexLines.push(`        <vertex x="${formatNum(p[0])}" y="${formatNum(p[1])}" z="${formatNum(p[2])}" />`)
    }

    // Write triangles with global index offset
    for (let i = 0; i < indices.length; i += 3) {
      const v1 = indices[i] + vertexOffset
      const v2 = indices[i + 1] + vertexOffset
      const v3 = indices[i + 2] + vertexOffset
      triangleLines.push(`        <triangle v1="${v1}" v2="${v2}" v3="${v3}" />`)
    }

    vertexOffset += vertCount
  }

  return [
    '<?xml version="1.0" encoding="UTF-8"?>',
    '<model unit="millimeter" xml:lang="en-US" xmlns="http://schemas.microsoft.com/3dmanufacturing/core/2015/02">',
    '  <resources>',
    '    <object id="1" type="model">',
    '      <mesh>',
    '        <vertices>',
    ...vertexLines,
    '        </vertices>',
    '        <triangles>',
    ...triangleLines,
    '        </triangles>',
    '      </mesh>',
    '    </object>',
    '  </resources>',
    '  <build>',
    '    <item objectid="1" />',
    '  </build>',
    '</model>',
  ].join('\n')
}

/* ── Public API ───────────────────────────────────── */

/**
 * Generate a 3MF file from mesh data and trigger a download.
 */
export function export3MF(meshes: MeshData[], filename = 'model.3mf') {
  const zipData = buildZip([
    { name: '[Content_Types].xml', data: buildContentTypes() },
    { name: '_rels/.rels', data: buildRels() },
    { name: '3D/3dmodel.model', data: buildModel(meshes) },
  ])

  const blob = new Blob([zipData.buffer as ArrayBuffer], { type: 'application/vnd.ms-package.3dmanufacturing' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = filename
  a.click()
  URL.revokeObjectURL(url)
}
