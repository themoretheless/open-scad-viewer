import {readFileSync} from 'node:fs'
import {describe, expect, it} from 'vitest'
import {candidateExactSign, generatePredicateCandidateCorpus, renderPredicateCandidateJson,
  renderPredicateCandidateRust} from './support/predicateCandidateCorpus'

describe('independent native predicate candidate corpus', () => {
  it('reproduces the readable and native fixtures from independent exact rational arithmetic', () => {
    expect(readFileSync(new URL('./fixtures/predicate-candidate-corpus-v1.json', import.meta.url), 'utf8')).toBe(renderPredicateCandidateJson())
    expect(readFileSync(new URL('./fixtures/predicate-candidate-corpus-v1.rs', import.meta.url), 'utf8')).toBe(renderPredicateCandidateRust())
  })

  it('covers all three predicates, every sign, source bit extremes and canonical rational constants', () => {
    const corpus = generatePredicateCandidateCorpus()
    expect(new Set(corpus.map(test => test.id)).size).toBe(corpus.length)
    expect(corpus.filter(test => test.mustDecide).length).toBeGreaterThan(100)
    for (const predicate of ['orient2d', 'orient3d', 'compare_squared_distance']) {
      expect(new Set(corpus.filter(test => test.predicate === predicate).map(test => test.expectedSign))).toEqual(new Set([-1, 0, 1]))
    }
    for (const test of corpus) expect(test.expectedSign, test.id).toBe(candidateExactSign(test))
    expect(corpus.flatMap(test => test.leaves).some(leaf => leaf.kind === 'binary64' && leaf.bits === '8000000000000000')).toBe(true)
    expect(corpus.flatMap(test => test.leaves).some(leaf => leaf.kind === 'binary64' && leaf.bits === '0000000000000001')).toBe(true)
    expect(corpus.flatMap(test => test.leaves).some(leaf => leaf.kind === 'binary64' && leaf.bits === '7fefffffffffffff')).toBe(true)
    expect(corpus.flatMap(test => test.leaves).some(leaf => leaf.kind === 'rational' && leaf.denominator === '3')).toBe(true)
  })

  it('contains cases that fail rounded determinant and rounded rational shortcuts', () => {
    const cases = new Map(generatePredicateCandidateCorpus().map(test => [test.id, test]))
    expect((1 + Number.EPSILON) * (1 - Number.EPSILON) - 1).toBe(0)
    expect(cases.get('o2-cancel-one-bit-012')?.expectedSign).toBe(-1)
    expect(cases.get('o3-dense-cancellation-0123')?.expectedSign).toBe(1)
    expect(cases.get('d3-cancel-one-bit')?.expectedSign).toBe(1)
    expect(cases.get('d2-binary-third-vs-rational')?.expectedSign).toBe(-1)
    expect(cases.get('d2-rational-third-vs-binary')?.expectedSign).toBe(1)
    expect(cases.get('d2-u64-rational-boundary')?.expectedSign).toBe(1)
    expect(cases.get('o2-i64-min-rational')?.expectedSign).toBe(-1)
    for (const test of cases.values()) {
      if (test.tags.includes('must-not-zero')) expect(test.expectedSign, test.id).not.toBe(0)
    }
  })
})
