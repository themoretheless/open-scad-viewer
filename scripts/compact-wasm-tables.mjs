// Compact long, smooth runs of 32-bit table entries. Reconstruction uses
// wrapping integer arithmetic in the original linear memory before any export runs.
import binaryen from 'binaryen'
import {Buffer} from 'node:buffer'

const words = 64
const blockBytes = 4 * words
const packedBytes = 8 + words - 2

function unsigned(n) {
  const bytes = []
  do {
    const part = n % 128
    n = Math.floor(n / 128)
    bytes.push(part | (n ? 128 : 0))
  } while (n)
  return Buffer.from(bytes)
}

function readUnsigned(bytes, cursor) {
  let value = 0
  for (let shift = 0; shift < 35 && cursor.at < bytes.length; shift += 7) {
    const part = bytes[cursor.at++]
    value += (part & 127) * 2 ** shift
    if (!(part & 128)) return value
  }
  throw new Error('Malformed WASM length')
}

function sections(bytes) {
  const list = []
  const cursor = {at: 8}
  while (cursor.at < bytes.length) {
    const id = bytes[cursor.at++]
    const size = readUnsigned(bytes, cursor)
    const stop = cursor.at + size
    if (stop > bytes.length) throw new Error('Truncated WASM section')
    list.push({id, data: bytes.subarray(cursor.at, stop)})
    cursor.at = stop
  }
  return list
}

function appendEntry(data, entry) {
  const cursor = {at: 0}
  const count = readUnsigned(data, cursor)
  return Buffer.concat([unsigned(count + 1), data.subarray(cursor.at), entry])
}

function compactTable(bytes, base) {
  const pieces = []
  const runs = []
  let rawStart = 0
  function append(kind, data) {
    if (!data.length) return
    pieces.push(data)
    if (runs.at(-1)?.kind === kind) runs.at(-1).length += data.length
    else runs.push({kind, length: data.length})
  }
  for (let at = (4 - base % 4) % 4; at + blockBytes <= bytes.length; at += blockBytes) {
    const residuals = Buffer.allocUnsafe(packedBytes)
    let value = bytes.readUInt32LE(at)
    let delta = (bytes.readUInt32LE(at + 4) - value) | 0
    residuals.writeUInt32LE(value, 0)
    residuals.writeInt32LE(delta, 4)
    value = bytes.readUInt32LE(at + 4)
    let i = 2
    for (; i < words; i++) {
      const next = bytes.readUInt32LE(at + i * 4)
      const difference = (next - value) | 0
      const residual = (difference - delta) | 0
      if (residual < -128 || residual > 127) break
      residuals.writeInt8(residual, 6 + i)
      delta = difference
      value = next
    }
    if (i !== words) continue
    append('raw', bytes.subarray(rawStart, at))
    append('delta', residuals)
    rawStart = at + blockBytes
  }
  append('raw', bytes.subarray(rawStart))
  const length = pieces.reduce((n, part) => n + part.length, 0)
  if (bytes.length - length < 1024 + 32 * runs.length) return
  return {data: Buffer.concat(pieces), runs, offset: base + bytes.length - length}
}

// At each block boundary the unread suffix is no larger than the remaining
// destination. Load each residual before its output store, including the final byte.
function expandBlocks(count) {
  return `(local.set $blocks (i32.const ${count}))
  (loop $blocksLoop
    (local.set $value (i32.load (local.get $src)))
    (local.set $delta (i32.load offset=4 (local.get $src)))
    (i32.store (local.get $dst) (local.get $value))
    (local.set $value (i32.add (local.get $value) (local.get $delta)))
    (i32.store offset=4 (local.get $dst) (local.get $value))
    (local.set $src (i32.add (local.get $src) (i32.const 8)))
    (local.set $dst (i32.add (local.get $dst) (i32.const 8)))
    (local.set $left (i32.const ${words - 2}))
    (loop $word
      (local.set $delta (i32.add (local.get $delta) (i32.load8_s (local.get $src))))
      (local.set $value (i32.add (local.get $value) (local.get $delta)))
      (i32.store (local.get $dst) (local.get $value))
      (local.set $src (i32.add (local.get $src) (i32.const 1)))
      (local.set $dst (i32.add (local.get $dst) (i32.const 4)))
      (br_if $word (local.tee $left (i32.sub (local.get $left) (i32.const 1)))))
    (br_if $blocksLoop (local.tee $blocks (i32.sub (local.get $blocks) (i32.const 1)))))`
}

