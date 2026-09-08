import { checkModelGraphNumericType, type ModelGraphNumericType } from './modelGraphNumericType'
import { resolveInterval } from './modelGraphRange'
import { planetarySpinnerTemplate } from './planetarySpinnerTemplate'
import { buildModelGraphGear } from './modelGraphGears'
import { buildModelGraphThread } from './modelGraphThreads'
import { buildModelGraphPlanetary } from './modelGraphPlanetary'
import { MECHANICAL_GENERATOR_GUIDE, createMechanicalDocument } from './mechanicalGeneratorContract'
import { buildModelGraphLoft } from './modelGraphLoft'
import { placeAssembly, multiplyFrames, type Matrix, type Frame, type AssemblyComponent } from './modelGraphAssembly'
import { solveModelGraphSketch, type SketchConstraint } from './modelGraphSketch'
import { z } from 'zod/v4'
import { validateProfilePolygon } from './modelGraphProfiles'
import { createUnitArithmetic, LENGTH, ANGLE, SCALAR, magnitude, dimensionOf, type Dimension, type NumericValue, type Quantity, type Unit } from './modelGraphUnits'
import { sha256Hex } from '../core/sha256'

const id = z.string().regex(/^[A-Za-z][A-Za-z0-9_]{0,31}$/)
const unit = z.enum(['mm', 'cm', 'm', 'in', 'deg', 'rad'])
const number = z.number().finite().min(-1_000_000).max(1_000_000)
export type Expression =
  { op: 'checked'; checks: Expression[]; value: Expression } |
  { op: 'typed'; type: ModelGraphNumericType; value: Expression } |
  { op: 'quantity'; value: number; unit: Unit }
  |   { op: 'geometry'; function: string; args: Record<string, Expression> }
  |   { op: 'lambda'; parameters: string[]; body: Expression }
  | { op: 'apply'; function: Expression; args: Expression[] }
  | { op: 'list'; items: Expression[] }
  | { op: 'range'; count: Expression; start: Expression; step: Expression }
  | { op: 'interval'; start: Expression; end: Expression; inclusive: boolean; count?: Expression; step?: Expression }
  | { op: 'zip'; inputs: Expression[] }
  | { op: 'enumerate'; input: Expression }
  | { op: 'map' | 'filter' | 'flatmap'; input: Expression; function: Expression }
  | { op: 'reduce'; input: Expression; function: Expression; initial: Expression }
  | { op: 'at'; input: Expression; index: Expression }
  | { op: 'length'; input: Expression }
  | number | { param: string } | { local: string }
  | { op: 'add' | 'subtract' | 'multiply' | 'divide' | 'min' | 'max' | 'pow' | 'mod' | 'lt' | 'le' | 'eq' | 'and' | 'or'; args: [Expression, Expression] }
  | { op: 'negate' | 'abs' | 'sqrt' | 'sin' | 'cos' | 'floor' | 'ceil' | 'not'; value: Expression }
  | { op: 'if'; condition: Expression; then: Expression; else: Expression }
  | { op: 'let'; name: string; value: Expression; body: Expression }
  | { op: 'call'; function: string; args: Record<string, Expression> }
export const expressionSchema: z.ZodType<Expression> = z.lazy(() => z.union([
  z.object({op:z.literal('checked'),checks:z.array(expressionSchema).max(256),value:expressionSchema}).strict(),
  z.object({op:z.literal('typed'),type:z.enum(['int','f32','f64','length','angle']),value:expressionSchema}).strict(),
  z.object({ op: z.literal('quantity'), value: number, unit }).strict(),
  z.object({ op: z.literal('geometry'), function: id, args: z.record(id, expressionSchema) }).strict(),
  z.object({ op: z.literal('lambda'), parameters: z.array(id).max(32), body: expressionSchema }).strict(),
  z.object({ op: z.literal('apply'), function: expressionSchema, args: z.array(expressionSchema).max(32) }).strict(),
  z.object({ op: z.literal('list'), items: z.array(expressionSchema).max(256) }).strict(),
  z.object({ op: z.literal('range'), count: expressionSchema, start: expressionSchema, step: expressionSchema }).strict(),
  z.object({ op: z.literal('interval'), start: expressionSchema, end: expressionSchema, inclusive: z.boolean(), count: expressionSchema.optional(), step: expressionSchema.optional() }).strict(),
  z.object({ op: z.literal('zip'), inputs: z.array(expressionSchema).min(2).max(8) }).strict(),
  z.object({ op: z.literal('enumerate'), input: expressionSchema }).strict(),
  z.object({ op: z.enum(['map', 'filter', 'flatmap']), input: expressionSchema, function: expressionSchema }).strict(),
  z.object({ op: z.literal('reduce'), input: expressionSchema, function: expressionSchema, initial: expressionSchema }).strict(),
  z.object({ op: z.literal('at'), input: expressionSchema, index: expressionSchema }).strict(),
  z.object({ op: z.literal('length'), input: expressionSchema }).strict(),
  number, z.object({ param: id }).strict(), z.object({ local: id }).strict(),
  z.object({ op: z.enum(['add', 'subtract', 'multiply', 'divide', 'min', 'max', 'pow', 'mod', 'lt', 'le', 'eq', 'and', 'or']), args: z.tuple([expressionSchema, expressionSchema]) }).strict(),
  z.object({ op: z.enum(['negate', 'abs', 'sqrt', 'sin', 'cos', 'floor', 'ceil', 'not']), value: expressionSchema }).strict(),
  z.object({ op: z.literal('if'), condition: expressionSchema, then: expressionSchema, else: expressionSchema }).strict(),
  z.object({ op: z.literal('let'), name: id, value: expressionSchema, body: expressionSchema }).strict(),
  z.object({ op: z.literal('call'), function: id, args: z.record(id, expressionSchema) }).strict(),
]))
const scalar = expressionSchema
const vector = z.tuple([scalar, scalar, scalar])
const vector2 = z.tuple([scalar, scalar])
const sketchConstraint = z.discriminatedUnion('kind', [
  z.object({ id, kind: z.literal('fix'), point: id, at: vector2 }).strict(),
  z.object({ id, kind: z.enum(['horizontal', 'vertical', 'coincident']), a: id, b: id }).strict(),
  z.object({ id, kind: z.literal('distance'), a: id, b: id, value: scalar }).strict(),
  z.object({ id, kind: z.enum(['parallel', 'perpendicular', 'equal_length']), a: id, b: id, c: id, d: id }).strict(),
])
const frameSchema = z.object({ origin: vector, rotation: vector }).strict()
const affineRow = z.tuple([scalar, scalar, scalar, scalar])
const node = z.discriminatedUnion('op', [
  z.object({id,op:z.literal('gear'),teeth:scalar,module:scalar,pressure_angle:scalar.default(20),thickness:scalar,bore:scalar.default(0),backlash:scalar.default(0.15),clearance:scalar.default(0.5),internal:z.boolean().default(false),rim_width:scalar.default(6),flank_segments:scalar.default(6)}).strict(),
  z.object({id,op:z.literal('planetary_spinner'),inner_radius:scalar,outer_radius:scalar,bore:scalar,gap:scalar,height:scalar,helix_angle:scalar}).strict(),
  z.object({id,op:z.literal('planetary_gears'),sun_teeth:scalar,planet_teeth:scalar,planet_count:scalar,module:scalar,pressure_angle:scalar.default(20),thickness:scalar,bore:scalar.default(0),backlash:scalar.default(0.15),clearance:scalar.default(0.5),rim_width:scalar.default(6),flank_segments:scalar.default(6),carrier_angle:scalar.default(0)}).strict(),
  z.object({id,op:z.literal('thread'),diameter:scalar,pitch:scalar,length:scalar,internal:z.boolean().default(false),wall:scalar.default(3),clearance:scalar.default(0.2),starts:scalar.default(1),left_handed:z.boolean().default(false),segments_per_turn:scalar.default(32)}).strict(),

  z.object({ id, op: z.literal('affine'), input: id, rows: z.tuple([affineRow, affineRow, affineRow]) }).strict(),
  z.object({ id, op: z.literal('hull'), inputs: z.array(id).min(1).max(32) }).strict(),
  z.object({ id, op: z.literal('mirror'), normal: vector, input: id }).strict(),
  z.object({ id, op: z.literal('offset'), distance: scalar, input: id, mode: z.enum(['radius', 'delta']).optional() }).strict(),
  z.object({ id, op: z.literal('projection'), input: id }).strict(),
  z.object({ id, op: z.literal('section'), height: scalar, input: id }).strict(),
  z.object({ id, op: z.literal('advanced_extrude'), input: id, height: scalar, twist: scalar, top_scale: vector2, slices: z.number().int().min(1).max(64), center: z.boolean().default(false) }).strict(),
  z.object({ id, op: z.literal('cone'), radius_bottom: scalar, radius_top: scalar, height: scalar, center: z.boolean().default(false) }).strict(),
  z.object({ id, op: z.literal('torus'), major_radius: scalar, minor_radius: scalar }).strict(),
  z.object({ id, op: z.literal('linear_pattern'), input: id, count: scalar, step: vector }).strict(),
  z.object({ id, op: z.literal('circular_pattern'), input: id, count: scalar, angle_step: scalar }).strict(),

  z.object({ id, op: z.literal('loft'), profile: z.array(vector2).min(3).max(64), sections: z.array(z.object({ z: scalar, scale: vector2, offset: vector2 }).strict()).min(2).max(32) }).strict(),
  z.object({ id, op: z.literal('assembly'), components: z.array(z.object({ id, input: id, anchors: z.array(z.object({ id, origin: vector, rotation: vector }).strict()).max(32), placement: frameSchema.optional(), mate: z.object({ component: id, anchor: id, own_anchor: id, gap: scalar, rotation: vector, joint: z.object({ kind: z.enum(['revolute', 'slider']), position: scalar, min: scalar, max: scalar }).strict().optional() }).strict().optional() }).strict()).min(1).max(32) }).strict(),
  z.object({ id, op: z.literal('sketch'), points: z.array(z.object({ id, position: vector2 }).strict()).min(3).max(16), boundary: z.array(id).min(3).max(16), constraints: z.array(sketchConstraint).max(48), allow_underconstrained: z.boolean().default(false) }).strict(),
  z.object({ id, op: z.literal('rectangle'), size: vector2, center: z.boolean().default(false) }).strict(),
  z.object({ id, op: z.literal('circle'), radius: scalar }).strict(),
  z.object({ id, op: z.literal('polygon'), points: z.array(vector2).min(3).max(256) }).strict(),
  z.object({ id, op: z.literal('extrude'), input: id, height: scalar, center: z.boolean().default(false) }).strict(),
  z.object({ id, op: z.literal('revolve'), input: id, angle: scalar }).strict(),
  z.object({ id, op: z.literal('evaluate'), value: scalar }).strict(),
  z.object({ id, op: z.literal('call'), function: id, args: z.record(id, scalar) }).strict(),
  z.object({ id, op: z.literal('if'), condition: scalar, then: id, else: id }).strict(),
  z.object({ id, op: z.literal('collect'), values: scalar, binding: id, input: id }).strict(),
  z.object({ id, op: z.literal('group'), inputs: z.array(id).max(256) }).strict(),
  z.object({ id, op: z.literal('map'), count: scalar, index: id, input: id }).strict(),
  z.object({ id, op: z.literal('box'), size: vector, center: z.boolean().default(false) }).strict(),
  z.object({ id, op: z.literal('sphere'), radius: scalar }).strict(),
  z.object({ id, op: z.literal('cylinder'), radius: scalar, height: scalar, center: z.boolean().default(false) }).strict(),
  ...(['translate', 'rotate', 'scale'] as const).map(op => z.object({ id, op: z.literal(op), vector, input: id }).strict()),
  ...(['union', 'intersection'] as const).map(op => z.object({ id, op: z.literal(op), inputs: z.array(id).min(1).max(32) }).strict()),
  z.object({ id, op: z.literal('difference'), base: id, subtract: z.array(id).min(1).max(32) }).strict(),
])

