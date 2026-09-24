import { isModelGraphText } from './modelGraphTextDetect'
import { parseParameterPresets, type ParameterPreset } from './parameterPresets'

/**
 * Versioned persistence contract for the local single-document workspace.
 *
 * Keeping the persisted payload behind this small boundary lets the UI evolve
 * to a multi-file workspace without coupling migrations to Vue components.
 */

export const WORKSPACE_SCHEMA_VERSION = 2 as const
export const WORKSPACE_STORAGE_KEY = 'open-scad-viewer.workspace'
export const WORKSPACE_RECOVERY_KEY_PREFIX = 'open-scad-viewer.workspace.recovery.'
export const WORKSPACE_LEGACY_SOURCE_KEY = 'scad-code'
export const WORKSPACE_LEGACY_TABS_KEY = 'scad-tabs'
export const WORKSPACE_LEGACY_ACTIVE_TAB_KEY = 'scad-active-tab'
export const WORKSPACE_LEGACY_TABS_BACKUP_KEY = 'open-scad-viewer.legacy-tabs-backup'
export const WORKSPACE_RECOVERY_VERSION = 1 as const

export interface WorkspaceDocumentSnapshot {
  readonly schemaVersion: typeof WORKSPACE_SCHEMA_VERSION
  readonly documentId: string
  readonly fileName: string
  readonly source: string
  readonly parameterPresets: readonly ParameterPreset[]
  /** Monotonic persistence generation for both source and metadata edits. */
  readonly mutation: number
  readonly revision: number
  readonly updatedAt: number
}

export interface WorkspaceStorage {
  getItem(key: string): string | null
  setItem(key: string, value: string): void
}

export type WorkspaceRecoveryBase =
  | { readonly status: 'unknown' }
  | { readonly status: 'empty' }
  | { readonly status: 'found'; readonly snapshot: WorkspaceDocumentSnapshot }

export interface WorkspaceRecoveryRecord {
  readonly snapshot: WorkspaceDocumentSnapshot
  readonly base: WorkspaceRecoveryBase
  readonly writerId: string | null
}

export interface CreateWorkspaceDocumentOptions {
  documentId?: string
  fileName?: string
  mutation?: number
  revision?: number
  updatedAt?: number
}

export const MAX_WORKSPACE_SOURCE_LENGTH = 250_000
export const MAX_WORKSPACE_FILE_NAME_LENGTH = 255
const MAX_LEGACY_TABS = 64
const MAX_LEGACY_TABS_SERIALIZED_LENGTH = 4 * 1024 * 1024

function finiteRevision(value: unknown): number | null {
  return typeof value === 'number' && Number.isSafeInteger(value) && value >= 0 ? value : null
}

function validDocumentId(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= 128
}

function validFileName(value: unknown): value is string {
  return typeof value === 'string' && value.length > 0 && value.length <= MAX_WORKSPACE_FILE_NAME_LENGTH
}

export const workspaceSourceByteLength=(value:string):number=>new TextEncoder().encode(value).byteLength

function fallbackDocumentId(): string {
  const cryptoApi = globalThis.crypto
  if (cryptoApi && typeof cryptoApi.randomUUID === 'function') return cryptoApi.randomUUID()
  return `document-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 10)}`
}

export function createWorkspaceDocument(
  source: string,
  options: CreateWorkspaceDocumentOptions = {},
): WorkspaceDocumentSnapshot {
  const revision = options.revision ?? 0
  return requireValidWorkspaceDocument({
    schemaVersion: WORKSPACE_SCHEMA_VERSION,
    documentId: options.documentId ?? fallbackDocumentId(),
    fileName: options.fileName ?? 'model.scad',
    source,
    parameterPresets: [],
    mutation: options.mutation ?? revision,
    revision,
    updatedAt: options.updatedAt ?? Date.now(),
  })
}

/**
 * Advance the geometry revision only when source changed. Metadata edits still
 * update the durable snapshot, but must not invalidate an in-flight build of
 * the identical source.
 */
