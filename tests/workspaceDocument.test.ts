import { describe, expect, it } from 'vitest'
import {
  WORKSPACE_STORAGE_KEY,
  createWorkspaceDocument,
  loadWorkspaceDocument,
  parseWorkspaceDocument,
  saveWorkspaceDocument,
  updateWorkspaceDocument,
  type WorkspaceStorage,
} from '../src/services/workspaceDocument'

class MemoryStorage implements WorkspaceStorage {
  readonly values = new Map<string, string>()
  getItem(key: string) { return this.values.get(key) ?? null }
  setItem(key: string, value: string) { this.values.set(key, value) }
}

describe('workspace document', () => {
  it('increments a stable document revision only for content changes', () => {
    const initial = createWorkspaceDocument('cube();', {
      documentId: 'doc-1', fileName: 'part.scad', revision: 4, updatedAt: 10,
    })
    expect(updateWorkspaceDocument(initial, { source: initial.source }, 11)).toBe(initial)

    const changed = updateWorkspaceDocument(initial, { source: 'sphere(2);' }, 12)
    expect(changed).toMatchObject({ documentId: 'doc-1', revision: 5, updatedAt: 12 })
    expect(changed.source).toBe('sphere(2);')

    const renamed = updateWorkspaceDocument(changed, { fileName: 'renamed.scad' }, 13)
    expect(renamed).toMatchObject({ fileName: 'renamed.scad', revision: 5, updatedAt: 13 })
  })

  it('round-trips a validated, versioned snapshot', () => {
    const storage = new MemoryStorage()
    const snapshot = createWorkspaceDocument('cube(5);', {
      documentId: 'doc-2', fileName: 'cube.scad', revision: 7, updatedAt: 20,
    })
    expect(saveWorkspaceDocument(storage, snapshot)).toBe(true)
    expect(parseWorkspaceDocument(storage.values.get(WORKSPACE_STORAGE_KEY)!)).toEqual(snapshot)
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
})
