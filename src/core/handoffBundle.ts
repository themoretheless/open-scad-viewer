import {
  MAX_GEOMETRY_SOURCE_CHARACTERS,
  canonicalJson,
  geometryEngineClassForLanguageContract,
  parseGeometrySourceRoutingHeader,
  type GeometryEngineClass,
  type GeometryLanguageContract,
} from './geometryExecution'
import { sha256Hex } from './sha256'
import { isWellFormedUnicode } from './unicode'

export const OPENSCAD_HANDOFF_BUNDLE_SCHEMA = 'openscad-viewer/handoff-bundle-v1' as const

export type HandoffOrigin = 'browser-workspace' | 'mcp-catalog'

export type HandoffParameterPresets = Record<string, Record<string, string | number | boolean>>

export interface OpenScadHandoffDocument {
  readonly id: string | null
  readonly name: string
  readonly source: string
  readonly source_sha256: string
  readonly revision: number | null
  readonly parameter_presets?: HandoffParameterPresets | undefined
}

export interface OpenScadHandoffRouting {
  readonly language_contract: GeometryLanguageContract
  readonly engine_class: GeometryEngineClass
  readonly required_capabilities: string[]
  readonly automatic_fallback: false
}

export interface OpenScadHandoffBundleV1 {
  readonly schema: typeof OPENSCAD_HANDOFF_BUNDLE_SCHEMA
  readonly origin: HandoffOrigin
  readonly created_at: string
  readonly document: OpenScadHandoffDocument
  readonly routing: OpenScadHandoffRouting
  readonly bundle_sha256: string
}

export class HandoffBundleValidationError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'HandoffBundleValidationError'
  }
}

export class HandoffConflictError extends Error {
  constructor(
    message: string,
    readonly modelId: string | null,
    readonly expectedRevision: number | null,
    readonly actualRevision: number | null,
  ) {
    super(message)
    this.name = 'HandoffConflictError'
  }
}

const MAX_NAME_LENGTH = 255
const MAX_ID_LENGTH = 128
const MAX_PRESETS = 32
const MAX_PRESET_PARAMS = 64

function normalizePresets(presets: unknown): HandoffParameterPresets | undefined {
  if (presets === undefined) return undefined
  if (!presets || typeof presets !== 'object' || Array.isArray(presets)) {
    throw new HandoffBundleValidationError('parameter_presets must be a JSON object')
  }
  const entries = Object.entries(presets as Record<string, unknown>)
  if (entries.length > MAX_PRESETS) {
    throw new HandoffBundleValidationError(`parameter_presets is limited to ${MAX_PRESETS} presets`)
  }
  const normalized: HandoffParameterPresets = {}
  for (const [presetName, rawValues] of entries) {
    if (!presetName || presetName.length > 128 || !isWellFormedUnicode(presetName)) {
      throw new HandoffBundleValidationError('Invalid preset name in parameter_presets')
    }
    if (!rawValues || typeof rawValues !== 'object' || Array.isArray(rawValues)) {
      throw new HandoffBundleValidationError(`Preset ${presetName} must be an object`)
    }
    const paramEntries = Object.entries(rawValues as Record<string, unknown>)
    if (paramEntries.length > MAX_PRESET_PARAMS) {
      throw new HandoffBundleValidationError(`Preset ${presetName} exceeds ${MAX_PRESET_PARAMS} parameters`)
    }
    const values: Record<string, string | number | boolean> = {}
    for (const [key, val] of paramEntries) {
      if (!key || key.length > 128 || !isWellFormedUnicode(key)) {
        throw new HandoffBundleValidationError('Invalid parameter name in preset')
      }
      if (typeof val === 'string') {
        if (val.length > 1024 || !isWellFormedUnicode(val)) {
          throw new HandoffBundleValidationError('Invalid string value in preset')
        }
        values[key] = val
      } else if (typeof val === 'number') {
        if (!Number.isFinite(val)) throw new HandoffBundleValidationError('Preset number must be finite')
        values[key] = val
      } else if (typeof val === 'boolean') {
        values[key] = val
      } else {
        throw new HandoffBundleValidationError('Preset values must be string, finite number, or boolean')
      }
    }
    normalized[presetName] = values
  }
  return normalized
}