/**
 * Preset lists are immutable — every update replaces the array (`[...]`,
 * `.filter`), and `parseParameterPresets` rebuilds it — so the serialization
 * can be memoized by array identity without invalidation risk.
 */
const parameterPresetsJsonCache = new WeakMap<readonly ParameterPreset[], string>()
function parameterPresetsJson(presets: readonly ParameterPreset[]): string {
  const cached = parameterPresetsJsonCache.get(presets)
  if (cached !== undefined) return cached
  const serialized = JSON.stringify(presets)
  parameterPresetsJsonCache.set(presets, serialized)
  return serialized
}

export function updateWorkspaceDocument(
  previous: WorkspaceDocumentSnapshot,
  update: { source?: string; fileName?: string; parameterPresets?: readonly ParameterPreset[] },
  updatedAt = Date.now(),
): WorkspaceDocumentSnapshot {
  const source = update.source ?? previous.source
  const fileName = update.fileName ?? previous.fileName
  const parameterPresets = update.parameterPresets ?? previous.parameterPresets
  if (source === previous.source && fileName === previous.fileName
    && (parameterPresets === previous.parameterPresets
      || parameterPresetsJson(parameterPresets) === parameterPresetsJson(previous.parameterPresets))) return previous
  return requireValidWorkspaceDocument({
    ...previous,
    source,
    fileName,
    parameterPresets,
    mutation: previous.mutation + 1,
    revision: previous.revision + (source === previous.source ? 0 : 1),
    updatedAt,
  })
}

/**
 * Normal persistence may only advance one document lineage. Replacing the
 * active document (for example with a shared import) is an explicit workflow,
 * not a regular autosave.
 */
export function workspaceDocumentAdvances(
  previous: WorkspaceDocumentSnapshot,
  next: WorkspaceDocumentSnapshot,
): boolean {
  if (workspaceDocumentsEqual(previous, next)) return true
  if (previous.documentId !== next.documentId
    || next.mutation <= previous.mutation
    || next.revision < previous.revision) return false
  return previous.source === next.source
    ? next.revision === previous.revision
    : next.revision > previous.revision
}

export function parseWorkspaceDocument(serialized: string): WorkspaceDocumentSnapshot | null {
  try {
    return parseWorkspaceDocumentValue(JSON.parse(serialized) as unknown)
  } catch {
    return null
  }
}

/**
 * Parse the atomic recovery envelope. Plain schema-v1 snapshots are accepted
 * as legacy journals, but their causal IDB base is deliberately unknown.
 */
export function parseWorkspaceRecovery(serialized: string): WorkspaceRecoveryRecord | null {
  try {
    const value = JSON.parse(serialized) as unknown
    if (!value || typeof value !== 'object') return null
    const candidate = value as {
      recoveryVersion?: unknown
      snapshot?: unknown
      base?: { status?: unknown; snapshot?: unknown }
      writerId?: unknown
    }
    if (candidate.recoveryVersion !== WORKSPACE_RECOVERY_VERSION) {
      const snapshot = parseWorkspaceDocumentValue(value)
      return snapshot ? { snapshot, base: { status: 'unknown' }, writerId: null } : null
    }
    const snapshot = parseWorkspaceDocumentValue(candidate.snapshot)
    if (!snapshot || !candidate.base || typeof candidate.base !== 'object') return null
    const writerId = candidate.writerId === undefined || candidate.writerId === null
      ? null
      : (typeof candidate.writerId === 'string'
          && candidate.writerId.length > 0
          && candidate.writerId.length <= 128
        ? candidate.writerId
        : undefined)
    if (writerId === undefined) return null
    if (candidate.base.status === 'unknown') return { snapshot, base: { status: 'unknown' }, writerId }
    if (candidate.base.status === 'empty') return { snapshot, base: { status: 'empty' }, writerId }
    if (candidate.base.status !== 'found') return null
    const baseSnapshot = parseWorkspaceDocumentValue(candidate.base.snapshot)
    return baseSnapshot && workspaceDocumentAdvances(baseSnapshot, snapshot)
      ? { snapshot, base: { status: 'found', snapshot: baseSnapshot }, writerId }
      : null
  } catch {
    return null
  }
}

