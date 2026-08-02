import type { GeometryEvaluationResult, GeometryQuality } from '../core/build'
import {
  GEOMETRY_MANIFEST_ARCHIVE,
  MAX_GEOMETRY_SOURCE_CHARACTERS,
} from '../core/geometryExecution'
import { sha256Hex } from '../core/sha256'
import { isGeometryEvaluationResultPayload } from '../services/geometryWorkerProtocol'

export const MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION = 1 as const

/**
 * The disposable Node qualification lane intentionally admits a smaller IPC
 * payload than the browser protocol-v5 ceiling. It is a differential evidence
 * path, not a bulk export transport.
 */
export const MCP_MANIFOLD_PLAN_QUALIFICATION_IPC_LIMITS = Object.freeze({
  meshes: 256,
  triangles: 250_000,
  bytes: 32 * 1024 * 1024,
})

const legacyManifest = GEOMETRY_MANIFEST_ARCHIVE['manifold-node-v1']

/**
 * Explicitly non-production identity for the supervised G1 adapter lane. It
 * binds the underlying pinned artifact without pretending that the adapter is
 * the legacy-direct provider recorded by the production-v5 manifest.
 */
export const MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY = Object.freeze({
  adapterVersion: 'semantic-manifold-plan-qualification-v1' as const,
  semanticProgramSchema: '1.2' as const,
  engineClass: 'manifold' as const,
  engineKey: 'manifold-plan-qualification-v1' as const,
  kernelFingerprint: legacyManifest.kernelFingerprint,
  dependencyVersion: legacyManifest.dependency.version,
  sourceManifestDigest: legacyManifest.manifestDigest,
  qualificationOnly: true as const,
  automaticFallback: false as const,
})

export type McpManifoldPlanQualificationIdentity =
  typeof MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY

export interface McpManifoldPlanQualificationRequest {
  readonly protocolVersion: typeof MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION
  readonly type: 'evaluate'
  readonly workerEpoch: number
  readonly jobId: number
  readonly source: string
  readonly sourceSha256: string
  readonly quality: GeometryQuality
  readonly identity: McpManifoldPlanQualificationIdentity
}

export interface McpManifoldPlanQualificationCancel {
  readonly protocolVersion: typeof MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION
  readonly type: 'cancel'
  readonly workerEpoch: number
  readonly jobId: number
  readonly sourceSha256: string
  readonly reason: 'cancelled' | 'deadline'
  readonly identity: McpManifoldPlanQualificationIdentity
}

export type McpManifoldPlanQualificationCommand =
  | McpManifoldPlanQualificationRequest
  | McpManifoldPlanQualificationCancel

export interface McpManifoldPlanQualificationError {
  readonly name: string
  readonly message: string
  readonly code: string | null
  readonly line: number | null
  readonly column: number | null
  readonly start: number | null
  readonly end: number | null
}

interface McpManifoldPlanQualificationTerminalEnvelope {
  readonly protocolVersion: typeof MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION
  readonly workerEpoch: number
  readonly jobId: number
  readonly sourceSha256: string
  readonly identity: McpManifoldPlanQualificationIdentity
}

export interface McpManifoldPlanQualificationStarted
  extends McpManifoldPlanQualificationTerminalEnvelope {
  readonly status: 'started'
}

export interface McpManifoldPlanQualificationSuccess
  extends McpManifoldPlanQualificationTerminalEnvelope {
  readonly status: 'succeeded'
  readonly result: GeometryEvaluationResult
}

export interface McpManifoldPlanQualificationFailure
  extends McpManifoldPlanQualificationTerminalEnvelope {
  readonly status: 'failed'
  readonly error: McpManifoldPlanQualificationError
}

export type McpManifoldPlanQualificationTerminal =
  | McpManifoldPlanQualificationSuccess
  | McpManifoldPlanQualificationFailure

