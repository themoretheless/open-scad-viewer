import {callGeometryRust} from './geometry/kernel'
import type { SculptBrush } from './geometryEditing'
import { booleanPolygonMeshes, brushPolygonMesh, sculptPolygonMesh, deformPolygonMesh, extrudePolygonProfile, exportPolygonStl, inspectPolygonMesh, normalizePolygonMesh, type PolygonMesh } from './geometry/polygon'
import { stringifyMeshJson } from './meshJson'
import { parseBinaryStl } from './stlImport'
import { parseMeshDocument } from './meshEditing/parseMeshDocument'
export { parseMeshDocument, stlBufferToPolygonMesh, createBoxMesh, createUvSphereMesh, transformMesh, moveVertices, moveVerticesProportional, deleteFaces, flipFaces, subdivideFaces, extrudeSelectedFaces, insetSelectedFaces, mergeByDistance, booleanMeshObjects, duplicateObject, meshObjectStats, exportMeshObjectStl, buildEdgeList, knifeSplitEdges, separateFaces, joinMeshes } from './meshEditing/parseMeshDocument'
export { symmetrizeMesh, brushDisplace, sculptMesh, meshCentroid, twistMesh } from './meshEditing/symmetrizeMesh'

export type MeshSelectMode = 'object' | 'vertex' | 'edge' | 'face'
export type MeshEditTool = 'select' | 'grab' | 'rotate' | 'scale' | 'extrude' | 'inset' | 'knife'

export interface MeshObject {
  id: string
  name: string
  mesh: PolygonMesh
  visible: boolean
}

export interface MeshWorkspaceDocument {
  version: 1
  objects: MeshObject[]
}

export const emptyMeshDocument = (): MeshWorkspaceDocument => ({ version: 1, objects: [] })

export const clone = <T>(value: T): T => structuredClone(value)
export const finite = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v) && Math.abs(v) <= 1e6

interface MeshSnapshot { document: MeshWorkspaceDocument; characters: number; text: string }
export class MeshHistory {
  private past: MeshSnapshot[] = []
  private future: MeshSnapshot[] = []
  private current: MeshSnapshot
  constructor(document = emptyMeshDocument()) {
    const validated = parseMeshDocument(stringifyMeshJson(document))
    const validatedText = stringifyMeshJson(validated)
    this.current = {document: validated, characters: validatedText.length, text: validatedText}
  }
  get document() { return clone(this.current.document) }
  get canUndo() { return this.past.length > 0 }
  get canRedo() { return this.future.length > 0 }
  commit(document: MeshWorkspaceDocument) {
    const next = parseMeshDocument(stringifyMeshJson(document))
    const nextText = stringifyMeshJson(next)
    if (nextText.length === this.current.characters && nextText === this.current.text) return
    this.past.push(this.current)
    // Preserve the exact serialized-array character budget, including brackets
    // and commas, without serializing retained geometry on every commit.
    let retained = this.past.reduce((sum, snapshot) => sum + snapshot.characters, 0) + this.past.length + 1
    while (this.past.length > 80 || (this.past.length > 1 && retained > 24_000_000)) {
      retained -= this.past.shift()!.characters + 1
    }
    this.current = {document: next, characters: nextText.length, text: nextText}
    this.future = []
  }
  undo() {
    const d = this.past.pop()
    if (d) { this.future.push(this.current); this.current = d }
    return this.document
  }
  redo() {
    const d = this.future.pop()
    if (d) { this.past.push(this.current); this.current = d }
    return this.document
  }
}