/** Validate a structured-cloned snapshot read from IndexedDB. */
export function parseWorkspaceDocumentValue(value: unknown): WorkspaceDocumentSnapshot | null {
  if (!value || typeof value !== 'object') return null
  const candidate = value as Partial<Record<keyof WorkspaceDocumentSnapshot, unknown>>
  if ((candidate.schemaVersion !== WORKSPACE_SCHEMA_VERSION && candidate.schemaVersion !== 1)
    || !validDocumentId(candidate.documentId)
    || !validFileName(candidate.fileName)
    || typeof candidate.source !== 'string'
    || workspaceSourceByteLength(candidate.source) > MAX_WORKSPACE_SOURCE_LENGTH) return null
  const parameterPresets = candidate.schemaVersion === 1 ? [] : parseParameterPresets(candidate.parameterPresets)
  if (parameterPresets === null) return null
  const revision = finiteRevision(candidate.revision)
  // Schema v1 originally shipped without a general mutation counter. Treat
  // its geometry revision as the migration baseline, then persist mutation on
  // the next write. This keeps old local snapshots readable.
  const mutation = candidate.mutation === undefined
    ? revision
    : finiteRevision(candidate.mutation)
  const updatedAt = finiteRevision(candidate.updatedAt)
  if (revision === null || mutation === null || mutation < revision || updatedAt === null) return null
  return {
    schemaVersion: WORKSPACE_SCHEMA_VERSION,
    documentId: candidate.documentId,
    fileName: candidate.fileName,
    source: candidate.source,
    parameterPresets,
    mutation,
    revision,
    updatedAt,
  }
}

export function requireValidWorkspaceDocument(value: unknown): WorkspaceDocumentSnapshot {
  const document = parseWorkspaceDocumentValue(value)
  if (!document) throw new RangeError('Workspace document is outside the supported persistence limits')
  return document
}

export function workspaceDocumentsEqual(
  left: WorkspaceDocumentSnapshot | null,
  right: WorkspaceDocumentSnapshot | null,
): boolean {
  if (left === right) return true
  if (!left || !right) return false
  return left.schemaVersion === right.schemaVersion
    && left.documentId === right.documentId
    && left.fileName === right.fileName
    && left.source === right.source
    && parameterPresetsJson(left.parameterPresets) === parameterPresetsJson(right.parameterPresets)
    && left.mutation === right.mutation
    && left.revision === right.revision
    && left.updatedAt === right.updatedAt
}

/** Read only an actually persisted localStorage snapshot/legacy source. */
export function loadStoredWorkspaceDocument(
  storage: WorkspaceStorage,
  legacySourceKey = WORKSPACE_LEGACY_SOURCE_KEY,
): WorkspaceDocumentSnapshot | null {
  return loadVersionedWorkspaceDocument(storage)
    ?? loadLegacyWorkspaceDocument(storage, legacySourceKey)
}

export function loadVersionedWorkspaceDocument(
  storage: WorkspaceStorage,
): WorkspaceDocumentSnapshot | null {
  return loadWorkspaceRecovery(storage)?.snapshot ?? null
}

export function loadWorkspaceRecovery(
  storage: WorkspaceStorage,
  storageKey = WORKSPACE_STORAGE_KEY,
): WorkspaceRecoveryRecord | null {
  try {
    const serialized = storage.getItem(storageKey)
    return serialized ? parseWorkspaceRecovery(serialized) : null
  } catch {
    return null
  }
}

export function loadLegacyWorkspaceDocument(
  storage: WorkspaceStorage,
  legacySourceKey = WORKSPACE_LEGACY_SOURCE_KEY,
): WorkspaceDocumentSnapshot | null {
  try {
    const tabsDocument = loadLegacyTabsWorkspaceDocument(storage)
    if (tabsDocument) return tabsDocument
    const legacySource = storage.getItem(legacySourceKey)
    return legacySource === null ? null : createWorkspaceDocument(legacySource)
  } catch {
    return null
  }
}

