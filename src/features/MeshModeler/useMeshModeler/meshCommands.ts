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
import type {MeshCommandsContext} from './meshCommandsContext'
export function createMeshCommands(context: MeshCommandsContext) {
function addPrimitive(kind: 'box' | 'sphere') {
  context.run(() => {
    const d = context.history.document
    const id = `mesh-${Date.now().toString(36)}`
    d.objects.push({
      id,
      name: kind === 'box' ? 'Cube' : 'UV Sphere',
      mesh: kind === 'box' ? createBoxMesh([20, 20, 20]) : createUvSphereMesh(10, 24, 16),
      visible: true,
    })
    context.commit(d)
    context.selection.value = id
    context.selectMode.value = 'object'
  })
}
function transformSelected() {
  context.run(() => {
    if (!context.selected.value) return
    const d = context.history.document
    const index = d.objects.findIndex(o => o.id === context.selection.value)
    if ((context.selectMode.value === 'vertex' && context.selectedVerts.value.length) ||
        (context.selectMode.value === 'edge' && context.selectedEdges.value.length)) {
      const vertexIds = context.selectMode.value === 'edge'
        ? [...new Set(context.selectedEdges.value.flatMap(edge => context.getEdgeList(d.objects[index].mesh)[edge] ?? []))]
        : context.selectedVerts.value
      d.objects[index] = {
        ...d.objects[index],
        mesh: context.selectMode.value === 'vertex' && context.proportionalEdit.value
          ? moveVerticesProportional(
              d.objects[index].mesh,
              vertexIds,
              [context.dx.value, context.dy.value, context.dz.value],
              context.proportionalRadius.value,
            )
          : moveVertices(d.objects[index].mesh, vertexIds, [context.dx.value, context.dy.value, context.dz.value]),
      }
    } else {
      d.objects[index] = {
        ...d.objects[index],
        mesh: transformMesh(d.objects[index].mesh, [context.dx.value, context.dy.value, context.dz.value], context.angle.value, context.scale.value),
      }
    }
    context.commit(d)
  })
}
function applyExtrude() {
  context.run(() => {
    if (!context.selected.value || !context.selectedFaces.value.length) throw new Error(context.label('Выберите грани.', 'Select faces.'))
    const d = context.history.document
    const index = d.objects.findIndex(o => o.id === context.selection.value)
    d.objects[index] = {
      ...d.objects[index],
      mesh: extrudeSelectedFaces(d.objects[index].mesh, context.selectedFaces.value, context.extrudeDistance.value),
    }
    context.commit(d)
    context.selectedFaces.value = []
  })
}
function applyInset() {
  context.run(() => {
    if (!context.selected.value || !context.selectedFaces.value.length) throw new Error(context.label('Выберите грани.', 'Select faces.'))
    const d = context.history.document
    const index = d.objects.findIndex(o => o.id === context.selection.value)
    d.objects[index] = {
      ...d.objects[index],
      mesh: insetSelectedFaces(d.objects[index].mesh, context.selectedFaces.value, context.insetAmount.value),
    }
    context.commit(d)
  })
}
function applySubdivide() {
  context.run(() => {
    if (!context.selected.value) return
    const d = context.history.document
    const index = d.objects.findIndex(o => o.id === context.selection.value)
    const faces = context.selectedFaces.value.length ? context.selectedFaces.value : undefined
    d.objects[index] = { ...d.objects[index], mesh: subdivideFaces(d.objects[index].mesh, faces) }
    context.commit(d)
    context.selectedFaces.value = []
  })
}
function applyKnife() {
  context.run(() => {
    if (!context.selected.value || !context.selectedEdges.value.length) throw new Error(context.label('Выберите рёбра.', 'Select edges.'))
    const d = context.history.document
    const index = d.objects.findIndex(o => o.id === context.selection.value)
    d.objects[index] = { ...d.objects[index], mesh: knifeSplitEdges(d.objects[index].mesh, context.selectedEdges.value) }
    context.commit(d)
    context.selectedEdges.value = []
  })
}
function applyDeleteFaces() {
  context.run(() => {
    if (!context.selected.value || !context.selectedFaces.value.length) throw new Error(context.label('Выберите грани.', 'Select faces.'))
    const d = context.history.document
    const index = d.objects.findIndex(o => o.id === context.selection.value)
    d.objects[index] = { ...d.objects[index], mesh: deleteFaces(d.objects[index].mesh, context.selectedFaces.value) }
    context.commit(d)
    context.selectedFaces.value = []
  })
}
function applyFlip() {
  context.run(() => {
    if (!context.selected.value) return
    const d = context.history.document
    const index = d.objects.findIndex(o => o.id === context.selection.value)
    d.objects[index] = {
      ...d.objects[index],
      mesh: flipFaces(d.objects[index].mesh, context.selectedFaces.value.length ? context.selectedFaces.value : undefined),
    }
    context.commit(d)
  })
}
function applyMerge() {
  context.run(() => {
    if (!context.selected.value) return
    const d = context.history.document
    const index = d.objects.findIndex(o => o.id === context.selection.value)
    d.objects[index] = { ...d.objects[index], mesh: mergeByDistance(d.objects[index].mesh, context.mergeDistance.value) }
    context.commit(d)
    context.selectedVerts.value = []
    context.selectedFaces.value = []
  })
}
async function applyRepair() {
  const object = context.selected.value
  if (!object || context.meshTaskPending.value) return
  context.error.value = ''
  context.repairNotice.value = ''
  context.importNotice.value = ''
  context.booleanNotice.value = ''
  const snapshot = context.document.value, sourceId = object.id
  await context.runMeshTask({kind: 'meshRepair', mesh: object.mesh}, result => {
    if (context.document.value !== snapshot || context.selection.value !== sourceId) return
    const d = context.history.document
    const index = d.objects.findIndex(o => o.id === sourceId)
    if (index < 0) return
    if (result.repair) {
      d.objects[index] = { ...d.objects[index], mesh: result.mesh }
      context.commit(d, context.label('Починить меш (Full)', 'Repair mesh (Full)'))
      context.selectedVerts.value = []
      context.selectedFaces.value = []
      context.selectedEdges.value = []
    }
    context.repairNotice.value = formatMeshRepairNotice(result, context.ru.value ? 'ru' : 'en')
  }, caught => {
    const reason = caught instanceof Error ? caught.message : String(caught)
    context.error.value = `${context.label('Repair не выполнен', 'Repair failed')}: ${reason} ${context.label('Документ не изменён.', 'Document was not changed.')}`
  })
}
function applySeparate() {
  context.run(() => {
    if (!context.selected.value || !context.selectedFaces.value.length) throw new Error(context.label('Выберите грани.', 'Select faces.'))
    const d = context.history.document
    const index = d.objects.findIndex(o => o.id === context.selection.value)
    const { kept, separated } = separateFaces(d.objects[index].mesh, context.selectedFaces.value)
    d.objects[index] = { ...d.objects[index], mesh: kept }
    const id = `mesh-${Date.now().toString(36)}`
    d.objects.push({ id, name: `${d.objects[index].name} part`, mesh: separated, visible: true })
    context.commit(d)
    context.selectedFaces.value = []
    context.selection.value = id
  })
}
function applyJoin() {
  context.run(() => {
    if (!context.selected.value || !context.booleanTarget.value || context.booleanTarget.value === context.selection.value) {
      throw new Error(context.label('Выберите два разных объекта.', 'Pick two different objects.'))
    }
    const d = context.history.document
    const a = d.objects.find(o => o.id === context.selection.value)!
    const b = d.objects.find(o => o.id === context.booleanTarget.value)
    if (!b) throw new Error(context.label('Второй объект не найден.', 'Second object missing.'))
    a.mesh = joinMeshes([a.mesh, b.mesh])
    d.objects = d.objects.filter(o => o.id !== b.id)
    context.commit(d)
    context.booleanTarget.value = ''
  })
}
function applySymmetrize() {
  context.run(() => {
    if (!context.selected.value) return
    const d = context.history.document
    const index = d.objects.findIndex(o => o.id === context.selection.value)
    d.objects[index] = { ...d.objects[index], mesh: symmetrizeMesh(d.objects[index].mesh, 0) }
    context.commit(d)
  })
}
function applyBrush() {
  context.run(() => {
    if (!context.selected.value) return
    const mesh = context.selected.value.mesh
    // Selected vertices define the stroke center when present; otherwise the object centroid.
    const center = meshCentroid(mesh, context.selectedVerts.value)
    const brush = buildSculptBrush({ kind: context.brushKind.value, radius: context.brushRadius.value, strength: context.brushStrength.value, falloff: context.brushFalloff.value, mirror: context.brushMirror.value }, center)
    const d = context.history.document
    const index = d.objects.findIndex(o => o.id === context.selection.value)
    d.objects[index] = { ...d.objects[index], mesh: sculptMesh(mesh, brush) }
    context.commit(d)
  })
}
function applyTwist() {
  context.run(() => {
    if (!context.selected.value) return
    const d = context.history.document
    const index = d.objects.findIndex(o => o.id === context.selection.value)
    d.objects[index] = { ...d.objects[index], mesh: twistMesh(d.objects[index].mesh, context.twistAmount.value) }
    context.commit(d)
  })
}
function applyDuplicate() {
  context.run(() => {
    if (!context.selected.value) return
    const d = context.history.document
    const copy = duplicateObject(context.selected.value)
    d.objects.push(copy)
    context.commit(d)
    context.selection.value = copy.id
  })
}
async function applyBoolean() {
  if (context.meshTaskPending.value) return
  context.error.value = ''
  context.booleanNotice.value = ''
  context.repairNotice.value = ''
  const fail = (caught: unknown) => {
    const reason = caught instanceof Error ? caught.message : String(caught)
    context.error.value = `${context.label('Булева операция не выполнена', 'Boolean failed')}: ${reason} ${context.label('Документ не изменён.', 'Document was not changed.')}`
  }
  if (!context.selected.value || !context.booleanTarget.value || context.booleanTarget.value === context.selection.value) {
    fail(new Error(context.label('Выберите два разных объекта.', 'Pick two different objects.')))
    return
  }
  const snapshot = context.document.value, sourceId = context.selection.value, targetId = context.booleanTarget.value
  const a = snapshot.objects.find(object => object.id === sourceId)
  const b = snapshot.objects.find(object => object.id === targetId)
  if (!a || !b) {
    fail(new Error(context.label('Второй объект не найден.', 'Second object missing.')))
    return
  }
  const preferences = manifoldPreferenceSnapshot()
  await context.runMeshTask({kind: 'meshBoolean', a: a.mesh, b: b.mesh, operation: context.booleanOp.value, options: {weldEpsilonMm: preferences.booleanWeldEpsilonMm, relativeTolerance: preferences.booleanRelativeTolerance}}, ({mesh, report}) => {
    if (context.document.value !== snapshot || context.selection.value !== sourceId || context.booleanTarget.value !== targetId) return
    const d = context.history.document
    const index = d.objects.findIndex(object => object.id === sourceId)
    if (index < 0 || !d.objects.some(object => object.id === targetId)) return
    d.objects[index] = {...d.objects[index], mesh}
    d.objects = d.objects.filter(object => object.id !== targetId)
    context.commit(d)
    context.booleanTarget.value = ''
    context.booleanNotice.value = formatMeshBooleanNotice(report, context.ru.value ? 'ru' : 'en')
  }, fail)
}
function removeSelected() {
  context.run(() => {
    if (!context.selection.value) return
    const d = context.history.document
    d.objects = d.objects.filter(o => o.id !== context.selection.value)
    context.commit(d)
    context.selection.value = ''
  })
}
async function importMesh(event: Event) {
  const input = event.target as HTMLInputElement
  const file = input.files?.[0]
  if (!file || context.meshTaskPending.value) { input.value = ''; return }
  try {
    context.error.value = ''
    context.importNotice.value = ''
    const fileName = file.name
    const bytes = new Uint8Array(await file.arrayBuffer())
    if (context.meshTaskPending.value) return
    const snapshot = context.document.value, preferences = manifoldPreferenceSnapshot()
    await context.runMeshTask({kind: 'meshImport', fileName, input: bytes, options: {weld: preferences.importWeldEpsilonMm, repair: preferences.importRepair}}, imported => {
      if (context.document.value !== snapshot) return
      const mesh: PolygonMesh = {positions: imported.positions, indices: imported.indices}
      const d = context.history.document
      const id = `${imported.format}-${Date.now().toString(36)}`
      d.objects.push({id, name: stripMeshExtension(fileName) || imported.format.toUpperCase(), mesh, visible: true})
      context.commit(d)
      context.selection.value = id
      context.importNotice.value = formatMeshImportNotice(imported, context.ru.value ? 'ru' : 'en')
    }, caught => { context.error.value = caught instanceof Error ? caught.message : String(caught) })
  } catch (caught) {
    context.error.value = caught instanceof Error ? caught.message : String(caught)
  } finally {
    input.value = ''
  }
}
function downloadStl() {
  context.run(() => {
    if (!context.selected.value) throw new Error(context.label('Выберите объект.', 'Select an object.'))
    const blob = new Blob([exportMeshObjectStl(context.selected.value.mesh)], { type: 'model/stl' })
    const url = URL.createObjectURL(blob)
    const a = window.document.createElement('a')
    a.href = url
    a.download = `${context.selected.value.name || 'mesh'}.stl`
    a.click()
    URL.revokeObjectURL(url)
  })
}
async function downloadMesh() {
  if (context.meshTaskPending.value) return
  const object = context.selected.value
  if (!object) { context.error.value = context.label('Выберите объект.', 'Select an object.'); return }
  context.error.value = ''
  const snapshot = context.document.value, id = object.id, name = object.name || 'mesh', target = context.exportFormat.value
  await context.runMeshTask({ kind: 'meshExport', mesh: object.mesh, target, options: {} }, artifact => {
    if (!context.props.open || context.document.value !== snapshot || context.selected.value?.id !== id) return
    downloadBytes(artifact.data, artifact.mimeType, `${name}.${artifact.extension}`)
  }, caught => { context.error.value = caught instanceof Error ? caught.message : String(caught) })
}
return {addPrimitive, transformSelected, applyExtrude, applyInset, applySubdivide, applyKnife, applyDeleteFaces, applyFlip, applyMerge, applyRepair, applySeparate, applyJoin, applySymmetrize, applyBrush, applyTwist, applyDuplicate, applyBoolean, removeSelected, importMesh, downloadStl, downloadMesh}
}
