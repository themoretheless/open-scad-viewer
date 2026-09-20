import {expect, it} from 'vitest'
import {resolveTrussScenario, type TrussScenario} from '../src/services/trussScenario'
import {solveTruss} from '../src/services/trussAnalysis'

function scenario(): TrussScenario {
  return {nodesMm: [[0,0,0],[10,0,0]], members: [{nodes: [0,1], youngMpa: 2000, areaMm2: 2}],
    cases: [100,25].map((force, index) => ({id: `case-${index}`,
      restrained: [[true,true,true],[false,true,true]],
      loads: [{nodes: [1], originMm: [10,0,0], forceN: [force,0,0], momentNmm: [0,0,0]}]})),
    combinations: [{id: 'combined', terms: [{caseId: 'case-0', factor: 1.2}, {caseId: 'case-1', factor: -1.5}]}],
    activeId: 'combined'}
}

it('solves signed combinations on the shared structure through actual WASM', () => {
  const input = scenario(), original = structuredClone(input), resolved = resolveTrussScenario(input)
  const result = solveTruss(resolved.model)
  expect(result.axialForcesN[0]).toBeCloseTo(82.5, 10)
  expect(result.axialStressesMpa[0]).toBeCloseTo(41.25, 10)
  expect(result.displacementsMm[1][0]).toBeCloseTo(.20625, 12)
  expect(result.reactionsN[0][0]).toBeCloseTo(-82.5, 10)
  expect(resolved.terms).toEqual(input.combinations[0].terms)
  input.activeId = 'case-1'
  expect(solveTruss(resolveTrussScenario(input).model).axialForcesN[0]).toBeCloseTo(25, 10)
  input.activeId = original.activeId
  expect(input).toEqual(original)
})

it('preserves exact zero loads and native singular refusals instead of inventing a minimum force', () => {
  const input = scenario()
  input.combinations[0].terms.forEach(term => {term.factor = 0})
  const resolved = resolveTrussScenario(input)
  expect(resolved.model.loads.every(load => load.forceN.every(v => v === 0))).toBe(true)
  expect(solveTruss(resolved.model).maxDeflectionMm).toBe(0)
  input.cases.forEach(item => {item.restrained[1] = [false,false,false]})
  expect(() => solveTruss(resolveTrussScenario(input).model)).toThrowError(expect.objectContaining({code: 'TRUSS_SINGULAR'}))
})

it('refuses support unions, including zero-factor cases with incompatible restraints', () => {
  const input = scenario(); input.cases[1].restrained[1][0] = true
  for (const factor of [1, 0]) {
    input.combinations[0].terms[1].factor = factor
    expect(() => resolveTrussScenario(input)).toThrowError(expect.objectContaining({code: 'TRUSS_SCENARIO_SUPPORTS'}))
  }
  input.activeId = 'case-1'
  expect(solveTruss(resolveTrussScenario(input).model).freeDofs).toBe(0)
})

it('snapshots every selected input without sharing mutable arrays with the editor', () => {
  const input = scenario(), resolved = resolveTrussScenario(input), before = structuredClone(resolved)
  input.nodesMm[1][0] = 20; input.members[0].nodes[0] = 1; input.members[0].youngMpa = 1
  input.cases[0].restrained[0][0] = false; input.cases[0].loads[0].nodes[0] = 0
  input.cases[0].loads[0].originMm[0] = 0; input.cases[0].loads[0].forceN[0] = 0
  input.cases[0].loads[0].momentNmm[0] = 100; input.combinations[0].terms[0].factor = 99
  input.activeId = 'missing'
  expect(resolved).toEqual(before)
})

it('rejects missing, duplicate and recursive references instead of falling back to another case', () => {
  const changes: ((input: TrussScenario) => void)[] = [
    input => {input.activeId = 'missing'}, input => {input.cases[1].id = input.cases[0].id},
    input => {input.combinations[0].id = input.cases[0].id}, input => {input.cases[0].id = ''},
    input => {input.combinations[0].terms[0].caseId = 'missing'},
    input => {input.combinations[0].terms[0].caseId = 'combined'},
    input => {input.combinations[0].terms[1].caseId = 'case-0'},
    input => {input.combinations[0].terms = []},
  ]
  for (const change of changes) {
    const input = scenario(); change(input)
    expect(() => resolveTrussScenario(input)).toThrowError(expect.objectContaining({code: 'TRUSS_SCENARIO_INVALID'}))
  }
})

