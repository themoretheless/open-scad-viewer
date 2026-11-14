import { MAX_ARTIFACT_BYTES, MAX_PENDING_GEOMETRY_JOBS } from '../../geometryService'
import { CACHE_FIVE_MINUTES, CACHE_ONE_DAY, MCP_GEOMETRY_HOST_CONTRACT, OFFICIAL_LANGUAGE_RESOURCE_URI, OFFICIAL_LANGUAGE_SUMMARY } from '../../createServer'
import { DEFAULT_MAX_IN_FLIGHT_REQUESTS, DEFAULT_MAX_PENDING_CONTROL_MESSAGES, DEFAULT_MAX_PENDING_OUTBOUND_MESSAGES, MAX_MCP_STRING_REQUEST_ID_LENGTH, MAX_MCP_SUBSCRIPTIONS } from '../../boundedTransport'
import { GEOMETRY_MANIFEST_ARCHIVE } from '../../../core/geometryExecution'
import { INDEPENDENT_OPENSCAD_ENGINE_ATTESTATION } from '../../independentOpenScadExecution'
import { MAX_INDEPENDENT_ARTIFACT_CACHE_BYTES, MAX_INDEPENDENT_ARTIFACT_CACHE_ENTRIES } from '../../independentArtifactCache'
import { MAX_MODEL_SOURCE_LENGTH } from '../../modelStore'
import { ResourceNotFoundError, ResourceTemplate, SUPPORTED_PROTOCOL_VERSIONS } from '@modelcontextprotocol/server'
import { OPENSCAD_2021_01_CONTRACT } from '../../../core/openScad2021Contract'
import { OPENSCAD_PROJECT_MAX_BLOB_BYTES, OPENSCAD_PROJECT_MAX_FILES, OPENSCAD_PROJECT_MAX_SOURCE_BYTES, OPENSCAD_PROJECT_MAX_TOTAL_BYTES } from '../../../language/openScadProject'
import { engineRegistryToWire, officialCapabilitiesToWire, resourceVariable, sha256 } from '../officialToolInputSchema'
import { canonicalJson, immutableEngineManifestToWire, staticEngineManifestToWire } from '../../engineManifest'
import type { McpServerContext } from './context'

/** Fixed resources: capabilities, runtime status, language contract, engine manifests and parity. */
export function registerStaticResources(ctx: McpServerContext): void {
  registerCapabilities(ctx)
  registerGeometryEngineManifest(ctx)
  registerGeometryParity(ctx)
}

