import { sha256Hex } from './sha256'
import type { NativeGeometryArtifact } from './nativeGeometry'

export type TopoKind = 'vertex' | 'edge' | 'loop' | 'face' | 'shell' | 'body' | 'control-point'
export type ChangeKind = 'persisted' | 'generated' | 'modified' | 'split' | 'merge' | 'deleted'
export type LineageKind = 'persist' | 'split' | 'merge'
export type TopoId = string

export interface ChangeProvenance {
  readonly operation: string
  readonly operand: string | null
  readonly occurrence: string
}

export interface TopologyChange {
  readonly kind: ChangeKind
  readonly topoKind: TopoKind
  readonly parents: readonly TopoId[]
  readonly children: readonly TopoId[]
  readonly provenance: ChangeProvenance
  readonly role: string
  readonly anchor: string | null
}

export interface RustChangeSet {
  readonly schema: 1
  readonly nodes: readonly { readonly id: TopoId; readonly kind: TopoKind }[]
  readonly changes: readonly TopologyChange[]
}

export interface LineageRecord {
  readonly kind: LineageKind
  readonly parents: readonly TopoId[]
  readonly children: readonly TopoId[]
}

export type SelectionTransfer =
  | { readonly status: 'persistent'; readonly id: TopoId }
  | { readonly status: 'followed'; readonly id: TopoId }
  | { readonly status: 'confirmation-required'; readonly ids: readonly TopoId[] }
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
  readonly schema: 2
  readonly revision: string
  readonly nodes: Readonly<Record<TopoId, TopoKind>>
  readonly changes: readonly TopologyChange[]
  readonly selection: TopoId | null
}

const PREFIX: Readonly<Record<TopoKind, string>> = Object.freeze({
  vertex: 'v',
  edge: 'e',
  loop: 'l',
  face: 'f',
  shell: 's',
  body: 'b',
  'control-point': 'cp',
})
const ID = /^(v|e|l|f|s|b|cp):[0-9a-f]{32}$/
const CHANGE_KINDS = new Set<ChangeKind>([
  'persisted', 'generated', 'modified', 'split', 'merge', 'deleted',
])

export function isTopoId(value: unknown): value is TopoId {
  return typeof value === 'string' && ID.test(value)
}

export function topoIdFromParts(
  high: bigint | number,
  low: bigint | number,
  kind: TopoKind = 'face',
): TopoId {
  const hi = BigInt(high)
  const lo = BigInt(low)
  if (hi < 0n || hi > 0xffffffffffffffffn || lo < 0n || lo > 0xffffffffffffffffn) {
    throw new Error('Topology ID parts must be unsigned 64-bit integers')
  }
  return `${PREFIX[kind]}:${hi.toString(16).padStart(16, '0')}${lo.toString(16).padStart(16, '0')}`
}

function requireId(id: TopoId, label: string, kind?: TopoKind): TopoId {
  if (!isTopoId(id)) throw new Error(`Invalid ${label} topology id`)
  if (kind && !id.startsWith(`${PREFIX[kind]}:`)) {
    throw new Error(`${label} topology ID prefix does not match ${kind}`)
  }
  return id
}

function exactKeys(value: object, keys: readonly string[], label: string): void {
  const actual = Object.keys(value).sort()
  const expected = [...keys].sort()
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    throw new Error(`${label} fields do not match the schema`)
  }
}

function freezeChange(change: TopologyChange): TopologyChange {
  return Object.freeze({
    ...change,
    parents: Object.freeze([...change.parents]),
    children: Object.freeze([...change.children]),
    provenance: Object.freeze({ ...change.provenance }),
  })
}

