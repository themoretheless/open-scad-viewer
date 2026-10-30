import {createMeshCommands} from '.././useMeshModeler/meshCommands'
import VrControls from '../../../components/VrControls.vue'
import { prepareVrPolygons } from '../../../services/vrScene'
import { computed, nextTick, onUnmounted, ref, shallowRef, watch } from 'vue'
import { stringifyMeshJson } from '../../../services/meshJson'
import ModelingFloorGrid from '../../../components/ModelingFloorGrid.vue'
import ModelingGridControls from '../../../components/ModelingGridControls.vue'
import WorkspaceActionBar from '../../../components/WorkspaceActionBar.vue'
import CommandPalette from '../../../components/CommandPalette.vue'
import type { PaletteCommand } from '../../../services/commandSearch'
import { SCULPT_FALLOFFS, SCULPT_KINDS, buildSculptBrush, isFractionalSculptKind, type SculptFalloff, type SculptKind } from '../../../services/geometryEditing'
import { meshCentroid, sculptMesh } from '../../../services/meshEditing'
import {
  MeshHistory,
  buildEdgeList,
  createBoxMesh,
  createUvSphereMesh,
  deleteFaces,
  duplicateObject,
  emptyMeshDocument,
  exportMeshObjectStl,
  extrudeSelectedFaces,
  flipFaces,
  insetSelectedFaces,
  joinMeshes,
  knifeSplitEdges,
  mergeByDistance,
  meshObjectStats,
  moveVertices,
  moveVerticesProportional,
  parseMeshDocument,
  separateFaces,
  subdivideFaces,
  symmetrizeMesh,
  transformMesh,
  twistMesh,
  type MeshSelectMode,
  type MeshWorkspaceDocument,
} from '../../../services/meshEditing'
import { createSolidPreviewWorker } from '../../../services/solidPreviewWorker'
import { useMainSolidTask } from '../.././useMainSolidTask'
import { defaultDirectCamera, projectDirectPoint } from '../../../services/directModelingTools'
import { projectMeshModelerObject,cachedMeshModelerStats } from '../../../services/meshModelerProjection'
import { isGeometryKernelReady, warmGeometryKernel } from '../../../services/geometry/kernel'
import { storageGet, storageSet } from '../../../services/safeStorage'
import { MESH_IMPORT_ACCEPT, stripMeshExtension } from '../../../services/meshImport'
import { manifoldPreferenceSnapshot } from '../../../services/manifoldPreferences'
import ManifoldSettings from '../../../components/ManifoldSettings.vue'
import { formatMeshImportNotice } from '../../../services/meshImportReport'
import { formatMeshBooleanNotice } from '../../../services/meshBooleanReport'
import { formatMeshRepairNotice } from '../../../services/meshRepairReport'
import { meshManifoldDefectLabels } from '../../../services/meshManifoldDefects'
import { MESH_EXPORT_FORMATS, MESH_FORMAT_LABELS, type MeshExportFormat } from '../../../services/meshConvert'
import { downloadBytes } from '../../../services/downloadArtifact'
import type { PolygonMesh } from '../../../services/geometry/polygon'
import type { KernelManifoldReport } from '../../../services/geometry/manifoldAnalysis'
import type {MeshModelerProps,MeshModelerEmit} from '../useMeshModeler'
import type {MeshInteractionContext} from './meshInteractionContext'
export function createMeshInteraction(context: MeshInteractionContext) {
function downloadJson() {
  const blob = new Blob([stringifyMeshJson(context.document.value)], { type: 'application/json' })
  const url = URL.createObjectURL(blob)
  const a = window.document.createElement('a')
  a.href = url
  a.download = 'mesh-workspace.json'
  a.click()
  URL.revokeObjectURL(url)
}
function togglePick(kind: 'vertex' | 'face' | 'edge', id: number) {
  const list = kind === 'vertex' ? context.selectedVerts : kind === 'face' ? context.selectedFaces : context.selectedEdges
  const set = new Set(list.value)
  set.has(id) ? set.delete(id) : set.add(id)
  list.value = [...set]
}
function getEdgeList(mesh: PolygonMesh): Array<[number, number]> {
  let edges = context.edgeListCache.get(mesh)
  if (!edges) {
    edges = buildEdgeList(mesh)
    context.edgeListCache.set(mesh, edges)
  }
  return edges
}
function projected(object: MeshWorkspaceDocument['objects'][number]) {
  return projectMeshModelerObject(object,context.camera.value,context.selectMode.value,getEdgeList,context.manifoldReports.value.get(object.id))
}
function screenDeltaToWorld(dxPx: number, dyPx: number, svg: SVGSVGElement): [number, number, number] {
  const unit = 200 / Math.max(1, Math.min(svg.clientWidth, svg.clientHeight)) / (context.view.value / 100)
  const sx = dxPx * unit, sy = -dyPx * unit
  // Inverse of projectDirectPoint's rotation applied to the screen-plane vector (sx, sy, 0).
  const cam = context.camera.value, cy = Math.cos(cam.yaw), sy0 = Math.sin(cam.yaw), cp = Math.cos(cam.pitch), sp = Math.sin(cam.pitch)
  return [sx * cy + sy * sy0 * sp, -sx * sy0 + sy * cy * sp, -sy * cp]
}
function startElementDrag(event: PointerEvent, objectId: string, kind: 'vertex' | 'edge' | 'face', id: number) {
  if (event.button !== 0 || event.shiftKey || event.altKey || context.selectMode.value === 'object') { onPointerDown(event); return }
  if (context.selectMode.value !== kind) { onPointerDown(event); return }
  event.stopPropagation()
  const object = context.document.value.objects.find(o => o.id === objectId)
  if (!object) return
  if (context.selection.value !== objectId) { context.selection.value = objectId; context.selectedVerts.value = []; context.selectedFaces.value = []; context.selectedEdges.value = [] }
  const list = kind === 'vertex' ? context.selectedVerts : kind === 'face' ? context.selectedFaces : context.selectedEdges
  if (!list.value.includes(id)) { list.value = [...list.value, id] }
  const mesh = object.mesh
  const vertexIds = kind === 'vertex' ? [...context.selectedVerts.value]
    : kind === 'edge' ? [...new Set(context.selectedEdges.value.flatMap(edge => getEdgeList(mesh)[edge] ?? []))]
      : [...new Set(context.selectedFaces.value.flatMap(face => [mesh.indices[face * 3], mesh.indices[face * 3 + 1], mesh.indices[face * 3 + 2]]))]
  const target = event.currentTarget as SVGGraphicsElement
  const svg = target.ownerSVGElement ?? (target as unknown as SVGSVGElement)
  // Meshes may carry per-face duplicates of a corner; move every index that shares the position.
  const byPosition = new Map<string, number[]>()
  const keyOf = (i: number) => `${mesh.positions[i * 3].toFixed(5)},${mesh.positions[i * 3 + 1].toFixed(5)},${mesh.positions[i * 3 + 2].toFixed(5)}`
  for (let i = 0; i < mesh.positions.length / 3; i++) { const key = keyOf(i); const list = byPosition.get(key); if (list) list.push(i); else byPosition.set(key, [i]) }
  const expanded = [...new Set(vertexIds.flatMap(i => byPosition.get(keyOf(i)) ?? [i]))]
  context.elementDrag = { objectId, vertexIds: expanded, base: { positions: mesh.positions.slice(), indices: mesh.indices.slice() }, x: event.clientX, y: event.clientY, moved: false, svg }
  context.dragSwallowsClick = false
  ;(event.currentTarget as Element).setPointerCapture(event.pointerId)
}
function moveElementDrag(event: PointerEvent) {
  const drag = context.elementDrag
  if (!drag) return
  const dxPx = event.clientX - drag.x, dyPx = event.clientY - drag.y
  if (!drag.moved && Math.hypot(dxPx, dyPx) < 3) return
  drag.moved = true
  context.dragSwallowsClick = true
  const delta = screenDeltaToWorld(dxPx, dyPx, drag.svg)
  try {
    const mesh = context.selectMode.value === 'vertex' && context.proportionalEdit.value
      ? moveVerticesProportional(drag.base, drag.vertexIds, delta, context.proportionalRadius.value)
      : moveVertices(drag.base, drag.vertexIds, delta)
    const current = context.history.document
    context.document.value = { ...current, objects: current.objects.map(o => o.id === drag.objectId ? { ...o, mesh } : o) }
  } catch (e) { context.error.value = e instanceof Error ? e.message : String(e) }
}
function endElementDrag() {
  const drag = context.elementDrag
  if (!drag) return
  context.elementDrag = null
  if (!drag.moved) return
  const preview = context.document.value
  context.document.value = context.history.document
  context.run(() => context.commit(preview))
}
function cancelElementDrag() {
  if (!context.elementDrag) return
  context.elementDrag = null
  context.document.value = context.history.document
}
function onPointerDown(event: PointerEvent) {
  if (event.button > 2) return
  context.dragSwallowsClick = false
  context.orbit = {
    x: event.clientX, y: event.clientY, yaw: context.camera.value.yaw, pitch: context.camera.value.pitch,
    pan: event.button === 1 || event.shiftKey, center: [context.center.value[0], context.center.value[1]], moved: false,
  }
  ;(event.currentTarget as Element).setPointerCapture(event.pointerId)
}
function onPointerMove(event: PointerEvent) {
  if (context.elementDrag) { moveElementDrag(event); return }
  if (!context.orbit) return
  const dx = event.clientX - context.orbit.x, dy = event.clientY - context.orbit.y
  if (!context.orbit.moved && Math.hypot(dx, dy) < 4) return
  context.orbit.moved = true
  context.dragSwallowsClick = true
  if (context.orbit.pan) {
    const svg = event.currentTarget as SVGSVGElement
    const unit = 200 / Math.max(1, Math.min(svg.clientWidth, svg.clientHeight))
    context.center.value = [context.orbit.center[0] + dx * unit, context.orbit.center[1] + dy * unit]
    return
  }
  context.camera.value = {
    ...context.camera.value,
    yaw: context.orbit.yaw + dx * 0.01,
    pitch: Math.max(-1.4, Math.min(1.4, context.orbit.pitch + dy * 0.01)),
  }
}
function onPointerUp() { context.orbit = null; endElementDrag() }
function onClickCapture(event: MouseEvent) {
  if (!context.dragSwallowsClick) return
  context.dragSwallowsClick = false
  event.stopPropagation()
  event.preventDefault()
}
function resetView() { context.camera.value = defaultDirectCamera(); context.view.value = 140; context.center.value = [0, 0] }
function onWheel(event: WheelEvent) {
  event.preventDefault()
  context.view.value = Math.max(20, Math.min(800, context.view.value * (event.deltaY > 0 ? 0.9 : 1.1)))
}
function vrSnapshot() { return prepareVrPolygons(context.document.value.objects.filter(object => object.visible).map(object => object.mesh)) }
return {downloadJson, togglePick, getEdgeList, projected, screenDeltaToWorld, startElementDrag, moveElementDrag, endElementDrag, cancelElementDrag, onPointerDown, onPointerMove, onPointerUp, onClickCapture, resetView, onWheel, vrSnapshot}
}
