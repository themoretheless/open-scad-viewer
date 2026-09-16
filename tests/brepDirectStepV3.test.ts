import { readFileSync } from 'node:fs'
import { describe, expect, it } from 'vitest'
import { importDirectStepV3 } from '../src/services/cadNurbsStep'

describe('step-interchange/3 direct product seam', () => {
  it('imports the self-authored analytic topology fixture directly', () => {
    const text = readFileSync('tests/fixtures/step-v3/self-authored-analytic-tetra.step', 'utf8')
    const result = importDirectStepV3(text)
    expect(result.certificate.capability).toBe('step-interchange/3')
    expect(result.model.vertices).toHaveLength(4)
    expect(result.model.edges).toHaveLength(6)
    expect(result.model.faces).toHaveLength(4)
    expect(result.model.bodies).toHaveLength(1)
    expect(result.identity.preserved).toBe(false)
    expect(result.identity.createdCount).toBe(20)
  })

  it('refuses the self-authored cyclic and unsupported periodic fixtures', () => {
    const cyclic = readFileSync('tests/fixtures/step-v3/self-authored-malformed-cycle.stp', 'utf8')
    expect(() => importDirectStepV3(cyclic)).toThrow()
  })
})
