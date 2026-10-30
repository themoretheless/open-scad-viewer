import {
  ResourceNotFoundError,
  ResourceTemplate,
  SUPPORTED_PROTOCOL_VERSIONS,
  type McpServer,
} from '@modelcontextprotocol/server'

import { z } from 'zod/v4'

import { GEOMETRY_MANIFEST_ARCHIVE } from '../../../core/geometryExecution'

import { OPENSCAD_2021_01_CONTRACT } from '../../../core/openScad2021Contract'

import { EXAMPLE_CATALOG, EXAMPLES } from '../../../data/examples'

import {
  OPENSCAD_PROJECT_MAX_BLOB_BYTES,
  OPENSCAD_PROJECT_MAX_FILES,
  OPENSCAD_PROJECT_MAX_SOURCE_BYTES,
  OPENSCAD_PROJECT_MAX_TOTAL_BYTES,
} from '../../../services/openScadProject'

import {
  DEFAULT_MAX_IN_FLIGHT_REQUESTS,
  DEFAULT_MAX_PENDING_CONTROL_MESSAGES,
  DEFAULT_MAX_PENDING_OUTBOUND_MESSAGES,
  MAX_MCP_SUBSCRIPTIONS,
  MAX_MCP_STRING_REQUEST_ID_LENGTH,
} from '../../boundedTransport'

import {
  MAX_ARTIFACT_BYTES,
  MAX_PENDING_GEOMETRY_JOBS,
} from '../../geometryService'

import {
  MAX_INDEPENDENT_ARTIFACT_CACHE_BYTES,
  MAX_INDEPENDENT_ARTIFACT_CACHE_ENTRIES,
} from '../../independentArtifactCache'

import { INDEPENDENT_OPENSCAD_ENGINE_ATTESTATION } from '../../independentOpenScadExecution'

import {
  canonicalJson,
  immutableEngineManifestToWire,
  staticEngineManifestToWire,
} from '../../engineManifest'

import { MAX_MODEL_SOURCE_LENGTH } from '../../modelStore'

import {
  artifactUri,
  buildUri,
  CACHE_FIVE_MINUTES,
  CACHE_ONE_DAY,
  independentArtifactUri,
  MCP_GEOMETRY_HOST_CONTRACT,
  modelUri,
  OFFICIAL_LANGUAGE_RESOURCE_URI,
  OFFICIAL_LANGUAGE_SUMMARY,
  officialArtifactUri,
  sha256,
} from '.././contracts'

import { idSchema, promptGoalSchema } from '.././schemas'

import {
  buildToWire,
  engineRegistryToWire,
  officialCapabilitiesToWire,
  resourceId,
  resourceVariable,
} from '.././wireMappers'

import type { McpToolRegistrationDeps } from '.././registerTools'

export function register_review_model(deps: McpToolRegistrationDeps): void {

  const {
    server,
    store,
    geometry,
    officialRuntime,
    officialArtifacts,
    independentArtifacts,
  } = deps


  const promptRevisionSchema = z.string().max(16).regex(/^\d+$/)
    .refine(value => Number.isSafeInteger(Number(value)), 'Revision must be a safe integer')
    .describe('Optional immutable model revision written as a decimal integer.')


  server.registerPrompt('openscad_review_model', {
    title: 'Review a saved OpenSCAD model',
    description: 'Guide a reproducible geometry and print-readiness review without modifying the catalog.',
    argsSchema: z.object({
      model_id: idSchema,
      revision: promptRevisionSchema.optional(),
      goal: promptGoalSchema.optional(),
    }),
  }, ({ model_id, revision, goal }) => {
    const selector = revision === undefined
      ? { model_id }
      : { model_id, revision: Number(revision) }
    return {
      messages: [{
        role: 'user' as const,
        content: {
          type: 'text' as const,
          text: `Review the saved OpenSCAD model selected by ${JSON.stringify(selector)}.${goal ? ` Goal: ${goal}` : ''}
Call openscad_check with full quality. Report dimensions, volume, surface area, mesh and triangle counts, boundary/non-manifold/degenerate topology, warnings, and relevant source provenance. Distinguish proven facts from printability assumptions. Do not save or export unless I explicitly ask.`,
        },
      }],
    }
  })


  server.registerPrompt('openscad_customize_workflow', {
    title: 'Customize a saved OpenSCAD model',
    description: 'Guide safe Customizer exploration with an explicit revision check before any save.',
    argsSchema: z.object({
      model_id: idSchema,
      revision: promptRevisionSchema.optional(),
      goal: promptGoalSchema,
    }),
  }, ({ model_id, revision, goal }) => {
    const selector = revision === undefined
      ? { model_id }
      : { model_id, revision: Number(revision) }
    return {
      messages: [{
        role: 'user' as const,
        content: {
          type: 'text' as const,
          text: `Prepare a safe customization of ${JSON.stringify(selector)} for this goal: ${goal}
First call openscad_check in preview quality and inspect its declared top-level Customizer parameters. Explore only valid declared values with openscad_customize and check the returned source. Do not save automatically. If I approve a save, read the current model first and call openscad_customize_model with its revision as expected_revision so concurrent changes cannot be overwritten.`,
        },
      }],
    }
  })


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
