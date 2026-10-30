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

export function register_independent_export_artifact(deps: McpToolRegistrationDeps): void {

  const {
    server,
    store,
    geometry,
    officialRuntime,
    officialArtifacts,
    independentArtifacts,
  } = deps

  const independentArtifactTemplate = new ResourceTemplate(
    'openscad://independent-artifacts/{sha256}',
    {
      list: async () => ({
        resources: independentArtifacts.list().map(artifact => ({
          uri: independentArtifactUri(artifact.id),
          name: artifact.fileName,
          description: `Independent OpenSCAD ${artifact.format.toUpperCase()} export (${artifact.byteLength.toLocaleString()} bytes); session-local`,
          mimeType: artifact.mimeType,
        })),
      }),
      complete: {
        sha256: value => independentArtifacts.list().map(artifact => artifact.id)
          .filter(id => id.startsWith(value)),
      },
    },
  )
  server.registerResource('independent-export-artifact', independentArtifactTemplate, {
    title: 'Independent OpenSCAD export artifact',
    description: 'A bounded content-addressed STL or OBJ produced by the repository-owned engine and retained only for this MCP server session.',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'private' },
  }, async (uri, variables) => {
    const id = resourceVariable(uri, variables.sha256)
    if (!/^[a-f0-9]{64}$/.test(id)) throw new ResourceNotFoundError(uri.href)
    const artifact = independentArtifacts.get(id)
    if (!artifact || artifact.sha256 !== id || sha256(artifact.data) !== id) {
      throw new ResourceNotFoundError(uri.href)
    }
    return artifact.format === 'obj'
      ? {
          contents: [{
            uri: uri.href,
            mimeType: artifact.mimeType,
            text: new TextDecoder().decode(artifact.data),
          }],
        }
      : {
          contents: [{
            uri: uri.href,
            mimeType: artifact.mimeType,
            blob: Buffer.from(artifact.data.buffer, artifact.data.byteOffset, artifact.data.byteLength).toString('base64'),
          }],
        }
  })


  const officialArtifactTemplate = new ResourceTemplate('openscad://official-artifacts/{id}', {
    list: async () => ({
      resources: officialArtifacts.list().map(artifact => ({
        uri: officialArtifactUri(artifact.id),
        name: artifact.fileName,
        description: `Official OpenSCAD ${artifact.format.toUpperCase()} export (${artifact.byteLength.toLocaleString()} bytes); session-local`,
        mimeType: artifact.mimeType,
      })),
    }),
    complete: {
      id: value => officialArtifacts.list().map(artifact => artifact.id)
        .filter(id => id.startsWith(value)),
    },
  })

  server.registerResource('official-export-artifact', officialArtifactTemplate, {
    title: 'Official OpenSCAD export artifact',
    description: 'A bounded content-addressed upstream-runtime export retained only for this MCP server session.',
  }, async (uri, variables) => {
    const id = resourceVariable(uri, variables.id)
    if (!/^[a-f0-9]{64}$/.test(id)) throw new ResourceNotFoundError(uri.href)
    const artifact = officialArtifacts.get(id)
    if (!artifact) throw new ResourceNotFoundError(uri.href)
    return {
      contents: [{
        uri: uri.href,
        mimeType: artifact.mimeType,
        blob: Buffer.from(artifact.data.buffer, artifact.data.byteOffset, artifact.data.byteLength).toString('base64'),
      }],
    }
  })


  const exampleNames = EXAMPLE_CATALOG.map(example => example.id).sort()

  const examplesById = new Map(EXAMPLE_CATALOG.map(example => [example.id, example]))

  const exampleTemplate = new ResourceTemplate('openscad://examples/{name}', {
    list: async () => ({
      resources: exampleNames.map(name => ({
        uri: `openscad://examples/${name}`,
        name: `${name}.scad`,
        description: examplesById.get(name)?.description.en ?? `Bundled OpenSCAD example: ${name}`,
        mimeType: 'text/x-openscad',
      })),
    }),
    complete: { name: value => exampleNames.filter(name => name.startsWith(value)) },
  })

  server.registerResource('example', exampleTemplate, {
    title: 'Bundled OpenSCAD example',
    description: 'An example from the browser workspace.',
    mimeType: 'text/x-openscad',
    cacheHint: { ttlMs: CACHE_ONE_DAY, cacheScope: 'public' },
  }, async (uri, variables) => {
    const name = resourceVariable(uri, variables.name)
    const source = EXAMPLES[name]
    if (source === undefined) throw new ResourceNotFoundError(uri.href)
    return { contents: [{ uri: uri.href, mimeType: 'text/x-openscad', text: source }] }
  })
}
