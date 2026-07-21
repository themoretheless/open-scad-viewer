import { afterEach, describe, expect, it, vi } from 'vitest'
import {
  setStorageFailureHandler,
  storageGet,
  storageGetEnum,
  storageGetJSON,
  storageSet,
  storageSetJSON,
} from '../src/services/safeStorage'

function stubStorage(overrides: Partial<Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>> = {}) {
  const backing = new Map<string, string>()
  vi.stubGlobal('localStorage', {
    getItem: (key: string) => backing.get(key) ?? null,
    setItem: (key: string, value: string) => { backing.set(key, String(value)) },
    removeItem: (key: string) => { backing.delete(key) },
    ...overrides,
  })
  return backing
}

afterEach(() => {
  vi.unstubAllGlobals()
  setStorageFailureHandler(null)
})

describe('safeStorage', () => {
  it('invokes the failure handler and returns false when a write hits the quota', () => {
    stubStorage({
      setItem: () => { throw new DOMException('The quota has been exceeded.', 'QuotaExceededError') },
    })
    const failures: Array<{ key: string; error: unknown }> = []
    setStorageFailureHandler((key, error) => failures.push({ key, error }))

    expect(storageSet('scad-code', 'cube(1);')).toBe(false)
    expect(storageSetJSON('scad-command-mru', ['render'])).toBe(false)

    expect(failures.map(failure => failure.key)).toEqual(['scad-code', 'scad-command-mru'])
    expect((failures[0].error as DOMException).name).toBe('QuotaExceededError')
  })

  it('does not invoke the failure handler on successful writes', () => {
    const backing = stubStorage()
    const handler = vi.fn()
    setStorageFailureHandler(handler)

    expect(storageSet('scad-lang', 'en')).toBe(true)
    expect(storageSetJSON('scad-command-mru', ['render'])).toBe(true)
    expect(handler).not.toHaveBeenCalled()
    expect(backing.get('scad-lang')).toBe('en')
    expect(backing.get('scad-command-mru')).toBe('["render"]')
  })

  it('reads fall back safely when storage throws or holds malformed data', () => {
    stubStorage({ getItem: () => { throw new Error('storage disabled') } })
    expect(storageGet('anything')).toBe(null)
    expect(storageGetJSON('anything', ['fallback'])).toEqual(['fallback'])
    expect(storageGetEnum('anything', ['ru', 'en'] as const, 'ru')).toBe('ru')

    const backing = stubStorage()
    backing.set('broken', '{not json')
    expect(storageGetJSON('broken', 'fallback')).toBe('fallback')
    backing.set('list', '[1,2]')
    expect(storageGetJSON<unknown[]>('list', [], Array.isArray)).toEqual([1, 2])
    expect(storageGetJSON('list', 'fallback', value => typeof value === 'string')).toBe('fallback')
  })

  it('only returns enum values from the allowed set', () => {
    const backing = stubStorage()
    backing.set('scad-lang', 'en')
    expect(storageGetEnum('scad-lang', ['ru', 'en'] as const, 'ru')).toBe('en')
    backing.set('scad-lang', 'zz')
    expect(storageGetEnum('scad-lang', ['ru', 'en'] as const, 'ru')).toBe('ru')
    expect(storageGetEnum('missing', ['ru', 'en'] as const, 'ru')).toBe('ru')
  })
})
