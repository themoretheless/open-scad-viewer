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

// Encode non-overlapping active segments in their own memory range.
function compactMatches(bytes, base, extended) {
  const last = new Int32Array(65536).fill(-1)
  const chain = extended ? new Int32Array(65536).fill(-1) : null
  const payload = []
  let gain = 0, stop = 0, end = 0
  let start = 0
  function literal(end) {
    while (start < end) {
      const length = Math.min(128, end - start)
      payload.push(length - 1, ...bytes.subarray(start, start + length))
      start += length
    }
  }
  for (let at = 0; at + 4 <= bytes.length;) {
    const key = Math.imul(bytes.readUInt32LE(at), 0x9e3779b1) >>> 16
    let previous = last[key]
    if (chain) chain[at & 65535] = previous
    last[key] = at
    let length = 0, best = previous
    for (let probe = 0; probe < (chain ? 48 : 1) && previous >= 0 && at - previous <= 65535; probe++) {
      let matched = 0
      while (matched < 131 && at + matched < bytes.length && bytes[previous + matched] === bytes[at + matched]) matched++
      if (matched > length) {
        length = matched
        best = previous
        if (length === 131) break
      }
      if (chain) previous = chain[previous & 65535]
    }
    if (length < 5) { at++; continue }
    literal(at)
    const offset = at - best
    payload.push(128 + length - 4, offset & 255, offset >>> 8)
    if (chain) {
      for (let j = at + 1; j < at + length && j + 4 <= bytes.length; j++) {
        const hash = Math.imul(bytes.readUInt32LE(j), 0x9e3779b1) >>> 16
        chain[j & 65535] = last[hash]
        last[hash] = j
      }
    }
    at += length
    start = at
    if (at - payload.length > gain) {
      gain = at - payload.length
      stop = payload.length
      end = at
    }
  }
  // Keep the suffix raw from the last point of maximum accumulated saving.
  // Every previous output boundary then falls at or before the unread input.
  if (gain < (extended ? 200 : 1024)) return
  const data = Buffer.concat([Buffer.from(payload.slice(0,stop)),bytes.subarray(end)])
  const runs = [{kind:'lz', length:stop, output:end}]
  if (end < bytes.length) runs.push({kind:'raw',length:bytes.length-end})
  return {data, runs, offset:base+gain}
}

function expandMatches(src,dst,length) {
  return `(local.set $src (i32.const ${src}))
    (local.set $dst (i32.const ${dst}))
    ${expandMatchTokens(`(i32.const ${src+length})`)}`
}