function validateChange(
  change: TopologyChange,
  kinds: ReadonlyMap<TopoId, TopoKind>,
): void {
  exactKeys(change, ['kind', 'topoKind', 'parents', 'children', 'provenance', 'role', 'anchor'], 'Change')
  exactKeys(change.provenance, ['operation', 'operand', 'occurrence'], 'Change provenance')
  if (!CHANGE_KINDS.has(change.kind)) throw new Error('Unknown topology change kind')
  if (!(change.topoKind in PREFIX)) throw new Error('Unknown topology kind')
  if (!change.provenance.operation || !change.provenance.occurrence || !change.role) {
    throw new Error('Topology change provenance and role must be non-empty')
  }
  if (change.provenance.operand !== null && typeof change.provenance.operand !== 'string') {
    throw new Error('Invalid topology change operand')
  }
  if (change.anchor !== null && typeof change.anchor !== 'string') {
    throw new Error('Invalid topology anchor')
  }
  const parents = new Set(change.parents)
  const children = new Set(change.children)
  if (parents.size !== change.parents.length || children.size !== change.children.length) {
    throw new Error('Topology change endpoints must be unique')
  }
  const cardinality = (
    ((change.kind === 'persisted' || change.kind === 'modified')
      && change.parents.length === 1 && change.children.length === 1)
    || (change.kind === 'generated' && change.parents.length === 0 && change.children.length === 1)
    || (change.kind === 'split' && change.parents.length === 1 && change.children.length >= 2)
    || (change.kind === 'merge' && change.parents.length >= 2 && change.children.length === 1)
    || (change.kind === 'deleted' && change.parents.length === 1 && change.children.length === 0)
  )
  if (!cardinality) throw new Error('Topology change cardinality does not match its kind')
  const identityOkay = (
    (change.kind === 'persisted' && change.parents[0] === change.children[0])
    || (change.kind === 'modified' && change.parents[0] !== change.children[0])
    // One split branch or merge operand may legitimately retain its ID.
    // Cardinality remains strict and self-edges do not participate in the DAG.
    || change.kind === 'split'
    || change.kind === 'merge'
    || change.kind === 'generated'
    || change.kind === 'deleted'
  )
  if (!identityOkay) throw new Error('Topology change identity overlap does not match its kind')
  for (const id of [...change.parents, ...change.children]) {
    requireId(id, 'change endpoint', change.topoKind)
    if (kinds.get(id) !== change.topoKind) throw new Error('Unknown or mistyped topology change endpoint')
  }
}

function assertAcyclic(nodes: Iterable<TopoId>, changes: readonly TopologyChange[]): void {
  const edges = new Map<TopoId, Set<TopoId>>()
  for (const change of changes) {
    for (const parent of change.parents) for (const child of change.children) {
      if (parent === child) continue
      const outgoing = edges.get(parent) ?? new Set<TopoId>()
      outgoing.add(child)
      edges.set(parent, outgoing)
    }
  }
  const visiting = new Set<TopoId>()
  const visited = new Set<TopoId>()
  const visit = (id: TopoId): void => {
    if (visited.has(id)) return
    if (visiting.has(id)) throw new Error('Topology lineage must be acyclic')
    visiting.add(id)
    for (const child of edges.get(id) ?? []) visit(child)
    visiting.delete(id)
    visited.add(id)
  }
  for (const id of nodes) visit(id)
}

export class TopologyLineage {
  private readonly kinds = new Map<TopoId, TopoKind>()
  private readonly changes: TopologyChange[] = []

  introduce(id: TopoId, kind: TopoKind): TopoId {
    const next = requireId(id, 'introduced', kind)
    const existing = this.kinds.get(next)
    if (existing !== undefined && existing !== kind) throw new Error('Duplicate topology ID with conflicting kind')
    this.kinds.set(next, kind)
    return next
  }

  applyChangeSet(changeSet: RustChangeSet): void {
    exactKeys(changeSet, ['schema', 'nodes', 'changes'], 'Change set')
    if (changeSet.schema !== 1) throw new Error('Unsupported Rust change-set schema')
    const combinedKinds = new Map(this.kinds)
    const incoming = new Set<TopoId>()
    for (const node of changeSet.nodes) {
      exactKeys(node, ['id', 'kind'], 'Topology node')
      if (incoming.has(node.id)) throw new Error('Duplicate topology node ID')
      incoming.add(node.id)
      requireId(node.id, 'change-set node', node.kind)
      const existing = combinedKinds.get(node.id)
      if (existing !== undefined && existing !== node.kind) {
        throw new Error('Duplicate topology ID with conflicting kind')
      }
      combinedKinds.set(node.id, node.kind)
    }
    for (const change of changeSet.changes) {
      validateChange(change, combinedKinds)
    }
    const combinedChanges = [...this.changes, ...changeSet.changes.map(freezeChange)]
    assertAcyclic(combinedKinds.keys(), combinedChanges)
    for (const [id, kind] of combinedKinds) this.kinds.set(id, kind)
    this.changes.push(...changeSet.changes.map(freezeChange))
  }

  persist(parent: TopoId, child: TopoId): void {
    this.compatibilityRecord(parent === child ? 'persisted' : 'modified', [parent], [child])
  }

