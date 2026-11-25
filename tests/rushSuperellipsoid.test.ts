import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbs } from '../src/language/rushGraphNurbsKernel'
import { superellipsoidPatches } from '../src/kernel/nurbsConstructors'

const source = readFileSync('examples/rush/superellipsoid-patches.r', 'utf8')
const build = (code: string) => buildOwnNurbs(compileRushFrontend(code).document, { action: 'build' })

it('renders every signed octant with the requested center and semiaxes', () => {
  const positions = build(source).mesh!.positions
  expect(positions.length).toBeGreaterThan(0)
  for (const [axis, center, radius] of [[0,1,4],[1,2,3],[2,3,2]] as const) {
    let lo = Infinity, hi = -Infinity
    for (let i = axis as number; i < positions.length; i += 3) {
      lo = Math.min(lo, positions[i]!)
      hi = Math.max(hi, positions[i]!)
    }
    expect(lo).toBeCloseTo(center - radius, 6)
    expect(hi).toBeCloseTo(center + radius, 6)
  }
})

it('reports the exhausted cell budget and preserves geometry in all eight octants', () => {
  const limited = superellipsoidPatches([1,2,3],[4,3,2],[0.8,1.25],1e-9,8)
  expect(limited.report).toMatchObject({stop:'cellBudget', cells:8, subdivisions:1, roundingCertified:false, continuousBound:false})
  expect(limited.report.analyticInterpolationEstimateMm).toBeGreaterThan(1e-9)
  expect(new Set(limited.report.domains.map(p => p.signs.join(','))).size).toBe(8)
  expect(limited.patches).toHaveLength(8)
  const coarse = superellipsoidPatches([1,2,3],[4,3,2],[1,1],100,8)
  expect(coarse.report.stop).toBe('analyticEstimateReached')
  expect(() => build(source.replace('0.8,1.25','0.8mm,1.25'))).toThrow()
})
