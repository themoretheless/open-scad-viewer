/**
 * Name binding for a compiled OpenSCAD operation IR.
 *
 * Syntax stays in `openscadCompiler.ts`. This pass only walks the frozen AST,
 * records module/function declarations and resolves call names. It does not
 * evaluate expressions or touch a geometry kernel.
 */
import { sha256Hex } from '../core/sha256'
import { OPENSCAD_2021_01_BUILTIN_FUNCTION_NAMES } from './openScadBuiltinFunctions'
import {
  compileOpenSCAD,
  type CallNode,
  type Expr,
  type ExpressionArgument,
  type FunctionNode,
  type ModuleNode,
  type OpenScadLanguageProfile,
  type Statement,
} from './openscadCompiler'

export const OPENSCAD_BUILTIN_MODULE_NAMES = Object.freeze([
  'assign', 'assert', 'child', 'children', 'circle', 'color', 'cube', 'cylinder',
  'difference', 'dxf_linear_extrude', 'dxf_rotate_extrude', 'echo', 'for', 'group',
  'hull', 'if', 'import', 'import_dxf', 'import_off', 'import_stl',
  'intersection', 'intersection_for', 'let', 'linear_extrude', 'minkowski',
  'mirror', 'multmatrix', 'offset', 'polygon', 'polyhedron', 'projection',
  'render', 'resize', 'rotate', 'rotate_extrude', 'scale', 'sphere', 'square',
  'surface', 'text', 'translate', 'union',
] as const)

export type OpenScadBuiltinModuleName = typeof OPENSCAD_BUILTIN_MODULE_NAMES[number]

export type BoundCallKind = 'builtin-module' | 'user-module' | 'unresolved'
export type BoundFunctionKind = 'builtin-function' | 'user-function' | 'unresolved'

export interface BoundCall {
  readonly operationId: string
  readonly name: string
  readonly kind: BoundCallKind
  readonly p: number
  readonly end: number
}

export interface BoundFunctionRef {
  readonly name: string
  readonly kind: BoundFunctionKind
  readonly p: number
  readonly end: number
}

export interface BindDiagnostic {
  readonly severity: 'warning'
  readonly p: number
  readonly end: number
  readonly message: string
}

export interface BoundProgram {
  readonly languageProfile: OpenScadLanguageProfile
  readonly program: readonly Statement[]
  readonly modules: ReadonlyMap<string, ModuleNode>
  readonly functions: ReadonlyMap<string, FunctionNode>
  readonly calls: readonly BoundCall[]
  readonly functionRefs: readonly BoundFunctionRef[]
  readonly diagnostics: readonly BindDiagnostic[]
  readonly cacheKey: string
}

export interface BindOpenScadOptions {
  readonly languageProfile?: OpenScadLanguageProfile
  readonly source?: string
}

export interface PrepareOpenScadFrontEndOptions {
  readonly languageProfile?: OpenScadLanguageProfile
  readonly compile?: (source: string) => readonly Statement[]
  readonly now?: () => number
}

export interface PreparedOpenScadFrontEnd {
  readonly program: readonly Statement[]
  readonly bound: BoundProgram
  readonly cacheHit: boolean
  readonly cacheKey: string
  readonly compileMs: number
  readonly bindMs: number
}

const BUILTIN_MODULES = new Set<string>(OPENSCAD_BUILTIN_MODULE_NAMES)
const BUILTIN_FUNCTIONS = new Set<string>(OPENSCAD_2021_01_BUILTIN_FUNCTION_NAMES)
const FRONT_END_CACHE_LIMIT = 32

const frontEndCache = new Map<string, PreparedOpenScadFrontEnd>()

export function bindProgramCacheKey(source: string, languageProfile: OpenScadLanguageProfile): string {
  return sha256Hex(`${languageProfile}\n${source}`)
}

export function resetCompileBindCache(): void {
  frontEndCache.clear()
}

export function compileBindCacheSize(): number {
  return frontEndCache.size
}

