import { readFileSync } from 'node:fs'
import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbsAsync } from '../src/language/rushGraphNurbsKernel'

const source = readFileSync('examples/rush/affine-profile-domain.r', 'utf8')

it('retains rational curve positions after an affine change of domain', async () => {
  const { document } = compileRushFrontend(source)
  const original = document.nodes[0]!.id
  const mapped = document.nodes.find(n => n.op === 'curve_reparameterize')!.id
  const parameters = [0, 0.125, 0.5, 0.875, 1]
  const evaluations = parameters.flatMap(u => [{ node: original, u }, { node: mapped, u: 2 + 4 * u }])
  const result = await buildOwnNurbsAsync(document, { action: 'evaluate', evaluations })
  for (let i = 0; i < result.evaluations.length; i += 2) {
    expect(result.evaluations[i + 1]!.point).toEqual(result.evaluations[i]!.point)
  }
})

it('rejects a collapsed parameter domain', async () => {
  const { document } = compileRushFrontend(source.replace('[2,6]', '[2,2]'))
  await expect(buildOwnNurbsAsync(document, { action: 'build' })).rejects.toThrow(/finite and increasing/)
})
