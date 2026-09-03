import { describe, expect, it } from 'vitest'
import { sha256Hex } from '../src/core/sha256'
import {
  OPENSCAD_PROJECT_ERROR_CODES,
  OPENSCAD_PROJECT_MAX_BLOB_BYTES,
  OPENSCAD_PROJECT_MAX_FILES,
  OPENSCAD_PROJECT_MAX_SOURCE_BYTES,
  OPENSCAD_PROJECT_MAX_TOTAL_BYTES,
  OPENSCAD_PROJECT_SCHEMA_VERSION,
  OpenScadProject,
  OpenScadProjectError,
  normalizeOpenScadProjectPath,
  resolveOpenScadProjectPath,
  type OpenScadProjectFileInput,
} from '../src/services/openScadProject'

function expectProjectError(
  operation: () => unknown,
  code: typeof OPENSCAD_PROJECT_ERROR_CODES[number],
): OpenScadProjectError {
  try {
    operation()
  } catch (error) {
    expect(error).toBeInstanceOf(OpenScadProjectError)
    expect(error).toMatchObject({ code })
    return error as OpenScadProjectError
  }
  throw new Error(`Expected ${code}`)
}

function minimalProject(
  extraFiles: readonly OpenScadProjectFileInput[] = [],
): OpenScadProject {
  return new OpenScadProject({
    entrypoint: 'main.scad',
    files: [
      { kind: 'source', path: 'main.scad', source: 'include <lib/part.scad>\npart();' },
      ...extraFiles,
    ],
  })
}