const REQUEST_KEYS = Object.freeze([
  'protocolVersion', 'type', 'workerEpoch', 'jobId', 'source', 'sourceSha256',
  'quality', 'identity',
])
const CANCEL_KEYS = Object.freeze([
  'protocolVersion', 'type', 'workerEpoch', 'jobId', 'sourceSha256', 'reason', 'identity',
])
const TERMINAL_KEYS = Object.freeze([
  'protocolVersion', 'workerEpoch', 'jobId', 'sourceSha256', 'identity', 'status',
])
const IDENTITY_KEYS = Object.freeze(Object.keys(MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY))
const ERROR_KEYS = Object.freeze(['name', 'message', 'code', 'line', 'column', 'start', 'end'])
const MAX_ERROR_MESSAGE_CHARACTERS = 250_512

function record(value: unknown): Record<string, unknown> | null {
  if (value === null || Array.isArray(value) || typeof value !== 'object') return null
  const prototype = Object.getPrototypeOf(value)
  return prototype === Object.prototype || prototype === null
    ? value as Record<string, unknown>
    : null
}

function exactDataKeys(value: Record<string, unknown>, keys: readonly string[]): boolean {
  const descriptors = Object.getOwnPropertyDescriptors(value)
  const actual = Reflect.ownKeys(descriptors)
  return actual.length === keys.length
    && actual.every(key => typeof key === 'string' && keys.includes(key))
    && keys.every(key => {
      const descriptor = descriptors[key]
      return descriptor !== undefined
        && descriptor.enumerable
        && Object.hasOwn(descriptor, 'value')
    })
}

function nonNegativeSafeInteger(value: unknown): value is number {
  return Number.isSafeInteger(value) && (value as number) >= 0
}

function wellFormed(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const unit = value.charCodeAt(index)
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(++index)
      if (!(next >= 0xdc00 && next <= 0xdfff)) return false
    } else if (unit >= 0xdc00 && unit <= 0xdfff) return false
  }
  return true
}

function truncateWellFormed(value: string, maximum: number): string {
  if (value.length <= maximum) return value
  let end = maximum
  const last = value.charCodeAt(end - 1)
  if (last >= 0xd800 && last <= 0xdbff) end--
  return value.slice(0, end)
}

function isIdentity(value: unknown): value is McpManifoldPlanQualificationIdentity {
  const candidate = record(value)
  if (candidate === null || !exactDataKeys(candidate, IDENTITY_KEYS)) return false
  return IDENTITY_KEYS.every(key => (
    Object.is(candidate[key], MCP_MANIFOLD_PLAN_QUALIFICATION_IDENTITY[
      key as keyof McpManifoldPlanQualificationIdentity
    ])
  ))
}

function nullableInteger(value: unknown, positive = false): boolean {
  return value === null || (nonNegativeSafeInteger(value) && (!positive || value > 0))
}

function isFailure(value: unknown, sourceLength: number): value is McpManifoldPlanQualificationError {
  const candidate = record(value)
  if (candidate === null
    || !exactDataKeys(candidate, ERROR_KEYS)
    || typeof candidate.name !== 'string'
    || candidate.name.length === 0
    || candidate.name.length > 128
    || !wellFormed(candidate.name)
    || typeof candidate.message !== 'string'
    || candidate.message.length > MAX_ERROR_MESSAGE_CHARACTERS
    || !wellFormed(candidate.message)
    || !(candidate.code === null
      || (typeof candidate.code === 'string' && candidate.code.length <= 80 && wellFormed(candidate.code)))
    || !nullableInteger(candidate.line, true)
    || !nullableInteger(candidate.column, true)
    || !nullableInteger(candidate.start)
    || !nullableInteger(candidate.end)) return false
  const start = candidate.start as number | null
  const end = candidate.end as number | null
  return (start === null) === (end === null)
    && (start === null || (end! >= start && end! <= sourceLength))
}

function ownData(value: object, key: string): unknown {
  const descriptor = Object.getOwnPropertyDescriptor(value, key)
  return descriptor !== undefined && Object.hasOwn(descriptor, 'value')
    ? descriptor.value
    : undefined
}

