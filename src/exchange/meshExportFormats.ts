import {
  export3mfInKernel,
  export3mfModelInKernel,
  exportMeshArtifactInKernel,
} from '../kernel/geometry/meshArtifactExport'
import { type NurbsMesh } from '../kernel/geometry/tessellation'
import type { ExportPartMetadata } from './meshExportAdapter'
import { MESH_EXPORT_FORMATS, type MeshExportFormat } from './meshFormats'
import { crc32 } from './openScadImport/parseOpenScadAmf'

export { MESH_EXPORT_FORMATS, type MeshExportFormat }

const MAX_EXPORT_BYTES = 4 * 1024 * 1024
const UTF8_ENCODER = new TextEncoder()
const UTF8_DECODER = new TextDecoder()

const OPC_TYPES_BYTES = UTF8_ENCODER.encode(
  '<?xml version="1.0" encoding="UTF-8"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="model" ContentType="application/vnd.ms-package.3dmanufacturing-3dmodel+xml"/></Types>',
)
const OPC_RELS_BYTES = UTF8_ENCODER.encode(
  '<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Target="/3D/3dmodel.model" Id="rel0" Type="http://schemas.microsoft.com/3dmanufacturing/2013/01/3dmodel"/></Relationships>',
)
const OPC_PATHS = [
  UTF8_ENCODER.encode('[Content_Types].xml'),
  UTF8_ENCODER.encode('_rels/.rels'),
  UTF8_ENCODER.encode('3D/3dmodel.model'),
] as const

type ExportMesh = NurbsMesh & {
  parts?: NurbsMesh[]
  partMeta?: readonly ExportPartMetadata[]
  title?: string
}

function escapeXml(value: string): string {
  return value
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&apos;')
}

function sanitizeObjIdent(value: string): string {
  return value.trim().replace(/[^A-Za-z0-9_.-]+/g, '_') || 'part'
}

function formatNumber(n: number): string {
  const clean = Math.abs(n) <= 1e-12 ? 0 : n
  return Number.isInteger(clean) ? String(clean) : String(Number(clean.toPrecision(15)))
}

function rgbaTo3mfHex(color: readonly [number, number, number, number]): string {
  const hexByte = (v: number) =>
    Math.max(0, Math.min(255, Math.round(v * 255)))
      .toString(16)
      .toUpperCase()
      .padStart(2, '0')
  const [r, g, b, a] = color
  const base = `#${hexByte(r)}${hexByte(g)}${hexByte(b)}`
  return a < 0.999 ? `${base}${hexByte(a)}` : base
}

function collectMultiPartGroups(partMeta: readonly ExportPartMetadata[]) {
  const buckets = new Map<string, { name: string; partIds: number[] }>()
  for (let i = 0; i < partMeta.length; i++) {
    const meta = partMeta[i]!
    if (!meta.groupKey) continue
    const partId = i + 1
    const existing = buckets.get(meta.groupKey)
    if (existing) existing.partIds.push(partId)
    else buckets.set(meta.groupKey, { name: meta.groupName ?? 'assembly', partIds: [partId] })
  }
  const groups: Array<{ key: string; id: number; name: string; partIds: number[] }> = []
  const groupByKey = new Map<string, { id: number; name: string; partIds: number[] }>()
  let nextId = partMeta.length + 1
  for (const [key, value] of buckets) {
    if (value.partIds.length < 2) continue
    const group = { key, id: nextId++, name: value.name, partIds: value.partIds }
    groups.push(group)
    groupByKey.set(key, group)
  }
  return { groups, groupByKey }
}

