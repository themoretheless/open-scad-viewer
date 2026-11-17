import { describe, expect, it } from 'vitest'
import { APP_STRINGS, translate } from '../src/i18n/appStrings'
import { languageErrorReason, presentBuildError } from '../src/i18n/kernelErrors'
import { OPENSCAD_PROJECT_ERROR_CODES, OpenScadProject } from '../src/language/openScadProject'
import {
  OPENSCAD_PROJECT_COMPILE_ERROR_CODES,
  OpenScadProjectCompileError,
  compileOpenScadProject,
  normalizeOpenScadLibraryPaths,
} from '../src/language/openScadProjectCompiler'

function project(files: Record<string, string>, entrypoint = 'main.scad'): OpenScadProject {
  return new OpenScadProject({
    entrypoint,
    files: Object.entries(files).map(([path, source]) => ({ kind: 'source' as const, path, source })),
  })
}

function callNames(statements: readonly { type: string; name?: string }[]): string[] {
  return statements.filter(statement => statement.type === 'call').map(statement => statement.name ?? '')
}

function thrown(operation: () => unknown): { code?: string; message: string; details?: unknown } {
  try {
    operation()
  } catch (error) {
    return error as { code?: string; message: string; details?: unknown }
  }
  throw new Error('Expected a failure')
}

describe('OpenSCAD include/use resolution through the project VFS', () => {
  it('resolves relative paths next to the importer, including nested parents', () => {
    const compiled = compileOpenScadProject(
      project({
        'main.scad': 'include <parts/a.scad>\nsphere(1);',
        'parts/a.scad': 'include <../shared/b.scad>\ncube(1);',
        'shared/b.scad': 'cylinder(h=1, r=1);',
      }),
    )
    expect(callNames(compiled)).toEqual(['cylinder', 'cube', 'sphere'])
  })

  it('falls back to library paths in order after the importer directory', () => {
    const files = {
      'main.scad': 'include <shapes.scad>',
      'vendor/shapes.scad': 'cube(1);',
      'libraries/shapes.scad': 'sphere(1);',
    }
    expect(callNames(compileOpenScadProject(project(files), { libraryPaths: ['libraries', 'vendor'] }))).toEqual([
      'sphere',
    ])
    expect(callNames(compileOpenScadProject(project(files), { libraryPaths: ['vendor/', 'libraries'] }))).toEqual([
      'cube',
    ])
  })

  it('prefers a file next to the importer over a library file of the same name', () => {
    const compiled = compileOpenScadProject(
      project({
        'main.scad': 'include <shapes.scad>',
        'shapes.scad': 'cylinder(h=1, r=1);',
        'libraries/shapes.scad': 'sphere(1);',
      }),
      { libraryPaths: ['libraries'] },
    )
    expect(callNames(compiled)).toEqual(['cylinder'])
  })

  it('resolves nested dependencies of a library file relative to that library file', () => {
    const compiled = compileOpenScadProject(
      project({
        'main.scad': 'use <bolts/bolt.scad>\nbolt();',
        'libraries/bolts/bolt.scad': 'include <thread.scad>\nmodule bolt() thread();',
        'libraries/bolts/thread.scad': 'module thread() cylinder(h=2, r=1);',
      }),
      { libraryPaths: ['libraries'] },
    )
    expect(callNames(compiled)).toEqual(['bolt'])
  })

  it('reports a cycle that passes through a library path with the full chain', () => {
    const error = thrown(() =>
      compileOpenScadProject(
        project({
          'main.scad': 'include <a.scad>',
          'libraries/a.scad': 'include <b.scad>',
          'libraries/b.scad': 'use <a.scad>',
        }),
        { libraryPaths: ['libraries'] },
      ),
    )
    expect(error).toBeInstanceOf(OpenScadProjectCompileError)
    expect(error.code).toBe('E_PROJECT_DEPENDENCY_CYCLE')
    expect((error as OpenScadProjectCompileError).details.chain).toEqual([
      'libraries/a.scad',
      'libraries/b.scad',
      'libraries/a.scad',
    ])
  })

  it('reports a missing dependency after searching every library path', () => {
    const error = thrown(() =>
      compileOpenScadProject(project({ 'main.scad': 'use <nothere.scad>', 'libraries/other.scad': '' }), {
        libraryPaths: ['libraries'],
      }),
    )
    expect(error.code).toBe('E_PROJECT_DEPENDENCY_MISSING')
    expect(error.details).toMatchObject({ directive: 'use', importer: 'main.scad', path: 'nothere.scad' })
  })

  it('rejects dependency paths and library paths that escape the project root', () => {
    expect(thrown(() => compileOpenScadProject(project({ 'main.scad': 'include <../../etc/passwd>' }))).code).toBe(
      'E_PROJECT_PATH_ESCAPE',
    )
    expect(
      thrown(() =>
        compileOpenScadProject(project({ 'main.scad': 'include <../../../x.scad>', 'libraries/x.scad': '' }), {
          libraryPaths: ['libraries'],
        }),
      ).code,
    ).toBe('E_PROJECT_PATH_ESCAPE')
    expect(thrown(() => normalizeOpenScadLibraryPaths(['../outside'])).code).toBe('E_PROJECT_INVALID_PATH')
    expect(thrown(() => normalizeOpenScadLibraryPaths(['/abs'])).code).toBe('E_PROJECT_INVALID_PATH')
  })
})

describe('project VFS error registration', () => {
  const codes = [...OPENSCAD_PROJECT_ERROR_CODES, ...OPENSCAD_PROJECT_COMPILE_ERROR_CODES]

  it('maps every project error code to a language reason with RU and EN text', () => {
    for (const code of codes) {
      const reason = languageErrorReason(code)
      expect(reason, code).toBeDefined()
      expect(APP_STRINGS.ru[`languageError.${reason}`], code).toBeTruthy()
      expect(APP_STRINGS.en[`languageError.${reason}`], code).toBeTruthy()
    }
    expect(languageErrorReason('E_PROJECT_DEPENDENCY_CYCLE')).toBe('dependencyCycle')
    expect(languageErrorReason('E_PROJECT_DEPENDENCY_MISSING')).toBe('dependencyMissing')
    expect(languageErrorReason('E_PROJECT_PATH_ESCAPE')).toBe('pathEscape')
  })

  it('presents missing and cyclic dependencies in Russian and English with the code', () => {
    const missing = {
      code: 'E_PROJECT_DEPENDENCY_MISSING',
      message: 'include from main.scad refers to missing x.scad.',
    }
    const ru = presentBuildError(missing, 'ru', key => translate('ru', key))
    const en = presentBuildError(missing, 'en', key => translate('en', key))
    expect(ru).toMatch(/не найден/)
    expect(en).toMatch(/found neither/)
    for (const text of [ru, en]) expect(text).toContain('(E_PROJECT_DEPENDENCY_MISSING)')

    const cycle = { code: 'E_PROJECT_DEPENDENCY_CYCLE', message: 'a -> b -> a' }
    expect(presentBuildError(cycle, 'ru', key => translate('ru', key))).toMatch(/по кругу.*a -> b -> a/)
    expect(presentBuildError(cycle, 'en', key => translate('en', key))).toMatch(/cycle.*a -> b -> a/)
  })
})
