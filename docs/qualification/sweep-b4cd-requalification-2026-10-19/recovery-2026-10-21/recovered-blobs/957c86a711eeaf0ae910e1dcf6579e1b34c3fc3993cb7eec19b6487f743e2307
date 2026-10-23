/**
 * Lightweight mesh format registry shared by the toolbar, the workbenches, the
 * converter and the MCP tool. Keep this module free of decoders so eager
 * importers stay small; the parsers live in `meshImport.ts`.
 */
export const MESH_IMPORT_FORMATS = ['stl', 'obj', 'ply', 'off', 'amf', '3mf'] as const
export type MeshImportFormat = typeof MESH_IMPORT_FORMATS[number]

export const MESH_IMPORT_MAX_BYTES = 20_000_000

export const MESH_IMPORT_EXTENSIONS: Readonly<Record<MeshImportFormat, readonly string[]>> = Object.freeze({
  stl: ['stl'],
  obj: ['obj'],
  ply: ['ply'],
  off: ['off'],
  amf: ['amf', 'amf.gz'],
  '3mf': ['3mf'],
})

const MESH_IMPORT_MIME_TYPES: Readonly<Record<MeshImportFormat, readonly string[]>> = Object.freeze({
  stl: ['model/stl', 'application/sla', 'application/vnd.ms-pki.stl'],
  obj: ['model/obj'],
  ply: ['application/x-ply', 'model/ply'],
  off: ['model/off'],
  amf: ['application/amf+xml', 'application/x-amf'],
  '3mf': ['model/3mf', 'application/vnd.ms-package.3dmanufacturing-3dmodel+xml'],
})

/** `accept` attribute value for file inputs that take any importable mesh. */
export const MESH_IMPORT_ACCEPT = MESH_IMPORT_FORMATS
  .flatMap(format => [...MESH_IMPORT_EXTENSIONS[format].map(extension => `.${extension}`), ...MESH_IMPORT_MIME_TYPES[format]])
  .filter((value, index, all) => all.indexOf(value) === index)
  .join(',')

export const MESH_EXPORT_FORMATS = ['stl', 'stl_binary', '3mf', 'obj', 'ply', 'off', 'amf'] as const
export type MeshExportFormat = typeof MESH_EXPORT_FORMATS[number]

/** Human-readable labels shared by the UI and MCP descriptions. */
export const MESH_FORMAT_LABELS: Readonly<Record<MeshExportFormat | MeshImportFormat, string>> = Object.freeze({
  stl: 'STL (ASCII)',
  stl_binary: 'STL (binary)',
  obj: 'OBJ',
  ply: 'PLY',
  off: 'OFF',
  amf: 'AMF',
  '3mf': '3MF',
})

/** Formats whose writers require closed, consistently oriented geometry. */
export const MESH_PRINTING_FORMATS: readonly MeshExportFormat[] = Object.freeze(['stl', 'stl_binary', '3mf', 'amf'])

export function isMeshExportFormat(value: string): value is MeshExportFormat {
  return (MESH_EXPORT_FORMATS as readonly string[]).includes(value)
}

/** Detect the import format from the file name, then from content sniffing. */
export function detectMeshImportFormat(fileName: string, bytes?: Uint8Array): MeshImportFormat | null {
  const lower = fileName.toLowerCase()
  for (const format of MESH_IMPORT_FORMATS) {
    if (MESH_IMPORT_EXTENSIONS[format].some(extension => lower.endsWith(`.${extension}`))) return format
  }
  if (!bytes || bytes.byteLength < 4) return null
  const head = asciiPrefix(bytes, 512).trimStart().toLowerCase()
  if (bytes[0] === 0x50 && bytes[1] === 0x4b && bytes[2] === 0x03 && bytes[3] === 0x04) return '3mf'
  if (head.startsWith('ply')) return 'ply'
  if (/^(off|noff|coff|ncoff|4off|off\s)/u.test(head)) return 'off'
  if (head.startsWith('solid')) return 'stl'
  if (head.startsWith('<?xml') || head.startsWith('<amf')) return head.includes('<amf') ? 'amf' : null
  if (/^(#|v |vn |vt |o |g |mtllib |usemtl )/mu.test(head)) return 'obj'
  if (bytes.byteLength >= 84) {
    const view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength)
    if (84 + view.getUint32(80, true) * 50 === bytes.byteLength) return 'stl'
  }
  return null
}

/** Strip a recognised mesh extension so a display name can be derived. */
export function stripMeshExtension(fileName: string): string {
  const lower = fileName.toLowerCase()
  for (const format of MESH_IMPORT_FORMATS) {
    for (const extension of MESH_IMPORT_EXTENSIONS[format]) {
      if (lower.endsWith(`.${extension}`)) return fileName.slice(0, fileName.length - extension.length - 1)
    }
  }
  return fileName
}

/** Latin-1 view of the first `length` bytes, for header sniffing. */
export function asciiPrefix(bytes: Uint8Array, length: number): string {
  let out = ''
  for (let i = 0; i < Math.min(length, bytes.byteLength); i++) out += String.fromCharCode(bytes[i])
  return out
}
