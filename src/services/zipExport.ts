/**
 * ZIP export utilities.
 * Reuses the same manual ZIP construction approach as threemfExport.ts.
 */

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

  for (const f of encodedFiles) {
    const crc = crc32(f.data)
    entries.push({ name: f.name, data: f.data, crc, offset })

    view.setUint32(offset, 0x04034b50, true); offset += 4
    view.setUint16(offset, 20, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    view.setUint32(offset, crc, true); offset += 4
    view.setUint32(offset, f.data.length, true); offset += 4
    view.setUint32(offset, f.data.length, true); offset += 4
    view.setUint16(offset, f.name.length, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    buf.set(f.name, offset); offset += f.name.length
    buf.set(f.data, offset); offset += f.data.length
  }

  const centralDirOffset = offset
  for (const entry of entries) {
    view.setUint32(offset, 0x02014b50, true); offset += 4
    view.setUint16(offset, 20, true); offset += 2
    view.setUint16(offset, 20, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    view.setUint32(offset, entry.crc, true); offset += 4
    view.setUint32(offset, entry.data.length, true); offset += 4
    view.setUint32(offset, entry.data.length, true); offset += 4
    view.setUint16(offset, entry.name.length, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    view.setUint16(offset, 0, true); offset += 2
    view.setUint32(offset, 0, true); offset += 4
    view.setUint32(offset, entry.offset, true); offset += 4
    buf.set(entry.name, offset); offset += entry.name.length
  }

  const centralDirLen = offset - centralDirOffset

  view.setUint32(offset, 0x06054b50, true); offset += 4
  view.setUint16(offset, 0, true); offset += 2
  view.setUint16(offset, 0, true); offset += 2
  view.setUint16(offset, entries.length, true); offset += 2
  view.setUint16(offset, entries.length, true); offset += 2
  view.setUint32(offset, centralDirLen, true); offset += 4
  view.setUint32(offset, centralDirOffset, true); offset += 4
  view.setUint16(offset, 0, true); offset += 2

  return buf
}

/* ── Batch export (all tabs as ZIP of .scad files) ─ */

export interface TabData {
  name: string
  code: string
}

export function exportAllTabsAsZip(tabs: TabData[], zipFileName: string = 'openscad-project.zip'): void {
  const files = tabs.map(tab => {
    let fileName = tab.name.replace(/[^a-zA-Z0-9_\-. ]/g, '_')
    if (!fileName.endsWith('.scad')) fileName += '.scad'
    return { name: fileName, data: tab.code }
  })

  const zipData = buildZip(files)
  const blob = new Blob([zipData.buffer as ArrayBuffer], { type: 'application/zip' })
  const url = URL.createObjectURL(blob)
  const a = document.createElement('a')
  a.href = url
  a.download = zipFileName
  document.body.appendChild(a)
  a.click()
  document.body.removeChild(a)
  URL.revokeObjectURL(url)
}