/** Select the donor editor's active tab without trusting its unversioned shape. */
export function loadLegacyTabsWorkspaceDocument(storage: WorkspaceStorage): WorkspaceDocumentSnapshot | null {
  try {
    const serialized = storage.getItem(WORKSPACE_LEGACY_TABS_KEY)
    if (!serialized || serialized.length > MAX_LEGACY_TABS_SERIALIZED_LENGTH) return null
    const value = JSON.parse(serialized) as unknown
    if (!Array.isArray(value) || value.length === 0 || value.length > MAX_LEGACY_TABS) return null
    const tabs = value.filter((candidate): candidate is { id: string; name: string; code: string } => (
      !!candidate && typeof candidate === 'object'
      && typeof (candidate as { id?: unknown }).id === 'string'
      && (candidate as { id: string }).id.length > 0
      && (candidate as { id: string }).id.length <= 128
      && typeof (candidate as { name?: unknown }).name === 'string'
      && typeof (candidate as { code?: unknown }).code === 'string'
      && (candidate as { code: string }).code.length <= MAX_WORKSPACE_SOURCE_LENGTH
    ))
    if (tabs.length !== value.length || new Set(tabs.map(tab => tab.id)).size !== tabs.length) return null
    const activeId = storage.getItem(WORKSPACE_LEGACY_ACTIVE_TAB_KEY)
    const selected = tabs.find(tab => tab.id === activeId) ?? tabs[0]
    const rawName = selected.name.trim()
    const fileName = rawName.length > 0 && rawName.length <= MAX_WORKSPACE_FILE_NAME_LENGTH
      ? (/\.(scad|mg)$/i.test(rawName)
          ? rawName
          : `${rawName.slice(0, MAX_WORKSPACE_FILE_NAME_LENGTH - 5)}${isModelGraphText(selected.code) ? '.mg' : '.scad'}`)
      : 'model.scad'
    return createWorkspaceDocument(selected.code, { fileName })
  } catch {
    return null
  }
}

export function loadWorkspaceDocument(
  storage: WorkspaceStorage,
  fallbackSource: string,
  legacySourceKey = WORKSPACE_LEGACY_SOURCE_KEY,
): WorkspaceDocumentSnapshot {
  return loadStoredWorkspaceDocument(storage, legacySourceKey)
    ?? createWorkspaceDocument(fallbackSource)
}

export function saveWorkspaceDocument(storage: WorkspaceStorage, document: WorkspaceDocumentSnapshot): boolean {
  try {
    storage.setItem(WORKSPACE_STORAGE_KEY, JSON.stringify(requireValidWorkspaceDocument(document)))
    return true
  } catch {
    return false
  }
}

/** Atomically persist a draft together with the IDB head it was based on. */
export function saveWorkspaceRecovery(
  storage: WorkspaceStorage,
  document: WorkspaceDocumentSnapshot,
  base: WorkspaceRecoveryBase,
  writerId?: string,
  storageKey = WORKSPACE_STORAGE_KEY,
): boolean {
  try {
    if (writerId !== undefined && (writerId.length === 0 || writerId.length > 128)) return false
    const snapshot = requireValidWorkspaceDocument(document)
    const validatedBase: WorkspaceRecoveryBase = base.status === 'found'
      ? { status: 'found', snapshot: requireValidWorkspaceDocument(base.snapshot) }
      : { status: base.status }
    if (validatedBase.status === 'found'
      && !workspaceDocumentAdvances(validatedBase.snapshot, snapshot)) return false
    storage.setItem(storageKey, JSON.stringify({
      recoveryVersion: WORKSPACE_RECOVERY_VERSION,
      snapshot,
      base: validatedBase,
      writerId: writerId ?? null,
    }))
    return true
  } catch {
    return false
  }
}