function deriveRoutingFromSource(source: string): OpenScadHandoffRouting {
  if (typeof source !== 'string' || !isWellFormedUnicode(source)) {
    throw new HandoffBundleValidationError('Handoff source must contain well-formed Unicode')
  }
  if (source.length > MAX_GEOMETRY_SOURCE_CHARACTERS) {
    throw new HandoffBundleValidationError(`Handoff source exceeds ${MAX_GEOMETRY_SOURCE_CHARACTERS} characters`)
  }
  try {
    const parsed = parseGeometrySourceRoutingHeader(source)
    return {
      language_contract: parsed.languageContract,
      engine_class: geometryEngineClassForLanguageContract(parsed.languageContract),
      required_capabilities: [...parsed.requiredCapabilities],
      automatic_fallback: false,
    }
  } catch (error) {
    throw new HandoffBundleValidationError(
      error instanceof Error ? error.message : 'Invalid source routing header in handoff bundle',
    )
  }
}

function computeBundleDigest(payload: Omit<OpenScadHandoffBundleV1, 'bundle_sha256'>): string {
  return sha256Hex(canonicalJson(payload))
}

export function createOpenScadHandoffBundle(input: {
  origin: HandoffOrigin
  id?: string | null | undefined
  name: string
  source: string
  revision?: number | null | undefined
  createdAt?: string | undefined
  parameterPresets?: HandoffParameterPresets | undefined
}): OpenScadHandoffBundleV1 {
  const id = input.id ?? null
  if (
    id !== null &&
    (typeof id !== 'string' || id.length === 0 || id.length > MAX_ID_LENGTH || !isWellFormedUnicode(id))
  ) {
    throw new HandoffBundleValidationError('Handoff document id must be 1-128 well-formed characters or null')
  }
  if (
    typeof input.name !== 'string' ||
    !input.name.trim() ||
    input.name.length > MAX_NAME_LENGTH ||
    !isWellFormedUnicode(input.name)
  ) {
    throw new HandoffBundleValidationError(`Handoff document name must be 1-${MAX_NAME_LENGTH} characters`)
  }
  const revision = input.revision ?? null
  if (revision !== null && (!Number.isSafeInteger(revision) || revision < 0)) {
    throw new HandoffBundleValidationError('Handoff revision must be a non-negative safe integer or null')
  }
  const createdAt = input.createdAt ?? new Date().toISOString()
  if (
    typeof createdAt !== 'string' ||
    createdAt.length === 0 ||
    createdAt.length > 64 ||
    !isWellFormedUnicode(createdAt)
  ) {
    throw new HandoffBundleValidationError('Invalid handoff created_at timestamp')
  }
  const presets = normalizePresets(input.parameterPresets)
  const routing = deriveRoutingFromSource(input.source)
  const document: OpenScadHandoffDocument = {
    id,
    name: input.name,
    source: input.source,
    source_sha256: sha256Hex(input.source),
    revision,
    ...(presets !== undefined ? { parameter_presets: presets } : {}),
  }
  const unsigned = {
    schema: OPENSCAD_HANDOFF_BUNDLE_SCHEMA,
    origin: input.origin,
    created_at: createdAt,
    document,
    routing,
  }
  return {
    ...unsigned,
    bundle_sha256: computeBundleDigest(unsigned),
  }
}