function registerCapabilities(ctx: McpServerContext): void {
  const { geometry, officialRuntime, server } = ctx
  server.registerResource('capabilities', 'openscad://capabilities', {
    title: 'OpenSCAD MCP capabilities',
    description: 'Machine-readable stable OpenSCAD inventory, official runtime, independent engine registry, source routes, safety limits, protocol support, and persistence boundaries.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
  }, async uri => {
    const [geometryCapabilities, officialCapabilities] = await Promise.all([
      geometry.capabilities(),
      officialRuntime.capabilities(),
    ])
    const geometryEngines = engineRegistryToWire(geometryCapabilities)
    return {
      contents: [{
        uri: uri.href,
        mimeType: 'application/json',
        text: JSON.stringify({
        server: { name: 'open-scad-viewer', version: '0.1.0' },
        protocol_versions: ['2026-07-28', ...SUPPORTED_PROTOCOL_VERSIONS],
        geometry_engines: geometryEngines,
        geometry_host: MCP_GEOMETRY_HOST_CONTRACT,
        parity_resource_uri: 'openscad://parity',
        official_language_contract: OFFICIAL_LANGUAGE_SUMMARY,
        official_runtime_resource_uri: 'openscad://official-runtime',
        official_runtime: officialCapabilitiesToWire(officialCapabilities),
        independent_runtime: {
          engine: INDEPENDENT_OPENSCAD_ENGINE_ATTESTATION,
          tools: {
            check: 'openscad_independent_check',
            export: 'openscad_independent_export',
          },
          export_formats: ['stl', 'obj'],
          artifact_resource_template: 'openscad://independent-artifacts/{sha256}',
          defaults: { quality: 'full', time: 0 },
          limits: {
            source_bytes: OPENSCAD_PROJECT_MAX_SOURCE_BYTES,
            project_files: OPENSCAD_PROJECT_MAX_FILES,
            project_file_bytes: Math.max(
              OPENSCAD_PROJECT_MAX_SOURCE_BYTES,
              OPENSCAD_PROJECT_MAX_BLOB_BYTES,
            ),
            project_bytes: OPENSCAD_PROJECT_MAX_TOTAL_BYTES,
            artifact_bytes: MAX_ARTIFACT_BYTES,
            artifact_cache_entries: MAX_INDEPENDENT_ARTIFACT_CACHE_ENTRIES,
            artifact_cache_bytes: MAX_INDEPENDENT_ARTIFACT_CACHE_BYTES,
          },
          persistence: 'session-local-content-addressed',
          legacy_duckdb_build_schema_used: false,
        },
        compiler: {
          language_scope: 'strict OpenSCAD subset',
          supported: [
            'cube/sphere/cylinder/polyhedron',
            'square/circle/polygon',
            'union/difference/intersection/hull',
            'transforms/color',
            'modules/children/for/if/let',
            'linear_extrude/rotate_extrude/projection/offset',
            'top-level Customizer parameters',
          ],
          unsupported: [
            'include/use/import',
            'text/surface/Minkowski',
            'user-defined functions',
          ],
        },
        limits: {
          source_characters: MAX_MODEL_SOURCE_LENGTH,
          artifact_bytes: MAX_ARTIFACT_BYTES,
          pending_geometry_jobs: MAX_PENDING_GEOMETRY_JOBS,
          stdio_in_flight_requests: DEFAULT_MAX_IN_FLIGHT_REQUESTS,
          stdio_pending_output: DEFAULT_MAX_PENDING_OUTBOUND_MESSAGES,
          stdio_pending_overload_replies: DEFAULT_MAX_PENDING_CONTROL_MESSAGES,
          stdio_subscriptions: MAX_MCP_SUBSCRIPTIONS,
          stdio_string_request_id_characters: MAX_MCP_STRING_REQUEST_ID_LENGTH,
        },
        persistence: {
          catalog: 'DuckDB',
          arbitrary_sql: false,
          browser_indexeddb_synchronized: false,
        },
        }, null, 2),
      }],
    }
  })
}

function registerGeometryEngineManifest(ctx: McpServerContext): void {
  const { officialRuntime, server } = ctx
  server.registerResource('official-runtime', 'openscad://official-runtime', {
    title: 'Official OpenSCAD runtime',
    description: 'Live availability, pinned integrity identity, isolation contract, supported exports, experiments, limits, and setup command for the opt-in upstream runtime.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
  }, async uri => ({
    contents: [{
      uri: uri.href,
      mimeType: 'application/json',
      text: JSON.stringify({
        language_contract: OFFICIAL_LANGUAGE_SUMMARY,
        official_runtime: officialCapabilitiesToWire(await officialRuntime.capabilities()),
      }, null, 2),
    }],
  }))

  server.registerResource('official-language-contract', OFFICIAL_LANGUAGE_RESOURCE_URI, {
    title: 'OpenSCAD 2021.01 stable language contract',
    description: 'Immutable source-derived inventory of stable built-ins, syntax, operators, variables, file semantics, compatibility tail, smoke fixtures, and the separately pinned execution runtime.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'public' },
  }, async uri => ({
    contents: [{
      uri: uri.href,
      mimeType: 'application/json',
      text: JSON.stringify({ contract: OPENSCAD_2021_01_CONTRACT }, null, 2),
    }],
  }))

  const engineManifestTemplate = new ResourceTemplate(
    'openscad://engines/{engine_class}/capabilities/{manifest_version}',
    {
      list: async () => ({
        resources: Object.values(GEOMETRY_MANIFEST_ARCHIVE).map(staticEngineManifestToWire).map(engine => ({
          uri: engine.manifest_resource_uri,
          name: `${engine.display_name} capability manifest`,
          description: `${engine.maturity} immutable manifest; permanent engine class`,
          mimeType: 'application/json',
        })),
      }),
    },
  )
  server.registerResource('geometry-engine-manifest', engineManifestTemplate, {
    title: 'Immutable geometry engine capability manifest',
    description: 'Versioned capabilities, deployment maturity, fingerprint, and isolation facts for one permanent engine class.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'public' },
  }, async (uri, variables) => {
    const engineClass = resourceVariable(uri, variables.engine_class)
    const manifestVersion = resourceVariable(uri, variables.manifest_version)
    if (engineClass !== 'mesh' && engineClass !== 'brep') {
      throw new ResourceNotFoundError(uri.href)
    }
    const manifest = immutableEngineManifestToWire(
      engineClass,
      manifestVersion,
    )
    if (!manifest) throw new ResourceNotFoundError(uri.href)
    return {
      contents: [{
        uri: uri.href,
        mimeType: 'application/json',
        text: JSON.stringify(manifest, null, 2),
      }],
    }
  })
}

