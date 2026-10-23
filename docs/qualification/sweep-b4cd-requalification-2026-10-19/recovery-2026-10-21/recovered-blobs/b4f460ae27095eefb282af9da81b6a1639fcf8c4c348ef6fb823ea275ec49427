import type { ExampleCatalogEntry } from '../data/examples'

export const MAX_EXAMPLE_COUNT = 64
export const MAX_EXAMPLE_QUERY_LENGTH = 128

export function assertExampleCatalog(entries: readonly ExampleCatalogEntry[]): void {
  if (entries.length === 0 || entries.length > MAX_EXAMPLE_COUNT) {
    throw new RangeError(`Example catalog must contain 1..${MAX_EXAMPLE_COUNT} entries`)
  }
  const ids = new Set<string>()
  for (const entry of entries) {
    if (!/^[a-z0-9][a-z0-9-]{0,63}$/.test(entry.id) || ids.has(entry.id)) {
      throw new TypeError(`Invalid or duplicate example id: ${entry.id}`)
    }
    if (!entry.source || entry.source.length > 250_000) throw new RangeError(`Invalid example source: ${entry.id}`)
    if (!entry.title.ru.trim() || !entry.title.en.trim()
      || !entry.description.ru.trim() || !entry.description.en.trim()) {
      throw new TypeError(`Example ${entry.id} is missing localized metadata`)
    }
    if (entry.tags.length > 16 || entry.tags.some(tag => !/^[a-z0-9-]{1,32}$/.test(tag))) {
      throw new TypeError(`Example ${entry.id} has invalid tags`)
    }
    ids.add(entry.id)
  }
}

/** Stable, literal token search over bounded curated metadata. */
export function filterExampleCatalog(
  entries: readonly ExampleCatalogEntry[],
  query: string,
  locale: 'ru' | 'en',
): readonly ExampleCatalogEntry[] {
  const normalized = query.slice(0, MAX_EXAMPLE_QUERY_LENGTH).trim().toLocaleLowerCase(locale)
  if (!normalized) return entries
  const tokens = normalized.split(/\s+/u)
  return entries.filter(entry => {
    const haystack = [entry.title[locale], entry.description[locale], entry.id, ...entry.tags]
      .join('\n').toLocaleLowerCase(locale)
    return tokens.every(token => haystack.includes(token))
  })
}