export function verifyOpenScadHandoffBundle(candidate: unknown): OpenScadHandoffBundleV1 {
  if (!candidate || typeof candidate !== 'object' || Array.isArray(candidate)) {
    throw new HandoffBundleValidationError('Handoff bundle must be an object')
  }
  const raw = candidate as Record<string, unknown>
  if (raw.schema !== OPENSCAD_HANDOFF_BUNDLE_SCHEMA) {
    throw new HandoffBundleValidationError(`Unsupported handoff bundle schema: ${String(raw.schema)}`)
  }
  if (raw.origin !== 'browser-workspace' && raw.origin !== 'mcp-catalog') {
    throw new HandoffBundleValidationError('Handoff origin must be browser-workspace or mcp-catalog')
  }
  if (
    typeof raw.created_at !== 'string' ||
    !raw.created_at ||
    raw.created_at.length > 64 ||
    !isWellFormedUnicode(raw.created_at)
  ) {
    throw new HandoffBundleValidationError('Handoff bundle has an invalid created_at timestamp')
  }
  if (!raw.document || typeof raw.document !== 'object' || Array.isArray(raw.document)) {
    throw new HandoffBundleValidationError('Handoff bundle document is missing')
  }
  const doc = raw.document as Record<string, unknown>
  if (doc.id !== null && typeof doc.id !== 'string') {
    throw new HandoffBundleValidationError('Handoff document id must be string or null')
  }
  if (typeof doc.name !== 'string') {
    throw new HandoffBundleValidationError('Handoff document name must be a string')
  }
  if (typeof doc.source !== 'string') {
    throw new HandoffBundleValidationError('Handoff document source must be a string')
  }
  if (typeof doc.source_sha256 !== 'string' || !/^[a-f0-9]{64}$/.test(doc.source_sha256)) {
    throw new HandoffBundleValidationError('Handoff document source_sha256 must be a lowercase SHA-256 hex string')
  }
  if (
    doc.revision !== null &&
    (typeof doc.revision !== 'number' || !Number.isSafeInteger(doc.revision) || doc.revision < 0)
  ) {
    throw new HandoffBundleValidationError('Handoff document revision must be a non-negative integer or null')
  }
  const expectedRouting = deriveRoutingFromSource(doc.source)
  const expectedSourceSha = sha256Hex(doc.source)
  if (doc.source_sha256 !== expectedSourceSha) {
    throw new HandoffBundleValidationError('Handoff source_sha256 does not match document source')
  }
  if (!raw.routing || typeof raw.routing !== 'object' || Array.isArray(raw.routing)) {
    throw new HandoffBundleValidationError('Handoff bundle routing is missing')
  }
  if (canonicalJson(raw.routing) !== canonicalJson(expectedRouting)) {
    throw new HandoffBundleValidationError('Handoff routing contradicts source @language/@requires headers')
  }
  if (typeof raw.bundle_sha256 !== 'string' || !/^[a-f0-9]{64}$/.test(raw.bundle_sha256)) {
    throw new HandoffBundleValidationError('Handoff bundle_sha256 must be a lowercase SHA-256 hex string')
  }
  const rebuilt = createOpenScadHandoffBundle({
    origin: raw.origin,
    id: doc.id,
    name: doc.name,
    source: doc.source,
    revision: doc.revision,
    createdAt: raw.created_at,
    parameterPresets: normalizePresets(doc.parameter_presets),
  })
  if (rebuilt.bundle_sha256 !== raw.bundle_sha256) {
    throw new HandoffBundleValidationError('Handoff bundle_sha256 verification failed')
  }
  return rebuilt
}

export function exportWorkspaceHandoffBundle(snapshot: {
  documentId: string
  fileName: string
  source: string
  revision: number
  updatedAt: string
  parameterPresets?: HandoffParameterPresets | undefined
}): OpenScadHandoffBundleV1 {
  return createOpenScadHandoffBundle({
    origin: 'browser-workspace',
    id: snapshot.documentId,
    name: snapshot.fileName,
    source: snapshot.source,
    revision: snapshot.revision,
    createdAt: snapshot.updatedAt,
    parameterPresets: snapshot.parameterPresets,
  })
}

export function prepareWorkspaceHandoffImport(
  bundle: unknown,
  current: { documentId: string; source: string; revision: number },
  options: { expectedWorkspaceRevision?: number | undefined } = {},
): {
  verified: OpenScadHandoffBundleV1
  nextSource: string
  nextRevision: number
  changed: boolean
} {
  const verified = verifyOpenScadHandoffBundle(bundle)
  if (current.source === verified.document.source) {
    return {
      verified,
      nextSource: current.source,
      nextRevision: current.revision,
      changed: false,
    }
  }
  if (options.expectedWorkspaceRevision !== current.revision) {
    throw new HandoffConflictError(
      `Workspace ${current.documentId} is at revision ${current.revision}; supply expectedWorkspaceRevision=${current.revision} to apply handoff`,
      current.documentId,
      options.expectedWorkspaceRevision ?? null,
      current.revision,
    )
  }
  return {
    verified,
    nextSource: verified.document.source,
    nextRevision: current.revision + 1,
    changed: true,
  }
}
