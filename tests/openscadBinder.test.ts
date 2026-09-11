import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import { compileOpenSCAD } from '../src/services/openscadCompiler'
import {
  bindOpenScad,
  bindProgramCacheKey,
  compileBindCacheSize,
  prepareOpenScadFrontEnd,
  resetCompileBindCache,
} from '../src/services/openscadBinder'
import { isKernelSectionHandle, isKernelSolidHandle, KernelHandleTable } from '../src/services/geometryKernel'
import { positionKernelError } from '../src/services/openscadErrors'

function allFrozen(value: unknown): boolean {
  if (!value || typeof value !== 'object') return true
  return Object.isFrozen(value) && Object.values(value).every(allFrozen)
}

describe('OpenSCAD bind phase', () => {
  it('resolves builtin and user modules without loading the geometry kernel', () => {
    const source = `
      module row() { cube(1); }
      translate([1, 0, 0]) row();
      missing();
    `
    const program = compileOpenSCAD(source)
    const bound = bindOpenScad(program, { source, languageProfile: 'openscad-viewer-subset@1' })
    expect(bound.modules.has('row')).toBe(true)
    expect(new Map(bound.calls.map(call => [call.name, call.kind]))).toEqual(new Map([
      ['translate', 'builtin-module'],
      ['row', 'user-module'],
      ['cube', 'builtin-module'],
      ['missing', 'unresolved'],
    ]))
    expect(bound.diagnostics.some(item => item.message.includes('missing()'))).toBe(true)
    expect(structuredClone({
      calls: bound.calls,
      diagnostics: bound.diagnostics,
    })).toEqual({
      calls: bound.calls,
      diagnostics: bound.diagnostics,
    })
    expect(allFrozen(bound.calls)).toBe(true)
  })

  it('content-addresses compile+bind by source digest and profile', () => {
    resetCompileBindCache()
    const source = 'cube(2);'
    const first = prepareOpenScadFrontEnd(source)
    const second = prepareOpenScadFrontEnd(source)
    const otherProfile = prepareOpenScadFrontEnd(source, { languageProfile: 'openscad/stable-2021.01' })
    expect(first.cacheHit).toBe(false)
    expect(second.cacheHit).toBe(true)
    expect(second.bound).toBe(first.bound)
    expect(otherProfile.cacheHit).toBe(false)
    expect(otherProfile.cacheKey).not.toBe(first.cacheKey)
    expect(first.cacheKey).toBe(bindProgramCacheKey(source, 'openscad-viewer-subset@1'))
    expect(compileBindCacheSize()).toBeGreaterThanOrEqual(2)
    resetCompileBindCache()
    expect(compileBindCacheSize()).toBe(0)
  })

  it('keeps compiler, binder and kernel-port sources free of Manifold imports', () => {
    const files = [
      '../src/services/openscadCompiler.ts',
      '../src/services/openscadBinder.ts',
      '../src/services/geometryKernel.ts',
    ]
    for (const file of files) {
      const source = readFileSync(fileURLToPath(new URL(file, import.meta.url)), 'utf8')
      expect(source).not.toContain('manifold-3d')
      expect(source).not.toContain('geometry/module')
    }
  })

  it('exposes opaque kernel handles and positions kernel errors', () => {
    const table = new KernelHandleTable<{ secret: number }, { plane: number }>()
    const solid = table.adoptSolid({ secret: 7 })
    const section = table.adoptSection({ plane: 2 })
    expect(isKernelSolidHandle(solid)).toBe(true)
    expect(isKernelSectionHandle(section)).toBe(true)
    expect(Object.keys(solid)).toEqual(['type', 'id'])
    expect(table.requireSolid(solid)).toEqual({ secret: 7 })
    table.clear()
    expect(() => table.requireSolid(solid)).toThrow(/Unknown kernel solid handle/)
    const error = positionKernelError('cube(1);', 0, new Error('Non-manifold input'))
    expect(error.line).toBe(1)
    expect(error.column).toBe(1)
    expect(error.detail).toContain('Geometry kernel: Non-manifold input')
  })
})