export const modelGraphSchema = z.object({
  language: z.literal('modelgraph/1'),
  units: z.literal('mm'),
  type_policy: z.enum(['legacy', 'strict']).optional(),
  constraints: z.array(z.object({ id, left: scalar, relation: z.enum(['le', 'ge', 'eq', 'lt', 'gt']), right: scalar, tolerance: scalar.optional(), message: z.string().min(1).max(256) }).strict()).max(64).optional(),
  parameters: z.array(z.object({ id, value: number, unit: unit.optional(), min: number.optional(), max: number.optional(), integer: z.boolean().optional() }).strict()).max(64),
  nodes: z.array(node).min(1).max(128),
  root: id,
  functions: z.array(z.discriminatedUnion('kind', [
    z.object({ id, kind: z.enum(['scalar', 'value']), parameters: z.array(id).max(32), body: scalar }).strict(),
    z.object({ id, kind: z.literal('geometry'), parameters: z.array(id).max(32), nodes: z.array(node).min(1).max(128), root: id }).strict(),
  ])).max(32).optional(),
  geometry_assertions: z.array(z.object({
    id, target: id, check: z.enum(['hasBodies', 'isWatertight', 'hasNoDegenerateTriangles', 'height', 'width', 'depth']),
    expected: scalar.optional(), tolerance: scalar.optional(), message: z.string().min(1).max(256),
  }).strict()).max(64).optional(),
  assertions: z.array(z.object({ condition: scalar, message: z.string().min(1).max(256) }).strict()).max(64).optional(),
  segments: z.number().int().min(12).max(128).default(48),
}).strict()
export type ModelGraph = z.infer<typeof modelGraphSchema>
export class ModelGraphError extends Error {
  constructor(readonly code: string, readonly path: string, message: string, readonly details?: unknown) { super(message) }
}

