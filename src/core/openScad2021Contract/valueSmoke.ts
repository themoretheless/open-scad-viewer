import type { OpenScadBuiltinFunctionContract, OpenScadBuiltinModuleContract, OpenScadSmokeCase } from '../openScad2021Contract'

export function valueSmoke(call: string, source?: string, fixtureIds: readonly string[] = []): OpenScadSmokeCase {
  return Object.freeze({
    call,
    source: source ?? `echo(${call}); cube(1);`,
    fixtureIds: Object.freeze([...fixtureIds]),
    expected: 'defined-value-and-3d-mesh' as const,
  })
}

export function meshSmoke(call: string, source?: string, fixtureIds: readonly string[] = []): OpenScadSmokeCase {
  return Object.freeze({
    call,
    source: source ?? `${call};`,
    fixtureIds: Object.freeze([...fixtureIds]),
    expected: '3d-mesh' as const,
  })
}

export function builtinFunction(
  name: string,
  category: OpenScadBuiltinFunctionContract['category'],
  call: string,
  source?: string,
  fixtureIds: readonly string[] = [],
): OpenScadBuiltinFunctionContract {
  return Object.freeze({ name, category, smoke: valueSmoke(call, source, fixtureIds) })
}

export function builtinModule(
  name: string,
  category: OpenScadBuiltinModuleContract['category'],
  call: string,
  source?: string,
  fixtureIds: readonly string[] = [],
): OpenScadBuiltinModuleContract {
  return Object.freeze({ name, category, smoke: meshSmoke(call, source, fixtureIds) })
}