describe('OpenSCAD ProjectBundle/VFS v2', () => {
  it('builds a canonical mixed source/binary snapshot with stable identity', () => {
    const binary = new Uint8Array([0, 255, 17, 34])
    const files: OpenScadProjectFileInput[] = [
      { kind: 'blob', path: 'assets/part.stl', data: binary },
      { kind: 'source', path: 'lib/cafe\u0301.scad', source: 'module part() { cube(1); }' },
      { kind: 'source', path: 'main.scad', source: 'include <lib/caf\u00e9.scad>\npart();' },
    ]
    const project = new OpenScadProject({ entrypoint: 'main.scad', files })
    const reordered = new OpenScadProject({ entrypoint: 'main.scad', files: [...files].reverse() })

    expect(project).toMatchObject({
      schemaVersion: OPENSCAD_PROJECT_SCHEMA_VERSION,
      entrypoint: 'main.scad',
      fileCount: 3,
      byteLength: expect.any(Number),
      sha256: expect.stringMatching(/^[a-f0-9]{64}$/),
    })
    expect(project.list().map(file => file.path)).toEqual([
      'assets/part.stl',
      'lib/caf\u00e9.scad',
      'main.scad',
    ])
    expect(project.sha256).toBe(reordered.sha256)
    expect(project.read('assets/part.stl')).toMatchObject({
      kind: 'blob',
      byteLength: binary.byteLength,
      sha256: sha256Hex(binary),
    })
    expect(project.readEntrypoint()).toMatchObject({
      kind: 'source',
      path: 'main.scad',
      source: 'include <lib/caf\u00e9.scad>\npart();',
    })
  })

  it('resolves source and binary dependencies relative to the importer', () => {
    const project = minimalProject([
      { kind: 'source', path: 'lib/part.scad', source: 'module part() {}' },
      { kind: 'blob', path: 'assets/part.stl', data: new Uint8Array([1, 2, 3]) },
    ])

    expect(project.resolve('main.scad', './lib/part.scad')).toMatchObject({
      kind: 'source',
      path: 'lib/part.scad',
    })
    expect(project.resolve('lib/part.scad', '../assets/part.stl')).toMatchObject({
      kind: 'blob',
      path: 'assets/part.stl',
    })
    expect(project.resolve('main.scad', 'missing.scad')).toBeNull()
    expect(resolveOpenScadProjectPath('nested/main.scad', '../root.scad')).toBe('root.scad')
  })

  it('rejects host paths, noncanonical segments, traversal and malformed Unicode with typed codes', () => {
    for (const path of [
      '/etc/passwd',
      'C:\\secret.scad',
      'https://example.test/model.scad',
      'a//b.scad',
      'a/./b.scad',
      'a/../b.scad',
      'a/\u0000.scad',
      'bad\ud800.scad',
    ]) {
      expectProjectError(() => normalizeOpenScadProjectPath(path), 'E_PROJECT_INVALID_PATH')
    }
    expectProjectError(
      () => resolveOpenScadProjectPath('main.scad', '../secret.scad'),
      'E_PROJECT_PATH_ESCAPE',
    )
    for (const specifier of ['/secret.scad', 'a//b.scad', 'https://example.test/a.scad', '..']) {
      expectProjectError(
        () => resolveOpenScadProjectPath('lib/main.scad', specifier),
        'E_PROJECT_INVALID_PATH',
      )
    }
  })

  it('detects duplicate paths after Unicode canonicalization', () => {
    const error = expectProjectError(() => new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: '' },
        { kind: 'source', path: 'lib/caf\u00e9.scad', source: '' },
        { kind: 'source', path: 'lib/cafe\u0301.scad', source: '' },
      ],
    }), 'E_PROJECT_DUPLICATE_PATH')
    expect(error.details).toEqual({ path: 'lib/caf\u00e9.scad' })
  })

  it('requires a source entrypoint that exists in the admitted snapshot', () => {
    expectProjectError(() => new OpenScadProject({
      entrypoint: 'missing.scad',
      files: [{ kind: 'source', path: 'main.scad', source: '' }],
    }), 'E_PROJECT_ENTRYPOINT_MISSING')
    expectProjectError(() => new OpenScadProject({
      entrypoint: 'main.scad',
      files: [{ kind: 'blob', path: 'main.scad', data: new Uint8Array() }],
    }), 'E_PROJECT_ENTRYPOINT_NOT_SOURCE')
  })

  it('copies binary input and every binary read while freezing structural snapshots', () => {
    const input = new Uint8Array([4, 5, 6])
    const project = minimalProject([
      { kind: 'blob', path: 'part.stl', data: input },
    ])
    input[0] = 99

    const first = project.read('part.stl')
    expect(first?.kind).toBe('blob')
    if (first?.kind !== 'blob') throw new Error('Expected a blob')
    first.data[1] = 88

    const second = project.read('part.stl')
    expect(second?.kind).toBe('blob')
    if (second?.kind !== 'blob') throw new Error('Expected a blob')
    expect([...second.data]).toEqual([4, 5, 6])
    expect(second.data).not.toBe(first.data)
    expect(Object.isFrozen(project)).toBe(true)
    expect(Object.isFrozen(first)).toBe(true)
    expect(Object.isFrozen(project.list())).toBe(true)
    expect(Object.isFrozen(project.list()[0])).toBe(true)

    const bundle = project.toBundle()
    const bundledBlob = bundle.files.find(file => file.kind === 'blob')
    if (bundledBlob?.kind !== 'blob') throw new Error('Expected bundled blob')
    bundledBlob.data[2] = 77
    const freshBundleBlob = project.toBundle().files.find(file => file.kind === 'blob')
    expect(freshBundleBlob?.kind).toBe('blob')
    if (freshBundleBlob?.kind !== 'blob') throw new Error('Expected fresh bundled blob')
    expect([...freshBundleBlob.data]).toEqual([4, 5, 6])
    expect(Object.isFrozen(bundle)).toBe(true)
    expect(Object.isFrozen(bundle.files)).toBe(true)
  })

  it('measures source limits in UTF-8 bytes and rejects malformed source or blob values', () => {
    const unicode = new OpenScadProject({
      entrypoint: 'main.scad',
      files: [{ kind: 'source', path: 'main.scad', source: '\ud83d\ude80' }],
    })
    expect(unicode.byteLength).toBe(4)
    expect(unicode.readEntrypoint().byteLength).toBe(4)

    expectProjectError(() => new OpenScadProject({
      entrypoint: 'main.scad',
      files: [{ kind: 'source', path: 'main.scad', source: 'bad\ud800' }],
    }), 'E_PROJECT_INVALID_SOURCE')
    expectProjectError(() => new OpenScadProject({
      entrypoint: 'main.scad',
      files: [{ kind: 'blob', path: 'main.scad', data: [1, 2, 3] as unknown as Uint8Array }],
    }), 'E_PROJECT_INVALID_BLOB')
  })

  it('enforces file-count, per-source, per-blob, and aggregate byte limits', () => {
    expectProjectError(() => new OpenScadProject({
      entrypoint: 'main.scad',
      files: Array.from({ length: OPENSCAD_PROJECT_MAX_FILES + 1 }, (_, index) => ({
        kind: 'source' as const,
        path: `${index}.scad`,
        source: '',
      })),
    }), 'E_PROJECT_FILE_COUNT_LIMIT')

    expectProjectError(() => new OpenScadProject({
      entrypoint: 'main.scad',
      files: [{
        kind: 'source',
        path: 'main.scad',
        source: 'x'.repeat(OPENSCAD_PROJECT_MAX_SOURCE_BYTES + 1),
      }],
    }), 'E_PROJECT_SOURCE_BYTES_LIMIT')

    expectProjectError(() => new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: '' },
        { kind: 'blob', path: 'oversized.bin', data: new Uint8Array(OPENSCAD_PROJECT_MAX_BLOB_BYTES + 1) },
      ],
    }), 'E_PROJECT_BLOB_BYTES_LIMIT')

    const halfPlusOne = Math.floor(OPENSCAD_PROJECT_MAX_TOTAL_BYTES / 2) + 1
    expectProjectError(() => new OpenScadProject({
      entrypoint: 'main.scad',
      files: [
        { kind: 'source', path: 'main.scad', source: '' },
        { kind: 'blob', path: 'left.bin', data: new Uint8Array(halfPlusOne) },
        { kind: 'blob', path: 'right.bin', data: new Uint8Array(halfPlusOne) },
      ],
    }), 'E_PROJECT_TOTAL_BYTES_LIMIT')
  })

  it('publishes one frozen, unique typed-error inventory', () => {
    expect(Object.isFrozen(OPENSCAD_PROJECT_ERROR_CODES)).toBe(true)
    expect(new Set(OPENSCAD_PROJECT_ERROR_CODES).size).toBe(OPENSCAD_PROJECT_ERROR_CODES.length)
    expect(OPENSCAD_PROJECT_ERROR_CODES).toContain('E_PROJECT_PATH_ESCAPE')
    expect(OPENSCAD_PROJECT_ERROR_CODES).toContain('E_PROJECT_TOTAL_BYTES_LIMIT')
  })
})
