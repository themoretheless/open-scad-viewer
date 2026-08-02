import { describe, expect, it } from 'vitest'
import {
  MAX_WORKSPACE_SOURCE_LENGTH,
  WORKSPACE_LEGACY_ACTIVE_TAB_KEY,
  WORKSPACE_LEGACY_TABS_KEY,
  WORKSPACE_STORAGE_KEY,
  createWorkspaceDocument,
  loadWorkspaceDocument,
  loadStoredWorkspaceDocument,
  loadLegacyTabsWorkspaceDocument,
  loadWorkspaceRecovery,
  parseWorkspaceDocument,
  parseWorkspaceDocumentValue,
  saveWorkspaceDocument,
  saveWorkspaceRecovery,
  updateWorkspaceDocument,
  type WorkspaceStorage,
} from '../src/services/workspaceDocument'

class MemoryStorage implements WorkspaceStorage {
  readonly values = new Map<string, string>()
  getItem(key: string) { return this.values.get(key) ?? null }
  setItem(key: string, value: string) { this.values.set(key, value) }
}

describe('workspace document', () => {
  it('migrates the donor editor active tab through a bounded validated shape', () => {
    const storage = new MemoryStorage()
    storage.values.set(WORKSPACE_LEGACY_TABS_KEY, JSON.stringify([
      { id: 'first', name: 'first', code: 'cube(1);', pinned: true },
      { id: 'active', name: 'active.scad', code: 'sphere(2);', savedCode: 'old' },
    ]))
    storage.values.set(WORKSPACE_LEGACY_ACTIVE_TAB_KEY, 'active')

    expect(loadLegacyTabsWorkspaceDocument(storage)).toMatchObject({
      fileName: 'active.scad',
      source: 'sphere(2);',
    })
    storage.values.set(WORKSPACE_LEGACY_TABS_KEY, JSON.stringify([{ id: 'bad', name: 'bad', code: 42 }]))
    expect(loadLegacyTabsWorkspaceDocument(storage)).toBeNull()
  })

  it('increments a stable document revision only for content changes', () => {
    const initial = createWorkspaceDocument('cube();', {
      documentId: 'doc-1', fileName: 'part.scad', revision: 4, updatedAt: 10,
    })
    expect(updateWorkspaceDocument(initial, { source: initial.source }, 11)).toBe(initial)

    const changed = updateWorkspaceDocument(initial, { source: 'sphere(2);' }, 12)
    expect(changed).toMatchObject({ documentId: 'doc-1', revision: 5, mutation: 5, updatedAt: 12 })
    expect(changed.source).toBe('sphere(2);')

    const renamed = updateWorkspaceDocument(changed, { fileName: 'renamed.scad' }, 13)
    expect(renamed).toMatchObject({ fileName: 'renamed.scad', revision: 5, mutation: 6, updatedAt: 13 })
  })

  it('round-trips a validated, versioned snapshot', () => {
    const storage = new MemoryStorage()
    const snapshot = createWorkspaceDocument('cube(5);', {
      documentId: 'doc-2', fileName: 'cube.scad', revision: 7, updatedAt: 20,
    })
    expect(saveWorkspaceDocument(storage, snapshot)).toBe(true)
    expect(parseWorkspaceDocument(storage.values.get(WORKSPACE_STORAGE_KEY)!)).toEqual(snapshot)
    expect(parseWorkspaceDocumentValue(structuredClone(snapshot))).toEqual(snapshot)
    expect(loadWorkspaceDocument(storage, 'fallback')).toEqual(snapshot)
  })

  it('migrates the legacy source without trusting malformed workspace data', () => {
    const storage = new MemoryStorage()
    storage.values.set(WORKSPACE_STORAGE_KEY, '{broken')
    storage.values.set('scad-code', 'cylinder(h=4, r=2);')

    const migrated = loadWorkspaceDocument(storage, 'fallback')
    expect(migrated.source).toBe('cylinder(h=4, r=2);')
    expect(migrated.fileName).toBe('model.scad')
    expect(migrated.revision).toBe(0)
  })

  it('rejects unsupported schemas and unsafe payload shapes', () => {
    expect(loadStoredWorkspaceDocument(new MemoryStorage())).toBeNull()
    expect(parseWorkspaceDocument(JSON.stringify({ schemaVersion: 2 }))).toBeNull()
    expect(parseWorkspaceDocument(JSON.stringify({
      schemaVersion: 1,
      documentId: 'doc',
      fileName: 'part.scad',
      source: 'cube();',
      revision: -1,
      updatedAt: 1,
    }))).toBeNull()
  })

  it('migrates pre-mutation schema-v1 snapshots and rejects oversized writers', () => {
    const legacyShape = {
      schemaVersion: 1,
      documentId: 'legacy-doc',
      fileName: 'legacy.scad',
      source: 'cube();',
      revision: 3,
      updatedAt: 9,
    }
    expect(parseWorkspaceDocument(JSON.stringify(legacyShape))).toMatchObject({ mutation: 3 })
    expect(() => createWorkspaceDocument('x'.repeat(MAX_WORKSPACE_SOURCE_LENGTH + 1))).toThrow(RangeError)

    const storage = new MemoryStorage()
    const valid = createWorkspaceDocument('sphere();')
    expect(saveWorkspaceDocument(storage, valid)).toBe(true)
    expect(saveWorkspaceDocument(storage, {
      ...valid,
      source: 'x'.repeat(MAX_WORKSPACE_SOURCE_LENGTH + 1),
    })).toBe(false)
    expect(parseWorkspaceDocument(storage.values.get(WORKSPACE_STORAGE_KEY)!)).toEqual(valid)
  })

  it('round-trips an atomic recovery snapshot with its causal IndexedDB base', () => {
    const storage = new MemoryStorage()
    const base = createWorkspaceDocument('base();', {
      documentId: 'recovery-doc', revision: 2, mutation: 2, updatedAt: 20,
    })
    const draft = updateWorkspaceDocument(base, { source: 'draft();' }, 21)

    expect(saveWorkspaceRecovery(storage, draft, { status: 'found', snapshot: base })).toBe(true)
    expect(loadWorkspaceRecovery(storage)).toEqual({
      snapshot: draft,
      base: { status: 'found', snapshot: base },
      writerId: null,
    })
    expect(loadStoredWorkspaceDocument(storage)).toEqual(draft)
  })
})
