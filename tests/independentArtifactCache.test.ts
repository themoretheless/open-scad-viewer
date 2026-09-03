import { createHash } from 'node:crypto'
import { describe, expect, it } from 'vitest'
import { IndependentArtifactCache } from '../src/mcp/independentArtifactCache'

function digest(data: Uint8Array): string {
  return createHash('sha256').update(data).digest('hex')
}

describe('IndependentArtifactCache', () => {
  it('uses the exact byte digest as its content address and returns defensive copies', () => {
    const cache = new IndependentArtifactCache(2, 32)
    const data = new Uint8Array([1, 2, 3])
    const stored = cache.put({
      format: 'stl',
      fileName: 'part.stl',
      mimeType: 'model/stl',
      data,
    })
    expect(stored).toEqual({
      id: digest(data),
      format: 'stl',
      fileName: 'part.stl',
      mimeType: 'model/stl',
      sha256: digest(data),
      byteLength: 3,
    })

    data[0] = 9
    const firstRead = cache.get(stored.id)!
    expect([...firstRead.data]).toEqual([1, 2, 3])
    firstRead.data[0] = 8
    expect([...cache.get(stored.id)!.data]).toEqual([1, 2, 3])
  })

  it('enforces metadata, entry and byte bounds with LRU eviction', () => {
    const cache = new IndependentArtifactCache(2, 5)
    const first = cache.put({ format: 'obj', fileName: 'a.obj', mimeType: 'model/obj', data: new Uint8Array([1, 1]) })
    const second = cache.put({ format: 'obj', fileName: 'b.obj', mimeType: 'model/obj', data: new Uint8Array([2, 2]) })
    cache.get(first.id)
    const third = cache.put({ format: 'obj', fileName: 'c.obj', mimeType: 'model/obj', data: new Uint8Array([3, 3]) })
    expect(cache.get(second.id)).toBeNull()
    expect(cache.list().map(artifact => artifact.id)).toEqual([third.id, first.id])

    expect(() => cache.put({
      format: 'stl',
      fileName: 'bad.obj',
      mimeType: 'model/stl',
      data: new Uint8Array(),
    })).toThrow(/invalid file name/)
    expect(() => cache.put({
      format: 'stl',
      fileName: 'bad.stl',
      mimeType: 'application/octet-stream',
      data: new Uint8Array(),
    })).toThrow(/must use model\/stl/)
    expect(() => cache.put({
      format: 'stl',
      fileName: 'large.stl',
      mimeType: 'model/stl',
      data: new Uint8Array(6),
    })).toThrow(/exceeds cache capacity/)
  })
})
