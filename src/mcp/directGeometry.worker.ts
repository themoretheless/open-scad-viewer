import { parentPort } from 'node:worker_threads'
import type { GeometryExecutionDescriptor } from '../core/geometryExecution'
import { meshTransferables } from '../core/mesh'
import {
  defaultGeometryBuildEngine,
  type GeometryBuildResult,
} from '../services/geometryBuildEngine'
import {
  DIRECT_GEOMETRY_IDENTITY,
  DIRECT_GEOMETRY_PROTOCOL_VERSION,
  directGeometryExecutionForError,
  isDirectGeometryCancel,
  isDirectGeometryNodeTerminal,
  isDirectGeometryRequest,
  isDirectGeometryStarted,
  serializeDirectGeometryError,
  type DirectGeometryBuildRequest,
  type DirectGeometryFailure,
  type DirectGeometryRequest,
  type DirectGeometryStarted,
  type DirectGeometryTerminal,
} from './directGeometryProtocol'

if (parentPort === null) throw new Error('Direct geometry worker requires a parent port')

let activeRequest: DirectGeometryRequest | null = null
let cancelled = false
let terminalPosted = false

function eventEnvelope(request: DirectGeometryRequest) {
  return {
    protocolVersion: DIRECT_GEOMETRY_PROTOCOL_VERSION,
    workerEpoch: request.workerEpoch,
    jobId: request.jobId,
    sourceSha256: request.sourceSha256,
    quality: request.quality,
    purpose: request.purpose,
    identity: DIRECT_GEOMETRY_IDENTITY,
    kind: request.type,
  } as const
}

function internalFailure(
  request: DirectGeometryRequest,
  execution: GeometryExecutionDescriptor | null,
): DirectGeometryFailure {
  return {
    ...eventEnvelope(request),
    status: 'failed',
    execution,
    error: serializeDirectGeometryError(null),
  }
}

function postTerminal(
  request: DirectGeometryRequest,
  terminal: DirectGeometryTerminal,
  transfer: readonly ArrayBuffer[] = [],
): void {
  // The supervisor owns terminate-and-join. Keep the port alive until then:
  // natural worker teardown can race terminate() and exceed its join deadline.
  if (terminalPosted) return
  let publishable = terminal
  let publishTransfer = transfer
  if (!isDirectGeometryNodeTerminal(publishable, request)) {
    publishable = internalFailure(request, null)
    publishTransfer = []
  }
  try {
    parentPort!.postMessage(publishable, [...publishTransfer])
    terminalPosted = true
  } catch {
    const fallback = internalFailure(request, null)
    if (!terminalPosted && isDirectGeometryNodeTerminal(fallback, request)) {
      try {
        parentPort!.postMessage(fallback)
        terminalPosted = true
      } catch {
        // Parent-side lifecycle code treats exit without a terminal as a crash.
        // The worker deliberately writes no child diagnostic to MCP stdio.
      }
    }
  } finally {
    // With no deliverable terminal, let exit report a crash instead of leaving
    // the parent waiting for the full job deadline.
    if (!terminalPosted) parentPort!.close()
  }
}

function postStarted(request: DirectGeometryRequest): void {
  const started: DirectGeometryStarted = {
    ...eventEnvelope(request),
    status: 'started',
  }
  if (!isDirectGeometryStarted(started, request)) {
    throw new TypeError('Direct geometry started envelope failed local validation')
  }
  parentPort!.postMessage(started)
}

async function build(request: DirectGeometryBuildRequest): Promise<void> {
  let planned: GeometryExecutionDescriptor | null = null
  try {
    planned = defaultGeometryBuildEngine.planSource(request.source, {
      quality: request.quality,
      purpose: request.purpose,
    })
    const built: GeometryBuildResult = await defaultGeometryBuildEngine.buildSource(
      request.source,
      { quality: request.quality, purpose: request.purpose },
      { shouldAbort: () => cancelled },
    )
    postTerminal(request, {
      ...eventEnvelope(request),
      status: 'succeeded',
      kind: 'build',
      built,
    }, meshTransferables(built.result.meshes))
  } catch (error) {
    const execution = directGeometryExecutionForError(error, planned)
    postTerminal(request, {
      ...eventEnvelope(request),
      status: 'failed',
      kind: 'build',
      execution,
      error: serializeDirectGeometryError(error),
    })
  }
}

async function capabilities(request: DirectGeometryRequest): Promise<void> {
  try {
    const snapshot = await defaultGeometryBuildEngine.capabilities()
    postTerminal(request, {
      ...eventEnvelope(request),
      status: 'succeeded',
      kind: 'capabilities',
      capabilities: snapshot,
    })
  } catch (error) {
    postTerminal(request, {
      ...eventEnvelope(request),
      status: 'failed',
      kind: 'capabilities',
      execution: null,
      error: serializeDirectGeometryError(error),
    })
  }
}

function accept(value: unknown): void {
  if (activeRequest === null) {
    if (!isDirectGeometryRequest(value)) {
      throw new TypeError('Direct geometry worker received an invalid request')
    }
    activeRequest = value
    postStarted(value)
    if (value.type === 'build') void build(value)
    else void capabilities(value)
    return
  }
  if (isDirectGeometryCancel(value, activeRequest)) cancelled = true
}

parentPort.on('message', accept)
