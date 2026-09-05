import { describe, expect, it } from 'vitest'
import { IDBFactory } from 'fake-indexeddb'
import { IndexedDbWorkspaceRepository, WorkspaceIndexedDbConflictError } from '../src/services/workspaceIndexedDb'
import { applyParameterPreset, captureParameterPreset, parameterTemplateHash, parseParameterPresets } from '../src/services/parameterPresets'
import { createWorkspaceDocument, parseWorkspaceDocumentValue, updateWorkspaceDocument, workspaceDocumentsEqual } from '../src/services/workspaceDocument'

const source = 'width = 10; // [1:1:40]\nlabel = "small";\nenabled = true;\ncube(width);'

describe('parameter presets', () => {
  it('restores all typed values atomically while preserving surrounding source', () => {
    const preset = captureParameterPreset(source, ' Small ', 'id-1')
    const edited = source.replace('10;', '25;').replace('"small"', '"large \\"quoted\\""').replace('true;', 'false;')
    expect(parameterTemplateHash(edited)).toBe(preset.templateHash)
    expect(applyParameterPreset(edited, preset)).toBe(source)
    expect(preset.name).toBe('Small')
  })

  it('rejects structural changes, changed comments, parameter types and malformed sets', () => {
    const preset = captureParameterPreset(source, 'Small', 'id-1')
    for (const changed of [source + '\n// change', source.replace('cube', 'sphere'), source.replace('10;', 'false;')]) {
      expect(() => applyParameterPreset(changed, preset)).toThrow('preset_incompatible')
    }
    expect(() => applyParameterPreset(source, { ...preset, values: preset.values.slice(1) })).toThrow()
    expect(() => captureParameterPreset('x=1;\nx=2;', 'Duplicate', 'id')).toThrow()
    expect(() => captureParameterPreset('cube(1);', 'Empty', 'id')).toThrow()
    expect(parseParameterPresets([{ ...preset, values: [{ name: 'width', value: Infinity }] }])).toBeNull()
    expect(parseParameterPresets(Array.from({ length: 21 }, (_, i) => ({ ...preset, id: `${i}`, name: `${i}` })))).toBeNull()
    expect(parseParameterPresets([preset, { ...preset, id: 'id-2' }])).toBeNull()
  })

  it('migrates old workspaces and saves presets as metadata without changing geometry revision', () => {
    const before = createWorkspaceDocument(source, { updatedAt: 1 })
    const legacy = { ...before, schemaVersion: 1, parameterPresets: undefined }
    expect(parseWorkspaceDocumentValue(legacy)).toEqual(before)
    const preset = captureParameterPreset(source, 'Small', 'id-1')
    const after = updateWorkspaceDocument(before, { parameterPresets: [preset] }, 2)
    expect(after.revision).toBe(before.revision)
    expect(after.mutation).toBe(before.mutation + 1)
    expect(parseWorkspaceDocumentValue(structuredClone(after))).toEqual(after)
    expect(workspaceDocumentsEqual(after, { ...after, parameterPresets: [] })).toBe(false)
    expect(parseWorkspaceDocumentValue({ ...after, parameterPresets: 'invalid' })).toBeNull()
    const cloned = parseWorkspaceDocumentValue(after)!
    expect(cloned.parameterPresets[0]).not.toBe(after.parameterPresets[0])
  })

  it('preserves preset metadata across repository reads and detects concurrent preset-only writes', async () => {
    const factory = new IDBFactory()
    const first = new IndexedDbWorkspaceRepository(factory)
    const second = new IndexedDbWorkspaceRepository(factory)
    try {
      const base = createWorkspaceDocument(source)
      await first.save(base, null)
      const saved = updateWorkspaceDocument(base, { parameterPresets: [captureParameterPreset(source, 'First', 'id-1')] })
      const concurrent = updateWorkspaceDocument(base, { parameterPresets: [captureParameterPreset(source, 'Second', 'id-2')] })
      await first.save(saved, base)
      await expect(second.save(concurrent, base)).rejects.toBeInstanceOf(WorkspaceIndexedDbConflictError)
      expect(await second.load()).toEqual({ status: 'found', snapshot: saved })
    } finally {
      first.close()
      second.close()
    }
  })
})
