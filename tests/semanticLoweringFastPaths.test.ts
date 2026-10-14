import { createHash } from 'node:crypto'
import { describe, expect, it } from 'vitest'
import { sha256Hex } from '../src/core/sha256'
import { utf8ByteLength, utf8Compare } from '../src/core/utf8Order'
import { lowerOpenSCADToSemanticProgram } from '../src/services/semanticProgramLowerer'
import { encodeSemanticProgram } from '../src/services/semanticProgramCodec'
import { normalizeSemanticProgram } from '../src/services/semanticProgramValidator'
import { warmGeometryKernel } from '../src/services/geometry/kernel'

// The semantic lowering fast paths must be observationally identical to the encode-based
// implementations they replace; these references are those implementations.
const encoder = new TextEncoder()
function referenceCompare(left: string, right: string): number {
  const a = encoder.encode(left)
  const b = encoder.encode(right)
  const shared = Math.min(a.length, b.length)
  for (let index = 0; index < shared; index++) if (a[index] !== b[index]) return a[index] - b[index]
  return a.length - b.length
}

let seed = 0x5eed
const random = () => (seed = (seed * 1_103_515_245 + 12_345) >>> 0) / 2 ** 32
const ranges: Array<[number, number]> = [[0x20, 0x7f], [0x80, 0x7ff], [0x800, 0xd7ff], [0xd800, 0xdbff], [0xdc00, 0xdfff], [0xe000, 0xffff]]
function randomUnit(): string {
  const roll = random()
  if (roll < 0.6) return String.fromCharCode(0x61 + Math.floor(random() * 4))
  if (roll < 0.75) return String.fromCodePoint(0x1f600 + Math.floor(random() * 3))
  const [low, high] = ranges[Math.floor(random() * ranges.length)]
  return String.fromCharCode(low + Math.floor(random() * (high - low + 1)))
}
const randomString = () => Array.from({ length: Math.floor(random() * 8) }, randomUnit).join('')

describe('semantic lowering fast paths', () => {
  it('compares and measures strings exactly like encoding them, including lone surrogates', () => {
    const fixed = ['', 'a', 'ab', 'b', '\u007f', '\u0080', '￿', '\ud800', '\udc00', '😀', 'a😀', 'a￿', 'é', 'é']
    for (const left of fixed) {
      expect(utf8ByteLength(left)).toBe(encoder.encode(left).length)
      for (const right of fixed) expect(utf8Compare(left, right)).toBe(referenceCompare(left, right))
    }
    for (let index = 0; index < 20_000; index++) {
      const left = randomString()
      const right = random() < 0.3 ? left + randomString() : randomString()
      expect(utf8Compare(left, right)).toBe(referenceCompare(left, right))
      expect(utf8ByteLength(left)).toBe(encoder.encode(left).length)
    }
  })

  it('hashes like node:crypto across block boundaries and for text input', () => {
    for (let length = 0; length <= 200; length++) {
      const bytes = Uint8Array.from({ length }, (_, index) => (index * 151 + length) & 0xff)
      expect(sha256Hex(bytes)).toBe(createHash('sha256').update(bytes).digest('hex'))
    }
    const text = 'semantic ключ 漢字 🙂 \ud800'
    expect(sha256Hex(text)).toBe(createHash('sha256').update(encoder.encode(text)).digest('hex'))
  })

  it('returns an already normalized program as is and still attests an exact source', async () => {
    await warmGeometryKernel()
    const source = 'difference(){cube(2);cube(1);}'
    const lowered = lowerOpenSCADToSemanticProgram(source)
    expect(normalizeSemanticProgram(lowered.program)).toBe(lowered.program)
    expect(normalizeSemanticProgram(lowered.program, source)).toBe(lowered.program)
    expect(() => normalizeSemanticProgram(lowered.program, `${source} `)).toThrow()
    // A structurally equal copy is not trusted and takes the full normalization path.
    const copy = JSON.parse(JSON.stringify(lowered.program))
    const normalizedCopy = normalizeSemanticProgram(copy)
    expect(normalizedCopy).not.toBe(copy)
    expect(encodeSemanticProgram(normalizedCopy)).toEqual(encodeSemanticProgram(lowered.program))
  })

  it('cannot be tricked into skipping validation by post-import prototype changes', async () => {
    await warmGeometryKernel()
    const { program } = lowerOpenSCADToSemanticProgram('cube(1);')
    const forged = { ...JSON.parse(JSON.stringify(program)), version: 'forged' }
    const originalHas = WeakSet.prototype.has
    const originalGet = WeakMap.prototype.get
    let forgedError: unknown = null
    let encodedLength = 0
    // Only the calls under test run while the prototypes are patched: vitest itself uses them.
    try {
      WeakSet.prototype.has = () => true
      WeakMap.prototype.get = () => new Uint8Array([1, 2, 3])
      try { normalizeSemanticProgram(forged) } catch (error) { forgedError = error }
      encodedLength = encodeSemanticProgram(program).length
    } finally {
      WeakSet.prototype.has = originalHas
      WeakMap.prototype.get = originalGet
    }
    expect(forgedError).toBeInstanceOf(Error)
    expect(encodedLength).toBe(encodeSemanticProgram(program).length)
    expect(encodedLength).toBeGreaterThan(3)
  })

  it('hands out independent copies of the cached envelope bytes', async () => {
    await warmGeometryKernel()
    const { program, canonicalBytes } = lowerOpenSCADToSemanticProgram('cube([1,2,3]);')
    const first = encodeSemanticProgram(program)
    first.fill(0)
    expect(encodeSemanticProgram(program)).toEqual(canonicalBytes)
  })
})
