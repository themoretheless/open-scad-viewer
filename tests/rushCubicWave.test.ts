import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbs } from '../src/language/rushGraphNurbsKernel'

const source = readFileSync('examples/rush/cubic-wave-surface.r', 'utf8')
const build = (code: string) => buildOwnNurbs(compileRushFrontend(code).document, { action: 'build' })

it('tessellates the authored Hermite wave through packaged Rush and WASM', () => {
  const mesh = build(source).mesh!
  expect(mesh.indices.length).toBeGreaterThan(0)
  const heights: number[] = []
  for (let i = 0; i < mesh.positions.length; i += 3) {
    const x = mesh.positions[i]!, y = mesh.positions[i + 1]!, z = mesh.positions[i + 2]!
    const half = Math.min(3, Math.floor(x / 6))
    const t = x / 6 - half
    expect(y).toBeGreaterThanOrEqual(0)
    expect(y).toBeLessThanOrEqual(8)
    expect(z).toBeCloseTo(2 + 3 * (-1) ** half * (1 - 6 * t ** 2 + 4 * t ** 3), 5)
    heights.push(z)
  }
  expect(Math.min(...heights)).toBeCloseTo(-1)
  expect(Math.max(...heights)).toBeCloseTo(5)
})

it('rejects incorrect dimensions and out-of-domain period counts', () => {
  expect(() => build(source.replace('3mm', '3deg'))).toThrow()
  expect(() => build(source.replace('periods:2', 'periods:2mm'))).toThrow()
  expect(() => build(source.replace('periods:2', 'periods:6'))).toThrow(/period/i)
})
