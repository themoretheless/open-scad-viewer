import { describe, expect, it } from 'vitest'
import {
  DEFAULT_PARSER_INPUT_BUDGETS,
  ParserInputError,
  preflightIgesInput,
  preflightStepInput,
  preflightStlInput,
} from '../src/language/parserInputBudgets'
import { buildStepRawArtifact } from '../src/services/stepRawArtifact'

const budgets = { ...DEFAULT_PARSER_INPUT_BUDGETS, entities: 2, depth: 2, faces: 1, controlPoints: 2 }
const thrown = (run: () => unknown): unknown => { try { run() } catch (error) { return error } throw new Error('Expected parser refusal') }

function binaryStl(count: number): ArrayBuffer {
  const buffer = new ArrayBuffer(84 + count * 50)
  new DataView(buffer).setUint32(80, count, true)
  return buffer
}

describe('unified pathological parser budgets', () => {
  it('uses typed errors with format, observed count, budget and limit', () => {
    try {
      preflightStepInput('#1=A();#2=B();#3=C();', budgets)
      expect.unreachable()
    } catch (error) {
      expect(error).toBeInstanceOf(ParserInputError)
      expect(error).toMatchObject({ format: 'step', budget: 'entities', observed: 3, limit: 2, code: 'entity-limit' })
    }
  })

  it('rejects aggregate nesting before STEP semantic geometry', () => {
    expect(() => preflightStepInput("#1=A('(((',((((0))));", budgets)).toThrowError(ParserInputError)
    expect(thrown(() => preflightStepInput('#1=A((((0))));', budgets))).toMatchObject({ budget: 'depth' })
  })

  it('charges STEP faces and conservative B-spline control references', () => {
    expect(thrown(() => preflightStepInput('#1=FACE();#2=ADVANCED_FACE();', budgets))).toMatchObject({ budget: 'faces' })
    expect(thrown(() => preflightStepInput('#1=B_SPLINE_CURVE_WITH_KNOTS((#2,#3,#4,#5));', budgets))).toMatchObject({ budget: 'controlPoints' })
  })

  it('charges IGES directory faces and declared NURBS controls', () => {
    const directory = (type: number, sequence: number) => `${String(type).padStart(8, ' ')}${' '.repeat(64)}D${String(sequence).padStart(7, ' ')}`
    const parameter = '128,100000,100000;'.padEnd(64, ' ') + ' '.repeat(8) + 'P' + '1'.padStart(7, ' ')
    expect(thrown(() => preflightIgesInput(`${directory(510, 1)}\n${directory(510, 2)}\n${directory(510, 3)}\n${directory(510, 4)}`, budgets))).toMatchObject({ budget: 'faces' })
    expect(thrown(() => preflightIgesInput(parameter, budgets))).toMatchObject({ budget: 'controlPoints' })
  })

  it('charges both binary and ASCII STL facets', () => {
    expect(thrown(() => preflightStlInput(binaryStl(2), budgets))).toMatchObject({ format: 'stl', budget: 'faces', observed: 2 })
    const ascii = new TextEncoder().encode('solid x\nfacet normal 0 0 0\nendfacet\nfacet normal 0 0 0\nendfacet\nendsolid')
    expect(thrown(() => preflightStlInput(ascii, budgets))).toMatchObject({ budget: 'faces' })
  })

  it('keeps raw syntax/reference artifacts immutable and reports dangling references', () => {
    const raw = buildStepRawArtifact('#1=A(#2);')
    expect(raw.entities[0]).toMatchObject({ id: 1, type: 'A', references: [2] })
    expect(raw.diagnostics[0]).toMatchObject({ code: 'dangling-reference', entityId: 1, referenceId: 2 })
    expect(Object.isFrozen(raw)).toBe(true)
    expect(Object.isFrozen(raw.entities)).toBe(true)
    expect(Object.isFrozen(raw.entities[0]?.references)).toBe(true)
    expect(raw.stats.entities).toBe(1)
  })
})
