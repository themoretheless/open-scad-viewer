import { LegacyBindingCompatibilityError, SemanticProgramBuilder, StableViewportRootSelection } from '../openscadSemanticLowerer'
import type { EvalContext, SemanticLoweringOptions, SemanticShape } from '../openscadSemanticLowerer'
import { compileOpenSCAD } from '../openscadCompiler'
import type { FunctionNode, ModuleNode, Value } from '../openscadCompiler'
import { OpenSCADParseError } from '../openscadErrors'
import { SEMANTIC_PROGRAM_CAPABILITY_GRAPH, SEMANTIC_PROGRAM_EXECUTION_FEATURE, SEMANTIC_PROGRAM_IDENTITY, deriveSemanticCapabilityClosure } from '../../core/semanticProgram'
import type { SemanticProgramCoreV1 } from '../../core/semanticProgram'
import type { SemanticLoweringSuccess } from '../semanticProgramTrust'
import { attestSemanticProgram, encodeSemanticProgram } from '../semanticProgramCodec'
import { canonicalizeCompletedDag, countStatements, resultFromShapes } from './evalUserModule'
import { collectFunctions, collectModules, detachedChildrenContinuationMap, evalNodes } from './evalExpression'
import { createOpenScadStableRuntimeVariables } from '../openScadStableRuntime'
import { evalPreparedNodes } from './evalPreparedNodes'
import { normalizeSemanticProgram, semanticSourceDescriptor } from '../semanticProgramValidator'
import { parseGeometrySourceRoutingHeader } from '../../core/geometryRouting'
import { registerOperations } from './semanticValueType'

