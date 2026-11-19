import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbsAsync } from '../src/language/rushGraphNurbsKernel'

const source = readFileSync('examples/rush/rational-loft-tube.r', 'utf8')

it('sews two rational circles into a closed body with planar caps', async () => {
  const { document } = compileRushFrontend(source)
  const { report } = await buildOwnNurbsAsync(document, { action: 'build' })
  expect(report.mesh?.closed).toBe(true)
  expect(report.mesh!.signedVolumeMm3 / (Math.PI * 25 * 12)).toBeCloseTo(1, 2)
})

it('rejects a section with a different number of boundary loops', async () => {
  const { document } = compileRushFrontend(source.replace('[[b]]', '[[b],[b]]'))
  await expect(buildOwnNurbsAsync(document, { action: 'build' })).rejects.toThrow()
})
