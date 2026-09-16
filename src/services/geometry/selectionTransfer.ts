/** N1 selection-transfer API over durable topology lineage with roles/anchors. */

import {
  TopologyLineage,
  type DurableTopologySnapshot,
  type SelectionTransfer,
  type TopoId,
  type TopoKind,
  isTopoId,
} from '../../core/topologyLineage'

export type { SelectionTransfer, TopoId, TopoKind, DurableTopologySnapshot }

export type SelectionRole = 'primary' | 'support' | 'derived'
export type SelectionAnchor = {
  readonly id: TopoId
  readonly role: SelectionRole
  readonly parameterHint?: string
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
    preSelection: 'f:box-top' as TopoId,
    expected: 'unique' as const,
    notes: 'Parameter edit keeps face identity via anchor; no nearest-face',
  }),
  Object.freeze({
    id: 'N1-02-boolean-split',
    kind: 'boolean-split' as const,
    preSelection: 'f:shared-wall' as TopoId,
    expected: 'ambiguous' as const,
    notes: 'Split must surface confirmation; never silent remap',
  }),
  Object.freeze({
    id: 'N1-03-symmetric-ambiguity',
    kind: 'symmetric-ambiguity' as const,
    preSelection: 'e:mid-symmetry' as TopoId,
    expected: 'ambiguous' as const,
    notes: 'Symmetric solids refuse unique transfer without authored role',
  }),
  Object.freeze({
    id: 'N1-04-reordered-siblings',
    kind: 'reordered-siblings' as const,
    preSelection: 'b:body-0' as TopoId,
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
    ok: transfer.status === 'ambiguous' || transfer.status === 'lost',
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

  assignRole(id: TopoId, role: SelectionRole): void {
    if (!isTopoId(id)) return
    this.roles.set(id, role)
  }

  roleOf(id: TopoId): SelectionRole | undefined {
    return this.roles.get(id)
  }

  /** Parameter-rebuild correspondence anchor (no nearest-face guessing). */
  bindAnchor(anchor: SelectionAnchor): void {
    if (!isTopoId(anchor.id)) return
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

  snapshot(selection: TopoId | null = null): DurableTopologySnapshot {
    return this.lineage.snapshot(selection)
  }

  restore(snapshot: DurableTopologySnapshot): void {
    this.lineage = TopologyLineage.restore(snapshot)
    this.roles.clear()
    this.anchors.clear()
  }
}

export function createSelectionTransferService(): SelectionTransferService {
  return new SelectionTransferService()
}
