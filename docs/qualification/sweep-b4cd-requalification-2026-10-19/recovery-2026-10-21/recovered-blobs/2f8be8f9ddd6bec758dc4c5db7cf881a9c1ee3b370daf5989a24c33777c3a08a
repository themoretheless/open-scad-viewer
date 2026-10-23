/** N1 selection-transfer API over durable topology lineage with roles/anchors. */

import {
  TopologyLineage,
  type DurableTopologySnapshot,
  type SelectionTransfer,
  type TopoId,
  type TopoKind,
  type RustChangeSet,
  isTopoId,
} from '../../core/topologyLineage'

export type { SelectionTransfer, TopoId, TopoKind, DurableTopologySnapshot, RustChangeSet }

export type SelectionRole = 'primary' | 'support' | 'derived'
export type SelectionAnchor = {
  readonly id: TopoId
  readonly role: SelectionRole
  readonly parameterHint?: string
}

export interface DurableSelectionTransferSnapshot {
  readonly schema: 1
  readonly topology: DurableTopologySnapshot
  readonly roles: readonly { readonly id: TopoId; readonly role: SelectionRole }[]
  readonly anchors: readonly SelectionAnchor[]
}

/** Rebuild / ambiguity corpus fixture for N1 qualification evidence notes. */
export type RebuildAmbiguityCase = {
  readonly id: string
  readonly kind: 'parameter-rebuild' | 'boolean-split' | 'symmetric-ambiguity' | 'reordered-siblings'
  readonly preSelection: TopoId
  readonly expected: 'unique' | 'ambiguous' | 'lost'
  readonly notes: string
}

export const N1_REBUILD_AMBIGUITY_CORPUS: readonly RebuildAmbiguityCase[] = Object.freeze([
  Object.freeze({
    id: 'N1-01-box-parameter-rebuild',
    kind: 'parameter-rebuild' as const,
    preSelection: 'f:00000000000000000000000000000001',
    expected: 'unique' as const,
    notes: 'Parameter edit keeps face identity via anchor; no nearest-face',
  }),
  Object.freeze({
    id: 'N1-02-boolean-split',
    kind: 'boolean-split' as const,
    preSelection: 'f:00000000000000000000000000000002',
    expected: 'ambiguous' as const,
    notes: 'Split must surface confirmation; never silent remap',
  }),
  Object.freeze({
    id: 'N1-03-symmetric-ambiguity',
    kind: 'symmetric-ambiguity' as const,
    preSelection: 'e:00000000000000000000000000000003',
    expected: 'ambiguous' as const,
    notes: 'Symmetric solids refuse unique transfer without authored role',
  }),
  Object.freeze({
    id: 'N1-04-reordered-siblings',
    kind: 'reordered-siblings' as const,
    preSelection: 'b:00000000000000000000000000000004',
    expected: 'unique' as const,
    notes: 'Sibling reorder preserves durable id; mesh order is not identity',
  }),
])

export function rebuildAmbiguityCase(id: string): RebuildAmbiguityCase | undefined {
  return N1_REBUILD_AMBIGUITY_CORPUS.find(entry => entry.id === id)
}

/**
 * Apply a corpus case against a live transfer service.
 * Ambiguous/lost expectations must not invent a unique face.
 */
export function evaluateRebuildAmbiguity(
  service: SelectionTransferService,
  caseId: string,
): { readonly ok: boolean; readonly transfer: SelectionTransfer; readonly notes: string } {
  const fixture = rebuildAmbiguityCase(caseId)
  if (!fixture) {
    return { ok: false, transfer: { status: 'lost' }, notes: 'unknown corpus case' }
  }
  const transfer = service.transferWithAnchor(fixture.preSelection)
  if (fixture.expected === 'unique') {
    const ok = transfer.status === 'persistent' || transfer.status === 'followed'
    return { ok, transfer, notes: fixture.notes }
  }
  if (fixture.expected === 'lost') {
    return { ok: transfer.status === 'lost', transfer, notes: fixture.notes }
  }
  return {
    ok: transfer.status === 'ambiguous'
      || transfer.status === 'confirmation-required'
      || transfer.status === 'lost',
    transfer,
    notes: fixture.notes,
  }
}

