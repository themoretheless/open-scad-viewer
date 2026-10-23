import { createHash } from 'node:crypto'
import {
  ACTIVE_GEOMETRY_MANIFEST_VERSIONS,
  GEOMETRY_MANIFEST_ARCHIVE,
} from '../../core/geometryExecution'
import {
  OPENSCAD_2021_01_CONTRACT,
  OPENSCAD_2021_01_INDEPENDENT_ENGINE,
} from '../../core/openScad2021Contract'
import {
  DIRECT_GEOMETRY_CANCEL_GRACE_MS,
  DIRECT_GEOMETRY_JOB_DEADLINE_MS,
  DIRECT_GEOMETRY_JOIN_TIMEOUT_MS,
  DIRECT_GEOMETRY_MAX_ADMITTED_JOBS,
  DIRECT_GEOMETRY_STARTUP_TIMEOUT_MS,
} from '../directGeometrySupervisor'

export const SERVER_INSTRUCTIONS = `Use openscad_check for the frozen legacy route and openscad_independent_check for the independent OpenSCAD 2021.01 engine under qualification; neither writes build history.
Use openscad_independent_export for a full-quality STL/OBJ from that repository-owned engine. Its content-addressed artifacts are session-local and do not enter the legacy DuckDB build schema. Use openscad_analyze when a durable legacy build record is useful, and openscad_export only for a legacy-route artifact.
The openscad_official_* tools are an upstream compatibility oracle only; they are not the production engine. Use them for differential qualification and reference exports. The upstream runtime is opt-in and reports a setup command when unavailable.
Read openscad://language/openscad-2021.01 for the immutable stable built-in inventory and compatibility-tail policy; official tool results identify this language contract separately from the newer execution snapshot.
Use openscad_compare for bounded metric/topology deltas; it is not an exact geometric boolean diff.
Geometry is routed by the source's leading @language contract: legacy/current uses Manifold and openscad-viewer/brep-1 uses the Rust B-rep/NURBS engine. Callers cannot override that route. Both engine classes are permanent, and an unavailable engine returns a typed error without cross-engine fallback.
For saved models, list or get the model first and pass expected_revision when saving changes. Use openscad_customize_model for an atomic Customizer update. Pass revision to check, analyze, customize, or export an immutable historical snapshot.
Customizer operations only replace declared top-level parameters. The independent development compiler accepts bounded include/use projects plus import(), surface(), and VFS-font text() assets through openscad_independent_check files; the official oracle remains available only for differential qualification and incomplete compatibility behavior.
Preview geometry can be reduced; use full quality for authoritative measurements and exports. DuckDB is independent from the browser IndexedDB workspace, so no browser draft is modified implicitly.`

export const CACHE_FIVE_MINUTES = 5 * 60 * 1_000
export const CACHE_ONE_DAY = 24 * 60 * 60 * 1_000
export const OFFICIAL_LANGUAGE_RESOURCE_URI = 'openscad://language/openscad-2021.01' as const
export const INDEPENDENT_ENGINE_ID = OPENSCAD_2021_01_INDEPENDENT_ENGINE.engineId
export const CURRENT_MESH_MANIFEST = GEOMETRY_MANIFEST_ARCHIVE[
  ACTIVE_GEOMETRY_MANIFEST_VERSIONS.mesh
]
export const CURRENT_BREP_MANIFEST = GEOMETRY_MANIFEST_ARCHIVE[
  ACTIVE_GEOMETRY_MANIFEST_VERSIONS.brep
]
export const OFFICIAL_LANGUAGE_SUMMARY = Object.freeze({
  id: OPENSCAD_2021_01_CONTRACT.id,
  release: OPENSCAD_2021_01_CONTRACT.languageTarget.release,
  tag: OPENSCAD_2021_01_CONTRACT.languageTarget.tag,
  commit: OPENSCAD_2021_01_CONTRACT.languageTarget.commit,
  scope: 'stable-builtins-38-functions-35-modules' as const,
  function_count: OPENSCAD_2021_01_CONTRACT.builtins.functions.length,
  module_count: OPENSCAD_2021_01_CONTRACT.builtins.modules.length,
  functions: Object.freeze(OPENSCAD_2021_01_CONTRACT.builtins.functions.map(entry => entry.name)),
  modules: Object.freeze(OPENSCAD_2021_01_CONTRACT.builtins.modules.map(entry => entry.name)),
  compatibility_tail_policy: OPENSCAD_2021_01_CONTRACT.executionRuntime.compatibilityTailPolicy,
  resource_uri: OFFICIAL_LANGUAGE_RESOURCE_URI,
})

/**
 * Mutable host/runtime facts deliberately kept outside immutable engine
 * manifests. Moving an unchanged provider behind a watchdog must not rewrite
 * its capability manifest or digest.
 */
export const MCP_GEOMETRY_HOST_CONTRACT = Object.freeze({
  id: 'mcp-geometry-host-isolation-v1' as const,
  contract_version: 1 as const,
  scope: 'node-mcp-stdio' as const,
  boundary: 'worker-thread' as const,
  provider_lifecycle: 'disposable-worker-per-job' as const,
  limits: Object.freeze({
    max_admitted_jobs: DIRECT_GEOMETRY_MAX_ADMITTED_JOBS,
    max_concurrent_workers: 1 as const,
    job_deadline_ms: DIRECT_GEOMETRY_JOB_DEADLINE_MS,
    startup_timeout_ms: DIRECT_GEOMETRY_STARTUP_TIMEOUT_MS,
    cancellation_grace_ms: DIRECT_GEOMETRY_CANCEL_GRACE_MS,
    worker_join_timeout_ms: DIRECT_GEOMETRY_JOIN_TIMEOUT_MS,
  }),
  enforcement: Object.freeze({
    queue_time_in_deadline: true as const,
    hard_terminate_after_cancel_grace: true as const,
    worker_terminal_settles_only_after_join: true as const,
    next_job_only_after_worker_join: true as const,
    join_timeout_rejects_and_quarantines: true as const,
    unjoined_worker_unref_on_quarantine: true as const,
    host_environment_inherited: false as const,
    quarantine_after_join_timeout: true as const,
    worker_reuse: false as const,
  }),
  residual_risks: Object.freeze({
    subprocess_boundary: false as const,
    os_enforced_memory_limit: false as const,
    filesystem_sandbox: false as const,
    network_sandbox: false as const,
    worker_threads_share_process: true as const,
  }),
})

export function sha256(data: string | Uint8Array): string {
  return createHash('sha256').update(data).digest('hex')
}

export function truncateWellFormed(value: string, maxLength: number): string {
  if (value.length <= maxLength) return value
  let end = maxLength
  const last = value.charCodeAt(end - 1)
  if (last >= 0xD800 && last <= 0xDBFF) end--
  return value.slice(0, end)
}

export function modelUri(id: string): string {
  return `openscad://models/${encodeURIComponent(id)}`
}

export function modelRevisionUri(id: string, revision: number): string {
  return `${modelUri(id)}/revisions/${revision}`
}

export function buildUri(id: string): string {
  return `openscad://builds/${encodeURIComponent(id)}`
}

export function artifactUri(id: string): string {
  return `openscad://artifacts/${encodeURIComponent(id)}`
}

export function independentArtifactUri(id: string): string {
  return `openscad://independent-artifacts/${id}`
}

export function officialArtifactUri(id: string): string {
  return `openscad://official-artifacts/${id}`
}