function collectModules(nodes: readonly Statement[], modules: Map<string, ModuleNode>, diagnostics: BindDiagnostic[]) {
  for (const node of nodes) {
    if (node.type === 'module') {
      if (modules.has(node.name)) {
        diagnostics.push(Object.freeze({
          severity: 'warning',
          p: node.p,
          end: node.end,
          message: `Module ${node.name}() is redefined; the later definition is used`,
        }))
      }
      modules.set(node.name, node)
    }
    if (node.type === 'call') {
      collectModules(node.children, modules, diagnostics)
      collectModules(node.alternative, modules, diagnostics)
    }
  }
}

function collectFunctions(nodes: readonly Statement[], functions: Map<string, FunctionNode>, diagnostics: BindDiagnostic[]) {
  for (const node of nodes) {
    if (node.type === 'function') {
      if (functions.has(node.name)) {
        diagnostics.push(Object.freeze({
          severity: 'warning',
          p: node.p,
          end: node.end,
          message: `Function ${node.name}() is redefined; the later definition is used`,
        }))
      }
      functions.set(node.name, node)
    }
    if (node.type === 'call') {
      collectFunctions(node.children, functions, diagnostics)
      collectFunctions(node.alternative, functions, diagnostics)
    } else if (node.type === 'module') collectFunctions(node.children, functions, diagnostics)
  }
}

function visitExpression(
  root: Expr,
  functions: ReadonlyMap<string, FunctionNode>,
  functionRefs: BoundFunctionRef[],
): void {
  const stack: Expr[] = [root]
  while (stack.length) {
    const expr = stack.pop()!
    switch (expr.kind) {
      case 'literal':
      case 'identifier':
        break
      case 'vector':
        for (let index = expr.items.length - 1; index >= 0; index--) stack.push(expr.items[index]!)
        break
      case 'range':
        stack.push(expr.end)
        if (expr.step) stack.push(expr.step)
        stack.push(expr.start)
        break
      case 'unary':
      case 'member':
      case 'lc-each':
        stack.push(expr.value)
        break
      case 'binary':
        stack.push(expr.right)
        stack.push(expr.left)
        break
      case 'ternary':
        stack.push(expr.no)
        stack.push(expr.yes)
        stack.push(expr.test)
        break
      case 'index':
        stack.push(expr.index)
        stack.push(expr.value)
        break
      case 'function':
        stack.push(expr.body)
        break
      case 'call':
        if (expr.name !== null) {
          functionRefs.push(Object.freeze({
            name: expr.name,
            kind: BUILTIN_FUNCTIONS.has(expr.name)
              ? 'builtin-function'
              : functions.has(expr.name)
                ? 'user-function'
                : 'unresolved',
            p: expr.p,
            end: expr.p,
          }))
        }
        for (let index = expr.args.length - 1; index >= 0; index--) stack.push(expr.args[index]!.value)
        stack.push(expr.callee)
        break
      case 'let':
      case 'lc-let':
      case 'lc-for':
        stack.push(expr.body)
        for (let index = expr.args.length - 1; index >= 0; index--) stack.push(expr.args[index]!.value)
        break
      case 'assert':
      case 'echo':
        if (expr.body) stack.push(expr.body)
        for (let index = expr.args.length - 1; index >= 0; index--) stack.push(expr.args[index]!.value)
        break
      case 'lc-for-c':
        stack.push(expr.body)
        for (let index = expr.update.length - 1; index >= 0; index--) stack.push(expr.update[index]!.value)
        stack.push(expr.condition)
        for (let index = expr.init.length - 1; index >= 0; index--) stack.push(expr.init[index]!.value)
        break
      case 'lc-if':
        if (expr.no) stack.push(expr.no)
        stack.push(expr.yes)
        stack.push(expr.condition)
        break
      default:
        break
    }
  }
}

function visitArguments(
  args: readonly ExpressionArgument[] | undefined,
  values: Record<string, Expr> | undefined,
  functions: ReadonlyMap<string, FunctionNode>,
  functionRefs: BoundFunctionRef[],
): void {
  if (args) {
    for (const argument of args) visitExpression(argument.value, functions, functionRefs)
    return
  }
  if (values) {
    for (const value of Object.values(values)) visitExpression(value, functions, functionRefs)
  }
}

