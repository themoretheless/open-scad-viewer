/** N1 selection-transfer API over durable topology lineage (post Phase D identity). */

import {
  TopologyLineage,
  type DurableTopologySnapshot,
  type SelectionTransfer,
  type TopoId,
  type TopoKind,
  isTopoId,
} from '../../core/topologyLineage'

export type { SelectionTransfer, TopoId, TopoKind, DurableTopologySnapshot }

export class SelectionTransferService {
  private lineage = new TopologyLineage()

  introduce(id: TopoId, kind: TopoKind): TopoId {
    return this.lineage.introduce(id, kind)
  }

  persist(parent: TopoId, child: TopoId): void {
    this.lineage.persist(parent, child)
  }

  split(parent: TopoId, children: readonly TopoId[]): void {
    this.lineage.split(parent, children)
  }

  merge(parents: readonly TopoId[], child: TopoId): void {
    this.lineage.merge(parents, child)
  }

  /** Map a pre-rebuild selection without nearest-face guessing. */
  transfer(selection: TopoId): SelectionTransfer {
    if (!isTopoId(selection)) return { status: 'lost' }
    return this.lineage.transfer(selection)
  }

  snapshot(selection: TopoId | null = null): DurableTopologySnapshot {
    return this.lineage.snapshot(selection)
  }

  restore(snapshot: DurableTopologySnapshot): void {
    this.lineage = TopologyLineage.restore(snapshot)
  }
}

export function createSelectionTransferService(): SelectionTransferService {
  return new SelectionTransferService()
}
