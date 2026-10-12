import type { GeometryAnalysis, McpGeometryService } from '../geometryService'
import type { GeometryBuildPurpose, GeometryExecutionDescriptor } from '../../core/geometryExecution'
import { GeometryCapabilityUnavailableError, GeometryEngineUnavailableError, GeometryLanguageContractError, geometryExecutionForError } from '../../services/geometryBuildEngine'
import type { IndependentArtifactFormat } from '../independentArtifactCache'
import { ModelNotFoundError, buildDiagnostic, isAbortError, publicToolError } from '../publicError'
import type { ModelStore, StoredBuild } from '../modelStore'
import { OfficialOpenScadRemoteError } from '../officialOpenScadRuntimeService'
import type { SelectedModel } from '../createServer'
import { buildObj } from '../../services/meshExport'
import { buildUri, executionToWire, sha256 } from './officialToolInputSchema'
import { truncateWellFormed } from '../../core/unicode'

export function buildToWire(build: StoredBuild) {
  return {
    id: build.id,
    model_id: build.modelId,
    model_revision: build.modelRevision,
    source_sha256: build.sourceSha256,
    source_attestation: build.sourceAttestation,
    execution: executionToWire(build.execution),
    quality: build.quality,
    status: build.status,
    duration_ms: build.durationMs,
    warnings: build.warnings,
    metrics: build.metrics ? {
      mesh_count: build.metrics.meshCount,
      triangle_count: build.metrics.triangleCount,
      volume: build.metrics.volume,
      surface_area: build.metrics.surfaceArea,
      reduced: build.metrics.reduced,
    } : null,
    error: build.error,
    created_at: build.createdAt,
    resource_uri: buildUri(build.id),
  }
}

export function textResult<T extends object>(structuredContent: T) {
  const serialized = JSON.stringify(structuredContent)
  return {
    content: [{ type: 'text' as const, text: serialized }],
    structuredContent: structuredContent as Record<string, unknown>,
  }
}

export function executionForFailure(error: unknown): GeometryExecutionDescriptor | undefined {
  return geometryExecutionForError(error)
    ?? (error instanceof GeometryEngineUnavailableError
    || error instanceof GeometryCapabilityUnavailableError
    ? error.execution
    : undefined)
}

export function errorResult(error: unknown, extra: Record<string, unknown> = {}) {
  const problem = publicToolError(error)
  if (problem.internal) {
    console.error(`MCP tool failure (${problem.error.correlation_id}):`, error)
  }
  const execution = executionForFailure(error)
  const structured = {
    ...extra,
    ...(execution && extra.execution === undefined
      ? { execution: executionToWire(execution) }
      : {}),
    error: problem.error,
  }
  return {
    content: [{ type: 'text' as const, text: JSON.stringify(structured, null, 2) }],
    structuredContent: structured,
    isError: true as const,
  }
}

export function officialErrorResult(error: unknown) {
  const exposesCompilerDiagnostics = error instanceof OfficialOpenScadRemoteError
    && (error.code === 'E_OPENSCAD_COMPILE'
      || error.code === 'E_OPENSCAD_NO_OUTPUT'
      || error.code === 'E_OPENSCAD_OUTPUT_LIMIT')
  return exposesCompilerDiagnostics
    ? errorResult(error, {
        official_logs: error.logs,
        duration_ms: error.durationMs,
      })
    : errorResult(error)
}

export async function resolveSource(
  store: ModelStore,
  input: { source?: string; model_id?: string; revision?: number },
): Promise<{ source: string; model: SelectedModel | null }> {
  if (input.source !== undefined) return { source: input.source, model: null }
  const model = input.revision === undefined
    ? await store.getModel(input.model_id!)
    : await store.getModelRevision(input.model_id!, input.revision)
  if (!model) throw new ModelNotFoundError(input.model_id!, input.revision)
  return { source: model.source, model }
}

export function analysisDimensions(analysis: GeometryAnalysis): [number, number, number] {
  if (!analysis.bounds) return [0, 0, 0]
  return analysis.bounds.max.map((value, index) => (
    value - analysis.bounds!.min[index]
  )) as [number, number, number]
}

export function comparisonSide(
  selected: Awaited<ReturnType<typeof resolveSource>>,
  analysis: GeometryAnalysis,
) {
  return {
    source_sha256: sha256(selected.source),
    model_id: selected.model?.id ?? null,
    model_revision: selected.model?.revision ?? null,
    execution: executionToWire(analysis.execution),
    mesh_count: analysis.meshCount,
    triangle_count: analysis.triangleCount,
    volume: analysis.volume,
    surface_area: analysis.surfaceArea,
    bounds: analysis.bounds,
    dimensions: analysisDimensions(analysis),
    topology: {
      boundary: analysis.topology.boundary,
      crease: analysis.topology.crease,
      non_manifold: analysis.topology.nonManifold,
      degenerate: analysis.topology.degenerate,
    },
    warnings: analysis.warnings,
  }
}