function visitStatements(
  nodes: readonly Statement[],
  modules: ReadonlyMap<string, ModuleNode>,
  functions: ReadonlyMap<string, FunctionNode>,
  calls: BoundCall[],
  functionRefs: BoundFunctionRef[],
  diagnostics: BindDiagnostic[],
): void {
  for (const node of nodes) {
    if (node.type === 'assign') {
      visitExpression(node.value, functions, functionRefs)
      continue
    }
    if (node.type === 'function') {
      visitExpression(node.body, functions, functionRefs)
      continue
    }
    if (node.type === 'module') {
      visitStatements(node.children, modules, functions, calls, functionRefs, diagnostics)
      continue
    }
    const kind: BoundCallKind = BUILTIN_MODULES.has(node.name)
      ? 'builtin-module'
      : modules.has(node.name)
        ? 'user-module'
        : 'unresolved'
    calls.push(Object.freeze({
      operationId: node.operationId ?? `op:legacy-offset-${node.p}`,
      name: node.name,
      kind,
      p: node.p,
      end: node.end,
    }))
    if (kind === 'unresolved') {
      diagnostics.push(Object.freeze({
        severity: 'warning',
        p: node.p,
        end: node.end,
        message: `Unresolved module ${node.name}()`,
      }))
    }
    visitArguments(node.callArguments, node.args, functions, functionRefs)
    visitStatements(node.children, modules, functions, calls, functionRefs, diagnostics)
    visitStatements(node.alternative, modules, functions, calls, functionRefs, diagnostics)
  }
}

function freezeBound(bound: BoundProgram): BoundProgram {
  Object.freeze(bound.calls)
  Object.freeze(bound.functionRefs)
  Object.freeze(bound.diagnostics)
  return Object.freeze(bound)
}

/** Bind names on an already-compiled, immutable operation IR. */
export function bindOpenScad(
  program: readonly Statement[],
  options: BindOpenScadOptions = {},
): BoundProgram {
  const languageProfile = options.languageProfile ?? 'openscad-viewer-subset@1'
  const diagnostics: BindDiagnostic[] = []
  const modules = new Map<string, ModuleNode>()
  const functions = new Map<string, FunctionNode>()
  collectModules(program, modules, diagnostics)
  collectFunctions(program, functions, diagnostics)
  const calls: BoundCall[] = []
  const functionRefs: BoundFunctionRef[] = []
  visitStatements(program, modules, functions, calls, functionRefs, diagnostics)
  return freezeBound({
    languageProfile,
    program,
    modules,
    functions,
    calls,
    functionRefs,
    diagnostics,
    cacheKey: options.source === undefined
      ? ''
      : bindProgramCacheKey(options.source, languageProfile),
  })
}

/**
 * Content-addressed compile+bind front-end. Cache keys are the language
 * profile and source digest; the geometry kernel is never loaded.
 */
export function prepareOpenScadFrontEnd(
  source: string,
  options: PrepareOpenScadFrontEndOptions = {},
): PreparedOpenScadFrontEnd {
  const languageProfile = options.languageProfile ?? 'openscad-viewer-subset@1'
  const cacheKey = bindProgramCacheKey(source, languageProfile)
  const cached = frontEndCache.get(cacheKey)
  if (cached) return { ...cached, cacheHit: true, compileMs: 0, bindMs: 0 }
  const now = options.now ?? (() => 0)
  const compile = options.compile ?? (text => compileOpenSCAD(text, { languageProfile }))
  const compileStarted = now()
  const program = compile(source)
  const compiledAt = now()
  const bound = bindOpenScad(program, { languageProfile, source })
  const boundAt = now()
  const prepared: PreparedOpenScadFrontEnd = {
    program,
    bound,
    cacheHit: false,
    cacheKey,
    compileMs: Math.max(0, compiledAt - compileStarted),
    bindMs: Math.max(0, boundAt - compiledAt),
  }
  frontEndCache.set(cacheKey, prepared)
  if (frontEndCache.size > FRONT_END_CACHE_LIMIT) {
    const oldest = frontEndCache.keys().next().value
    if (oldest !== undefined) frontEndCache.delete(oldest)
  }
  return prepared
}