  split(parent: TopoId, children: readonly TopoId[]): void {
    this.compatibilityRecord('split', [parent], children)
  }

  merge(parents: readonly TopoId[], child: TopoId): void {
    this.compatibilityRecord('merge', parents, [child])
  }

  kindOf(id: TopoId): TopoKind | undefined {
    return this.kinds.get(id)
  }

  transfer(selection: TopoId): SelectionTransfer {
    if (!this.kinds.has(selection)) return { status: 'lost' }
    const terminals = new Set<TopoId>()
    let confirmation = false
    let moved = false
    const visit = (id: TopoId, path: Set<TopoId>): void => {
      if (path.has(id)) throw new Error('Topology lineage must be acyclic')
      const outgoing = this.changes.filter(change => change.parents.includes(id))
      if (outgoing.length === 0) {
        terminals.add(id)
        return
      }
      if (outgoing.length > 1) confirmation = true
      const nextPath = new Set(path).add(id)
      for (const change of outgoing) {
        if (change.kind === 'deleted') continue
        if (change.kind === 'split' || change.kind === 'merge') confirmation = true
        for (const child of change.children) {
          if (child === id) terminals.add(child)
          else {
            moved = true
            visit(child, nextPath)
          }
        }
      }
    }
    visit(selection, new Set())
    const ids = [...terminals].sort()
    if (ids.length === 0) return { status: 'lost' }
    if (confirmation) return { status: 'confirmation-required', ids }
    if (ids.length > 1) return { status: 'ambiguous', ids }
    return moved ? { status: 'followed', id: ids[0]! } : { status: 'persistent', id: ids[0]! }
  }

  snapshot(selection: TopoId | null = null): DurableTopologySnapshot {
    if (selection !== null && !this.kinds.has(selection)) {
      throw new Error('Snapshot selection is not in this lineage')
    }
    const nodes = Object.freeze(Object.fromEntries(this.kinds))
    const changes = Object.freeze(this.changes.map(freezeChange))
    const body = JSON.stringify({ schema: 2, nodes, changes, selection })
    return Object.freeze({
      schema: 2 as const,
      revision: sha256Hex(body),
      nodes,
      changes,
      selection,
    })
  }

  static restore(snapshot: DurableTopologySnapshot): TopologyLineage {
    exactKeys(snapshot, ['schema', 'revision', 'nodes', 'changes', 'selection'], 'Topology snapshot')
    if (snapshot.schema !== 2) throw new Error('Unsupported topology snapshot schema')
    const lineage = new TopologyLineage()
    for (const [id, kind] of Object.entries(snapshot.nodes)) lineage.introduce(id, kind)
    for (const change of snapshot.changes) {
      validateChange(change, lineage.kinds)
      lineage.changes.push(freezeChange(change))
    }
    assertAcyclic(lineage.kinds.keys(), lineage.changes)
    if (snapshot.selection !== null
      && (!isTopoId(snapshot.selection) || !lineage.kinds.has(snapshot.selection))) {
      throw new Error('Topology snapshot selection is invalid')
    }
    const body = JSON.stringify({
      schema: 2,
      nodes: snapshot.nodes,
      changes: snapshot.changes,
      selection: snapshot.selection,
    })
    if (sha256Hex(body) !== snapshot.revision) throw new Error('Topology snapshot revision mismatch')
    return lineage
  }

  private compatibilityRecord(
    kind: 'persisted' | 'modified' | 'split' | 'merge',
    parents: readonly TopoId[],
    children: readonly TopoId[],
  ): void {
    const topoKind = this.kinds.get(parents[0]!)
    if (!topoKind) throw new Error('Unknown lineage parent')
    for (const parent of parents) requireId(parent, 'parent', topoKind)
    for (const child of children) {
      if (!this.kinds.has(child)) this.introduce(child, topoKind)
    }
    const change: TopologyChange = {
      kind,
      topoKind,
      parents,
      children,
      provenance: { operation: 'compatibility-api', operand: null, occurrence: parents[0]! },
      role: topoKind,
      anchor: null,
    }
    validateChange(change, this.kinds)
    this.changes.push(freezeChange(change))
    assertAcyclic(this.kinds.keys(), this.changes)
  }
}

export function nativeEdgeReference(
  artifact: NativeGeometryArtifact,
  edgeId: TopoId,
): NativeEdgeReference {
  requireId(edgeId, 'native edge', 'edge')
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
  requireId(controlPointId, 'native control-point', 'control-point')
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
