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

export function register_geometry_parity(deps: McpToolRegistrationDeps): void {

  const {
    server,
    store,
    geometry,
    officialRuntime,
    officialArtifacts,
    independentArtifacts,
  } = deps


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
              : 'qualification-pending',
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


  const modelTemplate = new ResourceTemplate('openscad://models/{id}', {
    list: async () => ({
      resources: (await store.listModels(500)).map(model => ({
        uri: modelUri(model.id),
        name: model.name,
        description: `OpenSCAD source, revision ${model.revision}`,
        mimeType: 'text/x-openscad',
      })),
    }),
    complete: { id: async value => (await store.listModels(500)).map(model => model.id).filter(id => id.startsWith(value)) },
  })

  server.registerResource('saved-model', modelTemplate, {
    title: 'Saved OpenSCAD model',
    description: 'Versioned OpenSCAD source stored in DuckDB.',
    mimeType: 'text/x-openscad',
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const model = await store.getModel(id)
    if (!model) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'text/x-openscad', text: model.source }] }
  })


  const modelRevisionTemplate = new ResourceTemplate('openscad://models/{id}/revisions/{revision}', {
    list: undefined,
  })

  server.registerResource('saved-model-revision', modelRevisionTemplate, {
    title: 'Saved OpenSCAD model revision',
    description: 'An immutable OpenSCAD source snapshot stored in DuckDB.',
    mimeType: 'text/x-openscad',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'private' },
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const revisionValue = resourceVariable(uri, variables.revision)
    if (!/^\d+$/.test(revisionValue)) throw new ResourceNotFoundError(uri.href)
    const revision = Number(revisionValue)
    if (!Number.isSafeInteger(revision)) throw new ResourceNotFoundError(uri.href)
    const model = await store.getModelRevision(id, revision)
    if (!model) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'text/x-openscad', text: model.source }] }
  })


  const buildTemplate = new ResourceTemplate('openscad://builds/{id}', {
    list: async () => ({
      resources: (await store.listBuilds({ limit: 500 })).map(build => ({
        uri: buildUri(build.id),
        name: `Build ${build.id}`,
        description: `${build.status} ${build.quality} build`,
        mimeType: 'application/json',
      })),
    }),
  })

  server.registerResource('build-result', buildTemplate, {
    title: 'OpenSCAD build result',
    description: 'A compact build result recorded in DuckDB.',
    mimeType: 'application/json',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'private' },
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const build = await store.getBuild(id)
    if (!build) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'application/json', text: JSON.stringify(buildToWire(build), null, 2) }] }
  })


  const artifactTemplate = new ResourceTemplate('openscad://artifacts/{id}', {
    list: async () => ({
      resources: (await store.listArtifacts(100)).map(artifact => ({
        uri: artifactUri(artifact.id),
        name: artifact.fileName,
        description: `${artifact.format.toUpperCase()} export (${artifact.byteLength.toLocaleString()} bytes)`,
        mimeType: artifact.mimeType,
      })),
    }),
  })

  server.registerResource('export-artifact', artifactTemplate, {
    title: 'OpenSCAD export artifact',
    description: 'A bounded STL or OBJ artifact stored in DuckDB.',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'private' },
  }, async (uri, variables) => {
    const id = resourceId(uri, variables.id)
    const artifact = await store.getArtifact(id)
    if (!artifact) throw new ResourceNotFoundError(uri.href)
    return artifact.format === 'obj'
      ? { contents: [{ uri: uri.href, mimeType: artifact.mimeType, text: new TextDecoder().decode(artifact.data) }] }
      : { contents: [{ uri: uri.href, mimeType: artifact.mimeType, blob: Buffer.from(artifact.data.buffer, artifact.data.byteOffset, artifact.data.byteLength).toString('base64') }] }
  })



}
