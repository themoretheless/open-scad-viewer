import { describe, expect, it } from 'vitest'
import { toWboitShader } from '../src/render/gpuWboit'
import { MESH_WGSL } from '../src/render/shaders'
import {
  accumulateWboit,
  compositeWboit,
  HALF_FLOAT_MAX,
  sortedOverReference,
  WBOIT_WEIGHT_MAX,
  wboitWeight,
  type Rgb,
  type WboitFragment,
} from '../src/render/wboitReference'

const layers: WboitFragment[] = [
  { color: [1, 0, 0], alpha: 0.35, depth: 0.2 },
  { color: [0, 1, 0], alpha: 0.5, depth: 0.5 },
  { color: [0, 0, 1], alpha: 0.65, depth: 0.8 },
]
const background: Rgb = [0.9, 0.9, 0.9]

function permutations<T>(items: readonly T[]): T[][] {
  if (items.length <= 1) return [[...items]]
  return items.flatMap((item, index) =>
    permutations([...items.slice(0, index), ...items.slice(index + 1)]).map(rest => [item, ...rest]),
  )
}

describe('WBOIT CPU reference qualification', () => {
  it('pins the reference weight to the WGSL emitted by toWboitShader', () => {
    const wgsl = toWboitShader(MESH_WGSL)
    expect(wgsl).toContain('let b = 1.0 - depth * 0.95;')
    expect(wgsl).toContain('return clamp(5e2 * b * b * b, 1e-2, 5e2);')
    expect(wgsl).toContain('vec4f(c.rgb * alpha, alpha) * w')
    expect(wgsl).toContain('vec4f(alpha, 0.0, 0.0, 0.0)')
  })

  it('accumulates premultiplied weighted color and multiplies revealage', () => {
    const pixel = accumulateWboit(layers)
    const weights = layers.map(layer => wboitWeight(layer.depth, layer.alpha))
    expect(pixel.accum[3]).toBeCloseTo(
      layers.reduce((sum, layer, index) => sum + layer.alpha * weights[index], 0),
      6,
    )
    expect(pixel.accum[0]).toBeCloseTo(layers[0].alpha * weights[0], 6)
    expect(pixel.reveal).toBeCloseTo(0.65 * 0.5 * 0.35, 12)
  })

  it('is order independent for three overlapping layers', () => {
    const reference = compositeWboit(accumulateWboit(layers), background)
    for (const order of permutations(layers)) {
      const result = compositeWboit(accumulateWboit(order), background)
      for (let channel = 0; channel < 3; channel++) expect(result[channel]).toBeCloseTo(reference[channel], 9)
    }
  })

  it('matches exact over-compositing for a single layer', () => {
    const single = [layers[1]]
    const wboit = compositeWboit(accumulateWboit(single), background)
    const exact = sortedOverReference(single, background)
    for (let channel = 0; channel < 3; channel++) expect(wboit[channel]).toBeCloseTo(exact[channel], 9)
  })

  it('keeps coverage exact and color inside the energy bounds of its inputs', () => {
    const pixel = accumulateWboit(layers)
    const coverage = 1 - pixel.reveal
    // Coverage equals sorted compositing's coverage: 1 - prod(1 - alpha), in [max alpha, 1].
    expect(coverage).toBeGreaterThanOrEqual(Math.max(...layers.map(layer => layer.alpha)))
    expect(coverage).toBeLessThanOrEqual(1)
    const result = compositeWboit(pixel, background)
    for (let channel = 0; channel < 3; channel++) {
      const inputs = [...layers.map(layer => layer.color[channel]), background[channel]]
      expect(result[channel]).toBeGreaterThanOrEqual(Math.min(...inputs) - 1e-12)
      expect(result[channel]).toBeLessThanOrEqual(Math.max(...inputs) + 1e-12)
    }
    // Fully transparent pixels are discarded and keep the opaque background.
    expect(compositeWboit(accumulateWboit([]), background)).toEqual(background)
  })

  it('prefers nearer layers and does not saturate with alpha', () => {
    // The weight falls monotonically with depth for any alpha.
    for (const alpha of [0.1, 0.25, 0.5, 1]) {
      expect(wboitWeight(0, alpha)).toBe(WBOIT_WEIGHT_MAX)
      expect(wboitWeight(0.25, alpha)).toBeGreaterThan(wboitWeight(0.5, alpha))
      expect(wboitWeight(0.5, alpha)).toBeGreaterThan(wboitWeight(0.75, alpha))
      expect(wboitWeight(0.75, alpha)).toBeGreaterThan(wboitWeight(1, alpha))
    }
    // Swapping which layer is in front moves the result towards the front layer,
    // as sorted compositing does.
    const redNear = [
      { ...layers[0], depth: 0.01 },
      { ...layers[2], depth: 0.99 },
    ]
    const blueNear = [
      { ...layers[0], depth: 0.99 },
      { ...layers[2], depth: 0.01 },
    ]
    const a = compositeWboit(accumulateWboit(redNear), background)
    const b = compositeWboit(accumulateWboit(blueNear), background)
    expect(a[0]).toBeGreaterThan(b[0])
    expect(b[2]).toBeGreaterThan(a[2])
  })

  it('keeps rgba16float headroom for 131 opaque layers at the weight cap', () => {
    expect(Math.floor(HALF_FLOAT_MAX / WBOIT_WEIGHT_MAX)).toBe(131)
    const opaque = (count: number) =>
      accumulateWboit(Array.from({ length: count }, () => ({ color: [1, 1, 1] as Rgb, alpha: 1, depth: 0 })))
    expect(opaque(131).accum[3]).toBeLessThanOrEqual(HALF_FLOAT_MAX)
    expect(opaque(132).accum[3]).toBeGreaterThan(HALF_FLOAT_MAX)
  })
})
