import {register_review_model} from './registerResourcesAndPrompts/review_model'
import {register_geometry_parity} from './registerResourcesAndPrompts/geometry_parity'
import {register_independent_export_artifact} from './registerResourcesAndPrompts/independent_export_artifact'
import {
  ResourceNotFoundError,
  ResourceTemplate,
  SUPPORTED_PROTOCOL_VERSIONS,
  type McpServer,
} from '@modelcontextprotocol/server'
import { z } from 'zod/v4'
import { GEOMETRY_MANIFEST_ARCHIVE } from '../../core/geometryExecution'
import { OPENSCAD_2021_01_CONTRACT } from '../../core/openScad2021Contract'
import { EXAMPLE_CATALOG, EXAMPLES } from '../../data/examples'
import {
  OPENSCAD_PROJECT_MAX_BLOB_BYTES,
  OPENSCAD_PROJECT_MAX_FILES,
  OPENSCAD_PROJECT_MAX_SOURCE_BYTES,
  OPENSCAD_PROJECT_MAX_TOTAL_BYTES,
} from '../../services/openScadProject'
import {
  DEFAULT_MAX_IN_FLIGHT_REQUESTS,
  DEFAULT_MAX_PENDING_CONTROL_MESSAGES,
  DEFAULT_MAX_PENDING_OUTBOUND_MESSAGES,
  MAX_MCP_SUBSCRIPTIONS,
  MAX_MCP_STRING_REQUEST_ID_LENGTH,
} from '../boundedTransport'
import {
  MAX_ARTIFACT_BYTES,
  MAX_PENDING_GEOMETRY_JOBS,
} from '../geometryService'
import {
  MAX_INDEPENDENT_ARTIFACT_CACHE_BYTES,
  MAX_INDEPENDENT_ARTIFACT_CACHE_ENTRIES,
} from '../independentArtifactCache'
import { INDEPENDENT_OPENSCAD_ENGINE_ATTESTATION } from '../independentOpenScadExecution'
import {
  canonicalJson,
  immutableEngineManifestToWire,
  staticEngineManifestToWire,
} from '../engineManifest'
import { MAX_MODEL_SOURCE_LENGTH } from '../modelStore'
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
} from './contracts'
import { idSchema, promptGoalSchema } from './schemas'
import {
  buildToWire,
  engineRegistryToWire,
  officialCapabilitiesToWire,
  resourceId,
  resourceVariable,
} from './wireMappers'
import type { McpToolRegistrationDeps } from './registerTools'

export function registerMcpPromptsAndResources(deps: McpToolRegistrationDeps): void {
  register_review_model(deps)
  register_geometry_parity(deps)
  register_independent_export_artifact(deps)
}
