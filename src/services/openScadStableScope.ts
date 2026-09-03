import type {
  Expr,
  FunctionNode,
  ModuleNode,
  Statement,
  Value,
} from './openscadCompiler'

interface VariableBinding {
  readonly firstStatement: number
  readonly value: Expr
}

export interface OpenScadFunctionDeclaration {
  readonly node: FunctionNode
  readonly scope: OpenScadStableScope
}

export interface OpenScadModuleDeclaration {
  readonly node: ModuleNode
  readonly scope: OpenScadStableScope
}

export interface OpenScadVariableResolution {
  readonly found: boolean
  readonly value: Value
}

export interface OpenScadScopeEvaluationSite {
  readonly scope: OpenScadStableScope
  readonly visibleBefore: number
  readonly env: Map<string, Value>
}

/**
 * One activated OpenSCAD lexical statement scope.
 *
 * The plan is textual, while values are lazy and cached per activation. This
 * captures OpenSCAD's unusual assignment rule: the last assignment supplies
 * the value everywhere in the scope, but its RHS sees only bindings which were
 * already present when that target was first introduced.
 */
export class OpenScadStableScope {
  readonly env: Map<string, Value>
  private readonly variables = new Map<string, VariableBinding>()
  private readonly functions = new Map<string, FunctionNode>()
  private readonly modules = new Map<string, ModuleNode>()
  private readonly values = new Map<string, Value>()
  private readonly resolving = new Set<string>()

  constructor(
    statements: readonly Statement[],
    readonly parent: OpenScadStableScope | null,
    env: ReadonlyMap<string, Value>,
  ) {
    this.env = new Map(env)
    statements.forEach((statement, statementIndex) => {
      if (statement.type === 'assign') {
        const previous = this.variables.get(statement.name)
        this.variables.set(statement.name, {
          firstStatement: previous?.firstStatement ?? statementIndex,
          value: statement.value,
        })
      } else if (statement.type === 'function') {
        this.functions.set(statement.name, statement)
      } else if (statement.type === 'module') {
        this.modules.set(statement.name, statement)
      }
    })
  }

  dynamicVariableNames(): readonly string[] {
    return [...this.variables.keys()].filter(name => name.startsWith('$'))
  }

  resolveLocalVariable(
    name: string,
    visibleBefore: number,
    evaluate: (expression: Expr, site: OpenScadScopeEvaluationSite) => Value,
    warn: (message: string) => void,
  ): OpenScadVariableResolution {
    const binding = this.variables.get(name)
    if (binding === undefined || binding.firstStatement >= visibleBefore) {
      return { found: false, value: undefined }
    }
    if (this.values.has(name)) return { found: true, value: this.values.get(name) }
    if (this.resolving.has(name)) {
      warn(`Ignoring cyclic variable reference '${name}'`)
      return { found: true, value: undefined }
    }
    this.resolving.add(name)
    try {
      const value = evaluate(binding.value, {
        scope: this,
        visibleBefore: binding.firstStatement,
        env: new Map(this.env),
      })
      this.values.set(name, value)
      return { found: true, value }
    } finally {
      this.resolving.delete(name)
    }
  }

  functionDeclaration(name: string): OpenScadFunctionDeclaration | undefined {
    const node = this.functions.get(name)
    if (node !== undefined) return { node, scope: this }
    return this.parent?.functionDeclaration(name)
  }

  moduleDeclaration(name: string): OpenScadModuleDeclaration | undefined {
    const node = this.modules.get(name)
    if (node !== undefined) return { node, scope: this }
    return this.parent?.moduleDeclaration(name)
  }
}
