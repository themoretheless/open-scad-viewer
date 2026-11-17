import { buildSculptBrush } from '../../../kernel/geometryEditing'
import { meshCentroid, sculptMesh } from '../../../kernel/meshEditing'
import {
  createBoxMesh,
  createUvSphereMesh,
  deleteFaces,
  duplicateObject,
  exportMeshObjectStl,
  extrudeSelectedFaces,
  flipFaces,
  insetSelectedFaces,
  joinMeshes,
  knifeSplitEdges,
  mergeByDistance,
  moveVertices,
  moveVerticesProportional,
  separateFaces,
  subdivideFaces,
  symmetrizeMesh,
  transformMesh,
  twistMesh,
} from '../../../kernel/meshEditing'
import { stripMeshExtension } from '../../../exchange/meshImport'
import { manifoldPreferenceSnapshot } from '../../../kernel/manifoldPreferences'
import { formatMeshImportNotice } from '../../../exchange/meshImportReport'
import { formatMeshBooleanNotice } from '../../../kernel/meshBooleanReport'
import { formatMeshRepairNotice } from '../../../kernel/meshRepairReport'
import { downloadBytes } from '../../../exchange/downloadArtifact'
import type { PolygonMesh } from '../../../kernel/geometry/polygon'
import type { MeshWorkspaceDocument } from '../../../kernel/meshEditing'
import type { MeshCommandsContext } from './meshCommandsContext'
export function createMeshCommands(context: MeshCommandsContext) {
  /** Index and object of the current selection in a working document; the callers check `context.selected` first. */
  function selectedEntry(d: MeshWorkspaceDocument) {
    const index = d.objects.findIndex(o => o.id === context.selection.value)
    const current = d.objects[index]
    if (!current) throw new Error(context.label('Объект не найден.', 'Object missing.'))
    return { index, current }
  }
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
      const { index, current } = selectedEntry(d)
      if (
        (context.selectMode.value === 'vertex' && context.selectedVerts.value.length) ||
        (context.selectMode.value === 'edge' && context.selectedEdges.value.length)
      ) {
        const vertexIds =
          context.selectMode.value === 'edge'
            ? [...new Set(context.selectedEdges.value.flatMap(edge => context.getEdgeList(current.mesh)[edge] ?? []))]
            : context.selectedVerts.value
        d.objects[index] = {
          ...current,
          mesh:
            context.selectMode.value === 'vertex' && context.proportionalEdit.value
              ? moveVerticesProportional(
                  current.mesh,
                  vertexIds,
                  [context.dx.value, context.dy.value, context.dz.value],
                  context.proportionalRadius.value,
                )
              : moveVertices(current.mesh, vertexIds, [context.dx.value, context.dy.value, context.dz.value]),
        }
      } else {
        d.objects[index] = {
          ...current,
          mesh: transformMesh(
            current.mesh,
            [context.dx.value, context.dy.value, context.dz.value],
            context.angle.value,
            context.scale.value,
          ),
        }
      }
      context.commit(d)
    })
  }
  function applyExtrude() {
    context.run(() => {
      if (!context.selected.value || !context.selectedFaces.value.length)
        throw new Error(context.label('Выберите грани.', 'Select faces.'))
      const d = context.history.document
      const { index, current } = selectedEntry(d)
      d.objects[index] = {
        ...current,
        mesh: extrudeSelectedFaces(current.mesh, context.selectedFaces.value, context.extrudeDistance.value),
      }
      context.commit(d)
      context.selectedFaces.value = []
    })
  }
  function applyInset() {
    context.run(() => {
      if (!context.selected.value || !context.selectedFaces.value.length)
        throw new Error(context.label('Выберите грани.', 'Select faces.'))
      const d = context.history.document
      const { index, current } = selectedEntry(d)
      d.objects[index] = {
        ...current,
        mesh: insetSelectedFaces(current.mesh, context.selectedFaces.value, context.insetAmount.value),
      }
      context.commit(d)
    })
  }
  function applySubdivide() {
    context.run(() => {
      if (!context.selected.value) return
      const d = context.history.document
      const { index, current } = selectedEntry(d)
      const faces = context.selectedFaces.value.length ? context.selectedFaces.value : undefined
      d.objects[index] = { ...current, mesh: subdivideFaces(current.mesh, faces) }
      context.commit(d)
      context.selectedFaces.value = []
    })
  }
  function applyKnife() {
    context.run(() => {
      if (!context.selected.value || !context.selectedEdges.value.length)
        throw new Error(context.label('Выберите рёбра.', 'Select edges.'))
      const d = context.history.document
      const { index, current } = selectedEntry(d)
      d.objects[index] = { ...current, mesh: knifeSplitEdges(current.mesh, context.selectedEdges.value) }
      context.commit(d)
      context.selectedEdges.value = []
    })
  }
  function applyDeleteFaces() {
    context.run(() => {
      if (!context.selected.value || !context.selectedFaces.value.length)
        throw new Error(context.label('Выберите грани.', 'Select faces.'))
      const d = context.history.document
      const { index, current } = selectedEntry(d)
      d.objects[index] = { ...current, mesh: deleteFaces(current.mesh, context.selectedFaces.value) }
      context.commit(d)
      context.selectedFaces.value = []
    })
  }
  function applyFlip() {
    context.run(() => {
      if (!context.selected.value) return
      const d = context.history.document
      const { index, current } = selectedEntry(d)
      d.objects[index] = {
        ...current,
        mesh: flipFaces(current.mesh, context.selectedFaces.value.length ? context.selectedFaces.value : undefined),
      }
      context.commit(d)
    })
  }
  function applyMerge() {
    context.run(() => {
      if (!context.selected.value) return
      const d = context.history.document
      const { index, current } = selectedEntry(d)
      d.objects[index] = { ...current, mesh: mergeByDistance(current.mesh, context.mergeDistance.value) }
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
    const snapshot = context.document.value,
      sourceId = object.id
    await context.runMeshTask(
      { kind: 'meshRepair', mesh: object.mesh },
      result => {
        if (context.document.value !== snapshot || context.selection.value !== sourceId) return
        const d = context.history.document
        const index = d.objects.findIndex(o => o.id === sourceId)
        const current = d.objects[index]
        if (!current) return
        if (result.repair) {
          d.objects[index] = { ...current, mesh: result.mesh }
          context.commit(d, context.label('Починить меш (Full)', 'Repair mesh (Full)'))
          context.selectedVerts.value = []
          context.selectedFaces.value = []
          context.selectedEdges.value = []
        }
        context.repairNotice.value = formatMeshRepairNotice(result, context.ru.value ? 'ru' : 'en')
      },
      caught => {
        const reason = caught instanceof Error ? caught.message : String(caught)
        context.error.value = `${context.label('Repair не выполнен', 'Repair failed')}: ${reason} ${context.label('Документ не изменён.', 'Document was not changed.')}`
      },
    )
  }
  function applySeparate() {
    context.run(() => {
      if (!context.selected.value || !context.selectedFaces.value.length)
        throw new Error(context.label('Выберите грани.', 'Select faces.'))
      const d = context.history.document
      const { index, current } = selectedEntry(d)
      const { kept, separated } = separateFaces(current.mesh, context.selectedFaces.value)
      d.objects[index] = { ...current, mesh: kept }
      const id = `mesh-${Date.now().toString(36)}`
      d.objects.push({ id, name: `${current.name} part`, mesh: separated, visible: true })
      context.commit(d)
      context.selectedFaces.value = []
      context.selection.value = id
    })
  }
  function applyJoin() {
    context.run(() => {
      if (
        !context.selected.value ||
        !context.booleanTarget.value ||
        context.booleanTarget.value === context.selection.value
      ) {
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
      const { index, current } = selectedEntry(d)
      d.objects[index] = { ...current, mesh: symmetrizeMesh(current.mesh, 0) }
      context.commit(d)
    })
  }
  function applyBrush() {
    context.run(() => {
      if (!context.selected.value) return
      const mesh = context.selected.value.mesh
      // Selected vertices define the stroke center when present; otherwise the object centroid.
      const center = meshCentroid(mesh, context.selectedVerts.value)
      const brush = buildSculptBrush(
        {
          kind: context.brushKind.value,
          radius: context.brushRadius.value,
          strength: context.brushStrength.value,
          falloff: context.brushFalloff.value,
          mirror: context.brushMirror.value,
        },
        center,
      )
      const d = context.history.document
      const { index, current } = selectedEntry(d)
      d.objects[index] = { ...current, mesh: sculptMesh(mesh, brush) }
      context.commit(d)
    })
  }
  function applyTwist() {
    context.run(() => {
      if (!context.selected.value) return
      const d = context.history.document
      const { index, current } = selectedEntry(d)
      d.objects[index] = { ...current, mesh: twistMesh(current.mesh, context.twistAmount.value) }
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
    if (
      !context.selected.value ||
      !context.booleanTarget.value ||
      context.booleanTarget.value === context.selection.value
    ) {
      fail(new Error(context.label('Выберите два разных объекта.', 'Pick two different objects.')))
      return
    }
    const snapshot = context.document.value,
      sourceId = context.selection.value,
      targetId = context.booleanTarget.value
    const a = snapshot.objects.find(object => object.id === sourceId)
    const b = snapshot.objects.find(object => object.id === targetId)
    if (!a || !b) {
      fail(new Error(context.label('Второй объект не найден.', 'Second object missing.')))
      return
    }
    const preferences = manifoldPreferenceSnapshot()
    await context.runMeshTask(
      {
        kind: 'meshBoolean',
        a: a.mesh,
        b: b.mesh,
        operation: context.booleanOp.value,
        options: {
          weldEpsilonMm: preferences.booleanWeldEpsilonMm,
          relativeTolerance: preferences.booleanRelativeTolerance,
        },
      },
      ({ mesh, report }) => {
        if (
          context.document.value !== snapshot ||
          context.selection.value !== sourceId ||
          context.booleanTarget.value !== targetId
        )
          return
        const d = context.history.document
        const index = d.objects.findIndex(object => object.id === sourceId)
        const current = d.objects[index]
        if (!current || !d.objects.some(object => object.id === targetId)) return
        d.objects[index] = { ...current, mesh }
        d.objects = d.objects.filter(object => object.id !== targetId)
        context.commit(d)
        context.booleanTarget.value = ''
        context.booleanNotice.value = formatMeshBooleanNotice(report, context.ru.value ? 'ru' : 'en')
      },
      fail,
    )
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
    if (!file || context.meshTaskPending.value) {
      input.value = ''
      return
    }
    try {
      context.error.value = ''
      context.importNotice.value = ''
      const fileName = file.name
      const bytes = new Uint8Array(await file.arrayBuffer())
      if (context.meshTaskPending.value) return
      const snapshot = context.document.value,
        preferences = manifoldPreferenceSnapshot()
      await context.runMeshTask(
        {
          kind: 'meshImport',
          fileName,
          input: bytes,
          options: { weld: preferences.importWeldEpsilonMm, repair: preferences.importRepair },
        },
        imported => {
          if (context.document.value !== snapshot) return
          const mesh: PolygonMesh = { positions: imported.positions, indices: imported.indices }
          const d = context.history.document
          const id = `${imported.format}-${Date.now().toString(36)}`
          d.objects.push({
            id,
            name: stripMeshExtension(fileName) || imported.format.toUpperCase(),
            mesh,
            visible: true,
          })
          context.commit(d)
          context.selection.value = id
          context.importNotice.value = formatMeshImportNotice(imported, context.ru.value ? 'ru' : 'en')
        },
        caught => {
          context.error.value = caught instanceof Error ? caught.message : String(caught)
        },
      )
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
    if (!object) {
      context.error.value = context.label('Выберите объект.', 'Select an object.')
      return
    }
    context.error.value = ''
    const snapshot = context.document.value,
      id = object.id,
      name = object.name || 'mesh',
      target = context.exportFormat.value
    await context.runMeshTask(
      { kind: 'meshExport', mesh: object.mesh, target, options: {} },
      artifact => {
        if (!context.props.open || context.document.value !== snapshot || context.selected.value?.id !== id) return
        downloadBytes(artifact.data, artifact.mimeType, `${name}.${artifact.extension}`)
      },
      caught => {
        context.error.value = caught instanceof Error ? caught.message : String(caught)
      },
    )
  }
  return {
    addPrimitive,
    transformSelected,
    applyExtrude,
    applyInset,
    applySubdivide,
    applyKnife,
    applyDeleteFaces,
    applyFlip,
    applyMerge,
    applyRepair,
    applySeparate,
    applyJoin,
    applySymmetrize,
    applyBrush,
    applyTwist,
    applyDuplicate,
    applyBoolean,
    removeSelected,
    importMesh,
    downloadStl,
    downloadMesh,
  }
}
