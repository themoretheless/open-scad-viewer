/**
 * Stage-2 migration gate: the Rust openscad-core evaluator (ABI op 11) must
 * evaluate the language conformance corpus' evaluate/evaluateReject cases
 * under the corpus contract profile (`openscad-viewer-subset@1`).
 *
 * Geometry modules return accounted shape descriptors until stage 3 wires the
 * shared geometry handle store, so this gate asserts evaluation success and
 * the resulting 3D shape count (`meshCount`); pinned volumes are verified by
 * the stage-3 gate through the shared kernel.
 */
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import type { LanguageDiagnosticCode } from '../src/core/languageContract'
import { scadEvalRust } from '../src/services/languages/kernel'

interface Fixture {
  contract: string
  version: number
  compile: Array<{ name: string; source: string; topLevel: string[]; covers: string[] }>
  evaluate: Array<{ name: string; source: string; meshCount: number; volume?: number; covers: string[] }>
  reject: Array<{ name: string; source: string; code: LanguageDiagnosticCode; covers: string[] }>
  evaluateReject: Array<{ name: string; source: string; covers: string[] }>
}

const fixture = JSON.parse(readFileSync(fileURLToPath(
  new URL('./fixtures/language-conformance-v1.json', import.meta.url),
), 'utf8')) as Fixture

describe('openscad-core Rust evaluator conformance gate (stage 2)', () => {
  for (const testCase of fixture.evaluate) {
    it(`evaluates ${testCase.name} with the pinned mesh count`, () => {
      const rust = scadEvalRust(testCase.source)
      expect(rust.ok, `Rust evaluation failed for ${JSON.stringify(testCase.source)}: ${
        rust.ok ? '' : JSON.stringify(rust.diagnostics)
      }`).toBe(true)
      if (!rust.ok) return
      expect(rust.shapes.length).toBe(testCase.meshCount)
      expect(rust.shapes.every(shape => shape.dimension === 3)).toBe(true)
    })
  }

  for (const testCase of fixture.evaluateReject) {
    it(`rejects ${testCase.name} at evaluation time`, () => {
      const rust = scadEvalRust(testCase.source)
      expect(rust.ok, `Rust evaluation unexpectedly succeeded for ${JSON.stringify(testCase.source)}`).toBe(false)
      if (rust.ok) return
      expect(rust.diagnostics?.length ?? 0).toBeGreaterThan(0)
    })
  }

  it('covers every evaluate/evaluateReject corpus case', () => {
    expect(fixture.evaluate.length).toBeGreaterThan(0)
    expect(fixture.evaluateReject.length).toBeGreaterThan(0)
  })

  it('evaluates the same programs under the stable 2021.01 profile', () => {
    // Under the stable profile top-level geometry is unioned (OpenSCAD CSG
    // tree semantics), so multi-mesh cases collapse to one shape.
    const rust = scadEvalRust('for (i = [0:2]) translate([i * 2, 0, 0]) cube(1);', 'openscad/stable-2021.01')
    expect(rust.ok).toBe(true)
    if (!rust.ok) return
    expect(rust.shapes).toEqual([{ name: 'union', dimension: 3 }])
  })
})
