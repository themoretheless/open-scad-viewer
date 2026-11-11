import type {TrussMember, TrussNodalWrench, TrussVector, TrussWrenchModel} from './trussAnalysis'

export interface TrussLoadCase {
  id: string
  restrained: TrussWrenchModel['restrained']
  loads: TrussNodalWrench[]
}
export interface TrussLoadTerm {caseId: string; factor: number}
export interface TrussLoadCombination {id: string; terms: TrussLoadTerm[]}
export interface TrussScenario {
  nodesMm: TrussVector[]
  members: TrussMember[]
  cases: TrussLoadCase[]
  combinations: TrussLoadCombination[]
  activeId: string
}
export interface ResolvedTrussScenario {
  activeId: string
  terms: TrussLoadTerm[]
  model: TrussWrenchModel
}
export class TrussScenarioError extends Error {
  constructor(readonly code: 'TRUSS_SCENARIO_INVALID' | 'TRUSS_SCENARIO_SUPPORTS' | 'TRUSS_SCENARIO_RANGE', message: string) {
    super(message); this.name = 'TrussScenarioError'
  }
}
function invalid(message: string): never {throw new TrussScenarioError('TRUSS_SCENARIO_INVALID', message)}
function fields(value: object, allowed: readonly string[], name: string): void {
  if (!value || typeof value !== 'object' || Array.isArray(value)
    || Object.keys(value).some(key => !allowed.includes(key))) invalid(`Invalid ${name} fields.`)
}
function array<T>(value: T[], min: number, max: number, name: string): T[] {
  if (!Array.isArray(value) || value.length < min || value.length > max) invalid(`Invalid ${name} count.`)
  for (let i = 0; i < value.length; i++) if (!Object.hasOwn(value, i)) invalid(`Sparse ${name} is not supported.`)
  return value
}
function id(value: string): string {
  if (typeof value !== 'string' || !value.trim() || value.length > 128) invalid('Scenario IDs must contain 1-128 characters.')
  return value
}
function vector(value: TrussVector): TrussVector {
  if (array(value, 3, 3, 'vector').some(v => typeof v !== 'number' || !Number.isFinite(v))) invalid('Scenario vectors must be finite.')
  return [...value]
}
function selected(nodes: number[], count: number, min: number, max: number): number[] {
  const result = [...array(nodes, min, max, 'selected nodes')]
  if (result.some(node => !Number.isInteger(node) || node < 0 || node >= count)
    || new Set(result).size !== result.length) invalid('Selected nodes must be unique existing indices.')
  return result
}
function restraints(value: TrussWrenchModel['restrained'], count: number): TrussWrenchModel['restrained'] {
  return array(value, count, count, 'restraints').map(mask => {
    if (array(mask, 3, 3, 'restraint mask').some(v => typeof v !== 'boolean')) invalid('XYZ restraints must be explicit booleans.')
    return [...mask]
  })
}
function load(value: TrussNodalWrench, factor: number, count: number): TrussNodalWrench {
  fields(value, ['nodes', 'originMm', 'forceN', 'momentNmm'], 'scenario load')
  const scale = (input: TrussVector): TrussVector => {
    const original = vector(input), output = original.map(v => v * factor) as TrussVector
    if (!output.every(Number.isFinite) || output.some((v, i) => v === 0 && original[i] !== 0 && factor !== 0)) {
      throw new TrussScenarioError('TRUSS_SCENARIO_RANGE', 'Scaled scenario load exceeds the numeric range.')
    }
    return output
  }
  return {nodes: selected(value.nodes, count, 1, 125), originMm: vector(value.originMm),
    forceN: scale(value.forceN), momentNmm: scale(value.momentNmm)}
}

/** Snapshot a case or a signed linear combination on one shared axial-bar model.
 * Geometry, solvability and resultant assembly remain the native solver's responsibility.
 */
export function resolveTrussScenario(scenario: TrussScenario): ResolvedTrussScenario {
  fields(scenario, ['nodesMm', 'members', 'cases', 'combinations', 'activeId'], 'truss scenario')
  const nodesMm = array(scenario.nodesMm, 1, 125, 'nodes').map(vector)
  const members = array(scenario.members, 1, 400, 'members').map(member => {
    fields(member, ['nodes', 'youngMpa', 'areaMm2'], 'member')
    if (!member || !Number.isFinite(member.youngMpa) || !Number.isFinite(member.areaMm2)) invalid('Invalid member properties.')
    return {nodes: selected(member.nodes, nodesMm.length, 2, 2) as [number, number],
      youngMpa: member.youngMpa, areaMm2: member.areaMm2}
  })
  const cases = array(scenario.cases, 1, 32, 'load cases')
  const combinations = array(scenario.combinations, 0, 32, 'load combinations')
  const ids = new Set<string>()
  for (const item of cases) fields(item, ['id', 'restrained', 'loads'], 'load case')
  for (const item of combinations) fields(item, ['id', 'terms'], 'load combination')
  for (const item of [...cases, ...combinations]) {
    const key = id(item?.id)
    if (ids.has(key)) invalid('Case and combination IDs must be unique.')
    ids.add(key)
  }
  const activeId = id(scenario.activeId)
  if (!ids.has(activeId)) invalid('The active load case or combination does not exist.')
  const combination = combinations.find(item => item.id === activeId)
  const terms = combination ? array(combination.terms, 1, 32, 'combination terms').map(term => {
    fields(term, ['caseId', 'factor'], 'combination term')
    if (!term || typeof term.factor !== 'number' || !Number.isFinite(term.factor)) invalid('Load factors must be finite numbers.')
    return {caseId: id(term.caseId), factor: term.factor}
  }) : [{caseId: activeId, factor: 1}]
  if (new Set(terms.map(term => term.caseId)).size !== terms.length) invalid('A combination must reference each load case at most once.')
  let restrained: TrussWrenchModel['restrained'] | undefined
  const loads: TrussNodalWrench[] = []
  for (const term of terms) {
    const source = cases.find(item => item.id === term.caseId)
    if (!source) invalid('A combination must reference existing load cases, not combinations.')
    const mask = restraints(source.restrained, nodesMm.length)
    if (restrained && mask.some((axes, i) => axes.some((fixed, k) => fixed !== restrained![i][k]))) {
      throw new TrussScenarioError('TRUSS_SCENARIO_SUPPORTS', 'Combined load cases must have identical XYZ restraints.')
    }
    restrained ??= mask
    const inputs = array(source.loads, 1, 32, 'case loads')
    if (loads.length + inputs.length > 32) invalid('A resolved scenario is limited to 32 loads.')
    for (const input of inputs) loads.push(load(input, term.factor, nodesMm.length))
  }
  return {activeId, terms, model: {nodesMm, members, restrained: restrained!, loads}}
}