function expandMatchTokens(limit) {
  return `(loop $tokenLoop
      (local.set $left (i32.load8_u (local.get $src)))
      (local.set $src (i32.add (local.get $src) (i32.const 1)))
      (if (i32.lt_u (local.get $left) (i32.const 128))
        (then
          (local.set $left (i32.add (local.get $left) (i32.const 1)))
          (memory.copy (local.get $dst) (local.get $src) (local.get $left))
          (local.set $src (i32.add (local.get $src) (local.get $left)))
          (local.set $dst (i32.add (local.get $dst) (local.get $left))))
        (else
          (local.set $left (i32.sub (local.get $left) (i32.const 124)))
          (local.set $delta (i32.load16_u (local.get $src)))
          (local.set $src (i32.add (local.get $src) (i32.const 2)))
          (loop $copy
            (i32.store8 (local.get $dst) (i32.load8_u (i32.sub (local.get $dst) (local.get $delta))))
            (local.set $dst (i32.add (local.get $dst) (i32.const 1)))
            (local.set $left (i32.sub (local.get $left) (i32.const 1)))
            (br_if $copy (local.get $left)))))
      (br_if $tokenLoop (i32.lt_u (local.get $src) ${limit})))`
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
  ${expandBlockWords()}`
}

function expandBlockWords() {
  return `(loop $blocksLoop
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
  const standard = compact(input, false)
  if (standard.length <= 20 * 1024 * 1024) return standard
  const denser = compact(input, true)
  if (denser.length <= 20 * 1024 * 1024) return denser
  const layered = compact(input, true, true)
  const best = [standard, denser, layered].reduce((a, b) => a.length <= b.length ? a : b)
  if (best.length <= 20 * 1024 * 1024) return best
  const shared = compact(input, true, true, true)
  return shared.length < best.length ? shared : best
}

function compact(input, extended, layerTables = false, shareDecoders = false) {
  const original = Buffer.from(input.buffer, input.byteOffset, input.byteLength)
  const list = sections(original)
  const section = id => list.find(s => s.id === id)
  // This packer targets the standalone geometry kernel's single owned memory
  // and constant active data segments. Other valid module layouts pass through.
  if (section(2) || section(8) || !section(1) || !section(3) || !section(10) ||
      !section(11) || section(5)?.data[0] !== 1) return original
  const data = section(11).data
  const cursor = {at: 0}
  const count = readUnsigned(data, cursor)
  let previousEnd = 0
  const encoded = [unsigned(count)], init = []
  const calls = []
  const emitLz = (src, dst, length) => {
    if (shareDecoders) calls.push({op:'lz', args:[src,dst,src+length]})
    else init.push(expandMatches(src,dst,length))
  }
  for (let segment = 0; segment < count; segment++) {
    const start = cursor.at
    if (readUnsigned(data, cursor) !== 0 || data[cursor.at++] !== 0x41) return original
    const base = readUnsigned(data, cursor)
    if ((data[cursor.at - 1] & 64) || data[cursor.at++] !== 11 || base < previousEnd) return original
    const length = readUnsigned(data, cursor), end = cursor.at + length
    if (end > data.length || base + length >= 0x40000000) return original
    previousEnd = base + length
    const segmentBytes = data.subarray(cursor.at,end)
    let compact = length >= 2048 && compactTable(segmentBytes,base)
    const matches = length >= (extended ? 256 : 2048) && compactMatches(segmentBytes, base, extended)
    // First reconstruct the compact delta table at its original staging offset.
    // Any raw suffix is already in place at the end of the segment.
    if (compact && layerTables) {
      const table = compactMatches(compact.data, compact.offset, true)
      if (table) {
        compact = {...compact, data: table.data, encodedOffset: table.offset,
          prefix: [table.offset, compact.offset, table.runs[0].length]}
      }
    }
    if (matches && (!compact || matches.data.length + 128 < compact.data.length)) compact = matches
    cursor.at = end
    if (!compact) {
      encoded.push(data.subarray(start, end))
      continue
    }
    // Positive signed LEB needs a terminating zero when bit 6 of the last byte is set.
    let offset = unsigned(compact.encodedOffset ?? compact.offset)
    if (offset.at(-1) & 64) {
      offset[offset.length - 1] |= 128
      offset = Buffer.concat([offset, Buffer.from([0])])
    }
    encoded.push(Buffer.from([0, 0x41]), offset, Buffer.from([11]), unsigned(compact.data.length), compact.data)
    if (compact.prefix) emitLz(...compact.prefix)
    let src = compact.offset, dst = base
    for (const run of compact.runs) {
      if (run.kind === 'lz') {
        emitLz(src,dst,run.length)
        dst += run.output
      } else if (run.kind === 'raw') {
        if (src !== dst && shareDecoders) calls.push({op:'copy',args:[dst,src,run.length]})
        else if (src !== dst) init.push(`(memory.copy (i32.const ${dst}) (i32.const ${src}) (i32.const ${run.length}))`)
        dst += run.length
      } else {
        if (shareDecoders) calls.push({op:'blocks',args:[src,dst,run.length / packedBytes]})
        else init.push(`(local.set $src (i32.const ${src})) (local.set $dst (i32.const ${dst}))`, expandBlocks(run.length / packedBytes))
        dst += run.length / packedBytes * blockBytes
      }
      src += run.length
    }
  }
  if ((!init.length && !calls.length) || cursor.at !== data.length) return original
  const index = readUnsigned(section(3).data, {at: 0})
  const type = readUnsigned(section(1).data, {at: 0})
  let startIndex = index
  if (shareDecoders) {
    startIndex = index + 2
    installSharedDecoders(section, type, index, calls)
  } else {
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
  section(1).data = appendEntry(section(1).data, Buffer.from([0x60, 0, 0]))
  section(3).data = appendEntry(section(3).data, unsigned(type))
  section(10).data = appendEntry(section(10).data, body)
  }
  section(11).data = Buffer.concat(encoded)
  list.splice(list.findIndex(s => [9,12,10].includes(s.id)), 0, {id: 8, data: unsigned(startIndex)})
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

// Emit one pair of decoder functions. The start function supplies only the
// addresses and bounds for each run. All original function indices stay valid.
function installSharedDecoders(section, type, index, calls) {
  const helper = binaryen.parseText(`(module (memory 1)
    (func (param $src i32) (param $dst i32) (param $limit i32)
      (local $left i32) (local $delta i32) ${expandMatchTokens('(local.get $limit)')})
    (func (param $src i32) (param $dst i32) (param $blocks i32)
      (local $left i32) (local $delta i32) (local $value i32) ${expandBlockWords()}))`)
  try {
    helper.setFeatures(binaryen.Features.BulkMemory | binaryen.Features.MVP)
    if (!helper.validate()) throw new Error('Invalid shared geometry initializer')
    const code = sections(Buffer.from(helper.emitBinary())).find(s => s.id === 10).data
    const cursor = {at:0}
    if (readUnsigned(code,cursor)!==2) throw new Error('Expected two geometry decoders')
    section(1).data = appendEntry(section(1).data,Buffer.from([0x60,3,0x7f,0x7f,0x7f,0]))
    for(let n=0;n<2;n++) {
      const pos=cursor.at
      const size=readUnsigned(code,cursor)
      cursor.at+=size
      section(3).data=appendEntry(section(3).data,unsigned(type))
      section(10).data=appendEntry(section(10).data,code.subarray(pos,cursor.at))
    }
    const start=[0] // No locals in the start function.
    for(const {op,args} of calls) {
      for(const arg of args) {
        const bytes=[...unsigned(arg)]
        if(bytes.at(-1)&64){bytes[bytes.length-1]|=128; bytes.push(0)}
        start.push(0x41,...bytes)
      }
      if(op==='copy') start.push(0xfc,10,0,0)
      else start.push(0x10,...unsigned(index+(op==='blocks'?1:0)))
    }
    start.push(11)
    section(1).data=appendEntry(section(1).data,Buffer.from([0x60,0,0]))
    section(3).data=appendEntry(section(3).data,unsigned(type+1))
    section(10).data=appendEntry(section(10).data,Buffer.concat([unsigned(start.length),Buffer.from(start)]))
  } finally { helper.dispose() }
}

// All source segments must be ordered, active initializers of owned memory.
// Keep the data segment count: every active segment is dropped at instantiation.
export function compactAdjacentWasmTables(input) {
  const original = Buffer.from(input.buffer, input.byteOffset, input.byteLength)
  const joined = coalesceStaticSegments(original, 512)
  if (joined === original) return original
  const candidate = compact(joined, true, true, true)
  if (candidate.length >= original.length) return original
  const memory = bytes => new WebAssembly.Instance(new WebAssembly.Module(bytes)).exports.memory
  const reference = memory(original)
  if (!(reference instanceof WebAssembly.Memory)) return original
  if (!Buffer.from(reference.buffer).equals(Buffer.from(memory(candidate).buffer))) {
    throw new Error('Coalescing static segments changed initial geometry memory')
  }
  return candidate
}
function coalesceStaticSegments(input, maxGap) {
  const bytes = input
  const list = sections(bytes)
  if(list.some(s=>[2,8].includes(s.id)) || list.find(s=>s.id===5)?.data[0]!==1) return bytes
  const data = list.find(s=>s.id===11)
  if(!data) return bytes
  const cursor={at:0}, groups=[]
  const count=readUnsigned(data.data,cursor)
  let end=0
  for(let i=0;i<count;i++){
    if(readUnsigned(data.data,cursor)!==0 || data.data[cursor.at++]!==65) return bytes
    const base=readUnsigned(data.data,cursor)
    if((data.data[cursor.at-1]&64) || data.data[cursor.at++]!==11 || base<end) return bytes
    const length=readUnsigned(data.data,cursor), group=groups.at(-1),gap=base-end
    if(cursor.at+length>data.data.length || base+length>=0x40000000) return bytes
    if(!group || gap>maxGap) groups.push({base,parts:[]})
    else if(gap) group.parts.push(Buffer.alloc(gap))
    groups.at(-1).parts.push(data.data.subarray(cursor.at,cursor.at+length))
    cursor.at+=length
    end=base+length
  }
  if(groups.length===count || cursor.at!==data.data.length) return bytes
  data.data=Buffer.concat([unsigned(count),Buffer.from(Array(count-groups.length).fill([0,65,0,11,0]).flat()),...groups.flatMap(g=>{
    const addr=unsigned(g.base), payload=Buffer.concat(g.parts)
    let offset=addr
    if(addr.at(-1)&64){addr[addr.length-1]|=128;offset=Buffer.concat([addr,Buffer.from([0])])}
    return [Buffer.from([0,65]),offset,Buffer.from([11]),unsigned(payload.length),payload]
  })])
  const result=Buffer.concat([bytes.subarray(0,8),...list.flatMap(s=>[Buffer.from([s.id]),unsigned(s.data.length),s.data])])
  return WebAssembly.validate(result) ? result : bytes
}

// Data indices can only be referenced when a DataCount section is present.
// Otherwise keep their indices using empty passive segments. Both have zero
// accessible bytes after instantiation, including for memory.init and data.drop.
export function pruneEmptyWasmSegments(input) {
  const original = Buffer.from(input.buffer, input.byteOffset, input.byteLength)
  const list = sections(original)
  const section = list.find(s => s.id === 11)
  if (!section) return original
  const indexed = list.some(s => s.id === 12)
  let changed = false
  const cursor = {at:0}, kept = []
  const count = readUnsigned(section.data,cursor)
  for (let i=0;i<count;i++) {
    const start=cursor.at
    if (readUnsigned(section.data,cursor)!==0 || section.data[cursor.at++]!==65) return original
    const offset=readUnsigned(section.data,cursor)
    if (section.data[cursor.at++]!==11) return original
    const length=readUnsigned(section.data,cursor)
    cursor.at+=length
    if (cursor.at>section.data.length) return original
    if (length || offset!==0) kept.push(section.data.subarray(start,cursor.at))
    else {
      changed = true
      if (indexed) kept.push(Buffer.from([1,0]))
    }
  }
  if (cursor.at!==section.data.length || !changed) return original
  section.data=Buffer.concat([unsigned(kept.length),...kept])
  const result=Buffer.concat([original.subarray(0,8),...list.flatMap(s=>[Buffer.from([s.id]),unsigned(s.data.length),s.data])])
  if (!WebAssembly.validate(result)) throw new Error('Invalid compact geometry data section')
  return result
}
