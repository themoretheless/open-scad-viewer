import { describe, expect, it } from 'vitest'
import {
  chooseDepthCandidate,
  MAX_NORMALIZED_DEPTH_CANDIDATES,
  normalizeDepthCandidates,
  type DepthCandidate,
} from '../src/services/selectionCycling'

function candidate(key: string, distance: number): DepthCandidate<string> {
  return { key, distance, value: key }
}

describe('depth selection cycling', () => {
  it('sorts front-to-back, removes mode-level duplicates and applies a budget', () => {
    expect(normalizeDepthCandidates([
      candidate('back', 8),
      candidate('front', 2),
      candidate('front', 3),
      candidate('middle', 5),
      candidate('invalid', Number.NaN),
    ], 2).map(item => item.key)).toEqual(['front', 'middle'])
  })

  it('cycles repeated clicks and wraps while the candidate signature is stable', () => {
    const candidates = [candidate('a', 1), candidate('b', 2), candidate('c', 3)]
    const first = chooseDepthCandidate(candidates, 100, 80, null, 1_000)
    const second = chooseDepthCandidate(candidates, 102, 81, first.state, 1_400)
    const third = chooseDepthCandidate(candidates, 101, 80, second.state, 1_700)
    const wrapped = chooseDepthCandidate(candidates, 100, 80, third.state, 1_900)

    expect([first, second, third, wrapped].map(result => result.candidate?.key)).toEqual(['a', 'b', 'c', 'a'])
  })

  it('resets after movement, timeout or a changed target list', () => {
    const candidates = [candidate('a', 1), candidate('b', 2)]
    const first = chooseDepthCandidate(candidates, 20, 20, null, 100)

    expect(chooseDepthCandidate(candidates, 30, 20, first.state, 200).candidate?.key).toBe('a')
    expect(chooseDepthCandidate(candidates, 20, 20, first.state, 2_000).candidate?.key).toBe('a')
    expect(chooseDepthCandidate([candidate('x', 1)], 20, 20, first.state, 200).candidate?.key).toBe('x')
  })

  it('keeps the original click anchor instead of drifting across repeated clicks', () => {
    const candidates = [candidate('a', 1), candidate('b', 2)]
    const first = chooseDepthCandidate(candidates, 20, 20, null, 100)
    const second = chooseDepthCandidate(candidates, 24, 20, first.state, 200)
    const third = chooseDepthCandidate(candidates, 28, 20, second.state, 300)

    expect(second.state).toMatchObject({ x: 20, y: 20, index: 1 })
    expect(third.candidate?.key).toBe('a')
    expect(third.state).toMatchObject({ x: 28, y: 20, index: 0 })
  })

  it('rejects empty and non-finite interactions', () => {
    expect(chooseDepthCandidate([], 0, 0, null, 0)).toEqual({ candidate: null, state: null })
    expect(chooseDepthCandidate([candidate('a', 1)], Number.NaN, 0, null, 0)).toEqual({ candidate: null, state: null })
    expect(normalizeDepthCandidates([candidate('', 1), candidate('negative', -1)])).toEqual([])
  })

  it('enforces its own hard cap even when callers request an excessive maximum', () => {
    const candidates = Array.from({ length: 100 }, (_, index) => candidate(String(index), index))
    expect(normalizeDepthCandidates(candidates, Number.MAX_SAFE_INTEGER)).toHaveLength(MAX_NORMALIZED_DEPTH_CANDIDATES)
  })

  it('keeps the nearest duplicate even when it arrives after the bounded window', () => {
    const candidates = [
      candidate('duplicate', 1_000),
      ...Array.from({ length: 40 }, (_, index) => candidate(`near-${index}`, index + 1)),
      candidate('duplicate', 0.5),
    ]
    const normalized = normalizeDepthCandidates(candidates, 4)

    expect(normalized.map(item => item.key)).toEqual(['duplicate', 'near-0', 'near-1', 'near-2'])
    expect(normalized[0].distance).toBe(0.5)
  })

})
