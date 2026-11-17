import { computed, shallowRef, watch, type Ref, type ShallowRef } from 'svet'
import type { DirectBody } from '../../app/directModeling'
import {
  captureEdges,
  captureFaces,
  resolveEdges,
  resolveFaces,
  type AnchorEdge,
  type AnchorFace,
  type SubshapeAnchor,
} from './subshapeSelection'

interface Options {
  selection: Ref<string>
  selectedBody: { readonly value: DirectBody | undefined }
  faceIndex: Ref<number>
  openingFaces: Ref<number[]>
  edgeIndex: Ref<number>
  edgeIndexes: Ref<number[]>
  topology: ShallowRef<{ faces: readonly AnchorFace[] }>
  topologyPending: Ref<boolean>
  isTopologyReadyFor: (body: DirectBody) => boolean
  authoredEdges: ShallowRef<readonly AnchorEdge[]>
}

type FacePart = Pick<SubshapeAnchor, 'body' | 'face' | 'openings'>
type EdgePart = Pick<SubshapeAnchor, 'body' | 'edge' | 'edges'>

/** Keeps the face/edge selection of the selected body as stable identities (C15): picks are
 * captured once the body's faces/edges are known, and every recomputed face/edge list (after
 * Undo/Redo or any edit) re-resolves them, dropping identities that no longer exist. */
export function useSubshapeSelection(o: Options) {
  const faces = shallowRef<FacePart | null>(null),
    edges = shallowRef<EdgePart | null>(null)
  // A pick made while the faces/edges were not ready yet is captured when they arrive instead
  // of being overwritten by the previous identity.
  let facesUncaptured = false,
    edgesUncaptured = false
  // The body whose edges `authoredEdges` holds: assigned only while that body is selected.
  let edgesBody: DirectBody | undefined
  // Indices written by a resolution keep the identity: a face/edge that is missing in this state
  // (a preview, an edit) is selected again when a later state has it, e.g. after Undo.
  let resolving = false
  watch(
    o.authoredEdges,
    list => {
      edgesBody = list.length ? o.selectedBody.value : undefined
    },
    { flush: 'sync', immediate: true },
  )

  const facesReady = () => !!o.selectedBody.value && o.isTopologyReadyFor(o.selectedBody.value)
  const edgesReady = () => !!o.selectedBody.value && edgesBody === o.selectedBody.value
  function captureFacePart() {
    if (resolving) return
    if (o.faceIndex.value < 0 && !o.openingFaces.value.length) {
      faces.value = null
      facesUncaptured = false
      return
    }
    if (!facesReady()) {
      facesUncaptured = true
      return
    }
    faces.value = {
      body: o.selection.value,
      ...captureFaces(o.topology.value.faces, o.faceIndex.value, o.openingFaces.value),
    }
    facesUncaptured = false
  }
  function captureEdgePart() {
    if (resolving) return
    if (o.edgeIndex.value < 0 && !o.edgeIndexes.value.length) {
      edges.value = null
      edgesUncaptured = false
      return
    }
    if (!edgesReady()) {
      edgesUncaptured = true
      return
    }
    edges.value = {
      body: o.selection.value,
      ...captureEdges(o.authoredEdges.value, o.edgeIndex.value, o.edgeIndexes.value),
    }
    edgesUncaptured = false
  }
  watch([o.faceIndex, o.openingFaces], captureFacePart, { flush: 'sync' })
  watch([o.edgeIndex, o.edgeIndexes], captureEdgePart, { flush: 'sync' })

  function resolveFacePart() {
    if (!facesReady()) return
    if (facesUncaptured || !faces.value) {
      captureFacePart()
      return
    }
    if (faces.value.body !== o.selection.value) {
      faces.value = null
      return
    }
    const next = resolveFaces(o.topology.value.faces, faces.value)
    resolving = true
    try {
      if (next.faceIndex !== o.faceIndex.value) o.faceIndex.value = next.faceIndex
      if (next.openingFaces.join() !== o.openingFaces.value.join()) o.openingFaces.value = next.openingFaces
    } finally {
      resolving = false
    }
  }
  function resolveEdgePart() {
    if (!edgesReady()) return
    if (edgesUncaptured || !edges.value) {
      captureEdgePart()
      return
    }
    if (edges.value.body !== o.selection.value) {
      edges.value = null
      return
    }
    const next = resolveEdges(o.authoredEdges.value, edges.value)
    resolving = true
    try {
      if (next.edgeIndex !== o.edgeIndex.value) o.edgeIndex.value = next.edgeIndex
      if (next.edgeIndexes.join() !== o.edgeIndexes.value.join()) o.edgeIndexes.value = next.edgeIndexes
    } finally {
      resolving = false
    }
  }
  // The topology arrives before its request settles; it is usable once nothing is pending.
  watch(() => [o.topology.value, o.topologyPending.value], resolveFacePart, { flush: 'sync' })
  watch(o.authoredEdges, resolveEdgePart, { flush: 'sync' })

  const anchor = computed<SubshapeAnchor | null>(() => {
    const f = faces.value,
      e = edges.value
    if (!f && !e) return null
    return {
      body: (f ?? e)!.body,
      face: f?.face ?? null,
      openings: f?.openings ?? [],
      edge: e?.edge ?? null,
      edges: e?.edges ?? [],
    }
  })

  /** Applies a remembered face/edge selection to the restored state; it is resolved now when the
   * restored body's faces/edges are known, otherwise when they arrive. */
  function restore(saved: SubshapeAnchor | null) {
    const hasFaces = !!saved && (!!saved.face || saved.openings.length > 0)
    const hasEdges = !!saved && (!!saved.edge || saved.edges.length > 0)
    // Indices of the previous state mean nothing here; clear them (which also clears the
    // captured identities) before installing the remembered ones.
    o.faceIndex.value = o.edgeIndex.value = -1
    o.openingFaces.value = []
    o.edgeIndexes.value = []
    faces.value = hasFaces ? { body: saved!.body, face: saved!.face, openings: saved!.openings } : null
    edges.value = hasEdges ? { body: saved!.body, edge: saved!.edge, edges: saved!.edges } : null
    facesUncaptured = edgesUncaptured = false
    resolveFacePart()
    resolveEdgePart()
  }

  return { anchor, restore }
}
