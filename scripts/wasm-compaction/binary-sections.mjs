import {Buffer} from 'node:buffer'

export function unsigned(n) {
  const bytes = []
  do {
    const part = n % 128
    n = Math.floor(n / 128)
    bytes.push(part | (n ? 128 : 0))
  } while (n)
  return Buffer.from(bytes)
}

export function readUnsigned(bytes, cursor) {
  let value = 0
  for (let shift = 0; shift < 35 && cursor.at < bytes.length; shift += 7) {
    const part = bytes[cursor.at++]
    value += (part & 127) * 2 ** shift
    if (!(part & 128)) return value
  }
  throw new Error('Malformed WASM length')
}

export function sections(bytes) {
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

export function appendEntry(data, entry) {
  const cursor = {at: 0}
  const count = readUnsigned(data, cursor)
  return Buffer.concat([unsigned(count + 1), data.subarray(cursor.at), entry])
}

// Encode non-overlapping active segments in their own memory range.
