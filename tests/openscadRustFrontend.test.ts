/**
 * Stage-1 migration gate: the Rust openscad-core frontend (lexer/parser/AST)
 * must accept/reject the same language conformance corpus as the TypeScript
 * compiler, with identical diagnostic codes and source positions. Evaluate and
 * evaluateReject cases are out of scope here (stage 2 covers the evaluator).
 */
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import type { LanguageDiagnosticCode } from '../src/core/languageContract'
import { compileOpenSCAD, type OpenScadLanguageProfile } from '../src/services/openscadCompiler'
import { scadCompileRust } from '../src/services/languages/kernel'

interface Fixture {
  contract: string
  version: number
  compile: Array<{ name: string; source: string; topLevel: string[]; covers: string[] }>
  evaluate: Array<{ name: string; source: string; meshCount: number; covers: string[] }>
  reject: Array<{ name: string; source: string; code: LanguageDiagnosticCode; covers: string[] }>
  evaluateReject: Array<{ name: string; source: string; covers: string[] }>
}

const fixture = JSON.parse(readFileSync(fileURLToPath(
  new URL('./fixtures/language-conformance-v1.json', import.meta.url),
), 'utf8')) as Fixture

const PROFILES: readonly OpenScadLanguageProfile[] = [
  'openscad-viewer-subset@1',
  'openscad/stable-2021.01',
]

interface TsOutcome {
  ok: boolean
  topLevel?: string[]
  code?: string
  detail?: string
  start?: number
  end?: number
  line?: number
  column?: number
}

function compileTs(source: string, profile: OpenScadLanguageProfile): TsOutcome {
  try {
    const statements = compileOpenSCAD(source, { languageProfile: profile })
    return {
      ok: true,
      topLevel: statements.map(statement => statement.type === 'call' ? statement.name : statement.type),
    }
  } catch (error) {
    const parseError = error as {
      code?: string; detail?: string; start: number; end: number; line: number; column: number
    }
    return {
      ok: false,
      code: parseError.code,
      detail: parseError.detail,
      start: parseError.start,
      end: parseError.end,
      line: parseError.line,
      column: parseError.column,
    }
  }
}

function expectRustMatchesTs(source: string, profile: OpenScadLanguageProfile) {
  const ts = compileTs(source, profile)
  const rust = scadCompileRust(source, profile)
  expect(rust.ok, `Rust/TS acceptance mismatch for ${JSON.stringify(source)} (${profile})`).toBe(ts.ok)
  if (!ts.ok) {
    if (rust.ok) throw new Error('unreachable: ok mismatch asserted above')
    const diagnostic = rust.diagnostics[0]
    expect(diagnostic).toBeDefined()
    expect({ ...diagnostic, code: diagnostic.code ?? undefined }).toEqual({
      code: ts.code,
      message: ts.detail,
      start: ts.start,
      end: ts.end,
      line: ts.line,
      column: ts.column,
    })
    return
  }
  if (!rust.ok) throw new Error('unreachable: ok mismatch asserted above')
  const topLevel = rust.ast.map((statement) => {
    const node = statement as { type: string; name?: string }
    return node.type === 'call' ? node.name : node.type
  })
  expect(topLevel).toEqual(ts.topLevel)
}

describe('openscad-core Rust frontend conformance gate (stage 1)', () => {
  for (const testCase of fixture.compile) {
    it(`compiles ${testCase.name} like the TS parser`, () => {
      const rust = scadCompileRust(testCase.source)
      expect(rust.ok).toBe(true)
      if (!rust.ok) return
      const topLevel = rust.ast.map((statement) => {
        const node = statement as { type: string; name?: string }
        return node.type === 'call' ? node.name : node.type
      })
      expect(topLevel).toEqual(testCase.topLevel)
      // Differential parity with the TS parser under both language profiles.
      for (const profile of PROFILES) expectRustMatchesTs(testCase.source, profile)
    })
  }

  for (const testCase of fixture.reject) {
    it(`rejects ${testCase.name} with the pinned code and position`, () => {
      const rust = scadCompileRust(testCase.source)
      expect(rust.ok).toBe(false)
      if (rust.ok) return
      expect(rust.diagnostics[0]?.code).toBe(testCase.code)
      // Exact position/message parity with the TS parser under both profiles.
      for (const profile of PROFILES) expectRustMatchesTs(testCase.source, profile)
    })
  }

  it('covers every compile/reject corpus case; evaluate cases are deferred to stage 2', () => {
    expect(fixture.compile.length).toBeGreaterThan(0)
    expect(fixture.reject.length).toBeGreaterThan(0)
    // Stage 2 (evaluator) owns evaluate/evaluateReject; stage 1 asserts parse parity only.
    expect(fixture.evaluate.length + fixture.evaluateReject.length).toBeGreaterThan(0)
  })

  it('rejects sources beyond the 250,000 character limit like the TS host', () => {
    const rust = scadCompileRust('cube;'.repeat(50_001))
    expect(rust.ok).toBe(false)
    if (rust.ok) return
    expect(rust.diagnostics[0]).toMatchObject({ message: 'Source exceeds 250,000 characters', start: 0 })
  })
})