/** @internal Use the trust-owning semanticProgramLowerer facade. */
export function lowerOpenSCADToSemanticProgramUnchecked(
  source: string,
  options: SemanticLoweringOptions = {},
): SemanticLoweringSuccess {
  const route = parseGeometrySourceRoutingHeader(source)
  const ast = compileOpenSCAD(source, { languageProfile: options.languageProfile })
  const modules = new Map<string, ModuleNode>()
  const functions = new Map<string, FunctionNode>()
  collectModules(ast, modules)
  collectFunctions(ast, functions)
  const moduleNames = new Set(modules.keys())
  const languageProfile = options.languageProfile ?? 'openscad-viewer-subset@1'
  const registered = registerOperations(
    ast,
    moduleNames,
    languageProfile,
  )
  let builder = new SemanticProgramBuilder(source, route.languageContract, registered)
  const budget = { ops: 0 }
  const reduced = { value: false }
  const valueBudget = { used: 0 }
  const quality = options.quality ?? 'full'
  const env = languageProfile === 'openscad/stable-2021.01'
    ? new Map<string, Value>(Array.from(
        createOpenScadStableRuntimeVariables({ quality, animationTime: options.animationTime }),
        ([name, value]) => [name, Array.isArray(value) ? [...value] : value as Value],
      ))
    : new Map<string, Value>([['$fn', 0], ['$fa', 12], ['$fs', 2]])
  const context: EvalContext = {
    source,
    languageProfile,
    env,
    functions,
    modules,
    builder,
    quality,
    depth: 0,
    functionStack: [],
    moduleStack: [],
    budget,
    reduced,
    valueBudget,
    valueWeights: new WeakMap(),
    valueDepths: new WeakMap(),
    parentOccurrence: null,
    pendingSlots: [],
    shouldAbort: options.shouldAbort,
  }
  let shapes: SemanticShape[] = []
  let terminalError: OpenSCADParseError | LegacyBindingCompatibilityError | null = null
  let terminalDiagnostic: number | null = null
  const captureFailure = (error: unknown): void => {
    const capturable = error instanceof OpenSCADParseError
      || error instanceof LegacyBindingCompatibilityError
    if (!options.captureTerminalFailure || !capturable) throw error
    if (builder.terminalOccurrence === null) {
      throw new Error('Capturable semantic failure has no interrupted occurrence')
    }
    terminalError = error
    terminalDiagnostic = builder.terminal(error, builder.terminalOccurrence)
  }
  try {
    shapes = evalNodes(ast, context, false)
  } catch (error) {
    if (error instanceof StableViewportRootSelection) {
      const detachedContinuations = detachedChildrenContinuationMap(
        error.node,
        error.context.callChildren,
      )
      const rootBuilder = new SemanticProgramBuilder(
        source,
        route.languageContract,
        registerOperations(ast, moduleNames, languageProfile, error.node, detachedContinuations),
      )
      rootBuilder.inheritDiagnostics(builder, error.diagnosticsBeforeCandidate)
      builder = rootBuilder
      try {
        shapes = evalPreparedNodes([error.node], {
          ...error.context,
          builder,
          parentOccurrence: null,
          pendingSlots: [],
          viewportRootLocked: true,
          viewportRootOwner: error.node,
        })
      } catch (rootError) {
        captureFailure(rootError)
      }
    } else {
      captureFailure(error)
    }
  }
  const canonical = canonicalizeCompletedDag(
    builder,
    terminalError === null ? resultFromShapes(shapes) : { tag: 'empty', type: 'never' },
    terminalDiagnostic,
    builder.terminalOccurrence,
  )
  const coreDraft: SemanticProgramCoreV1 = {
    schema: 'semantic-program-core',
    schemaVersion: { major: 1, minor: 2 },
    requiredFeatures: [SEMANTIC_PROGRAM_EXECUTION_FEATURE],
    identityVersion: SEMANTIC_PROGRAM_IDENTITY,
    language: {
      contract: route.languageContract,
      semanticsRevision: route.languageContract === 'legacy/current' ? '1.0.0' : 'brep-1.0.0',
      capabilityGraphVersion: SEMANTIC_PROGRAM_CAPABILITY_GRAPH,
    },
    units: {
      length: 'millimeter', angle: 'degree', handedness: 'right', upAxis: 'z',
      matrixLayout: 'column-major', composition: 'parent-times-local',
    },
    operations: builder.registered.operations,
    occurrences: builder.occurrences,
    nodes: builder.nodes,
    execution: canonical.execution,
    result: canonical.result,
    declaredCapabilities: route.requiredCapabilities,
    capabilityClosure: [],
    diagnosticTemplates: builder.diagnosticTemplates,
  }
  const core: SemanticProgramCoreV1 = {
    ...coreDraft,
    capabilityClosure: deriveSemanticCapabilityClosure(coreDraft),
  }
  const envelope = {
    schema: 'semantic-program-envelope' as const,
    schemaVersion: { major: 1 as const, minor: 2 as const },
    source: semanticSourceDescriptor(source),
    core,
    provenance: builder.registered.provenance,
    tessellationIntents: [...builder.tessellationIntents].sort((left, right) => left.occurrence - right.occurrence),
    diagnostics: builder.diagnostics,
  }
  const program = normalizeSemanticProgram(envelope, source)
  const canonicalSnapshot = encodeSemanticProgram(program)
  const attestation = attestSemanticProgram(program)
  const report = Object.freeze({
    astStatements: countStatements(ast),
    operations: program.core.operations.length,
    occurrences: program.core.occurrences.length,
    nodes: program.core.nodes.length,
    evaluationSteps: budget.ops,
    allocatedValueUnits: valueBudget.used,
  })
  if (terminalError !== null) Object.freeze(terminalError)
  const success: SemanticLoweringSuccess = Object.freeze({
    tag: 'success' as const,
    program,
    sourceText: source,
    get canonicalBytes() { return new Uint8Array(canonicalSnapshot) },
    attestation,
    warnings: Object.freeze([...builder.warnings]),
    fullEquivalent: !reduced.value,
    terminalError,
    report,
  })
  return success
}
