import { IDBFactory } from 'fake-indexeddb'
import { describe, expect, it } from 'vitest'
import { SvgDraftStore, validSvgDraft, type SvgDraft } from '../src/services/svgDraftStore'
const draft = (text = '<svg/>'): SvgDraft => ({ version: 1, text, filename: 'Знак.svg', height: 5, axis: 'z', dpi: 96, tolerance: 0.05, geometryMode: 'vector', rasterSize: 512, alphaThreshold: 0.5, fonts: [{ name: 'local.ttf', bytes: new Uint8Array([1, 2, 3]) }], opened: true })
describe('SVG draft persistence', () => {
  it('restores source, physical settings, disclosure state and exact font bytes through a new repository', async () => {
    const factory = new IDBFactory(), store = new SvgDraftStore(factory)
    expect(await store.load()).toBeNull()
    const value = { ...draft(), dpi: 144, geometryMode: 'silhouette' as const, rasterSize: 1024 }
    await store.save(value)
    const restored = await new SvgDraftStore(factory).load()
    expect(restored).toEqual(value)
    expect(restored!.fonts[0].bytes).toBeInstanceOf(Uint8Array)
  })
  it('preserves intentionally unfinished SVG as text, never as trusted rendered markup', async () => {
    const factory = new IDBFactory(), store = new SvgDraftStore(factory)
    await store.load(); await store.save(draft('<svg onload="draft only"><path'))
    expect((await new SvgDraftStore(factory).load())!.text).toContain('onload')
    expect(await new SvgDraftStore(factory).load()).not.toHaveProperty('preview')
  })
  it('serializes rapid saves and keeps the latest user edit on remount and reload', async () => {
    const factory = new IDBFactory(), store = new SvgDraftStore(factory)
    await store.load()
    const first = store.save(draft('first')), second = store.save(draft('second'))
    expect((await store.load())!.text).toBe('second')
    await Promise.all([first, second])
    expect((await new SvgDraftStore(factory).load())!.text).toBe('second')
  })
  it('detects another tab update instead of silently overwriting it', async () => {
    const factory = new IDBFactory(), first = new SvgDraftStore(factory), second = new SvgDraftStore(factory)
    await first.load(); await second.load()
    await first.save(draft('from first tab'))
    await expect(second.save(draft('from second tab'))).rejects.toMatchObject({ code: 'conflict' })
    expect((await second.load())!.text).toBe('from second tab')
    expect((await new SvgDraftStore(factory).load())!.text).toBe('from first tab')
  })
  it('keeps session data and reports unavailable durable storage', async () => {
    const store = new SvgDraftStore(undefined)
    await expect(store.load()).rejects.toMatchObject({ code: 'unavailable' })
    await expect(store.save(draft('session only'))).rejects.toMatchObject({ code: 'unavailable' })
    expect((await store.load())!.text).toBe('session only')
  })
  it('rejects invalid restored structures, UTF-8 byte overflow and unbounded fonts', async () => {
    expect(validSvgDraft(draft())).toBe(true)
    for (const invalid of [{ ...draft(), version: 2 }, { ...draft(), text: 'я'.repeat(2 * 1024 * 1024 + 1) }, { ...draft(), dpi: {} }, { ...draft(), fonts: [{ name: 'large', bytes: new Uint8Array(4 * 1024 * 1024 + 1) }] }, { ...draft(), fonts: Array.from({ length: 17 }, () => draft().fonts[0]) }]) expect(validSvgDraft(invalid)).toBe(false)
  })
})

it('keeps the latest artwork and unfinished numeric settings across reload', async () => {
  const factory = new IDBFactory(), store = new SvgDraftStore(factory)
  await store.load(); await store.save(draft('previous'))
  const editing = { ...draft('new unfinished artwork'), dpi: '', height: 0, rasterSize: '12' }
  await store.save(editing)
  expect(await new SvgDraftStore(factory).load()).toEqual(editing)
})