/**
 * Apply the smaller Node/MCP budget before the shared 128 MiB deep validator.
 * This pass reads only own data descriptors and typed-view metadata, so an
 * accessor or oversized payload cannot force the parent into the expensive
 * element/BVH scan first.
 */
function isWithinQualificationIpcLimits(value: unknown): boolean {
  const terminal = record(value)
  if (terminal === null || ownData(terminal, 'status') !== 'succeeded') return true
  const result = record(ownData(terminal, 'result'))
  if (result === null) return false
  const meshes = ownData(result, 'meshes')
  if (!Array.isArray(meshes)
    || meshes.length > MCP_MANIFOLD_PLAN_QUALIFICATION_IPC_LIMITS.meshes) return false

  let triangles = 0
  let bytes = 0
  for (let index = 0; index < meshes.length; index++) {
    const slot = Object.getOwnPropertyDescriptor(meshes, String(index))
    if (slot === undefined || !Object.hasOwn(slot, 'value')) return false
    const mesh = record(slot.value)
    if (mesh === null) return false
    const indices = ownData(mesh, 'indices')
    if (!(indices instanceof Uint32Array)) return false
    triangles += indices.length / 3
    const bvh = record(ownData(mesh, 'bvh'))
    if (bvh === null) return false
    const views = [
      ownData(mesh, 'vertices'), indices, ownData(mesh, 'edgeIndices'),
      ownData(mesh, 'faceIds'), ownData(mesh, 'transform'), ownData(bvh, 'bounds'),
      ownData(bvh, 'nodes'), ownData(bvh, 'triangles'),
    ]
    for (const view of views) {
      if (!ArrayBuffer.isView(view)) return false
      bytes += view.byteLength
    }
    if (triangles > MCP_MANIFOLD_PLAN_QUALIFICATION_IPC_LIMITS.triangles
      || bytes > MCP_MANIFOLD_PLAN_QUALIFICATION_IPC_LIMITS.bytes) return false
  }
  return true
}

export function isMcpManifoldPlanQualificationRequest(
  value: unknown,
): value is McpManifoldPlanQualificationRequest {
  try {
    const candidate = record(value)
    return candidate !== null
      && exactDataKeys(candidate, REQUEST_KEYS)
      && candidate.protocolVersion === MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION
      && candidate.type === 'evaluate'
      && nonNegativeSafeInteger(candidate.workerEpoch)
      && candidate.workerEpoch > 0
      && nonNegativeSafeInteger(candidate.jobId)
      && candidate.jobId > 0
      && typeof candidate.source === 'string'
      && candidate.source.length <= MAX_GEOMETRY_SOURCE_CHARACTERS
      && wellFormed(candidate.source)
      && typeof candidate.sourceSha256 === 'string'
      && /^[a-f0-9]{64}$/.test(candidate.sourceSha256)
      && sha256Hex(candidate.source) === candidate.sourceSha256
      && (candidate.quality === 'preview' || candidate.quality === 'full')
      && isIdentity(candidate.identity)
  } catch {
    return false
  }
}

export function isMcpManifoldPlanQualificationCancel(
  value: unknown,
  request: McpManifoldPlanQualificationRequest,
): value is McpManifoldPlanQualificationCancel {
  try {
    const candidate = record(value)
    return candidate !== null
      && exactDataKeys(candidate, CANCEL_KEYS)
      && candidate.protocolVersion === MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION
      && candidate.type === 'cancel'
      && candidate.workerEpoch === request.workerEpoch
      && candidate.jobId === request.jobId
      && candidate.sourceSha256 === request.sourceSha256
      && (candidate.reason === 'cancelled' || candidate.reason === 'deadline')
      && isIdentity(candidate.identity)
  } catch {
    return false
  }
}

