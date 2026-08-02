import { describe, expect, it } from 'vitest'
import { EXAMPLE_CATALOG, EXAMPLES } from '../src/data/examples'
import { assertExampleCatalog, filterExampleCatalog } from '../src/services/exampleCatalog'

describe('example catalog', () => {
  it('is complete, localized and points at canonical MCP example sources', () => {
    expect(() => assertExampleCatalog(EXAMPLE_CATALOG)).not.toThrow()
    expect(EXAMPLE_CATALOG.map(entry => entry.id).sort()).toEqual(Object.keys(EXAMPLES).sort())
    for (const entry of EXAMPLE_CATALOG) expect(entry.source).toBe(EXAMPLES[entry.id])
  })

  it('filters literal localized metadata with stable catalog order', () => {
    expect(filterExampleCatalog(EXAMPLE_CATALOG, 'цикл', 'ru').map(entry => entry.id)).toEqual(['tower'])
    expect(filterExampleCatalog(EXAMPLE_CATALOG, 'boolean intermediate', 'en').map(entry => entry.id))
      .toEqual(['csg'])
    expect(filterExampleCatalog(EXAMPLE_CATALOG, '', 'en')).toBe(EXAMPLE_CATALOG)
  })

  it('rejects duplicate identities and missing translations', () => {
    expect(() => assertExampleCatalog([EXAMPLE_CATALOG[0], EXAMPLE_CATALOG[0]])).toThrow(/duplicate/)
    expect(() => assertExampleCatalog([{ ...EXAMPLE_CATALOG[0], title: { ru: '', en: 'Basic' } }]))
      .toThrow(/localized/)
  })
})
