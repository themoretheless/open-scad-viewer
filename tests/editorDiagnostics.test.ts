import { describe, expect, it, vi } from 'vitest'
import { diagnosticFromBuildError, offsetForLineColumn, revealDiagnostic } from '../src/services/editorDiagnostics'

describe('editor diagnostics', () => {
  it('maps worker line/column coordinates to a bounded source token range', () => {
    const source = 'cube(1);\ntranslate([0,0,0]) sphere(2);'
    expect(offsetForLineColumn(source, 2, 20)).toBe(source.indexOf('sphere'))
    expect(diagnosticFromBuildError(source, {
      name: 'OpenSCADParseError', message: 'bad sphere', code: 'E_TEST', line: 2, column: 20,
    })).toMatchObject({ start: source.indexOf('sphere'), end: source.indexOf('sphere') + 6, line: 2, column: 20 })
    expect(offsetForLineColumn(source, 3, 1)).toBeNull()
  })

  it('focuses and selects a diagnostic without owning editor state', () => {
    const focus = vi.fn()
    const setSelectionRange = vi.fn()
    revealDiagnostic({ focus, setSelectionRange }, {
      severity: 'error', message: 'bad', line: 1, column: 1, start: 2, end: 5,
    })
    expect(focus).toHaveBeenCalledWith({ preventScroll: false })
    expect(setSelectionRange).toHaveBeenCalledWith(2, 5, 'forward')
  })

  it('prefers exact UTF-16 spans carried by the worker', () => {
    const source = 'label = "😀"; cube(];'
    const start = source.indexOf(']')
    expect(diagnosticFromBuildError(source, {
      name: 'OpenSCADParseError', message: 'bad token', line: 1, column: 21, start, end: start + 1,
    })).toMatchObject({ start, end: start + 1 })
  })
})
