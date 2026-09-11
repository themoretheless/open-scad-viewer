import { sha256Hex } from './sha256'
import type { NativeGeometryArtifact } from './nativeGeometry'

export type TopoKind = 'face' | 'edge' | 'vertex' | 'control-point'
export type LineageKind = 'persist' | 'split' | 'merge'

/** Durable 128-bit topology identity, encoded as 32 lowercase hex digits. */
export type TopoId = string

export interface LineageRecord {
  readonly kind: LineageKind
  readonly parents: readonly TopoId[]
  readonly children: readonly TopoId[]
}

export type SelectionTransfer =
  | { readonly status: 'persistent'; readonly id: TopoId }
  | { readonly status: 'followed'; readonly id: TopoId }
  | { readonly status: 'ambiguous'; readonly ids: readonly TopoId[] }
  | { readonly status: 'lost' }

export interface NativeEdgeReference {
  readonly nodeId: string
  readonly revision: string
  readonly kind: NativeGeometryArtifact['kind']
  readonly edgeId: TopoId
}

export interface NativeControlPointReference {
  readonly nodeId: string
  readonly revision: string
  readonly kind: NativeGeometryArtifact['kind']
  readonly controlPointId: TopoId
}

export interface DurableTopologySnapshot {
  readonly schema: 1
  readonly revision: string
  readonly nodes: Readonly<Record<TopoId, TopoKind>>
  readonly records: readonly LineageRecord[]
  readonly selection: TopoId | null
}

const HEX = /^[0-9a-f]{32}$/

export function isTopoId(value: unknown): value is TopoId {
  return typeof value === 'string' && HEX.test(value)
}

export function topoIdFromParts(high: bigint | number, low: bigint | number): TopoId {
  const hi = BigInt(high)
  const lo = BigInt(low)
  return `${hi.toString(16).padStart(16, '0')}${lo.toString(16).padStart(16, '0')}`
}

function requireId(id: TopoId, label: string): TopoId {
  if (!isTopoId(id)) throw new Error(`Invalid ${label} topology id`)
  return id
}

export class TopologyLineage {
  private readonly kinds = new Map<TopoId, TopoKind>()
  private readonly records: LineageRecord[] = []

  introduce(id: TopoId, kind: TopoKind): TopoId {
    const next = requireId(id, 'introduced')
    this.kinds.set(next, kind)
    return next
  }

  persist(parent: TopoId, child: TopoId): void {
    this.record('persist', [parent], [child])
  }

  split(parent: TopoId, children: readonly TopoId[]): void {
    if (children.length < 2) throw new Error('Split requires at least two children')
    this.record('split', [parent], children)
  }

  merge(parents: readonly TopoId[], child: TopoId): void {
    if (parents.length < 2) throw new Error('Merge requires at least two parents')
    this.record('merge', parents, [child])
  }

  kindOf(id: TopoId): TopoKind | undefined {
    return this.kinds.get(id)
  }

  transfer(selection: TopoId): SelectionTransfer {
    if (!this.kinds.has(selection)) return { status: 'lost' }
    const outgoing = this.records.filter(record => record.parents.includes(selection))
    if (outgoing.length === 0) return { status: 'persistent', id: selection }
    const latest = outgoing[outgoing.length - 1]!
    if (latest.kind === 'split') {
      return latest.children.length === 1
        ? { status: 'followed', id: latest.children[0]! }
        : { status: 'ambiguous', ids: latest.children }
    }
    if (latest.kind === 'merge') {
      return { status: 'followed', id: latest.children[0]! }
    }
    return { status: 'followed', id: latest.children[0]! }
  }

  snapshot(selection: TopoId | null = null): DurableTopologySnapshot {
    if (selection !== null && !this.kinds.has(selection)) {
      throw new Error('Snapshot selection is not in this lineage')
    }
    const nodes = Object.fromEntries(this.kinds)
    const records = this.records.map(record => Object.freeze({
      kind: record.kind,
      parents: Object.freeze([...record.parents]),
      children: Object.freeze([...record.children]),
    }))
    const body = JSON.stringify({ schema: 1, nodes, records, selection })
    return Object.freeze({
      schema: 1 as const,
      revision: sha256Hex(body),
      nodes: Object.freeze(nodes),
      records: Object.freeze(records),
      selection,
    })
  }

  static restore(snapshot: DurableTopologySnapshot): TopologyLineage {
    if (snapshot.schema !== 1) throw new Error('Unsupported topology snapshot schema')
    const lineage = new TopologyLineage()
    for (const [id, kind] of Object.entries(snapshot.nodes)) lineage.introduce(id, kind)
    for (const record of snapshot.records) lineage.record(record.kind, record.parents, record.children)
    return lineage
  }

  private record(kind: LineageKind, parents: readonly TopoId[], children: readonly TopoId[]): void {
    for (const parent of parents) {
      if (!this.kinds.has(requireId(parent, 'parent'))) throw new Error('Unknown lineage parent')
    }
    const childKind = this.kinds.get(parents[0]!)
    if (childKind === undefined) throw new Error('Unknown lineage parent')
    for (const child of children) {
      this.introduce(child, childKind)
    }
    this.records.push(Object.freeze({
      kind,
      parents: Object.freeze([...parents]),
      children: Object.freeze([...children]),
    }))
  }
}

export function nativeEdgeReference(
  artifact: NativeGeometryArtifact,
  edgeId: TopoId,
): NativeEdgeReference {
  if (!isTopoId(edgeId)) throw new Error('Invalid native edge ID')
  return Object.freeze({
    nodeId: artifact.nodeId,
    revision: artifact.revision,
    kind: artifact.kind,
    edgeId,
  })
}

export function nativeControlPointReference(
  artifact: NativeGeometryArtifact,
  controlPointId: TopoId,
): NativeControlPointReference {
  if (!isTopoId(controlPointId)) throw new Error('Invalid native control-point ID')
  return Object.freeze({
    nodeId: artifact.nodeId,
    revision: artifact.revision,
    kind: artifact.kind,
    controlPointId,
  })
}

export function assertNativeEdgeCurrent(
  artifact: NativeGeometryArtifact,
  reference: NativeEdgeReference,
): void {
  if (
    reference.nodeId !== artifact.nodeId
    || reference.kind !== artifact.kind
    || reference.revision !== artifact.revision
  ) throw new Error('Native geometry edge selection is stale')
}

export function assertNativeControlPointCurrent(
  artifact: NativeGeometryArtifact,
  reference: NativeControlPointReference,
): void {
  if (
    reference.nodeId !== artifact.nodeId
    || reference.kind !== artifact.kind
    || reference.revision !== artifact.revision
  ) throw new Error('Native geometry control-point selection is stale')
}
