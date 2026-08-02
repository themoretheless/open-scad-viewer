import { describe, expect, it } from 'vitest'
import {
  findLiteralMatches,
  replaceAllLiteral,
  replaceExpectedMatch,
  wrappedMatchIndex,
} from '../src/services/textSearch'

describe('bounded literal text search', () => {
  it('uses exact UTF-16 textarea offsets and treats regex characters literally', () => {
    const source = '😀 cube(1); CUBE(1); a+b a+b'
    expect(findLiteralMatches(source, 'cube(1)').matches.map(match => [match.start, match.end, match.text])).toEqual([
      [3, 10, 'cube(1)'], [12, 19, 'CUBE(1)'],
    ])
    expect(findLiteralMatches(source, 'a+b', { caseSensitive: true }).matches).toHaveLength(2)
  })

  it('is non-overlapping, wraps navigation, and reports truncation', () => {
    expect(findLiteralMatches('aaaa', 'aa').matches.map(match => match.start)).toEqual([0, 2])
    expect(findLiteralMatches('aaaa', 'a', { maxMatches: 2 })).toMatchObject({ truncated: true })
    expect(wrappedMatchIndex(1, 2, 1)).toBe(0)
    expect(wrappedMatchIndex(0, 2, -1)).toBe(1)
    expect(wrappedMatchIndex(0, 0, 1)).toBe(-1)
  })

  it('fails closed on stale single matches and replaces all atomically', () => {
    const match = findLiteralMatches('cube(); cube();', 'cube').matches[0]
    expect(replaceExpectedMatch('edited(); cube();', match, 'sphere')).toBeNull()
    expect(replaceExpectedMatch('cube(); cube();', match, 'sphere')).toBe('sphere(); cube();')
    expect(replaceAllLiteral('x x x', 'x', 'xx')).toEqual({ status: 'replaced', source: 'xx xx xx', count: 3 })
    expect(replaceAllLiteral('xxx', 'x', 'y', { maxMatches: 2 })).toEqual({ status: 'too-many', source: 'xxx', count: 2 })
    const maximum = 'x'.repeat(250_000)
    expect(replaceAllLiteral(maximum, 'x', 'xx')).toMatchObject({ status: 'too-many' })
    expect(replaceAllLiteral('x', 'x', 'y'.repeat(250_001))).toMatchObject({ status: 'too-large' })
  })
})
