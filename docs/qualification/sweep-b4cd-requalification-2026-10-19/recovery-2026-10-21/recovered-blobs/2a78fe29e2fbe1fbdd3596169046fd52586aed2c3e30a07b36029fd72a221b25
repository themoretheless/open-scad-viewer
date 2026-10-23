/** Cancel, lease, and LKG-stale drills for the shared geometry WASM host. */

export class GeometryLeaseError extends Error {
  constructor(message: string) {
    super(message)
    this.name = 'GeometryLeaseError'
  }
}

export interface GeometryLease {
  readonly id: string
  readonly epoch: number
  readonly createdAt: number
}

export interface LastKnownGoodSnapshot<T> {
  readonly epoch: number
  readonly value: T
  readonly stale: boolean
}

let epoch = 1
const live = new Map<string, GeometryLease>()
let cancelled = false

export function geometryKernelEpoch(): number {
  return epoch
}

export function bumpGeometryKernelEpoch(): number {
  epoch += 1
  live.clear()
  return epoch
}

export function issueGeometryLease(prefix = 'lease'): GeometryLease {
  if (cancelled) throw new GeometryLeaseError('Cannot issue a lease while the geometry kernel is cancelled')
  const lease: GeometryLease = Object.freeze({
    id: `${prefix}:${epoch}:${live.size + 1}`,
    epoch,
    createdAt: Date.now(),
  })
  live.set(lease.id, lease)
  return lease
}

export function releaseGeometryLease(lease: GeometryLease): void {
  const current = live.get(lease.id)
  if (!current) return
  if (current.epoch !== epoch) {
    throw new GeometryLeaseError('Refusing to release a stale geometry lease from a prior epoch')
  }
  live.delete(lease.id)
}

export function assertGeometryLeaseCurrent(lease: GeometryLease): void {
  if (cancelled) throw new GeometryLeaseError('Geometry kernel cancelled')
  const current = live.get(lease.id)
  if (!current || current.epoch !== epoch || current.epoch !== lease.epoch) {
    throw new GeometryLeaseError('Geometry lease is stale or unknown')
  }
}

/** Hard-cancel: subsequent leases refuse until reset; existing LKG marked stale. */
export function cancelGeometryKernel(): void {
  cancelled = true
  live.clear()
}

export function resetGeometryKernelCancel(): void {
  cancelled = false
  bumpGeometryKernelEpoch()
}

export function isGeometryKernelCancelled(): boolean {
  return cancelled
}

export function publishLastKnownGood<T>(value: T): LastKnownGoodSnapshot<T> {
  return Object.freeze({
    epoch,
    value,
    stale: cancelled,
  })
}

export function readLastKnownGood<T>(snapshot: LastKnownGoodSnapshot<T>): T {
  if (snapshot.stale || snapshot.epoch !== epoch || cancelled) {
    throw new GeometryLeaseError('Last-known-good geometry snapshot is stale; refuse silent reuse')
  }
  return snapshot.value
}
