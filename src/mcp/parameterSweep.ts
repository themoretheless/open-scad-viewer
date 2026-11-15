import type { GeometryQuality } from '../core/build'
import type { GeometryExecutionDescriptor } from '../core/geometryExecution'
import { sha256Hex } from '../core/sha256'
import type { CustomizerValue } from '../language/scadCustomizer'
import type { McpGeometryService } from './geometryService'
import { publicToolError } from './publicError'
import { isAbortError } from './publicError/isAbortError'

export const MAX_SWEEP_AXES = 4
export const MAX_SWEEP_AXIS_VALUES = 16
export const DEFAULT_SWEEP_MAX_COMBINATIONS = 16
export const MAX_SWEEP_COMBINATIONS = 32
export const DEFAULT_SWEEP_MAX_TOTAL_TRIANGLES = 250_000
export const MAX_SWEEP_TOTAL_TRIANGLES = 750_000
export const DEFAULT_SWEEP_TIMEOUT_MS = 10_000
export const MAX_SWEEP_TIMEOUT_MS = 30_000

export const SWEEP_RANK_OPTIONS = [
  'volume_asc',
  'volume_desc',
  'surface_area_asc',
  'surface_area_desc',
  'triangles_asc',
  'triangles_desc',
] as const

export type SweepRankBy = (typeof SWEEP_RANK_OPTIONS)[number]

export interface ParameterSweepAxisInput {
  readonly name: string
  readonly values: readonly CustomizerValue[]
}

export interface ParameterSweepOptions {
  readonly source: string
  readonly axes: readonly ParameterSweepAxisInput[]
  readonly fixedValues?: Readonly<Record<string, CustomizerValue>> | undefined
  readonly quality?: GeometryQuality | undefined
  readonly maxCombinations?: number | undefined
  readonly maxTotalTriangles?: number | undefined
  readonly timeoutMs?: number | undefined
  readonly rankBy?: SweepRankBy | undefined
  readonly signal?: AbortSignal | undefined
}

export interface ParameterSweepVariantResult {
  readonly index: number
  readonly rank: number | null
  readonly parameters: Record<string, CustomizerValue>
  readonly source_sha256: string
  readonly status: 'succeeded' | 'failed'
  readonly duration_ms?: number | undefined
  readonly metrics?:
    | {
        readonly mesh_count: number
        readonly triangle_count: number
        readonly vertex_count: number
        readonly volume: number
        readonly surface_area: number
        readonly bounds: { readonly min: [number, number, number]; readonly max: [number, number, number] } | null
      }
    | undefined
  readonly error?: { readonly code: string; readonly message: string } | undefined
}

export interface ParameterSweepResult {
  readonly quality: GeometryQuality
  readonly execution: GeometryExecutionDescriptor
  readonly rank_by: SweepRankBy
  readonly combination_count: number
  readonly evaluated_count: number
  readonly succeeded_count: number
  readonly failed_count: number
  readonly total_triangles: number
  readonly truncated_by_budget: boolean
  readonly stop_reason: 'completed' | 'triangle_budget_exceeded' | 'timeout'
  readonly ranking_order: number[]
  readonly variants: ParameterSweepVariantResult[]
}

export class SweepBudgetError extends RangeError {
  constructor(message: string) {
    super(message)
    this.name = 'SweepBudgetError'
  }
}

function boundedSweepInt(value: number, label: string, min: number, max: number): number {
  if (!Number.isSafeInteger(value) || value < min || value > max) {
    throw new SweepBudgetError(`${label} must be an integer between ${min} and ${max}`)
  }
  return value
}

function validateAndExpandCombinations(
  axes: readonly ParameterSweepAxisInput[],
  fixedValues: Readonly<Record<string, CustomizerValue>>,
  maxCombinations: number,
): Array<Record<string, CustomizerValue>> {
  if (axes.length < 1 || axes.length > MAX_SWEEP_AXES) {
    throw new SweepBudgetError(`Parameter sweep requires 1-${MAX_SWEEP_AXES} axes`)
  }
  const seenNames = new Set<string>()
  let totalCount = 1
  for (const axis of axes) {
    if (!axis.name.trim()) throw new SweepBudgetError('Sweep axis name must not be empty')
    if (seenNames.has(axis.name)) throw new SweepBudgetError(`Duplicate sweep axis ${axis.name}`)
    if (Object.hasOwn(fixedValues, axis.name)) {
      throw new SweepBudgetError(`Sweep axis ${axis.name} cannot also appear in fixed_values`)
    }
    seenNames.add(axis.name)
    if (axis.values.length < 1 || axis.values.length > MAX_SWEEP_AXIS_VALUES) {
      throw new SweepBudgetError(`Axis ${axis.name} must contain 1-${MAX_SWEEP_AXIS_VALUES} values`)
    }
    totalCount *= axis.values.length
    if (totalCount > maxCombinations) {
      throw new SweepBudgetError(`Sweep combinations (${totalCount}+) exceed max_combinations=${maxCombinations}`)
    }
  }
  let combos: Array<Record<string, CustomizerValue>> = [{ ...fixedValues }]
  for (const axis of axes) {
    const next: Array<Record<string, CustomizerValue>> = []
    for (const prefix of combos) {
      for (const val of axis.values) {
        next.push({ ...prefix, [axis.name]: val })
      }
    }
    combos = next
  }
  return combos
}