function registerGeometryParity(ctx: McpServerContext): void {
  const { geometry, server } = ctx
  server.registerResource('geometry-parity', 'openscad://parity', {
    title: 'Geometry engine browser-to-MCP parity status',
    description: 'Honest qualification state by engine; cross-engine metric comparison is never proof of geometric equivalence.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_FIVE_MINUTES, cacheScope: 'private' },
  }, async uri => {
    const registry = await geometry.capabilities()
    const policyPayload = {
      source_directed_routing: registry.sourceDirectedRouting,
      automatic_fallback: registry.automaticFallback,
      routes: registry.routes.map(route => ({
        language_contract: route.languageContract,
        engine_class: route.engineClass,
        fallback: route.fallback,
      })),
      manifests: registry.engines.map(engine => ({
        engine_class: engine.engineClass,
        capability_manifest_version: engine.capabilityManifestVersion,
        manifest_digest: engine.manifestDigest,
        kernel_fingerprint: engine.kernelFingerprint,
        limits: engine.limits,
      })),
      comparison: {
        mode: 'metrics-and-topology-only',
        cross_engine_geometric_equivalence: false,
        numeric_tolerances: null,
        comparator_qualification: 'pending',
      },
    }
    return {
      contents: [{
        uri: uri.href,
        mimeType: 'application/json',
        text: JSON.stringify({
        contract_version: 1,
        definition: 'Same source revision, language contract, engine class and fingerprint, policy, engine limits, and declared tolerances across browser and MCP. Mutable MCP host isolation is reported separately from immutable engine identity.',
        cross_engine_geometric_equivalence: false,
        mcp_host: MCP_GEOMETRY_HOST_CONTRACT,
        policy: {
          id: 'engine-routing-contract-v1',
          digest_algorithm: 'sha256',
          canonicalization: 'recursive-codepoint-key-sort-v1',
          payload: policyPayload,
          sha256: sha256(canonicalJson(policyPayload)),
        },
        corpus: {
          id: 'browser-mcp-engine-parity',
          version: null,
          sha256: null,
          result_sha256: null,
          qualified_at: null,
        },
        engines: registry.engines.map(engine => ({
          engine_class: engine.engineClass,
          capability_manifest_version: engine.capabilityManifestVersion,
          manifest_digest: engine.manifestDigest,
          kernel_fingerprint: engine.kernelFingerprint,
          runtime_availability: engine.availability,
          browser_target: engine.deployment === 'not-deployed' ? 'not-deployed' : 'wasm-worker',
          mcp_target: engine.deployment === 'not-deployed'
            ? 'not-deployed'
            : 'node-wasm-disposable-worker',
          browser_mcp_status: engine.deployment === 'not-deployed'
            ? 'not-deployed'
            : 'qualification-pending', // Provider qualification is not browser/MCP parity evidence.
          mcp_isolation: engine.isolation,
          mcp_provider_manifest_isolation: engine.isolation,
          mcp_host_isolation: engine.deployment === 'not-deployed'
            ? 'not-deployed'
            : MCP_GEOMETRY_HOST_CONTRACT.provider_lifecycle,
          mcp_host_contract_id: engine.deployment === 'not-deployed'
            ? null
            : MCP_GEOMETRY_HOST_CONTRACT.id,
          qualification: {
            status: engine.qualification.status,
            record_id: engine.qualification.recordId,
            corpus_version: engine.qualification.corpusVersion,
            target: engine.qualification.target,
          },
          result_sha256: null,
        })),
        }, null, 2),
      }],
    }
  })
}
