import type { DirectDocument } from '../app/directModeling'
import { sameMeshJson } from '../exchange/sameMeshJson'

/**
 * A document expressed against a base both sides hold: collection items equal to a base item
 * travel as that item's id, unchanged top-level fields are omitted. A manipulator move on a
 * large scene then crosses `postMessage` as the few edited objects, not the whole document.
 */
export interface DirectDocumentDelta {
  delta: 1
  /** Top-level keys of the document, in serialization order. */
  keys: string[]
  /** Values of the keys that differ from the base (own keys only; absent means unchanged). */
  fields: Record<string, unknown>
  /** Object collections: the id of an unchanged base item, or the changed/new item itself. */
  collections: Record<string, unknown[]>
}

const COLLECTIONS: ReadonlySet<string> = new Set(['sketches', 'bodies', 'curves', 'surfaces', 'sourceBodies'])
type Item = { id: string }
const itemsById = (value: unknown): Map<string, Item> => {
  const map = new Map<string, Item>()
  if (Array.isArray(value))
    for (const item of value as Item[])
      if (item && typeof item === 'object' && typeof item.id === 'string') map.set(item.id, item)
  return map
}

/** `next` relative to `base`. Equality is JSON equality, the same relation history uses. */
export function directDocumentDelta(base: DirectDocument, next: DirectDocument): DirectDocumentDelta {
  const baseRecord = base as unknown as Record<string, unknown>,
    nextRecord = next as unknown as Record<string, unknown>
  const delta: DirectDocumentDelta = { delta: 1, keys: Object.keys(nextRecord), fields: {}, collections: {} }
  for (const key of delta.keys) {
    const value = nextRecord[key]
    if (COLLECTIONS.has(key) && Array.isArray(value) && Array.isArray(baseRecord[key])) {
      const known = itemsById(baseRecord[key])
      delta.collections[key] = (value as Item[]).map(item => {
        const old = item && typeof item === 'object' && typeof item.id === 'string' ? known.get(item.id) : undefined
        return old !== undefined && sameMeshJson(old, item) ? item.id : item
      })
    } else if (!Object.hasOwn(baseRecord, key) || !sameMeshJson(baseRecord[key], value)) delta.fields[key] = value
  }
  return delta
}

export const isDirectDocumentDelta = (value: unknown): value is DirectDocumentDelta =>
  !!value && typeof value === 'object' && (value as { delta?: unknown }).delta === 1

/**
 * Rebuilds the document a delta describes. Unchanged items and fields are the base's own
 * objects (shared, not copied), so both documents must be treated as immutable, which
 * published UI documents already are. Returns undefined for a malformed delta or one that
 * names an item the base does not have.
 */
export function applyDirectDocumentDelta(base: DirectDocument, value: unknown): DirectDocument | undefined {
  if (!isDirectDocumentDelta(value)) return undefined
  const { keys, fields, collections } = value
  if (!Array.isArray(keys) || !fields || typeof fields !== 'object' || !collections || typeof collections !== 'object')
    return undefined
  const baseRecord = base as unknown as Record<string, unknown>
  const out: Record<string, unknown> = {}
  for (const key of keys) {
    if (typeof key !== 'string' || key === '__proto__' || Object.hasOwn(out, key)) return undefined
    if (Object.hasOwn(collections, key)) {
      const entries = collections[key]
      if (!COLLECTIONS.has(key) || !Array.isArray(entries) || Object.hasOwn(fields, key)) return undefined
      const known = itemsById(baseRecord[key])
      const items = new Array<unknown>(entries.length)
      for (let i = 0; i < entries.length; i++) {
        const entry = entries[i]
        if (typeof entry === 'string') {
          const item = known.get(entry)
          if (!item) return undefined
          items[i] = item
        } else if (entry && typeof entry === 'object') items[i] = entry
        else return undefined
      }
      out[key] = items
    } else if (Object.hasOwn(fields, key)) out[key] = fields[key]
    else if (Object.hasOwn(baseRecord, key)) out[key] = baseRecord[key]
    else return undefined
  }
  for (const key of Object.keys(collections)) if (!Object.hasOwn(out, key)) return undefined
  for (const key of Object.keys(fields)) if (!Object.hasOwn(out, key)) return undefined
  return out as unknown as DirectDocument
}
