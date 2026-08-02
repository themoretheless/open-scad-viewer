import type { GeometryEvaluationResult, GeometryQuality } from '../core/build'
import {
  semanticNodeProducerOperationNames,
  semanticResultItems,
  type SemanticProgramV1,
} from '../core/semanticProgram'
import {
  assembleLegacyV5Result,
  LegacyV5AssemblyError,
  type LegacyV5AssemblyOptions,
} from './legacyV5Assembler'
import {
  loadManifoldPlanBackend,
  type ManifoldPlanBackend,
  type ManifoldPlanPayloadFamily,
} from './manifoldPlanBackend'
import { AbortedError, OpenSCADParseError } from './openscadErrors'
import {
  executeSemanticProgram,
  SemanticProgramExecutionError,
  type SemanticExecutionResult,
} from './semanticProgramExecutor'
import {
  lowerOpenSCADToSemanticProgram,
  type SemanticLoweringSuccess,
} from './semanticProgramLowerer'

const MAX_SOURCE_LENGTH = 250_000

export interface ManifoldPlanEvaluationOptions extends LegacyV5AssemblyOptions {
  readonly quality?: GeometryQuality
  readonly now?: () => number
}

function producerPosition(artifact: SemanticLoweringSuccess, nodeIndex: number): number {
  for (const occurrence of artifact.program.core.occurrences) {
    if (occurrence.node !== nodeIndex || occurrence.outputOrdinal === null) continue
    const operation = artifact.program.core.operations[occurrence.operation]
    if (!semanticNodeProducerOperationNames(artifact.program.core.nodes[nodeIndex]).includes(operation.name)) {
      continue
    }
    return artifact.program.provenance[occurrence.operation]?.span.start ?? 0
  }
  return 0
}

function backendCauseText(cause: unknown): string {
  return cause instanceof Error ? cause.message : String(cause)
}

function preserveExecutionCleanup(
  primary: unknown,
  failure: SemanticProgramExecutionError,
  message: string,
): unknown {
  return failure.cleanupError === undefined
    ? primary
    : new AggregateError([primary, failure.cleanupError], message)
}

function legacyKernelError(
  source: string,
  artifact: SemanticLoweringSuccess,
  failure: SemanticProgramExecutionError,
): unknown {
  let primary: unknown = failure
  if (failure.code !== 'E_SEMANTIC_BACKEND_FAILURE' || failure.node === null) {
    return preserveExecutionCleanup(
      primary,
      failure,
      'Semantic execution failed and Manifold session cleanup also failed',
    )
  }
  const node = artifact.program.core.nodes[failure.node]
  const position = producerPosition(artifact, failure.node)
  const detail = backendCauseText(failure.backendCause)
  if (node.kind === 'polyhedron') {
    primary = new OpenSCADParseError(source, position, `Invalid manifold polyhedron: ${detail}`)
  } else if (node.kind === 'linear-extrude') {
    primary = new OpenSCADParseError(source, position, `linear_extrude() failed: ${detail}`)
  } else if (node.kind === 'rotate-extrude-polygonal') {
    primary = new OpenSCADParseError(source, position, `rotate_extrude() failed: ${detail}`)
  }
  return preserveExecutionCleanup(
    primary,
    failure,
    'Legacy kernel evaluation failed and Manifold session cleanup also failed',
  )
}

function abortIfRequested(options: ManifoldPlanEvaluationOptions): void {
  if (options.shouldAbort?.()) throw new AbortedError()
}

function publicAssemblyError(source: string, error: LegacyV5AssemblyError): Error {
  if (error.code === 'E_LEGACY_V5_ABORTED') return new AbortedError()
  return new OpenSCADParseError(source, 0, error.message)
}

