import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { compileOpenSCAD } from '../src/services/openscadCompiler'
import { OpenSCADParseError } from '../src/services/openscadErrors'

function allFrozen(value: unknown): boolean {
  if (!value || typeof value !== 'object') return true
  return Object.isFrozen(value) && Object.values(value).every(allFrozen)
}

describe('OpenSCAD compiler front-end', () => {
  it('produces structured-clone-safe, deeply frozen operation IR with stable spans and IDs', () => {
    const source = 'translate([1, 2, 3]) cube(size = 2);'
    const program = compileOpenSCAD(source)
    const translate = program[0]
    expect(translate).toMatchObject({
      type: 'call',
      name: 'translate',
      operationId: 'op:root/call%3Atranslate%230',
    })
    if (translate?.type !== 'call') throw new Error('Expected translate call')
    expect(source.slice(translate.argSpans._0.start, translate.argSpans._0.end)).toBe('[1, 2, 3]')
    expect(translate.children[0]).toMatchObject({
      type: 'call',
      name: 'cube',
      operationId: 'op:root/call%3Atranslate%230/children/call%3Acube%230',
    })
    expect(structuredClone(program)).toEqual(program)
    expect(allFrozen(program)).toBe(true)
  })

  it('keeps syntax diagnostics positioned without loading the geometry kernel', () => {
    expect(() => compileOpenSCAD('cube(];')).toThrow(OpenSCADParseError)
    try {
      compileOpenSCAD('\n cube(];')
    } catch (error) {
      expect(error).toMatchObject({ line: 2, column: 7 })
    }
    const compilerSource = readFileSync(fileURLToPath(new URL('../src/services/openscadCompiler.ts', import.meta.url)), 'utf8')
    expect(compilerSource).not.toContain('manifold-3d')
  })

  it('rejects unsupported language features with stable diagnostic codes', () => {
    expect(() => compileOpenSCAD('include <part.scad>')).toThrow(expect.objectContaining({ code: 'E_FEATURE_INCLUDE' }))
    expect(() => compileOpenSCAD('function f(x) = x;')).toThrow(expect.objectContaining({ code: 'E_FEATURE_USER_FUNCTION' }))
    expect(() => compileOpenSCAD('#cube(1);')).toThrow(expect.objectContaining({ code: 'E_FEATURE_VIEWPORT_MODIFIER' }))
    expect(compileOpenSCAD('*cube(1);')).toEqual([])
  })
})