export function compactWasmTables(input) {
  const original = Buffer.from(input.buffer, input.byteOffset, input.byteLength)
  const list = sections(original)
  const section = id => list.find(s => s.id === id)
  // This packer targets the standalone geometry kernel's single owned memory
  // and constant active data segments. Other valid module layouts pass through.
  if (section(2) || section(8) || !section(1) || !section(3) || !section(10) ||
      !section(11) || section(5)?.data[0] !== 1) return original
  const data = section(11).data, cursor = {at: 0}
  const count = readUnsigned(data, cursor)
  let previousEnd = 0
  const encoded = [unsigned(count)], init = []
  for (let segment = 0; segment < count; segment++) {
    const start = cursor.at
    if (readUnsigned(data, cursor) !== 0 || data[cursor.at++] !== 0x41) return original
    const base = readUnsigned(data, cursor)
    if ((data[cursor.at - 1] & 64) || data[cursor.at++] !== 11 || base < previousEnd) return original
    const length = readUnsigned(data, cursor), end = cursor.at + length
    if (end > data.length || base + length >= 0x40000000) return original
    previousEnd = base + length
    const compact = length >= 2048 && compactTable(data.subarray(cursor.at, end), base)
    cursor.at = end
    if (!compact) {
      encoded.push(data.subarray(start, end))
      continue
    }
    // Positive signed LEB needs a terminating zero when bit 6 of the last byte is set.
    let offset = unsigned(compact.offset)
    if (offset.at(-1) & 64) {
      offset[offset.length - 1] |= 128
      offset = Buffer.concat([offset, Buffer.from([0])])
    }
    encoded.push(Buffer.from([0, 0x41]), offset, Buffer.from([11]), unsigned(compact.data.length), compact.data)
    let src = compact.offset, dst = base
    for (const run of compact.runs) {
      if (run.kind === 'raw') {
        if (src !== dst) init.push(`(memory.copy (i32.const ${dst}) (i32.const ${src}) (i32.const ${run.length}))`)
        dst += run.length
      } else {
        init.push(`(local.set $src (i32.const ${src})) (local.set $dst (i32.const ${dst}))`, expandBlocks(run.length / packedBytes))
        dst += run.length / packedBytes * blockBytes
      }
      src += run.length
    }
  }
  if (!init.length || cursor.at !== data.length) return original
  const helper = binaryen.parseText(`(module (memory 1) (func (export "init")
    (local $src i32) (local $dst i32) (local $value i32) (local $delta i32)
    (local $blocks i32) (local $left i32) ${init.join('\n')}))`)
  let body
  try {
    helper.setFeatures(binaryen.Features.BulkMemory | binaryen.Features.MVP)
    if (!helper.validate()) throw new Error('Invalid static table initializer')
    const code = sections(Buffer.from(helper.emitBinary())).find(s => s.id === 10).data
    if (code[0] !== 1) throw new Error('Expected a single table initializer')
    body = code.subarray(1)
  } finally { helper.dispose() }
  const type = readUnsigned(section(1).data, {at: 0})
  const index = readUnsigned(section(3).data, {at: 0})
  section(1).data = appendEntry(section(1).data, Buffer.from([0x60, 0, 0]))
  section(3).data = appendEntry(section(3).data, unsigned(type))
  section(10).data = appendEntry(section(10).data, body)
  section(11).data = Buffer.concat(encoded)
  list.splice(list.findIndex(s => [9,12,10].includes(s.id)), 0, {id: 8, data: unsigned(index)})
  const result = Buffer.concat([original.subarray(0,8), ...list.flatMap(s => [Buffer.from([s.id]), unsigned(s.data.length), s.data])])
  if (result.length >= original.length) return original
  const reference = new WebAssembly.Module(original)
  if (!WebAssembly.Module.exports(reference).some(e => e.kind === 'memory' && e.name === 'memory')) return original
  const before = new WebAssembly.Instance(reference).exports.memory
  const after = new WebAssembly.Instance(new WebAssembly.Module(result)).exports.memory
  if (!Buffer.from(before.buffer).equals(Buffer.from(after.buffer))) {
    throw new Error('Static table compression changed initial geometry memory')
  }
  return result
}
