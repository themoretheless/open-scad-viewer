import { describe, expect, it } from 'vitest'
import { readFileSync } from 'node:fs'
import { z } from 'zod/v4'
import { compileModelGraph, modelGraphSchema, type Expression } from '../src/services/modelGraph'
import { HeadlessGeometryService } from '../src/mcp/geometryService'
import { DirectGeometrySupervisor } from '../src/mcp/directGeometrySupervisor'
import { defaultGeometryBuildEngine } from '../src/services/geometryBuildEngine'
const local = (name: string): Expression => ({ local: name })
const model = (size: Expression, extra = {}) => ({ language: 'modelgraph/1', units: 'mm', parameters: [], nodes: [{ id: 'part', op: 'box', size: [size, 1, 1] }], root: 'part', ...extra })
const width = (expr: Expression, extra = {}) => compileModelGraph(model(expr, extra)).source

describe('ModelGraph functional semantics', () => {
  it('captures lexical bindings and returns functions without dynamic scope leakage', () => {
    const expr: Expression = { op: 'let', name: 'x', value: 7, body: {
      op: 'let', name: 'f', value: { op: 'lambda', parameters: ['y'], body: { op: 'add', args: [local('x'), local('y')] } }, body: {
        op: 'let', name: 'x', value: 100, body: { op: 'apply', function: local('f'), args: [3] },
      },
    } }
    expect(width(expr)).toContain('cube([10,1,1]')
    expect(width({ op: 'apply', function: { op: 'call', function: 'offset', args: { x: 4 } }, args: [6] }, {
      functions: [{ id: 'offset', kind: 'value', parameters: ['x'], body: { op: 'lambda', parameters: ['y'], body: { op: 'add', args: [local('x'), local('y')] } } }],
    })).toContain('cube([10,1,1]')
  })
  it('passes first-class callbacks to named functions and keeps named scope lexical', () => {
    const functions = [{ id: 'twice', kind: 'value', parameters: ['f', 'x'], body: { op: 'apply', function: local('f'), args: [{ op: 'apply', function: local('f'), args: [local('x')] }] } }]
    expect(width({ op: 'call', function: 'twice', args: { f: { op: 'lambda', parameters: ['n'], body: { op: 'add', args: [local('n'), 3] } }, x: 4 } }, { functions })).toContain('cube([10,1,1]')
    expect(() => width({ op: 'let', name: 'x', value: 4, body: { op: 'call', function: 'leak', args: {} } }, { functions: [{ id: 'leak', kind: 'value', parameters: [], body: local('x') }] })).toThrow('Unknown local')
  })
  it('selects geometry branches without evaluating the inactive dimensions', () => {
    const compiled = compileModelGraph({ language: 'modelgraph/1', units: 'mm', parameters: [], nodes: [
      { id: 'good', op: 'box', size: [2, 2, 2] },
      { id: 'unused', op: 'box', size: [-1, 2, 2] },
      { id: 'choice', op: 'if', condition: 1, then: 'good', else: 'unused' },
    ], root: 'choice' })
    expect(compiled.source).toContain('cube([2,2,2]')
    expect(compiled.source).not.toContain('[-1')
  })
  it('composes range, map, filter and left fold with pure callbacks', () => {
    const expr: Expression = { op: 'reduce', initial: 0,
      input: { op: 'filter', input: { op: 'map', input: { op: 'range', count: 4, start: 1, step: 1 }, function: { op: 'lambda', parameters: ['x'], body: { op: 'multiply', args: [local('x'), 2] } } }, function: { op: 'lambda', parameters: ['x'], body: { op: 'lt', args: [4, local('x')] } } },
      function: { op: 'lambda', parameters: ['a', 'b'], body: { op: 'add', args: [local('a'), local('b')] } },
    }
    expect(width(expr)).toContain('cube([14,1,1]')
    expect(width({ op: 'length', input: { op: 'list', items: [1, 2, 3] } })).toContain('cube([3,1,1]')
    expect(width({ op: 'at', input: { op: 'list', items: [2, 9] }, index: 1 })).toContain('cube([9,1,1]')
  })
  it('evaluates conditionals lazily and supports bounded recursive pure functions', () => {
    const functions = [{ id: 'factorial', kind: 'scalar', parameters: ['n'], body: { op: 'if', condition: { op: 'le', args: [local('n'), 1] }, then: 1, else: { op: 'multiply', args: [local('n'), { op: 'call', function: 'factorial', args: { n: { op: 'subtract', args: [local('n'), 1] } } }] } } }]
    expect(width({ op: 'call', function: 'factorial', args: { n: 5 } }, { functions })).toContain('cube([120,1,1]')
    expect(width({ op: 'if', condition: 1, then: 2, else: { op: 'divide', args: [1, 0] } })).toContain('cube([2,1,1]')
    expect(() => width({ op: 'call', function: 'factorial', args: { n: 100 } }, { functions })).toThrow('depth')
  })
  it('builds composed geometry functions and indexed instances in the production worker', async () => {
    const doc = { language: 'modelgraph/1', units: 'mm', parameters: [], functions: [
      { id: 'block', kind: 'geometry', parameters: ['size'], nodes: [{ id: 'solid', op: 'box', size: [local('size'), 2, 2] }], root: 'solid' },
    ], nodes: [
      { id: 'block', op: 'call', function: 'block', args: { size: 2 } },
      { id: 'positioned', op: 'translate', vector: [{ op: 'multiply', args: [local('i'), 3] }, 0, 0], input: 'block' },
      { id: 'parts', op: 'map', index: 'i', count: 3, input: 'positioned' },
    ], root: 'parts' }
    const original = JSON.stringify(doc), compiled = compileModelGraph(doc)
    expect(JSON.stringify(doc)).toBe(original)
    expect(compileModelGraph(doc)).toEqual(compiled)
    expect(compiled.source_map.some(item => item.instance_path.includes('parts[2]'))).toBe(true)
    for (const item of compiled.source_map) expect(compiled.source.split('\n')[item.line - 1]).toBeTruthy()
    const runtime = new DirectGeometrySupervisor()
    try {
      const result = await new HeadlessGeometryService(defaultGeometryBuildEngine, runtime).analyze(compiled.source, 'full')
      expect(result.volume).toBeCloseTo(24)
      expect(result.bounds).toEqual({ min: [0, 0, 0], max: [8, 2, 2] })
    } finally { await runtime.close() }
  })
  it('rejects type errors, invalid domains, arity, assertion failures and unbounded allocation', () => {
    expect(() => width({ op: 'list', items: [1] })).toThrow('number')
    expect(() => width({ op: 'sqrt', value: -1 })).toThrow('finite')
    expect(() => width({ op: 'divide', args: [1, 0] })).toThrow('finite')
    expect(() => width({ op: 'apply', function: { op: 'lambda', parameters: ['x'], body: local('x') }, args: [] })).toThrow('arity')
    expect(() => width(local('missing'))).toThrow('Unknown local')
    expect(() => width(2, { assertions: [{ condition: 0, message: 'Wall too thin' }] })).toThrow('Wall too thin')
    expect(() => width({ op: 'at', input: { op: 'list', items: [] }, index: 0 })).toThrow('bounds')
    expect(() => width({ op: 'length', input: { op: 'map', input: { op: 'range', count: 256, start: 0, step: 1 }, function: { op: 'lambda', parameters: ['x'], body: { op: 'range', count: 256, start: 0, step: 1 } } } })).toThrow('allocation')
  })
  it('returns geometry from closures and passes geometry to higher-order composition', async () => {
    const compiled = compileModelGraph({ language: 'modelgraph/1', units: 'mm', parameters: [], functions: [
      { id: 'block', kind: 'geometry', parameters: ['w'], nodes: [{ id: 'solid', op: 'box', size: [local('w'), 2, 2] }], root: 'solid' },
      { id: 'move', kind: 'geometry', parameters: ['shape'], nodes: [{ id: 'value', op: 'evaluate', value: local('shape') }, { id: 'translated', op: 'translate', vector: [5, 0, 0], input: 'value' }], root: 'translated' },
    ], nodes: [{ id: 'result', op: 'evaluate', value: { op: 'geometry', function: 'move', args: { shape: { op: 'apply', function: { op: 'lambda', parameters: ['x'], body: { op: 'geometry', function: 'block', args: { w: local('x') } } }, args: [3] } } } }], root: 'result' })
    const runtime = new DirectGeometrySupervisor()
    try {
      const result = await new HeadlessGeometryService(defaultGeometryBuildEngine, runtime).analyze(compiled.source, 'full')
      expect(result.volume).toBeCloseTo(12)
      expect(result.bounds).toEqual({ min: [5, 0, 0], max: [8, 2, 2] })
    } finally { await runtime.close() }
  })
  it('ships the same schema that MCP exposes', () => {
    expect(JSON.parse(readFileSync('docs/languages/modelgraph-1.schema.json', 'utf8'))).toEqual(z.toJSONSchema(modelGraphSchema))
  })
})