function metricValue(variant: ParameterSweepVariantResult, rankBy: SweepRankBy): number {
  const m = variant.metrics!
  if (rankBy === 'volume_asc' || rankBy === 'volume_desc') return m.volume
  if (rankBy === 'surface_area_asc' || rankBy === 'surface_area_desc') return m.surface_area
  return m.triangle_count
}

function rankVariants(variants: ParameterSweepVariantResult[], rankBy: SweepRankBy): number[] {
  const descending = rankBy.endsWith('_desc')
  const succeeded = variants.filter(v => v.status === 'succeeded' && v.metrics !== undefined)
  succeeded.sort((left, right) => {
    const diff = metricValue(left, rankBy) - metricValue(right, rankBy)
    if (diff !== 0) return descending ? -diff : diff
    return left.index - right.index
  })
  const rankMap = new Map<number, number>()
  succeeded.forEach((variant, rankIdx) => rankMap.set(variant.index, rankIdx + 1))
  for (let i = 0; i < variants.length; i++) {
    const rank = rankMap.get(variants[i].index) ?? null
    if (rank !== null) variants[i] = { ...variants[i], rank }
  }
  return succeeded.map(v => v.index)
}

function isFatalSweepError(error: unknown, publicCode: string): boolean {
  return (
    publicCode === 'engine_unavailable' ||
    publicCode === 'language_contract_unsupported' ||
    publicCode === 'server_busy' ||
    publicCode === 'cancelled' ||
    isAbortError(error)
  )
}

export async function runOpenScadParameterSweep(
  geometry: McpGeometryService,
  options: ParameterSweepOptions,
): Promise<ParameterSweepResult> {
  if (options.signal?.aborted) {
    throw new DOMException('Parameter sweep cancelled before admission', 'AbortError')
  }
  const quality = options.quality ?? 'preview'
  const rankBy = options.rankBy ?? 'volume_asc'
  const maxCombinations = boundedSweepInt(
    options.maxCombinations ?? DEFAULT_SWEEP_MAX_COMBINATIONS,
    'max_combinations',
    1,
    MAX_SWEEP_COMBINATIONS,
  )
  const maxTotalTriangles = boundedSweepInt(
    options.maxTotalTriangles ?? DEFAULT_SWEEP_MAX_TOTAL_TRIANGLES,
    'max_total_triangles',
    1,
    MAX_SWEEP_TOTAL_TRIANGLES,
  )
  const timeoutMs = boundedSweepInt(
    options.timeoutMs ?? DEFAULT_SWEEP_TIMEOUT_MS,
    'timeout_ms',
    1,
    MAX_SWEEP_TIMEOUT_MS,
  )
  const fixedValues = options.fixedValues ?? {}

  const baseExecution = geometry.planSource(options.source, quality, 'analysis')
  const combos = validateAndExpandCombinations(options.axes, fixedValues, maxCombinations)
  const customized = combos.map(params => ({
    params,
    result: geometry.customize(options.source, params),
  }))

  const sweepController = new AbortController()
  let timedOut = false
  const timer = setTimeout(() => {
    timedOut = true
    sweepController.abort()
  }, timeoutMs)
  const onCallerAbort = () => sweepController.abort()
  options.signal?.addEventListener('abort', onCallerAbort, { once: true })

  const variants: ParameterSweepVariantResult[] = []
  let totalTriangles = 0
  let succeededCount = 0
  let failedCount = 0
  let stopReason: ParameterSweepResult['stop_reason'] = 'completed'

  try {
    for (const [index, entry] of customized.entries()) {
      if (options.signal?.aborted) throw new DOMException('Parameter sweep cancelled', 'AbortError')
      if (timedOut || sweepController.signal.aborted) {
        stopReason = 'timeout'
        break
      }
      const sourceSha = sha256Hex(entry.result.source)
      try {
        const compiled = await geometry.compile(entry.result.source, quality, sweepController.signal, 'analysis')
        const a = compiled.analysis
        totalTriangles += a.triangleCount
        succeededCount += 1
        variants.push({
          index,
          rank: null,
          parameters: entry.params,
          source_sha256: sourceSha,
          status: 'succeeded',
          duration_ms: a.durationMs,
          metrics: {
            mesh_count: a.meshCount,
            triangle_count: a.triangleCount,
            vertex_count: a.vertexCount,
            volume: a.volume,
            surface_area: a.surfaceArea,
            bounds: a.bounds,
          },
        })
        if (totalTriangles > maxTotalTriangles) {
          stopReason = 'triangle_budget_exceeded'
          break
        }
      } catch (error) {
        if (options.signal?.aborted) throw new DOMException('Parameter sweep cancelled', 'AbortError')
        if (timedOut) {
          stopReason = 'timeout'
          break
        }
        const mapped = publicToolError(error).error
        if (isFatalSweepError(error, mapped.code)) throw error
        failedCount += 1
        variants.push({
          index,
          rank: null,
          parameters: entry.params,
          source_sha256: sourceSha,
          status: 'failed',
          error: { code: mapped.code, message: mapped.message },
        })
      }
    }
  } finally {
    clearTimeout(timer)
    options.signal?.removeEventListener('abort', onCallerAbort)
  }

  const rankingOrder = rankVariants(variants, rankBy)
  return {
    quality,
    execution: baseExecution,
    rank_by: rankBy,
    combination_count: customized.length,
    evaluated_count: variants.length,
    succeeded_count: succeededCount,
    failed_count: failedCount,
    total_triangles: totalTriangles,
    truncated_by_budget: stopReason !== 'completed',
    stop_reason: stopReason,
    ranking_order: rankingOrder,
    variants,
  }
}
