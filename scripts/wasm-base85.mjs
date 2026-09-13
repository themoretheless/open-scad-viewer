// Fixed-width base85 for quoted JS literals; alphabet excludes quotes/backslashes.
export const alphabet = '0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ.-:+=^!/*?&<>()[]{}@%$#'
const rank = new Map(Array.from(alphabet, (c, i) => [c, i]))
function word(value) {
  const digits = new Array(5)
  for (let i = 4; i >= 0; i--) { digits[i] = alphabet[value % 85]; value = Math.floor(value / 85) }
  return digits.join('')
}
export function encodeBase85(bytes) {
  if (bytes.length > 0xffffffff) throw new Error('Base85 size overflow')
  const parts = ['b85:', word(bytes.length)]
  for (let i = 0; i < bytes.length; i += 4) {
    let value = 0
    for (let j = 0; j < 4; j++) value = value * 256 + (bytes[i+j] ?? 0)
    parts.push(word(value))
  }
  return parts.join('')
}
export function decodeBase85(text, maximum = 4 * 1024 * 1024 + 4) {
  if (!text.startsWith('b85:') || text.length < 9 || text.length > 9 + Math.ceil(maximum / 4) * 5) throw new Error('Base85 size limit')
  const valueAt = start => {
    let value = 0
    for (let i = 0; i < 5; i++) {
      const digit = rank.get(text[start+i])
      if (digit === undefined) throw new Error('Invalid base85 character')
      value = value * 85 + digit
    }
    if (value > 0xffffffff) throw new Error('Base85 word overflow')
    return value
  }
  const size = valueAt(4)
  if (size > maximum || text.length !== 9 + Math.ceil(size / 4) * 5) throw new Error('Base85 size mismatch')
  const bytes = new Uint8Array(size)
  for (let offset = 0, start = 9; offset < size; offset += 4, start += 5) {
    let value = valueAt(start)
    const block = new Uint8Array(4)
    for (let j = 3; j >= 0; j--) { block[j] = value % 256; value = Math.floor(value / 256) }
    for (let j = 0; j < 4; j++) {
      if (offset+j < size) bytes[offset+j] = block[j]
      else if (block[j] !== 0) throw new Error('Noncanonical base85 padding')
    }
  }
  return bytes
}