async function evaluateInternal(
  source: string,
  backend: ManifoldPlanBackend | (() => Promise<ManifoldPlanBackend>),
  options: ManifoldPlanEvaluationOptions,
): Promise<GeometryEvaluationResult> {
  const now = options.now ?? (() => performance.now())
  const startedAt = now()
  if (source.length > MAX_SOURCE_LENGTH) {
    throw new OpenSCADParseError(
      source,
      0,
      `Source exceeds ${MAX_SOURCE_LENGTH.toLocaleString()} characters`,
    )
  }
  const artifact = lowerOpenSCADToSemanticProgram(source, {
    quality: options.quality,
    shouldAbort: options.shouldAbort,
    captureTerminalFailure: true,
  })
  const parsedAt = now()
  abortIfRequested(options)
  // Match the pinned evaluator's phase order: source compile/lowering errors
  // precede a cold kernel load, while runtime terminals still begin a kernel
  // session before evaluation.
  const resolvedBackend = typeof backend === 'function' ? await backend() : backend
  const initializedAt = now()
  let execution: SemanticExecutionResult<ManifoldPlanPayloadFamily>
  try {
    execution = await executeSemanticProgram(artifact, resolvedBackend, {
      shouldAbort: options.shouldAbort,
    })
  } catch (error) {
    if (error instanceof SemanticProgramExecutionError && error.code === 'E_SEMANTIC_ABORTED') {
      throw preserveExecutionCleanup(
        new AbortedError(),
        error,
        'Semantic execution was aborted and Manifold session cleanup also failed',
      )
    }
    if (error instanceof SemanticProgramExecutionError
      && error.code === 'E_SEMANTIC_LANGUAGE_TERMINAL'
      && artifact.terminalError !== null) {
      throw preserveExecutionCleanup(
        artifact.terminalError,
        error,
        'Legacy language terminal and Manifold prefix cleanup both failed',
      )
    }
    if (error instanceof SemanticProgramExecutionError) {
      throw legacyKernelError(source, artifact, error)
    }
    throw error
  }

  let failed = false
  let primaryFailure: unknown
  try {
    // From the instant execution commits its root lease onward, every injected
    // callback and clock sample is guarded by the owning-result disposal.
    const evaluatedAt = now()
    const result = await assembleLegacyV5Result(artifact, execution, options)
    const analyzedAt = now()
    return {
      ...result,
      timings: {
        parseMs: Math.max(0, parsedAt - startedAt),
        initializeMs: Math.max(0, initializedAt - parsedAt),
        evaluateMs: Math.max(0, evaluatedAt - initializedAt),
        analyzeMs: Math.max(0, analyzedAt - evaluatedAt),
      },
    }
  } catch (error) {
    failed = true
    primaryFailure = error instanceof LegacyV5AssemblyError
      ? publicAssemblyError(source, error)
      : error
    throw primaryFailure
  } finally {
    try {
      await execution.dispose()
    } catch (disposeFailure) {
      if (failed) {
        throw new AggregateError(
          [primaryFailure, disposeFailure],
          'Manifold plan evaluation failed and result disposal also failed',
        )
      }
      throw disposeFailure
    }
  }
}

export type ManifoldPlanBackendLoader = () => Promise<ManifoldPlanBackend>

/**
 * One serialized qualification lane caches exactly one Manifold backend
 * generation at a time. An indeterminate cleanup evicts that generation; the
 * next job loads a fresh module instead of reopening or probing the quarantined
 * backend. Hard realm teardown and join belong to the outer Worker/process
 * supervisor, not to this in-process cache.
 */
export class ManifoldPlanQualificationLane {
  private backendPromise: Promise<ManifoldPlanBackend> | undefined
  private evaluationQueue: Promise<void> = Promise.resolve()
  private generation = 0

  constructor(private readonly loadBackend: ManifoldPlanBackendLoader = loadManifoldPlanBackend) {}

  /** Number of backend/module generations successfully admitted by this cache. */
  get moduleGeneration(): number {
    return this.generation
  }

  private backend(): Promise<ManifoldPlanBackend> {
    if (this.backendPromise === undefined) {
      const attempt = this.loadBackend()
      this.backendPromise = attempt
      attempt.then(() => {
        if (this.backendPromise === attempt) this.generation++
      }, () => {
        if (this.backendPromise === attempt) this.backendPromise = undefined
      })
    }
    return this.backendPromise
  }

  private retireIfQuarantined(
    backendPromise: Promise<ManifoldPlanBackend>,
    backend: ManifoldPlanBackend,
  ): void {
    if (backend.lifecycleState === 'quarantined' && this.backendPromise === backendPromise) {
      this.backendPromise = undefined
    }
  }

  private async evaluateOne(
    source: string,
    options: ManifoldPlanEvaluationOptions,
  ): Promise<GeometryEvaluationResult> {
    let acquired: {
      readonly promise: Promise<ManifoldPlanBackend>
      readonly backend: ManifoldPlanBackend
    } | undefined
    const acquireAfterLowering = async () => {
      const promise = this.backend()
      const backend = await promise
      acquired = { promise, backend }
      return backend
    }
    try {
      // Keep backend acquisition inside evaluateInternal: source admission and
      // complete lowering must fail before a cold WASM module is loaded.
      return await evaluateInternal(source, acquireAfterLowering, options)
    } finally {
      if (acquired !== undefined) {
        this.retireIfQuarantined(acquired.promise, acquired.backend)
      }
    }
  }

  evaluate(
    source: string,
    options: ManifoldPlanEvaluationOptions = {},
  ): Promise<GeometryEvaluationResult> {
    const result = this.evaluationQueue.then(
      () => this.evaluateOne(source, options),
      () => this.evaluateOne(source, options),
    )
    this.evaluationQueue = result.then(() => undefined, () => undefined)
    return result
  }
}

const defaultQualificationLane = new ManifoldPlanQualificationLane()

/**
 * Qualification-only source facade. It is intentionally not registered in the
 * protocol-v5 provider manifest; the pinned direct evaluator remains the sole
 * production path until the frozen differential corpus is fully green.
 */
export function evaluateOpenSCADViaManifoldPlanForQualification(
  source: string,
  options: ManifoldPlanEvaluationOptions = {},
): Promise<GeometryEvaluationResult> {
  return defaultQualificationLane.evaluate(source, options)
}

/** Test seam for lifecycle fakes without publishing a geometry provider. */
export function evaluateOpenSCADViaInjectedManifoldPlan(
  source: string,
  backend: ManifoldPlanBackend,
  options: ManifoldPlanEvaluationOptions = {},
): Promise<GeometryEvaluationResult> {
  return evaluateInternal(source, backend, options)
}

export function semanticOutputCount(program: SemanticProgramV1): number {
  return semanticResultItems(program.core.result).length
}