function enrich3mfModelXml(baseXml: string, partMeta: readonly ExportPartMetadata[], title?: string): string {
  const { groups, groupByKey } = collectMultiPartGroups(partMeta)
  const matResId = partMeta.length + groups.length + 1

  const palette: Array<{ name: string; displayColor: string }> = []
  const paletteIndexByHex = new Map<string, number>()
  for (const meta of partMeta) {
    if (!meta.color) continue
    const displayColor = rgbaTo3mfHex(meta.color)
    if (!paletteIndexByHex.has(displayColor)) {
      paletteIndexByHex.set(displayColor, palette.length)
      palette.push({ name: `color_${palette.length + 1}`, displayColor })
    }
  }

  const metadataXml =
    `${title ? `<metadata name="Title">${escapeXml(title)}</metadata>` : ''}` +
    `<metadata name="Application">OpenSCAD Viewer</metadata>`
  const basematerialsXml =
    palette.length > 0
      ? `<basematerials id="${matResId}">${palette
          .map(m => `<base name="${escapeXml(m.name)}" displaycolor="${m.displayColor}"/>`)
          .join('')}</basematerials>`
      : ''

  let xml = baseXml.replace('<resources>', `${metadataXml}<resources>${basematerialsXml}`)

  for (let i = 0; i < partMeta.length; i++) {
    const id = i + 1
    const meta = partMeta[i]!
    const pindex = meta.color ? paletteIndexByHex.get(rgbaTo3mfHex(meta.color)) : undefined
    const pidAttr = pindex !== undefined ? ` pid="${matResId}" pindex="${pindex}"` : ''
    xml = xml.replace(
      `<object id="${id}" name="Part ${id}" type="model">`,
      `<object id="${id}" name="${escapeXml(meta.name)}" type="model"${pidAttr}>`,
    )
  }

  if (groups.length > 0) {
    const groupsXml = groups
      .map(
        g =>
          `<object id="${g.id}" name="${escapeXml(g.name)}" type="model"><components>${g.partIds
            .map(pid => `<component objectid="${pid}"/>`)
            .join('')}</components></object>`,
      )
      .join('')
    xml = xml.replace('</resources>', `${groupsXml}</resources>`)
  }

  const buildIdx = xml.indexOf('<build>')
  if (buildIdx >= 0) {
    const seenGroups = new Set<string>()
    const items: string[] = []
    for (let i = 0; i < partMeta.length; i++) {
      const key = partMeta[i]!.groupKey
      const grp = key ? groupByKey.get(key) : undefined
      if (grp) {
        if (seenGroups.has(key!)) continue
        seenGroups.add(key!)
        items.push(`<item objectid="${grp.id}"/>`)
      } else {
        items.push(`<item objectid="${i + 1}"/>`)
      }
    }
    xml = `${xml.slice(0, buildIdx)}<build>${items.join('')}</build></model>`
  }
  return xml
}

async function deflateRawBytes(input: Uint8Array): Promise<Uint8Array> {
  const copy = new Uint8Array(input)
  const stream = new Blob([copy.buffer]).stream().pipeThrough(new CompressionStream('deflate-raw'))
  const reader = stream.getReader()
  const chunks: Uint8Array[] = []
  let total = 0
  while (true) {
    const { value, done } = await reader.read()
    if (done) break
    if (value && value.byteLength > 0) {
      chunks.push(value)
      total += value.byteLength
      if (total > MAX_EXPORT_BYTES) {
        await reader.cancel()
        throw new Error('Export exceeds 4 MiB.')
      }
    }
  }
  const out = new Uint8Array(total)
  let offset = 0
  for (const chunk of chunks) {
    out.set(chunk, offset)
    offset += chunk.byteLength
  }
  return out
}

function build3mfZip(
  rawFiles: readonly [Uint8Array, Uint8Array, Uint8Array],
  payloads: readonly [Uint8Array, Uint8Array, Uint8Array],
  compressed: boolean,
): Uint8Array {
  const overhead = 22 + OPC_PATHS.reduce((sum, name) => sum + 76 + 2 * name.byteLength, 0)
  const totalPayload = payloads.reduce((sum, p) => sum + p.byteLength, 0)
  const totalBytes = overhead + totalPayload
  if (totalBytes > MAX_EXPORT_BYTES) throw new Error('Export exceeds 4 MiB.')

  const out = new Uint8Array(totalBytes)
  const view = new DataView(out.buffer)
  const method = compressed ? 8 : 0
  let cursor = 0
  const offsets: number[] = []
  const crcs: number[] = []

  for (let i = 0; i < 3; i++) {
    const name = OPC_PATHS[i]!
    const raw = rawFiles[i]!
    const payload = payloads[i]!
    const crc = crc32(raw)
    offsets.push(cursor)
    crcs.push(crc)

    view.setUint32(cursor, 0x04034b50, true)
    view.setUint16(cursor + 4, 20, true)
    view.setUint16(cursor + 6, 0x0800, true)
    view.setUint16(cursor + 8, method, true)
    view.setUint16(cursor + 12, 33, true)
    view.setUint32(cursor + 14, crc, true)
    view.setUint32(cursor + 18, payload.byteLength, true)
    view.setUint32(cursor + 22, raw.byteLength, true)
    view.setUint16(cursor + 26, name.byteLength, true)
    out.set(name, cursor + 30)
    cursor += 30 + name.byteLength
    out.set(payload, cursor)
    cursor += payload.byteLength
  }

  const dirOffset = cursor
  for (let i = 0; i < 3; i++) {
    const name = OPC_PATHS[i]!
    const raw = rawFiles[i]!
    const payload = payloads[i]!
    view.setUint32(cursor, 0x02014b50, true)
    view.setUint16(cursor + 4, 20, true)
    view.setUint16(cursor + 6, 20, true)
    view.setUint16(cursor + 8, 0x0800, true)
    view.setUint16(cursor + 10, method, true)
    view.setUint16(cursor + 14, 33, true)
    view.setUint32(cursor + 16, crcs[i]!, true)
    view.setUint32(cursor + 20, payload.byteLength, true)
    view.setUint32(cursor + 24, raw.byteLength, true)
    view.setUint16(cursor + 28, name.byteLength, true)
    view.setUint32(cursor + 42, offsets[i]!, true)
    out.set(name, cursor + 46)
    cursor += 46 + name.byteLength
  }

  const dirSize = cursor - dirOffset
  view.setUint32(cursor, 0x06054b50, true)
  view.setUint16(cursor + 8, 3, true)
  view.setUint16(cursor + 10, 3, true)
  view.setUint32(cursor + 12, dirSize, true)
  view.setUint32(cursor + 16, dirOffset, true)
  return out
}