export class SelectionTransferService {
  private lineage = new TopologyLineage()
  private roles = new Map<TopoId, SelectionRole>()
  private anchors = new Map<TopoId, SelectionAnchor>()

  introduce(id: TopoId, kind: TopoKind, role: SelectionRole = 'primary'): TopoId {
    const introduced = this.lineage.introduce(id, kind)
    this.roles.set(introduced, role)
    return introduced
  }

  /** Consume the authoritative Rust change set; no geometry fallback exists. */
  applyRustChangeSet(changeSet: RustChangeSet): void {
    this.lineage.applyChangeSet(changeSet)
    for (const change of changeSet.changes) {
      for (const child of change.children) {
        const role = change.role === 'primary' || change.role === 'support' || change.role === 'derived'
          ? change.role
          : 'derived'
        this.roles.set(child, role)
        if (change.anchor !== null) {
          this.anchors.set(child, Object.freeze({ id: child, role, parameterHint: change.anchor }))
        }
      }
    }
  }

  assignRole(id: TopoId, role: SelectionRole): void {
    if (!isTopoId(id) || this.lineage.kindOf(id) === undefined) {
      throw new Error('Cannot assign a role to an unknown topology ID')
    }
    this.roles.set(id, role)
  }

  roleOf(id: TopoId): SelectionRole | undefined {
    return this.roles.get(id)
  }

  /** Parameter-rebuild correspondence anchor (no nearest-face guessing). */
  bindAnchor(anchor: SelectionAnchor): void {
    if (!isTopoId(anchor.id) || this.lineage.kindOf(anchor.id) === undefined) {
      throw new Error('Cannot anchor an unknown topology ID')
    }
    this.anchors.set(anchor.id, Object.freeze({ ...anchor }))
    this.roles.set(anchor.id, anchor.role)
  }

  anchorOf(id: TopoId): SelectionAnchor | undefined {
    return this.anchors.get(id)
  }

  persist(parent: TopoId, child: TopoId): void {
    this.lineage.persist(parent, child)
    const parentRole = this.roles.get(parent)
    if (parentRole) this.roles.set(child, parentRole)
  }

  split(parent: TopoId, children: readonly TopoId[]): void {
    this.lineage.split(parent, children)
    const parentRole = this.roles.get(parent) ?? 'derived'
    for (const child of children) this.roles.set(child, parentRole)
  }

  merge(parents: readonly TopoId[], child: TopoId): void {
    this.lineage.merge(parents, child)
    this.roles.set(child, 'derived')
  }

  /** Map a pre-rebuild selection without nearest-face guessing. */
  transfer(selection: TopoId): SelectionTransfer {
    if (!isTopoId(selection)) return { status: 'lost' }
    return this.lineage.transfer(selection)
  }

  /**
   * Rebuild correspondence: prefer bound anchors, else lineage transfer.
   * Never guesses nearest face.
   */
  transferWithAnchor(selection: TopoId): SelectionTransfer {
    const anchor = this.anchors.get(selection)
    if (anchor) {
      const next = this.lineage.transfer(anchor.id)
      if (next.status !== 'lost') return next
    }
    return this.transfer(selection)
  }

  snapshot(selection: TopoId | null = null): DurableSelectionTransferSnapshot {
    return Object.freeze({
      schema: 1 as const,
      topology: this.lineage.snapshot(selection),
      roles: Object.freeze([...this.roles].map(([id, role]) => Object.freeze({ id, role }))),
      anchors: Object.freeze([...this.anchors.values()].map(anchor => Object.freeze({ ...anchor }))),
    })
  }

  restore(snapshot: DurableSelectionTransferSnapshot | DurableTopologySnapshot): void {
    const wrapped = 'topology' in snapshot
    this.lineage = TopologyLineage.restore(wrapped ? snapshot.topology : snapshot)
    this.roles.clear()
    this.anchors.clear()
    if (wrapped) {
      for (const { id, role } of snapshot.roles) this.assignRole(id, role)
      for (const anchor of snapshot.anchors) this.bindAnchor(anchor)
    }
  }
}

export function createSelectionTransferService(): SelectionTransferService {
  return new SelectionTransferService()
}