export function isMcpManifoldPlanQualificationTerminal(
  value: unknown,
  request: McpManifoldPlanQualificationRequest,
): value is McpManifoldPlanQualificationTerminal {
  try {
    const candidate = record(value)
    if (candidate === null
      || !exactDataKeys(candidate, [...TERMINAL_KEYS, candidate.status === 'succeeded' ? 'result' : 'error'])
      || candidate.protocolVersion !== MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION
      || candidate.workerEpoch !== request.workerEpoch
      || candidate.jobId !== request.jobId
      || candidate.sourceSha256 !== request.sourceSha256
      || !isIdentity(candidate.identity)) return false
    if (candidate.status === 'succeeded') {
      return isGeometryEvaluationResultPayload(candidate.result)
        && candidate.result.quality === request.quality
    }
    return candidate.status === 'failed' && isFailure(candidate.error, request.source.length)
  } catch {
    return false
  }
}

/** Node/MCP qualification-only terminal guard with its stricter IPC budget. */
export function isMcpManifoldPlanQualificationNodeTerminal(
  value: unknown,
  request: McpManifoldPlanQualificationRequest,
): value is McpManifoldPlanQualificationTerminal {
  return isWithinQualificationIpcLimits(value)
    && isMcpManifoldPlanQualificationTerminal(value, request)
}

export function isMcpManifoldPlanQualificationStarted(
  value: unknown,
  request: McpManifoldPlanQualificationRequest,
): value is McpManifoldPlanQualificationStarted {
  try {
    const candidate = record(value)
    return candidate !== null
      && exactDataKeys(candidate, TERMINAL_KEYS)
      && candidate.protocolVersion === MCP_MANIFOLD_PLAN_QUALIFICATION_PROTOCOL_VERSION
      && candidate.workerEpoch === request.workerEpoch
      && candidate.jobId === request.jobId
      && candidate.sourceSha256 === request.sourceSha256
      && candidate.status === 'started'
      && isIdentity(candidate.identity)
  } catch {
    return false
  }
}

function inheritedData(value: object, key: string): unknown {
  let cursor: object | null = value
  for (let depth = 0; cursor !== null && depth < 8; depth++) {
    const descriptor = Object.getOwnPropertyDescriptor(cursor, key)
    if (descriptor !== undefined) return Object.hasOwn(descriptor, 'value') ? descriptor.value : undefined
    cursor = Object.getPrototypeOf(cursor)
  }
  return undefined
}

function optionalInteger(value: unknown, positive = false): number | null {
  return nonNegativeSafeInteger(value) && (!positive || value > 0) ? value : null
}

export function serializeMcpManifoldPlanQualificationError(
  error: unknown,
): McpManifoldPlanQualificationError {
  const object = error !== null && (typeof error === 'object' || typeof error === 'function')
    ? error as object
    : null
  const nameValue = object === null ? undefined : inheritedData(object, 'name')
  const messageValue = object === null ? undefined : inheritedData(object, 'message')
  const codeValue = object === null ? undefined : inheritedData(object, 'code')
  const name = typeof nameValue === 'string' && wellFormed(nameValue)
    ? truncateWellFormed(nameValue, 128)
    : 'Error'
  const message = typeof messageValue === 'string' && wellFormed(messageValue)
    ? truncateWellFormed(messageValue, MAX_ERROR_MESSAGE_CHARACTERS)
    : 'The qualification child rejected with an unserializable error'
  const code = typeof codeValue === 'string' && wellFormed(codeValue)
    ? truncateWellFormed(codeValue, 80)
    : null
  const start = optionalInteger(object === null ? undefined : inheritedData(object, 'start'))
  const endValue = optionalInteger(object === null ? undefined : inheritedData(object, 'end'))
  const end = start !== null && endValue !== null && endValue >= start ? endValue : start
  return Object.freeze({
    name,
    message,
    code,
    line: optionalInteger(object === null ? undefined : inheritedData(object, 'line'), true),
    column: optionalInteger(object === null ? undefined : inheritedData(object, 'column'), true),
    start,
    end,
  })
}