function buildNamedObj(mesh: ExportMesh): Uint8Array {
  const parts = mesh.parts!
  const partMeta = mesh.partMeta!
  const lines: string[] = ['# RushGraph; units: millimeter']
  let currentGroup: string | undefined
  let vertexBase = 1
  for (let i = 0; i < parts.length; i++) {
    const part = parts[i]!
    const meta = partMeta[i]!
    if (meta.groupName !== currentGroup) {
      currentGroup = meta.groupName
      lines.push(`g ${sanitizeObjIdent(currentGroup ?? 'default')}`)
    }
    lines.push(`o ${sanitizeObjIdent(meta.name)}`)
    const p = part.positions
    for (let v = 0; v < p.length; v += 3) {
      lines.push(`v ${formatNumber(p[v]!)} ${formatNumber(p[v + 1]!)} ${formatNumber(p[v + 2]!)}`)
    }
    const idx = part.indices
    for (let t = 0; t < idx.length; t += 3) {
      lines.push(`f ${idx[t]! + vertexBase} ${idx[t + 1]! + vertexBase} ${idx[t + 2]! + vertexBase}`)
    }
    vertexBase += p.length / 3
  }
  lines.push('')
  const bytes = UTF8_ENCODER.encode(lines.join('\n'))
  if (bytes.byteLength > MAX_EXPORT_BYTES) throw new Error('Export exceeds 4 MiB.')
  return bytes
}

function buildNamedAmf(mesh: ExportMesh): Uint8Array {
  // Re-run native part validation (closed, positive volume, consistent orientation, budgets).
  export3mfModelInKernel(mesh)
  const parts = mesh.parts!
  const partMeta = mesh.partMeta!
  const { groups } = collectMultiPartGroups(partMeta)
  const chunks: string[] = ['<?xml version="1.0" encoding="UTF-8"?><amf unit="millimeter" version="1.1">']
  for (let i = 0; i < parts.length; i++) {
    const part = parts[i]!
    const meta = partMeta[i]!
    chunks.push(`<object id="${i + 1}"><metadata type="name">${escapeXml(meta.name)}</metadata><mesh><vertices>`)
    const p = part.positions
    for (let v = 0; v < p.length; v += 3) {
      chunks.push(
        `<vertex><coordinates><x>${formatNumber(p[v]!)}</x><y>${formatNumber(p[v + 1]!)}</y><z>${formatNumber(p[v + 2]!)}</z></coordinates></vertex>`,
      )
    }
    chunks.push('</vertices><volume>')
    const idx = part.indices
    for (let t = 0; t < idx.length; t += 3) {
      chunks.push(`<triangle><v1>${idx[t]!}</v1><v2>${idx[t + 1]!}</v2><v3>${idx[t + 2]!}</v3></triangle>`)
    }
    chunks.push('</volume></mesh></object>')
  }
  for (const g of groups) {
    chunks.push(`<constellation id="${g.id}"><metadata type="name">${escapeXml(g.name)}</metadata>`)
    for (const pid of g.partIds) chunks.push(`<instance objectid="${pid}"/>`)
    chunks.push('</constellation>')
  }
  chunks.push('</amf>')
  const bytes = UTF8_ENCODER.encode(chunks.join(''))
  if (bytes.byteLength > MAX_EXPORT_BYTES) throw new Error('Export exceeds 4 MiB.')
  return bytes
}

