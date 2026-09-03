import { describe, expect, it } from 'vitest'
import { OfficialArtifactCache } from '../src/mcp/officialArtifactCache'

function artifact(value: string, fileName = `${value}.stl`) {
  return {
    format: 'stl',
    fileName,
    mimeType: 'model/stl',
    data: new TextEncoder().encode(value),
  }
}

describe('OfficialArtifactCache', () => {
  it('stores immutable, content-addressed artifacts and refreshes LRU reads', () => {
    const cache = new OfficialArtifactCache(2, 32)
    const first = cache.put(artifact('one'))
    const second = cache.put(artifact('two'))

    const read = cache.get(first.id)!
    read.data[0] = 0
    expect(new TextDecoder().decode(cache.get(first.id)!.data)).toBe('one')

    const third = cache.put(artifact('three'))
    expect(cache.get(second.id)).toBeNull()
    expect(cache.list().map(item => item.id)).toEqual([third.id, first.id])
  })

  it('deduplicates bytes while allowing the newest file name', () => {
    const cache = new OfficialArtifactCache(2, 32)
    const first = cache.put(artifact('same', 'first.stl'))
    const second = cache.put(artifact('same', 'second.stl'))

    expect(second.id).toBe(first.id)
    expect(cache.list()).toEqual([expect.objectContaining({
      id: first.id,
      fileName: 'second.stl',
      byteLength: 4,
    })])
  })

  it('evicts by aggregate byte budget and rejects an impossible single item', () => {
    const cache = new OfficialArtifactCache(8, 7)
    const first = cache.put(artifact('1234'))
    const second = cache.put(artifact('5678'))

    expect(cache.get(first.id)).toBeNull()
    expect(cache.get(second.id)).not.toBeNull()
    expect(() => cache.put(artifact('12345678'))).toThrow(/exceeds cache capacity/)
  })
})
