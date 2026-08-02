import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { describe, expect, it } from 'vitest'
import {
  LANGUAGE_CONTRACT_ID,
  LANGUAGE_CONTRACT_VERSION,
  LANGUAGE_SEMANTICS_REVISION,
  LANGUAGE_CONTRACT,
  type LanguageDiagnosticCode,
} from '../src/core/languageContract'
import { compileOpenSCAD } from '../src/services/openscadCompiler'
import { OpenSCADParseError } from '../src/services/openscadErrors'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { QUALITY_TARGETS } from '../src/core/qualityTargets'

interface Fixture {
  contract: string
  version: number
  semanticsRevision: string
  compile: Array<{ name: string; source: string; topLevel: string[]; covers: string[] }>
  evaluate: Array<{ name: string; source: string; meshCount: number; volume?: number; covers: string[] }>
  reject: Array<{ name: string; source: string; code: LanguageDiagnosticCode; covers: string[] }>
  evaluateReject: Array<{ name: string; source: string; covers: string[] }>
}

const fixture = JSON.parse(readFileSync(fileURLToPath(
  new URL('./fixtures/language-conformance-v1.json', import.meta.url),
), 'utf8')) as Fixture

describe('versioned language conformance corpus', () => {
  it('is pinned to the public language contract', () => {
    expect(fixture).toMatchObject({
      contract: LANGUAGE_CONTRACT_ID,
      version: LANGUAGE_CONTRACT_VERSION,
      semanticsRevision: LANGUAGE_SEMANTICS_REVISION,
    })
  })

  it('has executable coverage for every declared supported and unsupported feature', () => {
    const positiveCoverage = new Set([...fixture.compile, ...fixture.evaluate].flatMap(testCase => testCase.covers))
    const negativeCoverage = new Set([...fixture.reject, ...fixture.evaluateReject].flatMap(testCase => testCase.covers))
    expect([...positiveCoverage].sort()).toEqual([...LANGUAGE_CONTRACT.supported].sort())
    expect([...negativeCoverage].sort()).toEqual([...LANGUAGE_CONTRACT.unsupported].sort())
  })

  for (const testCase of fixture.compile) {
    it(`compiles ${testCase.name}`, () => {
      expect(compileOpenSCAD(testCase.source).map(statement => statement.type === 'call'
        ? statement.name
        : statement.type)).toEqual(testCase.topLevel)
    })
  }

  for (const testCase of fixture.evaluate) {
    it(`evaluates ${testCase.name}`, async () => {
      const result = await parseOpenSCAD(testCase.source, { quality: 'full' })
      expect(result.meshes).toHaveLength(testCase.meshCount)
      if (testCase.volume !== undefined) expect(result.volume).toBeCloseTo(testCase.volume, 5)
    })
  }

  for (const testCase of fixture.reject) {
    it(`rejects ${testCase.name} with a stable code`, () => {
      expect(() => compileOpenSCAD(testCase.source)).toThrow(expect.objectContaining({
        name: 'OpenSCADParseError',
        code: testCase.code,
      }))
    })
  }


  for (const testCase of fixture.evaluateReject) {
    it(`rejects unsupported evaluator feature ${testCase.name}`, async () => {
      await expect(parseOpenSCAD(testCase.source)).rejects.toBeInstanceOf(OpenSCADParseError)
    })
  }
})

describe('bounded generated compiler inputs', () => {
  it('returns clone-safe IR or a positioned domain diagnostic for a fixed corpus', () => {
    let state = 0x51f15e
    const random = () => {
      state = (Math.imul(state, 1664525) + 1013904223) >>> 0
      return state
    }
    const atoms = ['cube(1);', 'sphere(r=1,$fn=8);', 'translate([1,2,3])', 'for(i=[0:2])', '{', '}', '(', ')', ';', '#']

    for (let caseIndex = 0; caseIndex < QUALITY_TARGETS.generatedCompilerCases; caseIndex++) {
      const length = 1 + random() % 24
      let source = ''
      for (let index = 0; index < length; index++) source += atoms[random() % atoms.length]
      expect(source.length).toBeLessThan(512)
      try {
        expect(structuredClone(compileOpenSCAD(source))).toEqual(compileOpenSCAD(source))
      } catch (error) {
        expect(error).toBeInstanceOf(OpenSCADParseError)
        expect(error).toMatchObject({
          line: expect.any(Number),
          column: expect.any(Number),
          start: expect.any(Number),
          end: expect.any(Number),
        })
      }
    }
  })
})
