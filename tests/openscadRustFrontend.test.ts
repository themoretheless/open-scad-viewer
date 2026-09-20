/**
 * Stage-1 migration gate: the Rust openscad-core frontend (lexer/parser/AST)
 * must accept/reject the same language conformance corpus as the TypeScript
 * compiler, with identical diagnostic codes, source positions and complete ASTs
 * in the documented wire representation. Evaluation itself belongs to stage 2.
 */
import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import type { LanguageDiagnosticCode } from '../src/core/languageContract'
import { compileOpenSCAD, TT, type OpenScadLanguageProfile } from '../src/services/openscadCompiler'
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
  ast?: unknown
  code?: string
  detail?: string
  start?: number
  end?: number
  line?: number
  column?: number
}

// Match the documented wire representation, not a runtime AST adapter.
function wireAst(value: unknown): unknown {
  if (typeof value === 'number' && !Number.isFinite(value)) return { $number: String(value) }
  if (Array.isArray(value)) return value.map(wireAst)
  if (value !== null && typeof value === 'object') {
    const node = value as Record<string, unknown>
    return Object.fromEntries(Object.entries(node).flatMap(([key, item]) => {
      if (key === 'op' && (node.kind === 'binary' || node.kind === 'unary')) return [[key, TT[item as TT]]]
      if (item === undefined) return node.kind === 'literal' && key === 'value' ? [[key, null]] : []
      return [[key, wireAst(item)]]
    }))
  }
  return value
}

function compileTs(source: string, profile: OpenScadLanguageProfile): TsOutcome {
  try {
    const statements = compileOpenSCAD(source, { languageProfile: profile })
    return {
      ok: true,
      ast: wireAst(statements),
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
    if ('error' in rust) throw new Error(`Unexpected ABI failure: ${rust.error.message}`)
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
  expect(rust.ast).toEqual(ts.ast)
}

describe('openscad-core Rust frontend conformance gate (stage 1)', () => {
  it.each([
    'x=undef; y=1e999; z=-1e999; cube(1);',
    'function f(a,b=undef)=a ? b : -a; module m(a=2){translate([a,0,0]) cube(a);} #m();',
    'x=[for(i=[0:2:8]) if(i>0) let(j=i+1) each [j,j*2]]; cube(x[0]);',
    'x=[1+2,1-2,1*2,1/2,1%2,1^2,1<2,1>2,1<=2,1>=2,1==2,1!=2,true&&false,true||false,!true,+1,-1];',
    'f=function(a=1) a+1; x=f(2); y=[1,2,3].x; cube(x);',
  ])('preserves nested expressions and wire scalars: %s', source => {
    for (const profile of PROFILES) {
      if (profile === 'openscad/stable-2021.01') expect(compileTs(source, profile).ok).toBe(true)
      expectRustMatchesTs(source, profile)
    }
  })

  for (const testCase of [...fixture.evaluate, ...fixture.evaluateReject]) {
    it(`preserves the full AST of evaluator case ${testCase.name}`, () => {
      for (const profile of PROFILES) expectRustMatchesTs(testCase.source, profile)
    })
  }

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
      if ('error' in rust) throw new Error(`Unexpected ABI failure: ${rust.error.message}`)
      expect(rust.diagnostics[0]?.code).toBe(testCase.code)
      // Exact position/message parity with the TS parser under both profiles.
      for (const profile of PROFILES) expectRustMatchesTs(testCase.source, profile)
    })
  }

  it('covers every corpus section at the parser boundary', () => {
    expect(fixture.compile.length).toBeGreaterThan(0)
    expect(fixture.reject.length).toBeGreaterThan(0)
    // Evaluator cases also participate in AST parity, without executing geometry.
    expect(fixture.evaluate.length + fixture.evaluateReject.length).toBeGreaterThan(0)
  })

  it('rejects sources beyond the 250,000 character limit like the TS host', () => {
    const rust = scadCompileRust('cube;'.repeat(50_001))
    expect(rust.ok).toBe(false)
    if (rust.ok) return
    if ('error' in rust) throw new Error(`Unexpected ABI failure: ${rust.error.message}`)
    expect(rust.diagnostics[0]).toMatchObject({ message: 'Source exceeds 250,000 characters', start: 0 })
  })

  it('distinguishes the AST transport depth limit from a source parse failure', () => {
    for (const profile of PROFILES) {
      const source = (terms: number) => `x=${Array(terms).fill('1').join('+')};`
      expectRustMatchesTs(source(125), profile)
      expect(compileTs(source(126), profile).ok).toBe(true)
      const rust = scadCompileRust(source(126), profile)
      expect(rust).toEqual({ok: false, error: {
        code: 'LANGUAGE_TRANSPORT', message: 'Response exceeds transport limit',
      }})
      // A failed serialization must not poison the shared WASM instance.
      expectRustMatchesTs('cube(1);', profile)
    }
  })
})
