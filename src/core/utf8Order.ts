/** Byte-wise UTF-8 ordering, the canonical key order of semantic programs. */

const UTF8 = new TextEncoder()

function utf8CompareEncoded(left: string, right: string): number {
  const leftBytes = UTF8.encode(left)
  const rightBytes = UTF8.encode(right)
  const shared = Math.min(leftBytes.length, rightBytes.length)
  for (let index = 0; index < shared; index++) {
    if (leftBytes[index] !== rightBytes[index]) return leftBytes[index] - rightBytes[index]
  }
  return leftBytes.length - rightBytes.length
}

function isAsciiFrom(value: string, start: number): boolean {
  for (let index = start; index < value.length; index++) if (value.charCodeAt(index) >= 0x80) return false
  return true
}

export function utf8Compare(left: string, right: string): number {
  // While both strings are ASCII their UTF-8 bytes are their code units, so this returns
  // exactly what comparing the encoded bytes would; anything else takes the encoded path.
  const shared = Math.min(left.length, right.length)
  for (let index = 0; index < shared; index++) {
    const leftUnit = left.charCodeAt(index)
    const rightUnit = right.charCodeAt(index)
    if (leftUnit >= 0x80 || rightUnit >= 0x80) return utf8CompareEncoded(left, right)
    if (leftUnit !== rightUnit) return leftUnit - rightUnit
  }
  if (!isAsciiFrom(left, shared) || !isAsciiFrom(right, shared)) return utf8CompareEncoded(left, right)
  return left.length - right.length
}

/** The length of TextEncoder's output for `value`, without encoding it (lone surrogates count as U+FFFD). */
export function utf8ByteLength(value: string): number {
  let length = 0
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit < 0x80) length += 1
    else if (unit < 0x800) length += 2
    else if (unit >= 0xd800 && unit <= 0xdbff && index + 1 < value.length) {
      const next = value.charCodeAt(index + 1)
      if (next >= 0xdc00 && next <= 0xdfff) { length += 4; index++ } else length += 3
    } else length += 3
  }
  return length
}