export function comparisonDelta(
  left: ReturnType<typeof comparisonSide>,
  right: ReturnType<typeof comparisonSide>,
) {
  return {
    mesh_count: right.mesh_count - left.mesh_count,
    triangle_count: right.triangle_count - left.triangle_count,
    volume: right.volume - left.volume,
    surface_area: right.surface_area - left.surface_area,
    dimensions: right.dimensions.map((value, index) => (
      value - left.dimensions[index]
    )) as [number, number, number],
    topology: {
      boundary: right.topology.boundary - left.topology.boundary,
      crease: right.topology.crease - left.topology.crease,
      non_manifold: right.topology.non_manifold - left.topology.non_manifold,
      degenerate: right.topology.degenerate - left.topology.degenerate,
    },
  }
}

export function sameExecutionContext(
  left: GeometryExecutionDescriptor,
  right: GeometryExecutionDescriptor,
): boolean {
  const stableLimits = (limits: Record<string, number>) => JSON.stringify(
    Object.entries(limits).sort(([leftKey], [rightKey]) => leftKey.localeCompare(rightKey)),
  )
  return left.languageContract === right.languageContract
    && left.engineClass === right.engineClass
    && left.engineKey === right.engineKey
    && left.kernelFingerprint === right.kernelFingerprint
    && left.semanticProgramVersion === right.semanticProgramVersion
    && left.capabilityManifestVersion === right.capabilityManifestVersion
    && left.manifestDigest === right.manifestDigest
    && left.requiredCapabilities.join('\0') === right.requiredCapabilities.join('\0')
    && left.purpose === right.purpose
    && left.quality === right.quality
    && left.representation === right.representation
    && left.automaticFallback === right.automaticFallback
    && stableLimits(left.effectiveLimits) === stableLimits(right.effectiveLimits)
}

export function safeFileName(name: string, format: 'stl' | 'obj'): string {
  const extension = `.${format}`
  const trimmed = name.trim().replace(/[/\\\0]/g, '_') || `model${extension}`
  const base = trimmed.toLowerCase().endsWith(extension) ? trimmed.slice(0, -extension.length) : trimmed
  return `${truncateWellFormed(base, 255 - extension.length)}${extension}`
}

export function estimatedIndependentExportBytes(
  meshes: Parameters<typeof buildObj>[0],
  format: IndependentArtifactFormat,
): number {
  const triangles = meshes.reduce(
    (total, mesh) => total + Math.floor(mesh.indices.length / 3),
    0,
  )
  if (format === 'stl') return 84 + triangles * 50
  const vertices = meshes.reduce(
    (total, mesh) => total + Math.floor(mesh.vertices.length / 6),
    0,
  )
  return 64 + meshes.length * 64 + vertices * 100 + triangles * 50
}

export function throwIfIndependentExportAborted(signal: AbortSignal | undefined): void {
  if (signal?.aborted) throw new DOMException('OpenSCAD export was cancelled', 'AbortError')
}

export async function recordFailure(
  store: ModelStore,
  geometry: McpGeometryService,
  input: {
    model: Pick<SelectedModel, 'id' | 'revision'> | null
    source: string
    quality: 'preview' | 'full'
    purpose: GeometryBuildPurpose
    durationMs: number
    error: unknown
    execution?: GeometryExecutionDescriptor
  },
): Promise<StoredBuild | null> {
  try {
    if (input.error instanceof GeometryLanguageContractError) return null
    const execution = input.execution
      ?? executionForFailure(input.error)
      ?? geometry.planSource(input.source, input.quality, input.purpose)
    return await store.recordBuild({
      modelId: input.model?.id,
      modelRevision: input.model?.revision,
      source: input.source,
      sourceSha256: sha256(input.source),
      execution,
      quality: input.quality,
      durationMs: input.durationMs,
      warnings: [],
      status: persistedFailureStatus(input.error),
      error: buildDiagnostic(input.error),
    })
  } catch (storeError) {
    console.error('Could not record failed MCP build:', storeError)
    return null
  }
}

/** Persisted causality depends on the settled error, never a later signal state. */
export function persistedFailureStatus(error: unknown): 'cancelled' | 'failed' {
  return isAbortError(error) ? 'cancelled' : 'failed'
}