function canonical(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`
  if (value !== null && typeof value === 'object') return `{${Object.keys(value).sort().map(key => `${JSON.stringify(key)}:${canonical((value as Record<string, unknown>)[key])}`).join(',')}}`
  return JSON.stringify(value)
}

/** Own declarative frontend; v1 deliberately compiles to the existing Manifold route. */
export function compileModelGraph(value: unknown) {
  const pending = [{ value, depth: 0 }]
  let inputNodes = 0
  while (pending.length) {
    const item = pending.pop()!
    if (++inputNodes > 20000 || item.depth > 64) throw new ModelGraphError('input_limit', '/', 'Document nesting or size limit exceeded.')
    if (item.value && typeof item.value === 'object') for (const child of Object.values(item.value)) pending.push({ value: child, depth: item.depth + 1 })
  }
  const parsed = modelGraphSchema.safeParse(value)
  if (!parsed.success) {
    const issue = parsed.error.issues[0]
    throw new ModelGraphError('invalid_document', '/' + issue.path.join('/'), issue.message)
  }
  const document = parsed.data
  const fail = (code: string, path: string, message: string): never => { throw new ModelGraphError(code, path, message) }
  const arithmetic = createUnitArithmetic(fail)
  const strict = document.type_policy === 'strict'
  const parameters = new Map<string, NumericValue>()
  for (const [i, parameter] of document.parameters.entries()) {
    if (parameters.has(parameter.id)) throw new ModelGraphError('duplicate_id', `/parameters/${i}/id`, 'Parameter IDs must be unique.')
    const path = `/parameters/${i}`
    if ((parameter.min !== undefined && parameter.max !== undefined && parameter.min > parameter.max) || (parameter.min !== undefined && parameter.value < parameter.min) || (parameter.max !== undefined && parameter.value > parameter.max) || (parameter.integer && !Number.isInteger(parameter.value))) fail('parameter_constraint', path, `Parameter ${parameter.id} violates its declared bounds or integer requirement.`)
    parameters.set(parameter.id, parameter.unit ? arithmetic.quantity(parameter.value, parameter.unit, path) : parameter.value)
  }
  const functions = new Map((document.functions ?? []).map(fn => [fn.id, fn]))
  if (functions.size !== (document.functions ?? []).length) fail('duplicate_id', '/functions', 'Function IDs must be unique.')
  for (const fn of functions.values()) if (new Set(fn.parameters).size !== fn.parameters.length) fail('duplicate_id', `/functions/${fn.id}`, 'Function parameters must be unique.')
  type Closure = { kind: 'closure'; parameters: string[]; body: Expression; scope: Scope }
  type GeometryValue = { kind: 'geometry'; function: string; scope: Scope }
  type Value = number | Quantity | readonly Value[] | Closure | GeometryValue
  type Scope = ReadonlyMap<string, Value>
  let steps = 0, allocated = 0
  const allocate = (count: number, path: string) => { if ((allocated += count) > 16384) fail('allocation_limit', path, 'List allocation budget 16384 exceeded.') }
  const tick = (depth: number, path: string) => {
    if (depth > 32 || ++steps > 100_000) fail('evaluation_limit', path, 'Evaluation depth 32 or step budget 100000 exceeded.')
  }
  const bind = (name: string, args: Record<string, Expression>, scope: Scope, path: string, depth: number) => {
    const fn = functions.get(name)
    if (!fn) return fail('unknown_function', path, `Unknown function ${name}.`)
    if (Object.keys(args).length !== fn.parameters.length || fn.parameters.some(key => !Object.prototype.hasOwnProperty.call(args, key))) fail('invalid_arguments', path, 'Arguments must exactly match function parameters.')
    return { fn, scope: new Map(fn.parameters.map(key => [key, resolve(args[key]!, scope, `${path}/args/${key}`, depth + 1)])) }
  }
  const numericValue = (value: Value, path: string): NumericValue => typeof value === 'number' || (!Array.isArray(value) && 'kind' in value && value.kind === 'quantity') ? value as NumericValue : fail('type_error', path, 'Expected a number or quantity.')
  const numeric = (value: Value, path: string): number => arithmetic.scalar(numericValue(value, path), path)
  const sequence = (value: Value, path: string): readonly Value[] => Array.isArray(value) ? value : fail('type_error', path, 'Expected a list.')
  const invoke = (value: Value, args: Value[], path: string, depth: number): Value => {
    if (typeof value === 'number' || Array.isArray(value) || !('kind' in value) || value.kind !== 'closure') return fail('type_error', path, 'Expected a function value.')
    if (args.length !== value.parameters.length) fail('invalid_arguments', path, 'Lambda arity mismatch.')
    const scope = new Map(value.scope)
    value.parameters.forEach((key, i) => scope.set(key, args[i]!))
    return resolve(value.body, scope, path + '/apply', depth + 1)
  }
  const evaluate = (expr: Expression, scope: Scope, path: string, depth = 0): number => numeric(resolve(expr, scope, path, depth), path)
  const resolve = (expr: Expression, scope: Scope, path: string, depth = 0): Value => {
    tick(depth, path)
    if (typeof expr !== 'number' && 'op' in expr) {
      const sub = (value: Expression, field: string) => resolve(value, scope, `${path}/${field}`, depth + 1)
      if (expr.op === 'quantity') return arithmetic.quantity(expr.value, expr.unit, path)
      if (expr.op === 'geometry') {
        const bound = bind(expr.function, expr.args, scope, path, depth)
        if (bound.fn.kind !== 'geometry') return fail('type_error', path, 'Expected a geometry function.')
        return { kind: 'geometry', function: expr.function, scope: bound.scope }
      }
      if (expr.op === 'lambda') {
        if (new Set(expr.parameters).size !== expr.parameters.length) fail('duplicate_id', path, 'Lambda parameters must be unique.')
        return { kind: 'closure', parameters: expr.parameters, body: expr.body, scope: new Map(scope) }
      }
      if (expr.op === 'apply') return invoke(sub(expr.function, 'function'), expr.args.map((a, i) => sub(a, `args/${i}`)), path, depth + 1)
      if (expr.op === 'list') { allocate(expr.items.length, path); return expr.items.map((a, i) => sub(a, `items/${i}`)) }
      if (expr.op === 'interval') {
        const items = resolveInterval(numericValue(sub(expr.start, 'start'), path), numericValue(sub(expr.end, 'end'), path), expr.inclusive,
          { ...(expr.count === undefined ? {} : {count: numericValue(sub(expr.count, 'count'), path)}), ...(expr.step === undefined ? {} : {step: numericValue(sub(expr.step, 'step'), path)}) }, fail, path)
        allocate(items.length, path); return items
      }
      if (expr.op === 'zip') {
        const lists = expr.inputs.map((input, i) => sequence(sub(input, `inputs/${i}`), path))
        const count = lists[0]!.length
        if (lists.some(list => list.length !== count)) fail('length_mismatch', path, 'zip requires equal length sequences.')
        allocate(count * (lists.length + 1), path)
        return Array.from({length:count}, (_, i) => lists.map(list => list[i]!))
      }
      if (expr.op === 'enumerate') {
        const items = sequence(sub(expr.input, 'input'), path)
        allocate(items.length * 3, path); return items.map((value, i) => [i, value])
      }
      if (expr.op === 'range') {
        const count = numeric(sub(expr.count, 'count'), path)
        const start = numericValue(sub(expr.start, 'start'), path), step = numericValue(sub(expr.step, 'step'), path)
        arithmetic.equal(start, step, path)
        if (!Number.isInteger(count) || count < 0 || count > 256) fail('invalid_count', path, 'Range count must be an integer from 0 to 256.')
        allocate(count, path)
        return Array.from({ length: count }, (_, i) => arithmetic.binary('add', start, arithmetic.binary('multiply', i, step, path), path))
      }
      if (expr.op === 'length') return sequence(sub(expr.input, 'input'), path).length
      if (expr.op === 'at') {
        const items = sequence(sub(expr.input, 'input'), path), index = numeric(sub(expr.index, 'index'), path)
        if (!Number.isInteger(index) || index < 0 || index >= items.length) fail('invalid_index', path, 'List index is out of bounds.')
        return items[index]!
      }
      if (expr.op === 'map' || expr.op === 'filter' || expr.op === 'flatmap' || expr.op === 'reduce') {
        const items = sequence(sub(expr.input, 'input'), path), fn = sub(expr.function, 'function')
        allocate(items.length, path)
        if (expr.op === 'flatmap') {
          const output: Value[] = []
          for (const [i, a] of items.entries()) {
            const batch = sequence(invoke(fn, [a], `${path}[${i}]`, depth + 1), path)
            allocate(batch.length, path)
            if (output.length + batch.length > 256) fail('invalid_count', path, 'Generated sequence exceeds 256 values.')
            output.push(...batch)
          }
          return output
        }
        if (expr.op === 'map') return items.map((a, i) => invoke(fn, [a], `${path}[${i}]`, depth + 1))
        if (expr.op === 'filter') return items.filter((a, i) => numeric(invoke(fn, [a], `${path}[${i}]`, depth + 1), path) !== 0)
        if (!('initial' in expr)) return fail('type_error', path, 'Expected reduce initial value.')
        let accumulator = sub(expr.initial, 'initial')
        for (const [i, item] of items.entries()) accumulator = invoke(fn, [accumulator, item], `${path}[${i}]`, depth + 1)
        return accumulator
      }
    }
    let result: number
    if (typeof expr === 'number') result = expr
    else if ('param' in expr) return parameters.get(expr.param) ?? fail('unknown_parameter', path, `Unknown parameter ${expr.param}.`)
    else if ('local' in expr) return scope.get(expr.local) ?? fail('unknown_local', path, `Unknown local ${expr.local}.`)
    else if (expr.op === 'checked') {
      for(const [i,check] of expr.checks.entries())resolve(check,scope,`${path}/checks/${i}`,depth+1)
      return resolve(expr.value,scope,path+'/value',depth+1)
    }
    else if (expr.op === 'typed') return checkModelGraphNumericType(numericValue(resolve(expr.value,scope,path+'/value',depth+1),path),expr.type,message=>fail('type_error',path,message))
    else if (expr.op === 'if') return resolve(evaluate(expr.condition, scope, path + '/condition', depth + 1) !== 0 ? expr.then : expr.else, scope, path, depth + 1)
    else if (expr.op === 'let') {
      const next = new Map(scope); next.set(expr.name, resolve(expr.value, scope, path + '/value', depth + 1))
      return resolve(expr.body, next, path + '/body', depth + 1)
    } else if (expr.op === 'call') {
      const bound = bind(expr.function, expr.args, scope, path, depth)
      if (bound.fn.kind === 'geometry') return fail('type_error', path, 'Expected a scalar function.')
      const output = resolve(bound.fn.body, bound.scope, `${path}/call:${expr.function}`, depth + 1)
      return bound.fn.kind === 'scalar' ? numericValue(output, path) : output
    } else if ('args' in expr) {
      const a = numericValue(resolve(expr.args[0], scope, path + '/args/0', depth + 1), path)
      if (expr.op === 'and' && arithmetic.scalar(a, path) === 0) return 0
      if (expr.op === 'or' && arithmetic.scalar(a, path) !== 0) return 1
      const b = numericValue(resolve(expr.args[1], scope, path + '/args/1', depth + 1), path)
      return arithmetic.binary(expr.op, a, b, path)
    } else if ('value' in expr) {
      return arithmetic.unary(expr.op, numericValue(resolve(expr.value, scope, path + '/value', depth + 1), path), path, strict)
    }
    else return fail('type_error', path, 'Invalid expression.')
    if (!Number.isFinite(result) || Math.abs(result) > 1_000_000) fail('invalid_number', path, 'Expression must produce a finite number within +/-1000000.')
    return result
  }
  type Node = z.infer<typeof node>
  const children = (item: Node): string[] => item.op === 'assembly' ? item.components.map(c => c.input) : 'input' in item ? [item.input] : 'inputs' in item ? item.inputs : 'base' in item ? [item.base, ...item.subtract] : item.op === 'if' ? [item.then, item.else] : []
  const indexNodes = (items: Node[], root: string, path: string) => {
    const nodes = new Map(items.map(item => [item.id, item]))
    if (nodes.size !== items.length) fail('duplicate_id', path, 'Node IDs must be unique.')
    const active = new Set<string>(), visited = new Set<string>()
    const walk = (key: string, depth: number) => {
      if (depth > 32) fail('graph_limit', path, 'Maximum depth 32.')
      if (active.has(key)) fail('cycle', path, `Cycle through ${key}.`)
      if (visited.has(key)) return
      const item = nodes.get(key)
      if (!item) return fail('unknown_node', path, `Unknown node ${key}.`)
      active.add(key)
      for (const child of children(item)) walk(child, depth + 1)
      active.delete(key); visited.add(key)
    }
    walk(root, 1)
    if (visited.size !== nodes.size) fail('unreachable_node', path, 'Every node must be reachable from root; remove unused nodes.')
    return nodes
  }
  const main = indexNodes(document.nodes, document.root, '/nodes')
  const bodies = new Map<string, Map<string, Node>>()
  for (const fn of functions.values()) if (fn.kind === 'geometry') bodies.set(fn.id, indexNodes(fn.nodes, fn.root, `/functions/${fn.id}/nodes`))
  const empty = new Map<string, number>()
  for (const [i, assertion] of (document.assertions ?? []).entries()) if (!evaluate(assertion.condition, empty, `/assertions/${i}/condition`)) fail('assertion_failed', `/assertions/${i}`, assertion.message)
  const constraintIds = new Set<string>()
  const constraint_report = (document.constraints ?? []).map((constraint, i) => {
    const path = `/constraints/${i}`
    if (constraintIds.has(constraint.id)) fail('duplicate_id', path, 'Constraint IDs must be unique.')
    constraintIds.add(constraint.id)
    const left = numericValue(resolve(constraint.left, empty, path + '/left'), path)
    const right = numericValue(resolve(constraint.right, empty, path + '/right'), path)
    arithmetic.equal(left, right, path)
    if (['lt','gt'].includes(constraint.relation) && constraint.tolerance !== undefined) fail('invalid_tolerance', path, 'Strict comparisons do not accept tolerance.')
    let tolerance = 0
    if (constraint.tolerance !== undefined) {
      const value = numericValue(resolve(constraint.tolerance, empty, path + '/tolerance'), path)
      arithmetic.equal(left, value, path)
      tolerance = magnitude(value)
      if (tolerance < 0) fail('invalid_tolerance', path, 'Tolerance must be nonnegative.')
    }
    const a = magnitude(left), b = magnitude(right)
    const passed = constraint.relation === 'lt' ? a < b : constraint.relation === 'gt' ? a > b : constraint.relation === 'eq' ? Math.abs(a - b) <= tolerance : constraint.relation === 'le' ? a <= b + tolerance : a >= b - tolerance
    return { id: constraint.id, path, passed, status: passed ? 'passed' as const : 'failed' as const, actual: a, expected: b, relation: constraint.relation, tolerance, dimension: dimensionOf(left), message: constraint.message }
  })
  if (constraint_report.some(item => !item.passed)) throw new ModelGraphError('constraint_failed', '/constraints', 'Declared constraints were not satisfied.', constraint_report)
  const geometry_assertions = (document.geometry_assertions ?? []).map((check, i) => {
    const path = `/geometry_assertions/${i}`
    if (check.target !== document.root) fail('unsupported_assertion_target', path, 'Geometry checks currently target the shown root only.')
    if (document.geometry_assertions!.slice(0, i).some(c => c.id === check.id)) fail('duplicate_id', path, 'Check IDs must be unique.')
    const measured = ['height', 'width', 'depth'].includes(check.check)
    const expectedValue = check.expected === undefined ? undefined : numericValue(resolve(check.expected, empty, path), path)
    if (measured && expectedValue !== undefined) arithmetic.equal(expectedValue, arithmetic.quantity(1, 'mm', path), path)
    if (!measured && expectedValue !== undefined) arithmetic.scalar(expectedValue, path)
    const expected = expectedValue === undefined ? (check.check === 'hasBodies' || measured ? fail('missing_expected', path, 'Expected value required.') : 0) : magnitude(expectedValue)
    if (expected < 0 || (check.check === 'hasBodies' && !Number.isInteger(expected))) fail('invalid_expected', path, 'Expected value must be nonnegative; body count must be integral.')
    let tolerance = 0
    if (check.tolerance !== undefined) {
      const v = numericValue(resolve(check.tolerance, empty, path), path)
      if (!measured) fail('invalid_tolerance', path, 'Tolerance applies only to measurements.')
      arithmetic.equal(v, arithmetic.quantity(1, 'mm', path), path)
      tolerance = magnitude(v)
      if (tolerance < 0) fail('invalid_tolerance', path, 'Tolerance must be nonnegative.')
    }
    if (!measured && check.check !== 'hasBodies' && check.expected !== undefined) fail('invalid_expected', path, 'Topology checks take no expected argument.')
    return { ...check, expected, tolerance }
  })
  const lines = ['// Generated from modelgraph/1; execution target: legacy/current + Manifold', `$fn = ${document.segments};`]
  const sourceMap: Array<{ node_id: string; line: number; instance_path: string }> = []
  const sketch_solutions: Array<ReturnType<typeof solveModelGraphSketch> & { instance_path: string }> = []
  const assembly_components: Array<ReturnType<typeof placeAssembly>[number] & { instance_path: string; parent_path: string | null; is_assembly: boolean; source?: string }> = []
  let expanded = 0, profileWork = 0
  const mechanical_reports: Array<Record<string, unknown>> = []
  const mechanical_parts: Array<Record<string,unknown>> = []
  let mechanicalSourceCharacters = 0
  const emit = (key: string, nodes: Map<string, Node>, scope: Scope, path: string, depth = 1, expected: 'profile' | 'solid' = 'solid', assemblyParent: { path: string; matrix: Matrix } | null = null, allowAssembly = depth === 1): void => {
    if (++expanded > 4096 || depth > 32) fail('graph_limit', path, 'Maximum depth 32 and expanded node uses 4096.')
    const item = nodes.get(key)!
    const current = `${path}/${key}`
    const produced = ['sketch', 'rectangle', 'circle', 'polygon', 'offset', 'projection', 'section'].includes(item.op) ? 'profile' : ['box', 'sphere', 'cylinder', 'extrude', 'revolve', 'loft', 'advanced_extrude', 'cone', 'torus', 'gear', 'thread', 'planetary_gears', 'planetary_spinner'].includes(item.op) ? 'solid' : expected
    if (produced !== expected) fail('geometry_type_mismatch', current, `Expected ${expected}, received ${produced}. Extrude or revolve a profile before using it as a solid.`)
    sourceMap.push({ node_id: key, line: lines.length + 1, instance_path: current })
    const value = (expr: Expression, field: string, expected: Dimension = SCALAR) => arithmetic.field(numericValue(resolve(expr, scope, `${current}/${field}`), `${current}/${field}`), expected, `${current}/${field}`, strict)
    if (item.op === 'planetary_spinner') {
      if(depth!==1)fail('mechanism_scope',current,'Planetary spinner must be the root to preserve separate parts.')
      const r=value(item.inner_radius,'inner_radius',LENGTH),outer=value(item.outer_radius,'outer_radius',LENGTH),bore=value(item.bore,'bore',LENGTH),gap=value(item.gap,'gap',LENGTH),height=value(item.height,'height',LENGTH),helix=value(item.helix_angle,'helix_angle',ANGLE)
      const scale=r/30.845
      if(r<=0||outer<=r||height<=0||gap<0||gap>0.6*scale||helix<0||helix>=80||bore<0||bore>=2*(23.154388*scale-gap/2)*Math.cos(Math.PI/45)||outer*Math.cos(Math.PI/96)<=r+gap/2)fail('invalid_spinner',current,'Invalid spinner dimensions, gap, bore or rim thickness.')
      lines.push(`inner_radius=${r};outer_radius=${outer};center_hole_diameter=${bore};gap=${gap};spinner_height=${height};helix_angle=${helix};`,planetarySpinnerTemplate)
      mechanical_reports.push({node_id:key,kind:'planetary_spinner',sun_teeth:32,planet_teeth:4,ring_teeth:40,planet_count:18,gap_mm:gap,profile:'fixed sampled prototype',printability:'unknown'})
    } else if (item.op === 'gear' || item.op === 'thread' || item.op === 'planetary_gears') {
      if (item.op === 'planetary_gears' && depth !== 1) fail('mechanism_scope',current,'Planetary gears must be the document root; export or edit individual parts separately.')
      const length = (field:string) => value((item as unknown as Record<string,Expression>)[field],field,LENGTH)
      const angle = (field:string) => value((item as unknown as Record<string,Expression>)[field],field,ANGLE)
      const scalar = (field:string) => value((item as unknown as Record<string,Expression>)[field],field,SCALAR)
      try {
        const generated = item.op === 'thread'
          ? buildModelGraphThread({diameter:length('diameter'),pitch:length('pitch'),length:length('length'),wall:length('wall'),clearance:length('clearance'),starts:scalar('starts'),segments_per_turn:scalar('segments_per_turn'),internal:item.internal,left_handed:item.left_handed})
          : item.op === 'gear'
            ? buildModelGraphGear({teeth:scalar('teeth'),module:length('module'),pressure_angle:angle('pressure_angle'),thickness:length('thickness'),bore:length('bore'),backlash:length('backlash'),clearance:length('clearance'),internal:item.internal,rim_width:length('rim_width'),flank_segments:scalar('flank_segments')})
            : buildModelGraphPlanetary({sun_teeth:scalar('sun_teeth'),planet_teeth:scalar('planet_teeth'),planet_count:scalar('planet_count'),module:length('module'),pressure_angle:angle('pressure_angle'),thickness:length('thickness'),bore:length('bore'),backlash:length('backlash'),clearance:length('clearance'),rim_width:length('rim_width'),flank_segments:scalar('flank_segments'),carrier_angle:angle('carrier_angle')})
        mechanicalSourceCharacters += generated.source.length
        if (mechanicalSourceCharacters > 220000) fail('mechanical_limit',current,'Generated mechanical source exceeds 220000 characters; reduce resolution or instances.')
        mechanical_reports.push({node_id:key,instance_path:current,...generated.report})
        if ('parts' in generated) for (const part of generated.parts) mechanical_parts.push({id:part.id,role:part.role,pose:part.pose,document:createMechanicalDocument({kind:'gear',...part.gear_options})})
        lines.push(...generated.source.split('\n'))
      } catch (error) {
        if (error instanceof ModelGraphError) throw error
        fail('invalid_mechanical_geometry',current,error instanceof Error ? error.message : 'Mechanical generator failed.')
      }
    } else if (item.op === 'assembly') {
      if (!allowAssembly || expected !== 'solid') fail('assembly_scope', current, 'Assembly must be the root or a direct assembly component; boolean/transform wrappers are not allowed.')
      const frame = (f: z.infer<typeof frameSchema>, path: string): Frame => ({ origin: f.origin.map((v, i) => value(v, `${path}/origin/${i}`, LENGTH)) as Frame['origin'], rotation: f.rotation.map((v, i) => value(v, `${path}/rotation/${i}`, ANGLE)) as Frame['rotation'] })
      const components: AssemblyComponent[] = item.components.map((c, i) => ({ id: c.id, input: c.input, anchors: c.anchors.map((a, j) => ({ id: a.id, ...frame(a, `components/${i}/anchors/${j}`) })), ...(c.placement ? { placement: frame(c.placement, `components/${i}/placement`) } : {}), ...(c.mate ? { mate: { component: c.mate.component, anchor: c.mate.anchor, own_anchor: c.mate.own_anchor, ...(c.mate.joint ? { joint: { kind: c.mate.joint.kind, position: value(c.mate.joint.position, `components/${i}/joint/position`, c.mate.joint.kind === 'revolute' ? ANGLE : LENGTH), min: value(c.mate.joint.min, `components/${i}/joint/min`, c.mate.joint.kind === 'revolute' ? ANGLE : LENGTH), max: value(c.mate.joint.max, `components/${i}/joint/max`, c.mate.joint.kind === 'revolute' ? ANGLE : LENGTH) } } : {}), gap: value(c.mate.gap, `components/${i}/mate/gap`, LENGTH), rotation: c.mate.rotation.map((v, j) => value(v, `components/${i}/mate/rotation/${j}`, ANGLE)) as Frame['rotation'] } } : {}) }))
      let placed: ReturnType<typeof placeAssembly>
      try { placed = placeAssembly(components) } catch (error) { return fail('invalid_assembly', current, error instanceof Error ? error.message : 'Assembly placement failed.') }
      for (const component of placed) {
        if (assembly_components.length >= 64) fail('assembly_limit', current, 'Maximum 64 expanded assembly components.')
        const m = component.matrix
        lines.push(`multmatrix(${JSON.stringify([m.slice(0,4),m.slice(4,8),m.slice(8,12),m.slice(12,16)])}){`)
        const world = assemblyParent ? multiplyFrames(assemblyParent.matrix, m) : m
        if (world.some(v => !Number.isFinite(v) || Math.abs(v) > 1e6)) fail('assembly_limit', current, 'World transform exceeds numeric limits.')
        const componentPath = `${current}/components/${component.id}`
        const isAssembly = nodes.get(component.input)?.op === 'assembly'
        const record = { ...component, matrix: world, anchors: component.anchors.map(a => ({ ...a, matrix: assemblyParent ? multiplyFrames(assemblyParent.matrix, a.matrix) : a.matrix })), instance_path: componentPath, parent_path: assemblyParent?.path ?? null, is_assembly: isAssembly, source: undefined as string | undefined }
        assembly_components.push(record)
        const start = lines.length
        emit(component.input, nodes, scope, componentPath, depth + 1, 'solid', { path: componentPath, matrix: world }, true)
        if (!isAssembly) record.source = `$fn=${document.segments};\nmultmatrix(${JSON.stringify([world.slice(0,4),world.slice(4,8),world.slice(8,12),world.slice(12,16)])}){\n${lines.slice(start).join('\n')}\n}`
        lines.push('}')
      }
    } else if (item.op === 'evaluate') {
      const shape = resolve(item.value, scope, current + '/value')
      if (typeof shape === 'number' || Array.isArray(shape) || !('kind' in shape) || shape.kind !== 'geometry') return fail('type_error', current, 'Expected a geometry value.')
      const fn = functions.get(shape.function)!
      if (fn.kind !== 'geometry') return fail('type_error', current, 'Expected a geometry function.')
      emit(fn.root, bodies.get(fn.id)!, shape.scope, `${current}/value:${fn.id}`, depth + 1, expected)
    } else if (item.op === 'call') {
      const bound = bind(item.function, item.args, scope, current, 0)
      if (bound.fn.kind !== 'geometry') return fail('type_error', current, 'Expected a geometry function.')
      emit(bound.fn.root, bodies.get(bound.fn.id)!, bound.scope, `${current}/call:${bound.fn.id}`, depth + 1, expected)
    } else if (item.op === 'if') emit(value(item.condition, 'condition') !== 0 ? item.then : item.else, nodes, scope, current, depth + 1, expected)
    else if (item.op === 'group') {
      for (const child of item.inputs) emit(child, nodes, scope, current, depth + 1, expected)
    } else if (item.op === 'collect') {
      const items = sequence(resolve(item.values, scope, current + '/values'), current)
      if (items.length > 256) fail('invalid_count', current, 'Collection exceeds 256 elements.')
      for (const [i, v] of items.entries()) {
        const local = new Map(scope); local.set(item.binding, v)
        emit(item.input, nodes, local, `${current}[${i}]`, depth + 1, expected)
      }
    } else if (item.op === 'map') {
      const count = value(item.count, 'count')
      if (!Number.isInteger(count) || count < 1 || count > 256) fail('invalid_count', current, 'Map count must be an integer from 1 to 256.')
      lines.push('union(){')
      for (let i = 0; i < count; i++) { const local = new Map(scope); local.set(item.index, i); emit(item.input, nodes, local, `${current}[${i}]`, depth + 1, expected) }
      lines.push('}')
    } else if (item.op === 'affine') {
      const m = item.rows.map((row,i) => row.map((v,j) => value(v,`rows/${i}/${j}`,j === 3 ? LENGTH : SCALAR)))
      const det = m[0][0]*(m[1][1]*m[2][2]-m[1][2]*m[2][1])-m[0][1]*(m[1][0]*m[2][2]-m[1][2]*m[2][0])+m[0][2]*(m[1][0]*m[2][1]-m[1][1]*m[2][0])
      if (!Number.isFinite(det) || det === 0) fail('invalid_transform',current,'Affine matrix must be invertible.')
      if (expected === 'profile' && (m[2][0] !== 0 || m[2][1] !== 0 || m[2][3] !== 0)) fail('nonplanar_profile',current,'Affine transform must preserve XY.')
      lines.push(`multmatrix(${JSON.stringify([...m,[0,0,0,1]])}){`)
      emit(item.input,nodes,scope,current,depth+1,expected); lines.push('}')
    } else if (item.op === 'mirror') {
      const normal = item.normal.map((v, i) => value(v, `normal/${i}`))
      if (Math.hypot(...normal) === 0) fail('invalid_normal', current, 'Mirror normal must be nonzero.')
      if (expected === 'profile' && normal[2] !== 0) fail('nonplanar_profile', current, 'Profile mirror normal must be in XY.')
      lines.push(`mirror(${JSON.stringify(normal)}){`)
      emit(item.input, nodes, scope, current, depth + 1, expected); lines.push('}')
    } else if (item.op === 'projection' || item.op === 'section') {
      lines.push(`projection(cut=${item.op === 'section'}){`)
      if (item.op === 'section') lines.push(`translate([0,0,${-value(item.height, 'height', LENGTH)}]){`)
      emit(item.input, nodes, scope, current, depth + 1, 'solid')
      if (item.op === 'section') lines.push('}')
      lines.push('}')
    } else if (item.op === 'offset') {
      lines.push(`offset(${item.mode === 'delta' ? 'delta' : 'r'}=${value(item.distance, 'distance', LENGTH)}){`)
      emit(item.input, nodes, scope, current, depth + 1, 'profile'); lines.push('}')
    } else if (item.op === 'advanced_extrude') {
      const height = value(item.height, 'height', LENGTH)
      const twist = value(item.twist, 'twist', ANGLE)
      const scale = item.top_scale.map((v, i) => value(v, `top_scale/${i}`))
      if (height <= 0 || scale.some(v => v <= 0)) fail('invalid_dimension', current, 'Height and top scales must be positive.')
      if (Math.abs(twist) > 3600) fail('invalid_angle', current, 'Twist must be within +/-3600 degrees.')
      lines.push(`linear_extrude(height=${height},twist=${twist},scale=${JSON.stringify(scale)},slices=${item.slices},center=${item.center}){`)
      emit(item.input, nodes, scope, current, depth + 1, 'profile'); lines.push('}')
    } else if (item.op === 'cone') {
      const bottom = value(item.radius_bottom, 'radius_bottom', LENGTH), top = value(item.radius_top, 'radius_top', LENGTH), height = value(item.height, 'height', LENGTH)
      if (bottom < 0 || top < 0 || bottom + top <= 0 || height <= 0) fail('invalid_dimension', current, 'Cone needs nonnegative radii, at least one positive radius, and positive height.')
      lines.push(`cylinder(r1=${bottom},r2=${top},h=${height},center=${item.center});`)
    } else if (item.op === 'torus') {
      const major = value(item.major_radius, 'major_radius', LENGTH), minor = value(item.minor_radius, 'minor_radius', LENGTH)
      if (minor <= 0 || major <= minor) fail('invalid_dimension', current, 'Torus requires major_radius > minor_radius > 0.')
      lines.push(`rotate_extrude(angle=360){translate([${major},0,0]){circle(r=${minor});}}`)
    } else if (item.op === 'linear_pattern' || item.op === 'circular_pattern') {
      const count = value(item.count, 'count')
      if (!Number.isInteger(count) || count < 1 || count > 256) fail('invalid_count', current, 'Pattern count must be an integer from 1 to 256.')
      const step = item.op === 'linear_pattern' ? item.step.map((v, i) => value(v, `step/${i}`, LENGTH)) : [0, 0, value(item.angle_step, 'angle_step', ANGLE)]
      if (expected === 'profile' && item.op === 'linear_pattern' && step[2] !== 0) fail('nonplanar_profile', current, 'Profile pattern must remain in XY.')
      lines.push('union(){')
      for (let i = 0; i < count; i++) {
        const vector = step.map(v => v * i)
        if (vector.some(v => Math.abs(v) > 1e6)) fail('graph_limit', current, 'Pattern transform exceeds numeric limits.')
        lines.push(`${item.op === 'linear_pattern' ? 'translate' : 'rotate'}(${JSON.stringify(vector)}){`)
        emit(item.input, nodes, scope, `${current}[${i}]`, depth + 1, expected); lines.push('}')
      }
      lines.push('}')
    } else if (item.op === 'loft') {
      if ((profileWork += item.profile.length ** 2 + item.profile.length * item.sections.length) > 262144) fail('profile_limit', current, 'Loft work budget exceeded.')
      const profile = item.profile.map((p, i) => p.map((v, j) => value(v, `profile/${i}/${j}`, LENGTH)) as [number, number])
      const sections = item.sections.map((s, i) => ({ z: value(s.z, `sections/${i}/z`, LENGTH), scale: s.scale.map((v, j) => value(v, `sections/${i}/scale/${j}`, SCALAR)) as [number, number], offset: s.offset.map((v, j) => value(v, `sections/${i}/offset/${j}`, LENGTH)) as [number, number] }))
      let mesh: ReturnType<typeof buildModelGraphLoft>
      try { mesh = buildModelGraphLoft(profile, sections) } catch (error) { return fail('invalid_loft', current, error instanceof Error ? error.message : 'Invalid loft.') }
      lines.push(`polyhedron(points=${JSON.stringify(mesh.points)},faces=${JSON.stringify(mesh.faces)});`)
    } else if (item.op === 'sketch') {
      if (sketch_solutions.length >= 16) fail('sketch_limit', current, 'Maximum 16 sketch solves per compilation.')
      const points = item.points.map((point, i) => ({ id: point.id, position: point.position.map((v, j) => value(v, `points/${i}/position/${j}`, LENGTH)) as [number, number] }))
      const constraints: SketchConstraint[] = item.constraints.map((c, i) => c.kind === 'fix' ? { ...c, at: c.at.map((v, j) => value(v, `constraints/${i}/at/${j}`, LENGTH)) as [number, number] } : c.kind === 'distance' ? { ...c, value: value(c.value, `constraints/${i}/value`, LENGTH) } : c)
      if (new Set(item.boundary).size !== item.boundary.length || item.boundary.some(key => !points.some(p => p.id === key))) fail('invalid_boundary', current, 'Boundary IDs must exist and be unique; closing is implicit.')
      let solved: ReturnType<typeof solveModelGraphSketch>
      try { solved = solveModelGraphSketch(points, constraints) } catch (error) { return fail('invalid_sketch', current, error instanceof Error ? error.message : 'Invalid sketch.') }
      const report = { ...solved, instance_path: current }
      sketch_solutions.push(report)
      if (solved.status !== 'solved' && !(solved.status === 'underconstrained' && item.allow_underconstrained)) throw new ModelGraphError(`sketch_${solved.status}`, current, 'Sketch could not produce a fully constrained profile. Inspect the solver report.', report)
      const polygon = item.boundary.map(key => solved.points.find(p => p.id === key)!.position)
      validateProfilePolygon(polygon, current, fail)
      lines.push(`polygon(points=${JSON.stringify(polygon)});`)
    } else if (item.op === 'rectangle') {
      const size = item.size.map((v, i) => value(v, `size/${i}`, LENGTH))
      if (size.some(v => v <= 0)) fail('invalid_dimension', current, 'Rectangle dimensions must be positive.')
      lines.push(`square([${size.join(',')}],center=${item.center});`)
    } else if (item.op === 'circle') {
      const radius = value(item.radius, 'radius', LENGTH)
      if (radius <= 0) fail('invalid_dimension', current, 'Circle radius must be positive.')
      lines.push(`circle(r=${radius});`)
    } else if (item.op === 'polygon') {
      const points = item.points.map((point, i) => point.map((v, j) => value(v, `points/${i}/${j}`, LENGTH)) as [number, number])
      if ((profileWork += points.length ** 2) > 262144) fail('profile_limit', current, 'Polygon validation work budget exceeded.')
      validateProfilePolygon(points, current, fail)
      lines.push(`polygon(points=${JSON.stringify(points)});`)
    } else if (item.op === 'extrude' || item.op === 'revolve') {
      if (item.op === 'extrude') {
        const height = value(item.height, 'height', LENGTH)
        if (height <= 0) fail('invalid_dimension', current, 'Extrusion height must be positive.')
        lines.push(`linear_extrude(height=${height},center=${item.center}){`)
      } else {
        const angle = value(item.angle, 'angle', ANGLE)
        if (angle <= 0 || angle > 360) fail('invalid_angle', current, 'Revolution angle must be greater than zero and at most 360 degrees.')
        lines.push(`rotate_extrude(angle=${angle}){`)
      }
      emit(item.input, nodes, scope, current, depth + 1, 'profile')
      lines.push('}')
    } else if (item.op === 'box') {
      const size = item.size.map((v, i) => value(v, `size/${i}`, LENGTH))
      if (size.some(v => v <= 0)) fail('invalid_dimension', current + '/size', 'Box dimensions must be positive.')
      lines.push(`cube([${size.join(',')}],center=${item.center});`)
    } else if (item.op === 'sphere' || item.op === 'cylinder') {
      const radius = value(item.radius, 'radius', LENGTH)
      if (radius <= 0) fail('invalid_dimension', current + '/radius', 'Radius must be positive.')
      if (item.op === 'sphere') lines.push(`sphere(r=${radius});`)
      else {
        const height = value(item.height, 'height', LENGTH)
        if (height <= 0) fail('invalid_dimension', current + '/height', 'Height must be positive.')
        lines.push(`cylinder(r=${radius},h=${height},center=${item.center});`)
      }
    } else if ('vector' in item) {
      const vector = item.vector.map((v, i) => value(v, `vector/${i}`, item.op === 'rotate' ? ANGLE : item.op === 'translate' ? LENGTH : SCALAR))
      if (expected === 'profile' && ((item.op === 'translate' && vector[2] !== 0) || (item.op === 'rotate' && (vector[0] !== 0 || vector[1] !== 0)) || (item.op === 'scale' && vector[2] !== 1))) fail('nonplanar_profile', current, 'Profiles must remain in XY: translate Z=0, rotate X=Y=0, scale Z=1.')
      if (item.op === 'scale' && vector.some(v => v === 0)) fail('invalid_scale', current + '/vector', 'Scale factors must be nonzero.')
      lines.push(`${item.op}([${vector.join(',')}]){`)
      emit(item.input, nodes, scope, current, depth + 1, expected); lines.push('}')
    } else {
      lines.push(`${item.op}(){`)
      for (const child of children(item)) emit(child, nodes, scope, current, depth + 1, expected)
      lines.push('}')
    }
  }
  emit(document.root, main, empty, '')
  return { document, geometry_assertions, constraint_report, sketch_solutions, assembly_components, mechanical_reports, mechanical_parts, document_sha256: sha256Hex(canonical(document)), source: lines.join('\n'), source_map: sourceMap, execution_target: 'legacy/current+manifold' as const }
}

export function setModelGraphParameters(value: unknown, expectedHash: string, updates: Array<{ id: string; value: number }>) {
  const compiled = compileModelGraph(value)
  if (compiled.document_sha256 !== expectedHash) throw new ModelGraphError('revision_conflict', '/expected_document_sha256', 'Document changed; read the current document and retry.')
  const known = new Set(compiled.document.parameters.map(item => item.id))
  const seen = new Set<string>()
  for (const update of updates) {
    if (!known.has(update.id) || seen.has(update.id)) throw new ModelGraphError('invalid_update', '/updates', 'Update IDs must exist and be unique.')
    seen.add(update.id)
  }
  const changed = new Map(updates.map(item => [item.id, item.value]))
  return compileModelGraph({ ...compiled.document, parameters: compiled.document.parameters.map(item => ({ ...item, value: changed.get(item.id) ?? item.value })) })
}

export const MODELGRAPH_EXAMPLE = {
  language: 'modelgraph/1', units: 'mm', parameters: [{ id: 'width', value: 40 }], segments: 48,
  nodes: [{ id: 'body', op: 'box', size: [{ param: 'width' }, 30, 8] }, { id: 'hole', op: 'cylinder', radius: 4, height: 10 }, { id: 'placedHole', op: 'translate', vector: [20, 15, -1], input: 'hole' }, { id: 'part', op: 'difference', base: 'body', subtract: ['placedHole'] }], root: 'part',
}

export const MODELGRAPH_GUIDE = MECHANICAL_GENERATOR_GUIDE + "\n\n" + `SVG workflows: modelgraph_svg_extrude(svg,height) converts inline SVG geometry into self-contained SCAD and verifies the extrusion. SVG can be authored with rect/circle/ellipse/polygon/path and groups/transforms; curves are tessellated, holes and supported strokes are preserved. modelgraph_svg_export(document,axis=x|y|z,face?) creates a silhouette or flattens one planar mesh face. Optional face={meshIndex,triangleIndex} uses zero-based full-build indices. No curved-surface unwrapping. Dimensions are millimeters; SVG is cropped to its contour bounds and reimport resets the viewport origin. Limits: 256 KiB SVG input, 20000 contour points, 20000 source triangles, 4 MiB SVG output.\n\nMCP modelgraph_modify measures actual bounds and returns complete validated documents: modification {operation:"resize",size:[mm,mm,mm],anchor:"minimum"|"center"|"origin"}, or {operation:"split",axis:"x"|"y"|"z",position:mm}. Split returns negative and positive halves with no kerf. These edits use current measured bounds and must be repeated after parameter changes. Assemblies are excluded. Additional CAD operations: affine(input,rows:[[scalar,scalar,scalar,length],...]) applies three rows of an invertible 3x4 affine matrix, with implicit last row [0,0,0,1]. Profiles must remain in XY. hull(inputs) computes a convex hull of same-dimensional geometry. mirror(input,normal:[scalar,scalar,scalar]) reflects about a plane through the origin; normal is nonzero and must lie in XY for profiles. offset(input:profile,distance:length) is the current engine's rounded profile offset; negative distances erode. Corner discretization uses the engine default and is not controlled by document segments. projection(input:solid) projects onto XY; section(input:solid,height:length) intersects with the plane Z=height and returns an XY profile. Both require extrusion before using as root. advanced_extrude(input:profile,height:length,twist:angle,top_scale:[scalar,scalar],slices:1..64,center=false) makes a linearly tapered/twisted solid; height/scales must be positive and abs(twist)<=3600 degrees. cone(radius_bottom:length,radius_top:length,height:length,center=false) allows either radius zero, but not both. torus(major_radius:length,minor_radius:length) requires major>minor>0. linear_pattern(input,count:1..256,step:[length,length,length]) unions translated copies starting at index zero. circular_pattern(input,count:1..256,angle_step:angle) unions copies rotated around global Z; translate the input first for a nonzero orbit radius. Patterns preserve profile/solid dimension. All scalar fields support functional expressions and parameters. These are mesh operations and do not implement exact BRep fillets or face selection.
ModelGraph/1 is a declarative JSON modeling language for MCP. For our own rational NURBS kernel, read modelgraph_nurbs_language and select modelgraph/nurbs-1; it exposes curves, surfaces, mathematical editing and explicit mesh tessellation. Loft solids: {id,op:"loft",profile:[[length,length],...],sections:[{z:length,scale:[scalar,scalar],offset:[length,length]},...]}. The base XY polygon must be strictly convex, without collinear vertices; either winding is accepted. Use 3..64 vertices and 2..32 parallel XY sections, strictly increasing Z and positive scales. Each section scales the base profile about XY origin, then offsets it. Corresponding vertices are joined by explicit triangles; caps are closed. This is a piecewise planar mesh loft, not spline interpolation or NURBS. No twist, arbitrary profile changes or holes in the base profile; use solid difference for hollow adapters. Boolean results still require actual kernel validation. Produce documents matching the accompanying JSON Schema. Units: millimeters; rotations: degrees, OpenSCAD Euler order (X then Y then Z); right-handed Z-up. IDs match [A-Za-z][A-Za-z0-9_]{0,31}. Parameters and nodes each have unique IDs. Scalar fields accept numbers or {"param":"id"}; strings of code are forbidden; structured functional expressions are supported. box uses positive size[3], sphere positive radius, cylinder positive radius and height. Box/cylinder center defaults false. translate/rotate/scale take vector[3] and input node ID; scale factors cannot be zero. union/intersection take inputs; difference takes base and subtract. Transforms wrap the referenced geometry. Every node must be reachable from root. Shared references instantiate geometry at each use; cycles are forbidden. Limits: 64 parameters, 128 nodes, depth 32, 4096 expanded uses, segments 12..128 (default 48). Assembly nodes: {id,op:"assembly",components:[{id,input:solidNodeId,anchors:[{id,origin:[length,length,length],rotation:[angle,angle,angle]}],placement?:{origin,rotation},mate?:{component:targetComponentId,anchor:targetAnchorId,own_anchor:localAnchorId,gap:length,rotation:[angle,angle,angle]}}]}. An assembly must be the document root or a direct component of another assembly. Assemblies inside boolean operations, transforms or functions are rejected. There are at most 64 expanded components across all nesting levels. Each component must declare exactly one placement or mate. At most 32 components and 32 anchors per component. Component and local anchor IDs must be unique. Components remain separate top-level geometry objects; they are not implicitly unioned. References to the same solid instantiate independent components.
Placement is a fixed rigid frame using degrees with X then Y then Z rotation order. Fixed mates are acyclic dependencies, not a numerical joint solver. The transform is targetComponent * targetAnchor * localGapAndRotation * inverse(ownAnchor). Gap is a signed displacement along target anchor Z; rotation is an additional local Euler rotation. Axes align by default; opposing faces require an explicit rotation. Gaps are declared frame offsets, not measured surface clearances. Unknown components/anchors, cycles and ambiguous placements are errors. assembly_components in compile/check/report returns each component's id,input,world matrix and named world anchor matrices. Matrices are row-major. Combined STL/OBJ export does not preserve assembly identities; keep the ModelGraph JSON as the assembly source of truth. Nested assembly instances expose instance_path, parent_path and is_assembly; leaf components also expose standalone source in world coordinates. Optional mate.joint is {kind:"revolute"|"slider",position:scalar,min:scalar,max:scalar}. Revolute positions/limits are angles; slider positions/limits are lengths. Position must be within inclusive limits. The joint rotates around local Z or translates along local Z after gap/rotation and before inverse(ownAnchor). Parameters and expressions can drive positions. These are acyclic kinematic placements, not physics simulation or a closed-loop assembly constraint solver. Call modelgraph_interference to measure volume overlap between at most 8 leaf components at current positions. It returns pair paths, intersection_volume_mm3 and status overlap/no_volume_overlap/unknown. Kernel failures are unknown; never interpret them as clearance. A tolerance of 1e-6 mm3 applies. Contact, clearance and swept motion are not checked.
Call modelgraph_report for actual geometry measurements, operation provenance and front/top/isometric PNG views. Images are bounded to 20000 triangles and 8000000 raster candidates and may be unavailable; consult images_status. Reports do not certify printability or persistent CAD face identity. First call modelgraph_compile, fix errors using their JSON path, then modelgraph_check for actual geometry validation. Edit parameters through modelgraph_set_parameters using the returned document_sha256. Keep the returned document as the source of truth. Generated SCAD is an execution artifact usable with existing export tools. No tool here persists the graph. Stable IDs identify document nodes, not stable CAD faces. This frontend currently targets the existing Manifold mesh engine, not a new geometry kernel, exact B-rep, CUDA or a full OpenSCAD replacement.
Profiles and solid features: rectangle(size:[length,length],center=false), circle(radius:length), polygon(points:[[length,length],...]) create XY profiles. Polygon points implicitly close the boundary; do not repeat the first point. A polygon has 3..256 points and must be simple with nonzero area. Holes are modeled with profile difference; union/intersection/difference must combine the same geometry dimension. extrude(input:profileId,height:positiveLength,center=false) and revolve(input:profileId,angle:positiveAngle<=360deg) create solids. Revolve interprets profile X as radius and Y as height around Z; place the profile on one side of the axis and validate the actual result using modelgraph_check. Transforms of profiles must preserve XY: translate Z=0, rotate X=Y=0, scale Z=1. Root must be solid. Functions and geometry values can carry profiles; expected profile/solid dimension is checked when emitted. Node references use input as elsewhere. Polygon-validation work is capped at 262144 point-pair budget units across expanded uses. A sketch node adds a bounded straight-segment constraint solver; arc constraints and edge fillets are not implemented.`


export const MODELGRAPH_FUNCTIONAL_GUIDE = `Functional extension (backward compatible with ModelGraph/1):
Values are numbers, immutable lists, lexical closures and immutable geometry values. No mutation, IO, clock, random or eval. All numeric intermediates are finite and within +/-1000000. Predicates return 0 or 1; conditions treat zero as false. Trigonometry uses degrees.
Expressions: {param:id} reads a document parameter; {local:id} reads a lexical binding. Binary {op,args:[a,b]}: add, subtract, multiply, divide, mod, pow, min, max, lt, le, eq, and, or. Unary {op,value}: negate, abs, sqrt, sin, cos, floor, ceil, not. if uses condition, then, else and evaluates only the selected branch; and/or short circuit. let uses name,value,body; the new binding exists only in body. lambda uses parameters:[ids],body and captures its definition scope. apply uses function:expression,args:[expressions]; arguments are positional. Duplicate binders are errors; lexical shadowing is allowed.
Endpoint sequences: interval uses start,end,inclusive and optional count OR step expressions. Endpoints and step must share dimensions. Count is dimensionless integer 0..256, 0 gives empty, 1 gives start (exclusive equal endpoints with positive count are invalid); inclusive samples include end when count>1. Without count, step is nonzero; implicit +1 only for dimensionless integer endpoints. Wrong direction gives empty; unreachable endpoint is not appended. Count/step are mutually exclusive. zip(inputs) requires 2..8 equal-length lists; enumerate(input) produces [index,value] pairs. flatmap(input,function) concatenates returned sequences with maximum 256 output elements. Geometry collect node {id,op:"collect",values:expression,binding:localId,input:nodeId} expands separately emitted instances using each sequence value. group node {id,op:"group",inputs:[nodeIds]} likewise emits separate parts. They preserve separate scene meshes at root; enclosing booleans merge according to the boolean operation. Positional instance_path references are not persistent semantic keys. No joint/assembly semantics implied.
Lists: list uses items; range uses count,start,step (count 0..256); at uses input,index (zero based); length uses input. map and filter use input and function (one argument: element). reduce uses input,function,initial and folds left with callback(accumulator,element). Empty reduce returns initial. Lists may hold closures or lists; a geometric numeric field must resolve to a number.
Document functions: {id,kind:"scalar"|"value",parameters:[ids],body:expression} or {id,kind:"geometry",parameters:[ids],nodes:[...],root:id}. scalar must return a number; value can return a list or closure. Named call uses {op:"call",function:id,args:{parameter:expression}}. Geometry call is a node with an id and the same fields. Exact named argument matching is required. Named functions see document parameters and their own arguments, never caller locals. Pass closures as arguments to write higher-order value functions. The expression {op:"geometry",function:id,args:{name:expression}} constructs an immutable geometry value with bound arguments. A geometry node {id,op:"evaluate",value:expression} emits a geometry value. Closures may return geometry; lists and higher-order functions may carry geometry values. Geometry functions can accept geometry values and use evaluate nodes to compose them.
Geometry if nodes have condition,then:nodeId,else:nodeId. Geometry map nodes have count (1..256),index:localId,input:nodeId; each instance receives an immutable zero-based index and results are unioned. Node IDs are local to each geometry body. Document assertions:[{condition:expression,message:string}] run before geometry emission. source_map includes instance_path identifying function calls and map instances.
Bounds: 32 functions, 32 arguments per function, expression/call depth 32, 100000 expression steps, 16384 allocated list slots, 4096 expanded geometry nodes, geometry depth 32, 20000 input values and nesting depth 64. Recursion is permitted within these budgets; no tail-call optimization. Functions and inactive branches are schema checked; value-dependent errors are detected when evaluated. This is a bounded functional modeling DSL, not general-purpose JavaScript or a new CAD kernel.`

export const MODELGRAPH_FUNCTIONAL_EXAMPLE = {
  language: 'modelgraph/1', units: 'mm', parameters: [{ id: 'count', value: 3 }],
  functions: [{ id: 'block', kind: 'geometry', parameters: ['width'],
    nodes: [{ id: 'solid', op: 'box', size: [{ local: 'width' }, 2, 2] }], root: 'solid' }],
  assertions: [{ condition: { op: 'le', args: [{ param: 'count' }, 20] }, message: 'Use at most 20 blocks.' }],
  nodes: [
    { id: 'block', op: 'call', function: 'block', args: { width: { op: 'apply', function: { op: 'lambda', parameters: ['x'], body: { op: 'multiply', args: [{ local: 'x' }, 2] } }, args: [1] } } },
    { id: 'placed', op: 'translate', vector: [{ op: 'multiply', args: [{ local: 'i' }, 3] }, 0, 0], input: 'block' },
    { id: 'parts', op: 'map', count: { param: 'count' }, index: 'i', input: 'placed' },
  ], root: 'parts',
}


export const MODELGRAPH_UNITS_GUIDE = `Types, units and declared constraints:
For new documents use type_policy:"strict". Expressions {op:"quantity",value:number,unit:"mm"|"cm"|"m"|"in"|"deg"|"rad"} create typed lengths/angles. Values normalize internally to mm and degrees; returned documents retain the original units. Bare numbers are dimensionless. Default/legacy policy still interprets bare numbers as mm/degrees at geometry fields; strict policy requires correct dimensions there, except bare zero is accepted for any geometry dimension. Explicitly typed values are always checked, including in legacy mode.
Box sizes, radius, height and translation require length; rotation requires angle; scale, predicates, indices and counts require dimensionless numbers. Function arguments, closures, lists, geometry values and scalar-function returns preserve quantities. Checks occur during evaluation, not by whole-program static inference. Inactive branches retain lazy semantics.
Add/subtract/min/max/mod/comparisons require identical dimensions; implicit number-to-length promotion in arithmetic is forbidden. Multiply/divide combine dimensions; length/length is dimensionless. Powers require a dimensionless exponent; resulting length/angle exponents must be integers within +/-8. sqrt halves exponents. sin/cos accept angles, returning dimensionless values (legacy mode also accepts bare degrees). abs/negate preserve dimensions; floor/ceil round canonical mm/degrees, preserving dimensions. Typed ranges require matching start/step dimensions; count remains dimensionless. Numeric limits apply after conversion as well as during arithmetic.
Parameters may declare unit,min,max,integer. Bounds and update values are expressed in the parameter's declared unit; modelgraph_set_parameters preserves that unit and revalidates all bounds/constraints atomically. An omitted unit denotes a dimensionless parameter.
Document constraints:[{id,left:expression,relation:"le"|"ge"|"eq"|"lt"|"gt",right:expression,tolerance?:expression,message:string}] compare numeric values with identical dimensions. Tolerance must have the same dimensions and be nonnegative; omitted means exact comparison. lt/gt are strict comparisons without tolerance. le means left <= right+tolerance; ge means left >= right-tolerance; eq means abs(left-right) <= tolerance. IDs must be unique. Maximum 64 constraints. constraint_report contains id,path,passed,actual,expected,relation,tolerance,dimension:[lengthExponent,angleExponent],message; measurements use canonical mm/degrees. If any fail, compilation returns constraint_failed with the entire report in error.details and emits no geometry. Legacy assertions still work.
These are checks of declared expressions, not a constraint solver or measured mesh-wall analysis. A minimumWall constraint checks the expression you provide; it does not prove every wall in the finished mesh meets that minimum. No automatic parameter repair, sketches or geometric constraint solving is introduced.`

export const MODELGRAPH_UNITS_EXAMPLE = {
  language: 'modelgraph/1', units: 'mm', type_policy: 'strict',
  parameters: [{ id: 'width', value: 4, unit: 'cm', min: 1, max: 20 }, { id: 'wall', value: 2, unit: 'mm', min: 0.1, max: 10 }],
  constraints: [{ id: 'minimumWall', left: { param: 'wall' }, relation: 'ge', right: { op: 'quantity', value: 1.2, unit: 'mm' }, message: 'Declared plate thickness must be at least 1.2 mm.' }],
  nodes: [{ id: 'plate', op: 'box', size: [{ param: 'width' }, { op: 'quantity', value: 20, unit: 'mm' }, { param: 'wall' }] }], root: 'plate',
}


export const MODELGRAPH_SKETCH_GUIDE = `Sketch constraints (straight-segment profiles):
A profile node {id,op:"sketch",points:[{id,position:[x,y]}],boundary:[pointIds],constraints:[...],allow_underconstrained?:false} solves coordinates before profile emission. Positions are initial guesses, not fixed values. Point positions, fixed targets and distances accept length expressions, respecting strict units. Boundary is an ordered simple closed polygon; closure is implicit and IDs must be unique. Extra points may be construction points but their degrees of freedom still count.
Constraint forms: {id,kind:"fix",point:id,at:[x,y]}; {id,kind:"horizontal"|"vertical"|"coincident",a:pointId,b:pointId}; {id,kind:"distance",a,b,value:positiveLength}; {id,kind:"parallel"|"perpendicular"|"equal_length",a,b,c,d} relates segments a-b and c-d. Use coincident for zero distance. Parallel/perpendicular reject zero-length segments at the resulting solution.
The bounded numerical solver uses initial guesses to select a local solution. Limits: 3..16 points, at most 48 constraints, 64 iterations, 16 expanded sketch solves per compilation. Tolerance is 0.000001 mm. sketch_solutions contains solved coordinates, per-constraint residual_mm/satisfied, maximum residual, iterations, local degrees_of_freedom (Jacobian nullity) and redundant_equations (local equation-count minus rank). Rank is numerical, not a global uniqueness proof. Redundancy counts equations, not necessarily removable constraints.
Statuses: solved; underconstrained (satisfied but local freedom remains); inconsistent (unsatisfied linear system with augmented-rank conflict); not_converged (nonlinear solver exhausted its budget, singular start or degenerate segment). Unsatisfied residuals identify problematic constraints but are not a proven minimal conflict set. No global impossibility claim is made for nonlinear nonconvergence. By default only solved sketches emit geometry. allow_underconstrained:true explicitly accepts a locally underconstrained solution using the initial guess. Invalid or self-intersecting solved boundaries still fail profile validation. Compilation errors include the solver report in error.details; successful compilation and modelgraph_report include sketch_solutions.
No arc/circle/tangency solver, assembly solver or automatic repair is implemented by this node.`

export const MODELGRAPH_SKETCH_EXAMPLE = {
  language: 'modelgraph/1', units: 'mm', parameters: [{ id: 'width', value: 20 }],
  nodes: [{ id: 'outline', op: 'sketch', points: [
    { id: 'a', position: [0.2, -0.1] }, { id: 'b', position: [19, 1] }, { id: 'c', position: [21, 9] }, { id: 'd', position: [-1, 11] },
  ], boundary: ['a', 'b', 'c', 'd'], constraints: [
    { id: 'origin', kind: 'fix', point: 'a', at: [0, 0] },
    { id: 'bottom', kind: 'horizontal', a: 'a', b: 'b' }, { id: 'right', kind: 'vertical', a: 'b', b: 'c' },
    { id: 'top', kind: 'horizontal', a: 'c', b: 'd' }, { id: 'left', kind: 'vertical', a: 'd', b: 'a' },
    { id: 'width', kind: 'distance', a: 'a', b: 'b', value: { param: 'width' } }, { id: 'height', kind: 'distance', a: 'b', b: 'c', value: 10 },
  ] }, { id: 'part', op: 'extrude', input: 'outline', height: 3 }], root: 'part',
}


export const MODELGRAPH_ASSEMBLY_EXAMPLE = {
  language: 'modelgraph/1', units: 'mm', parameters: [{ id: 'gap', value: 0.3 }],
  nodes: [
    { id: 'body', op: 'box', size: [20, 10, 5] },
    { id: 'cover', op: 'box', size: [20, 10, 2] },
    { id: 'product', op: 'assembly', components: [
      { id: 'base', input: 'body', anchors: [{ id: 'top', origin: [0,0,5], rotation: [0,0,0] }], placement: { origin: [0,0,0], rotation: [0,0,0] } },
      { id: 'lid', input: 'cover', anchors: [{ id: 'bottom', origin: [0,0,0], rotation: [0,0,0] }], mate: { component: 'base', anchor: 'top', own_anchor: 'bottom', gap: { param: 'gap' }, rotation: [0,0,0] } },
    ] },
  ], root: 'product',
}

export const MODELGRAPH_LOFT_EXAMPLE = {
  language: 'modelgraph/1', units: 'mm', parameters: [],
  nodes: [{ id: 'adapter', op: 'loft', profile: [[-10,-10],[10,-10],[10,10],[-10,10]], sections: [
    { z: 0, scale: [1,1], offset: [0,0] },
    { z: 20, scale: [0.5,0.5], offset: [5,0] },
  ] }], root: 'adapter',
} as const