it('bounds cases, terms, selected arrays and the expanded native load count', () => {
  const input = scenario()
  input.cases[0].loads = Array.from({length: 16}, () => structuredClone(input.cases[0].loads[0]))
  input.cases[1].loads = Array.from({length: 16}, () => structuredClone(input.cases[1].loads[0]))
  expect(resolveTrussScenario(input).model.loads).toHaveLength(32)
  expect(solveTruss(resolveTrussScenario(input).model).axialForcesN[0]).toBeCloseTo(1320, 9)
  input.cases[1].loads.push(structuredClone(input.cases[1].loads[0]))
  expect(() => resolveTrussScenario(input)).toThrow('32 loads')
  const malformed = [
    {...scenario(), nodesMm: new Array(126)}, {...scenario(), members: new Array(401)},
    {...scenario(), cases: new Array(33)}, {...scenario(), combinations: new Array(33)},
    {...scenario(), nodesMm: new Array(2)},
    {...scenario(), combinations: [{id: 'combined', terms: new Array(33)}]},
  ]
  for (const value of malformed) expect(() => resolveTrussScenario(value)).toThrowError(expect.objectContaining({code: 'TRUSS_SCENARIO_INVALID'}))
  for (const nodes of [[], [1,1], [2], [-1], [.5], new Array(1)]) {
    const value = scenario(); value.cases[0].loads[0].nodes = nodes
    expect(() => resolveTrussScenario(value)).toThrowError(expect.objectContaining({code: 'TRUSS_SCENARIO_INVALID'}))
  }
})

it('refuses nonfinite factors, scaling overflow and silent load underflow', () => {
  for (const factor of [NaN, Infinity, -Infinity]) {
    const input = scenario(); input.combinations[0].terms[0].factor = factor
    expect(() => resolveTrussScenario(input)).toThrow('finite numbers')
  }
  for (const [force, factor] of [[1e308, 2], [1e-300, 1e-300]]) {
    const input = scenario(); input.cases[0].loads[0].forceN[0] = force; input.combinations[0].terms[0].factor = factor
    expect(() => resolveTrussScenario(input)).toThrowError(expect.objectContaining({code: 'TRUSS_SCENARIO_RANGE'}))
  }
})

it('refuses unsupported physics fields instead of dropping them from the native model', () => {
  for (const mutate of [
    (input: TrussScenario) => Object.assign(input, {forcesN: []}),
    (input: TrussScenario) => Object.assign(input.members[0], {bendingStiffness: 1}),
    (input: TrussScenario) => Object.assign(input.cases[0], {supports: []}),
    (input: TrussScenario) => Object.assign(input.cases[0].loads[0], {momentNm: [1,0,0]}),
    (input: TrussScenario) => Object.assign(input.combinations[0], {safety: 2}),
    (input: TrussScenario) => Object.assign(input.combinations[0].terms[0], {pressureMpa: 3}),
  ]) {
    const input = scenario(); mutate(input)
    expect(() => resolveTrussScenario(input)).toThrowError(expect.objectContaining({code: 'TRUSS_SCENARIO_INVALID'}))
  }
})

it('superposes signed forces and moments without changing their declared origins', () => {
  const input: TrussScenario = {nodesMm: [[10,0,0],[0,10,0],[0,0,0],[1,2,10],[1,2,20]],
    members: [3,4].flatMap(node => [0,1,2].map(anchor => ({nodes: [anchor,node] as [number,number], youngMpa: 2000, areaMm2: 2}))),
    cases: [[4,2,-3], [0,0,0]].map((force, i) => ({id: `case-${i}`,
      restrained: [[true,true,true],[true,true,true],[true,true,true],[false,false,false],[false,false,false]],
      loads: [{nodes: [3,4], originMm: [1,2,15], forceN: force as [number,number,number], momentNmm: i ? [100,0,0] : [0,0,0]}]})),
    combinations: [{id: 'combined', terms: [{caseId: 'case-0', factor: 1.2}, {caseId: 'case-1', factor: -1.5}]}], activeId: 'combined'}
  const first = solveTruss(resolveTrussScenario({...input, activeId: 'case-0'}).model)
  const second = solveTruss(resolveTrussScenario({...input, activeId: 'case-1'}).model)
  const combined = solveTruss(resolveTrussScenario(input).model)
  for (const field of ['displacementsMm', 'reactionsN', 'axialForcesN', 'axialStressesMpa'] as const) {
    const a = first[field].flat(), b = second[field].flat(), c = combined[field].flat()
    c.forEach((value, i) => expect(value).toBeCloseTo(1.2 * a[i] - 1.5 * b[i], 9))
  }
  expect(resolveTrussScenario(input).model.loads.map(load => load.originMm)).toEqual([[1,2,15],[1,2,15]])
})
