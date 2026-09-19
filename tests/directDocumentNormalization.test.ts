import { expect, it } from 'vitest'
import { deepStrictEqual } from 'node:assert'
import { parseDirectDocument, parseDirectDocumentAsync } from '../src/services/directModeling'
import { sampleCurve } from '../src/services/directSketchGeometry'

const wrap = (metadata: string) => `{"version":1,"sketches":[],"bodies":[],"metadata":${metadata}}`
const legacy = (text: string) => JSON.parse(JSON.stringify({ ...JSON.parse(text), curves: [], surfaces: [] }))

it('matches legacy normalization across deterministic mixed JSON trees', () => {
  let seed = 173
  const random = (max: number) => { seed = (Math.imul(seed, 1664525) + 1013904223) >>> 0; return seed % max }
  const leaves = ['-0', '-1e-999', '1e400', '-1e400', '1.2345678901234567', '1e-200', 'null', 'true', 'false', JSON.stringify('\ud800'), '"\\u0000"']
  const keys = ['a', '0', '__proto__', 'constructor', 'toJSON', '\u00e9']
  const tree = (depth: number): string => {
    if (depth === 0 || random(3) === 0) return leaves[random(leaves.length)]!
    if (random(2)) return `[${Array.from({ length: random(5) }, () => tree(depth - 1)).join(',')}]`
    return `{${keys.slice(0, random(keys.length + 1)).map(key => `${JSON.stringify(key)}:${tree(depth - 1)}`).join(',')}}`
  }
  for (let i = 0; i < 128; i++) {
    const text = wrap(tree(5))
    // A JSON data property named constructor must not become a type-identity check.
    deepStrictEqual(parseDirectDocument(text), legacy(text), text)
  }
})

it.each([
  '{"zero":-0,"underflow":-1e-999,"positive":1e400,"negative":-1e400}',
  '[null,true,false,"-0",0,-0,2.5,1e-200,{"nested":[1e400,-0]}]',
  '{"__proto__":{"value":-0},"constructor":1e400,"prototype":[-0],"toJSON":"not callable"}',
  '['.repeat(100) + '{"number":1e400,"zero":-0}' + ']'.repeat(100),
])('preserves JSON-clone normalization for metadata %s', async metadata => {
  const text = wrap(metadata)
  const expected = legacy(text)
  expect(parseDirectDocument(text)).toStrictEqual(expected)
  expect(await parseDirectDocumentAsync(text)).toStrictEqual(expected)
})

it('normalizes geometry numbers without sharing parsed objects between calls', async () => {
  const text = '{"version":1,"sketches":[{"id":"s","name":"s","closed":false,"points":[[-0,0],[1,-0]]}],"bodies":[]}'
  const first = parseDirectDocument(text)
  const second = await parseDirectDocumentAsync(text)
  expect(first).toStrictEqual(legacy(text))
  expect(Object.is(first.sketches[0]!.points[0]![0], -0)).toBe(false)
  first.sketches[0]!.points[0]![0] = 999
  first.curves!.push({} as never)
  expect(second).toStrictEqual(legacy(text))
  expect(parseDirectDocument(text)).toStrictEqual(legacy(text))
})

it('owns regenerated analytic sketch samples independently of other parses', async () => {
  const analytic = { kind: 'circle' as const, center: [0, 0] as [number, number], radius: 2, start: 0, sweep: 360 }
  const sketch = { id: 'circle', name: 'Circle', closed: false, points: [], analytic }
  const input = { version: 1, sketches: [sketch], bodies: [] }
  const text = JSON.stringify(input)
  const expected = legacy(JSON.stringify({ ...input, sketches: [{ ...sketch, closed: true, points: sampleCurve(analytic) }] }))
  const first = parseDirectDocument(text)
  const second = await parseDirectDocumentAsync(text)
  expect(first).toStrictEqual(expected)
  first.sketches[0]!.points[0]![0] = 999
  expect(second).toStrictEqual(expected)
  expect(parseDirectDocument(text)).toStrictEqual(expected)
})

it('keeps deeply nested serializer refusals rather than silently broadening admission', async () => {
  const text = wrap('['.repeat(20_000) + '0' + ']'.repeat(20_000))
  expect(() => legacy(text)).toThrow(RangeError)
  expect(() => parseDirectDocument(text)).toThrow(RangeError)
  await expect(parseDirectDocumentAsync(text)).rejects.toBeInstanceOf(RangeError)
})

it('does not publish geometry made invalid by numeric overflow', async () => {
  const text = '{"version":1,"sketches":[{"id":"s","name":"s","closed":false,"points":[[1e400,0],[1,0]]}],"bodies":[]}'
  expect(() => parseDirectDocument(text)).toThrow('Invalid sketch.')
  await expect(parseDirectDocumentAsync(text)).rejects.toThrow('Invalid sketch.')
})