function hasPartMetadata(mesh: ExportMesh): boolean {
  return Boolean(mesh.parts && mesh.partMeta && mesh.parts.length > 0 && mesh.parts.length === mesh.partMeta.length)
}

function exportNon3mf(
  mesh: ExportMesh,
  format: Exclude<MeshExportFormat, '3mf'>,
): {
  data: Uint8Array
  mimeType: string
  extension: string
} {
  if (format === 'obj') {
    const raw = exportMeshArtifactInKernel(mesh, 'obj')
    const data = hasPartMetadata(mesh) ? buildNamedObj(mesh) : raw
    return { data, mimeType: 'model/obj', extension: 'obj' }
  }
  if (format === 'amf') {
    const data =
      hasPartMetadata(mesh) && (mesh.parts?.length ?? 0) > 1
        ? buildNamedAmf(mesh)
        : exportMeshArtifactInKernel(mesh, 'amf')
    return { data, mimeType: 'application/amf+xml', extension: 'amf' }
  }
  const data = exportMeshArtifactInKernel(mesh, format)
  const mimeType = format === 'stl' || format === 'stl_binary' ? 'model/stl' : 'application/octet-stream'
  return { data, mimeType, extension: format === 'stl_binary' ? 'stl' : format }
}

function buildEnriched3mfModelBytes(mesh: ExportMesh): Uint8Array {
  const baseXml = UTF8_DECODER.decode(export3mfModelInKernel(mesh))
  return UTF8_ENCODER.encode(enrich3mfModelXml(baseXml, mesh.partMeta!, mesh.title))
}

export function meshExportBase64(data: Uint8Array) {
  let binary = ''
  for (let i = 0; i < data.length; i += 8192) binary += String.fromCharCode(...data.subarray(i, i + 8192))
  return btoa(binary)
}

function checkArtifact<T extends { data: Uint8Array }>(artifact: T): T {
  if (artifact.data.length > MAX_EXPORT_BYTES) throw new Error('Export exceeds 4 MiB.')
  return artifact
}

export function exportMeshFormat(mesh: ExportMesh, format: MeshExportFormat) {
  if (!MESH_EXPORT_FORMATS.includes(format)) throw new Error('Unsupported mesh export format.')
  if (format !== '3mf') return checkArtifact(exportNon3mf(mesh, format))
  if (!hasPartMetadata(mesh)) {
    return checkArtifact({ data: export3mfInKernel(mesh, false), mimeType: 'model/3mf', extension: '3mf' })
  }
  const modelBytes = buildEnriched3mfModelBytes(mesh)
  const rawFiles: readonly [Uint8Array, Uint8Array, Uint8Array] = [OPC_TYPES_BYTES, OPC_RELS_BYTES, modelBytes]
  return checkArtifact({
    data: build3mfZip(rawFiles, rawFiles, false),
    mimeType: 'model/3mf',
    extension: '3mf',
  })
}

/** Rust owns geometry validation; lossless DEFLATE packages standard or enriched 3MF. */
export async function exportMeshFormatCompressed(mesh: ExportMesh, format: MeshExportFormat) {
  if (!MESH_EXPORT_FORMATS.includes(format)) throw new Error('Unsupported mesh export format.')
  if (format !== '3mf') return checkArtifact(exportNon3mf(mesh, format))
  if (!hasPartMetadata(mesh)) {
    return checkArtifact({ data: export3mfInKernel(mesh, true), mimeType: 'model/3mf', extension: '3mf' })
  }
  const modelBytes = buildEnriched3mfModelBytes(mesh)
  const rawFiles: readonly [Uint8Array, Uint8Array, Uint8Array] = [OPC_TYPES_BYTES, OPC_RELS_BYTES, modelBytes]
  const payloads: readonly [Uint8Array, Uint8Array, Uint8Array] = [
    await deflateRawBytes(OPC_TYPES_BYTES),
    await deflateRawBytes(OPC_RELS_BYTES),
    await deflateRawBytes(modelBytes),
  ]
  return checkArtifact({
    data: build3mfZip(rawFiles, payloads, true),
    mimeType: 'model/3mf',
    extension: '3mf',
  })
}
