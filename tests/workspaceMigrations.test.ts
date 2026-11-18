import { describe, expect, it } from 'vitest'
import {
  MAX_WORKSPACE_PROJECT_TOTAL_BYTES,
  WORKSPACE_MIGRATIONS,
  WORKSPACE_SCHEMA_VERSION,
  createWorkspaceDocument,
  migrateWorkspaceRecord,
  parseWorkspaceDocument,
  parseWorkspaceDocumentValue,
  updateWorkspaceDocument,
} from '../src/app/workspaceDocument'

const base = { documentId: 'doc-1', fileName: 'main.scad', source: 'cube(1);', revision: 3, updatedAt: 10 }

describe('versioned workspace storage migrations', () => {
  it('migrates schema v1 forward through every step to the current version', () => {
    expect(WORKSPACE_SCHEMA_VERSION).toBe(3)
    const migrated = parseWorkspaceDocumentValue({ ...base, schemaVersion: 1, parameterPresets: [{ junk: true }] })
    expect(migrated).toMatchObject({
      schemaVersion: 3,
      parameterPresets: [],
      projectFiles: [],
      libraryPaths: [],
      mutation: 3,
      revision: 3,
    })
  })

  it('migrates schema v2 to v3 keeping presets and adding an empty project', () => {
    const v2 = { ...base, schemaVersion: 2, parameterPresets: [], mutation: 5 }
    expect(WORKSPACE_MIGRATIONS[2](v2)).toMatchObject({ schemaVersion: 3, projectFiles: [], libraryPaths: [] })
    expect(parseWorkspaceDocument(JSON.stringify(v2))).toMatchObject({ schemaVersion: 3, mutation: 5 })
  })

  it('rejects unknown and future schema versions instead of guessing', () => {
    expect(migrateWorkspaceRecord({ ...base, schemaVersion: 4 })).toBeNull()
    expect(migrateWorkspaceRecord({ ...base, schemaVersion: 0 })).toBeNull()
    expect(migrateWorkspaceRecord(null)).toBeNull()
  })

  it('round-trips project files and library paths in canonical form', () => {
    const document = createWorkspaceDocument('include <parts/a.scad>', {
      fileName: 'main.scad',
      projectFiles: [{ path: 'parts/a.scad', source: 'cube(1);' }],
      libraryPaths: ['libraries', 'libraries'],
    })
    expect(document.libraryPaths).toEqual(['libraries'])
    const restored = parseWorkspaceDocument(JSON.stringify(document))
    expect(restored?.projectFiles).toEqual([{ path: 'parts/a.scad', source: 'cube(1);' }])
  })

  it('rejects escaping, duplicate, and entrypoint-shadowing project paths', () => {
    const record = (projectFiles: unknown, libraryPaths: unknown = []) => ({
      ...base,
      schemaVersion: 3,
      parameterPresets: [],
      projectFiles,
      libraryPaths,
      mutation: 3,
    })
    expect(parseWorkspaceDocumentValue(record([{ path: '../x.scad', source: '' }]))).toBeNull()
    expect(parseWorkspaceDocumentValue(record([{ path: '/x.scad', source: '' }]))).toBeNull()
    expect(
      parseWorkspaceDocumentValue(
        record([
          { path: 'a.scad', source: '' },
          { path: 'a.scad', source: '' },
        ]),
      ),
    ).toBeNull()
    expect(parseWorkspaceDocumentValue(record([{ path: 'main.scad', source: '' }]))).toBeNull()
    expect(parseWorkspaceDocumentValue(record([], ['../libs']))).toBeNull()
    expect(
      parseWorkspaceDocumentValue(
        record([{ path: 'big.scad', source: 'x'.repeat(MAX_WORKSPACE_PROJECT_TOTAL_BYTES) }]),
      ),
    ).toBeNull()
  })

  it('advances the geometry revision when an auxiliary project file changes', () => {
    const document = createWorkspaceDocument('include <a.scad>', {
      projectFiles: [{ path: 'a.scad', source: 'cube(1);' }],
    })
    const edited = updateWorkspaceDocument(document, { projectFiles: [{ path: 'a.scad', source: 'cube(2);' }] })
    expect(edited.revision).toBe(document.revision + 1)
    expect(updateWorkspaceDocument(edited, { projectFiles: [{ path: 'a.scad', source: 'cube(2);' }] })).toBe(edited)
  })
})
