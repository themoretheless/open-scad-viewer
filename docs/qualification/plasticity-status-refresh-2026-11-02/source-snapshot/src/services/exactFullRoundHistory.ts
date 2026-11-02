/** Transport only: feature construction, history and proof credits belong to Rust. */
import { deepFreeze } from '../core/objects'
import { callGeometryRust } from './geometry/kernel'
import type { NurbsBrep } from './geometry/brep'
export interface ExactHistoryBudget {
  readonly maxRevisions?: number
  readonly maxBytes?: number
  readonly material?: Readonly<Partial<Record<'maxPairProjections'|'maxAtoms'|'maxRayCells'|'refinements'|'radiusHalvings',number>>>
}
export interface ExactFullRoundHistoryDocument {
  readonly format: 'exact-full-round-history/1'
  readonly cursor: number
  readonly revisions: readonly Readonly<{ edit:'root'|'rebase-source'|'reselect'; artifact:Readonly<Record<string,unknown>> }>[]
}
export interface ExactFullRoundHistoryState {
  readonly document: ExactFullRoundHistoryDocument
  readonly cursor: number
  readonly revisionCount: number
  readonly selectedSourceFaceIds: readonly [string,string,string]
  readonly work: Readonly<{pairProjections:number;atoms:number;rayCells:number}>
}
export type ExactFullRoundHistoryEdit =
  | Readonly<{action:'restore'|'undo'|'redo'|'branch'}>
  | Readonly<{action:'rebase-source';source:NurbsBrep}>
  | Readonly<{action:'reselect';faceIds:readonly [string,string,string]}>
  | Readonly<{action:'regenerate-selection';revision:number;faceIds:readonly [string,string,string]}>
export function createExactFullRoundHistory(artifact:Readonly<Record<string,unknown>>,budget?:ExactHistoryBudget):ExactFullRoundHistoryState {
  return deepFreeze(callGeometryRust('cad_full_round_exact_history',{action:'create',artifact,...(budget?{budget}:{})}))
}
/** Stored documents are untrusted until every revision has been replayed by Rust. */
export function restoreExactFullRoundHistory(document:unknown,budget?:ExactHistoryBudget):ExactFullRoundHistoryState {
  return deepFreeze(callGeometryRust('cad_full_round_exact_history',{action:'restore',history:document,...(budget?{budget}:{})}))
}
export function editExactFullRoundHistory(state:ExactFullRoundHistoryState,edit:ExactFullRoundHistoryEdit,budget?:ExactHistoryBudget):ExactFullRoundHistoryState {
  return deepFreeze(callGeometryRust('cad_full_round_exact_history',{...edit,history:state.document,op:'cad_full_round_exact_history',...(budget?{budget}:{})}))
}
export function currentExactFullRoundArtifact(state:ExactFullRoundHistoryState):Readonly<Record<string,unknown>> {
  const row=state.document.revisions[state.document.cursor]
  if(!row)throw new Error('Invalid exact history cursor')
  return row.artifact
}
