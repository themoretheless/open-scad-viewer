import type { GeometryBuildError, GeometryWorkerEvent } from '../geometryWorkerProtocol'
import type { GeometryBuildInput } from '../buildCoordinator'
import type { GeometryQuality } from '../../core/build'

export function sourceSpansFit(source: string, event: GeometryWorkerEvent): boolean {
  const isBoundary = (offset: number) => offset === 0
    || offset === source.length
    || !(source.charCodeAt(offset - 1) >= 0xd800
      && source.charCodeAt(offset - 1) <= 0xdbff
      && source.charCodeAt(offset) >= 0xdc00
      && source.charCodeAt(offset) <= 0xdfff)
  const spanFits = (start: number, end: number) => start <= end
    && end <= source.length
    && isBoundary(start)
    && isBoundary(end)
  if (event.status === 'failed') {
    const { start, end } = event.error
    return (start === undefined && end === undefined)
      || (start !== undefined && end !== undefined && spanFits(start, end))
  }
  if (event.status !== 'succeeded') return true
  return event.meshes.every(mesh => mesh.provenance.every(run => (
    run.source === null || spanFits(run.source.start, run.source.end)
  )))
}

export function deepFreezeWorkerSnapshot(value: unknown, seen = new WeakSet<object>()): void {
  if (value === null || typeof value !== 'object' || seen.has(value)) return
  seen.add(value)
  if (ArrayBuffer.isView(value)) {
    Object.freeze(value.buffer)
    return
  }
  for (const key of Reflect.ownKeys(value)) {
    const descriptor = Object.getOwnPropertyDescriptor(value, key)
    if (descriptor && Object.hasOwn(descriptor, 'value')) deepFreezeWorkerSnapshot(descriptor.value, seen)
  }
  Object.freeze(value)
}

export function collectWorkerEventBuffers(value: unknown, buffers: Set<ArrayBuffer>, seen = new WeakSet<object>()): void {
  if (value === null || typeof value !== 'object' || seen.has(value)) return
  seen.add(value)
  if (ArrayBuffer.isView(value)) {
    const { buffer } = value
    // SharedArrayBuffer is not transferable; it structured-clones by reference.
    if (buffer instanceof ArrayBuffer) buffers.add(buffer)
    return
  }
  for (const key of Reflect.ownKeys(value)) {
    const descriptor = Object.getOwnPropertyDescriptor(value, key)
    if (descriptor && Object.hasOwn(descriptor, 'value')) collectWorkerEventBuffers(descriptor.value, buffers, seen)
  }
}

export function freezeAttestedWorkerEvent(event: GeometryWorkerEvent, snapshot: boolean): GeometryWorkerEvent {
  if (!snapshot) return event
  // Same-realm doubles share object identity with the host, so the snapshot
  // must gain exclusive ownership of the attested mesh buffers. Move them
  // instead of copying: `transfer` detaches the sender's buffers and hands the
  // snapshot fresh ArrayBuffer identities over the same memory — exactly what
  // a real Worker boundary does — instead of duplicating megabyte payloads.
  const buffers = new Set<ArrayBuffer>()
  collectWorkerEventBuffers(event, buffers)
  const cloned = structuredClone(event, { transfer: [...buffers] }) as GeometryWorkerEvent
  deepFreezeWorkerSnapshot(cloned)
  return cloned
}

export function qualityRank(quality: GeometryQuality | null): number {
  if (quality === 'full') return 2
  if (quality === 'preview') return 1
  return 0
}

export function assertInput(input: GeometryBuildInput) {
  if (!Number.isSafeInteger(input.documentRevision) || input.documentRevision < 0) {
    throw new RangeError('documentRevision must be a non-negative safe integer')
  }
  if (input.quality !== 'preview' && input.quality !== 'full') {
    throw new TypeError('quality must be preview or full')
  }
  if (typeof input.source !== 'string') throw new TypeError('source must be a string')
}

export function workerError(message: string, cause?: unknown): GeometryBuildError {
  if (cause instanceof Error) return { name: cause.name || 'WorkerError', message: cause.message || message }
  return { name: 'WorkerError', message }
}
