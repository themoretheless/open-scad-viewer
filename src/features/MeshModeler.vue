<script setup lang="ts">
import VrControls from '../components/VrControls.vue'

import { prepareVrPolygons } from '../services/vrScene'

import { computed, nextTick, onUnmounted, ref, shallowRef, watch } from 'vue'

import { stringifyMeshJson } from '../services/meshJson'

import ModelingFloorGrid from '../components/ModelingFloorGrid.vue'

import ModelingGridControls from '../components/ModelingGridControls.vue'

import WorkspaceActionBar from '../components/WorkspaceActionBar.vue'

import CommandPalette from '../components/CommandPalette.vue'

import type { PaletteCommand } from '../services/commandSearch'

import { SCULPT_FALLOFFS, SCULPT_KINDS, buildSculptBrush, isFractionalSculptKind, type SculptFalloff, type SculptKind } from '../services/geometryEditing'

import { meshCentroid, sculptMesh } from '../services/meshEditing'

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
} from '../services/meshEditing'

import { createSolidPreviewWorker } from '../services/solidPreviewWorker'

import { useMainSolidTask } from './useMainSolidTask'

import { defaultDirectCamera, projectDirectPoint } from '../services/directModelingTools'

import { projectMeshModelerObject,cachedMeshModelerStats } from '../services/meshModelerProjection'

import { isGeometryKernelReady, warmGeometryKernel } from '../services/geometry/kernel'

import { storageGet, storageSet } from '../services/safeStorage'

import { MESH_IMPORT_ACCEPT, stripMeshExtension } from '../services/meshImport'

import { manifoldPreferenceSnapshot } from '../services/manifoldPreferences'

import ManifoldSettings from '../components/ManifoldSettings.vue'

import { formatMeshImportNotice } from '../services/meshImportReport'

import { formatMeshBooleanNotice } from '../services/meshBooleanReport'

import { formatMeshRepairNotice } from '../services/meshRepairReport'

import { meshManifoldDefectLabels } from '../services/meshManifoldDefects'

import { MESH_EXPORT_FORMATS, MESH_FORMAT_LABELS, type MeshExportFormat } from '../services/meshConvert'

import { downloadBytes } from '../services/downloadArtifact'

import type { PolygonMesh } from '../services/geometry/polygon'

import type { KernelManifoldReport } from '../services/geometry/manifoldAnalysis'
import {useMeshModeler} from './MeshModeler/useMeshModeler'
const props = defineProps<{ open: boolean; locale: string; seedDocument?: MeshWorkspaceDocument | null; paletteRequest?: number }>()
const emit = defineEmits<{ close: []; exportToSolid: [doc: MeshWorkspaceDocument] }>()
const {
  ru, label, storageKey, initial, history,
  document, undoable, redoable, undoOperation, redoOperation,
  error, importNotice, booleanNotice, repairNotice, solidWorker,
  meshTaskPending, meshTaskProgress, meshTaskKind, runMeshTask, cancelMeshTask,
  disposeMeshTask, manifoldWorker, manifoldTaskPending, manifoldTaskProgress, runManifoldTask,
  cancelManifoldTask, disposeManifoldTask, meshTaskStage, meshTaskPresentation, kernelReady,
  selection, selectMode, selectedVerts, selectedFaces, selectedEdges,
  dx, dy, dz, angle, scale,
  proportionalEdit, proportionalRadius, extrudeDistance, insetAmount, mergeDistance,
  brushRadius, brushStrength, brushKind, brushFalloff, brushMirror,
  brushIsFraction, twistAmount, booleanOp, booleanTarget, camera,
  view, center, workspace, fileInput, orbit,
  dragSwallowsClick, appliedSeeds, floorVisible, selected, stats,
  manifoldReports, refreshManifoldReports, expandedManifoldObject, manifoldDefects, syncHistoryFlags,
  commit, run, undo, redo, addPrimitive,
  transformSelected, applyExtrude, applyInset, applySubdivide, applyKnife,
  applyDeleteFaces, applyFlip, applyMerge, applyRepair, applySeparate,
  applyJoin, applySymmetrize, applyBrush, applyTwist, applyDuplicate,
  applyBoolean, removeSelected, importMesh, downloadStl, exportFormat,
  downloadMesh, paletteOpen, dockOpen, dockTab, propsOpen,
  meshCommands, executeMeshCommand, onWorkspaceKey, downloadJson, togglePick,
  edgeListCache, getEdgeList, projected, elementDrag, screenDeltaToWorld,
  startElementDrag, moveElementDrag, endElementDrag, cancelElementDrag, onPointerDown,
  onPointerMove, onPointerUp, onClickCapture, resetView, onWheel,
  vrSnapshot, scene, sceneFaces
} = useMeshModeler(props, emit)
defineExpose({ meshCommands, executeMeshCommand })
</script>

<template>
  <div v-if="open" ref="workspace" class="mesh-workspace" tabindex="0" @keydown="onWorkspaceKey">
    <header class="mesh-bar">
      <strong>{{ label('Mesh', 'Mesh') }}</strong>
      <button type="button" class="command-search" :title="label('Поиск команд · Ctrl/⌘ K', 'Search commands · Ctrl/⌘ K')" @click="paletteOpen = true"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/></svg><span>{{ label('Команда…', 'Command…') }}</span><kbd>Ctrl K</kbd></button>
      <span class="hint"></span>
      <span class="ws-count" aria-live="polite">{{ label('Выбрано', 'Selected') }}: {{ selected ? 1 : 0 }}</span>
      <button type="button" class="icon mesh-undo" :disabled="!undoable" :title="undoable ? `${label('Отменить', 'Undo')}: ${undoOperation} · Ctrl/⌘ Z` : label('Отменить · Ctrl/⌘ Z', 'Undo · Ctrl/⌘ Z')" :aria-label="label('Отменить', 'Undo')" @click="undo"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M9 14 4 9l5-5"/><path d="M4 9h11a5 5 0 0 1 0 10h-3"/></svg></button>
      <details class="file-menu mesh-file"><summary :title="label('Файл', 'File')"><svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linejoin="round" aria-hidden="true"><path d="M14 3H6a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V9z"/><path d="M14 3v6h6"/></svg><span>{{ label('Файл', 'File') }}</span><svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" aria-hidden="true"><path d="m6 9 6 6 6-6"/></svg></summary><div>
        <button type="button" @click="fileInput?.click()">{{ label('Импорт STL / OBJ / PLY / OFF / AMF / 3MF', 'Import STL / OBJ / PLY / OFF / AMF / 3MF') }}</button>
        <ManifoldSettings :locale="locale" />
        <button type="button" :disabled="!selected" @click="downloadStl">{{ label('Скачать STL выбранного', 'Download selected as STL') }}</button>
        <label class="file-row"><select v-model="exportFormat" :disabled="meshTaskPending" :aria-label="label('Формат экспорта', 'Export format')"><option v-for="format in MESH_EXPORT_FORMATS" :key="format" :value="format">{{ MESH_FORMAT_LABELS[format] }}</option></select><button type="button" :disabled="!selected || meshTaskPending" @click="downloadMesh">{{ label('Скачать', 'Download') }}</button></label>
        <button type="button" @click="downloadJson">{{ label('Скачать проект JSON', 'Download JSON project') }}</button>
        <button type="button" :disabled="!document.objects.length" @click="emit('exportToSolid', document)">{{ label('Открыть в Solid', 'Open in Solid') }}</button>
      </div></details>
      <button type="button" class="icon mesh-redo" :disabled="!redoable" :title="redoable ? `${label('Повторить', 'Redo')}: ${redoOperation} · Ctrl/⌘ Shift Z` : label('Повторить · Ctrl/⌘ Shift Z', 'Redo · Ctrl/⌘ Shift Z')" :aria-label="label('Повторить', 'Redo')" @click="redo"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="m15 14 5-5-5-5"/><path d="M20 9H9a5 5 0 0 0 0 10h3"/></svg></button>
    </header>
    <CommandPalette v-if="paletteOpen" :open="paletteOpen" :commands="meshCommands.filter(command => command.enabled !== false)" @close="paletteOpen = false" @execute="executeMeshCommand" />

    <div class="mesh-body">
      <input ref="fileInput" type="file" :accept="MESH_IMPORT_ACCEPT" hidden @change="importMesh" />
      <div class="pane-tools" role="toolbar" :aria-label="label('Инструменты Mesh', 'Mesh tools')">
        <VrControls :get-scene="vrSnapshot" :available="document.objects.some(object => object.visible && object.mesh.indices.length > 0)" :locale="locale" />
        <button type="button" class="tool-icon" :title="label('Куб', 'Cube')" :aria-label="label('Куб', 'Cube')" @click="addPrimitive('box')"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7M12 11v10"/></svg></button>
        <button type="button" class="tool-icon" :title="label('UV-сфера', 'UV Sphere')" :aria-label="label('UV-сфера', 'UV Sphere')" @click="addPrimitive('sphere')"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 3a9 9 0 1 0 0 18 9 9 0 0 0 0-18zM3 12h18M12 3c-3 2.5-3 15.5 0 18M12 3c3 2.5 3 15.5 0 18"/></svg></button>
        <button type="button" class="tool-icon" :title="label('Импорт сетки', 'Import mesh')" :aria-label="label('Импорт сетки', 'Import mesh')" @click="fileInput?.click()"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M12 21V9M6 15l6-6 6 6M4 5h16"/></svg></button>
        <span class="tool-divider" aria-hidden="true"></span>
        <button type="button" class="tool-icon" :title="label('Объект · 1', 'Object · 1')" :aria-label="label('Объект · 1', 'Object · 1')" :aria-pressed="selectMode==='object'" @click="selectMode='object'"><svg width="16" height="16" viewBox="0 0 24 24" fill="currentColor" fill-opacity="0.35" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7M12 11v10"/></svg></button>
        <button type="button" class="tool-icon" :title="label('Вершина · 2', 'Vertex · 2')" :aria-label="label('Вершина · 2', 'Vertex · 2')" :aria-pressed="selectMode==='vertex'" @click="selectMode='vertex'"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7"/><path d="M4 4.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5zM20 4.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5zM12 8.5a2.5 2.5 0 1 0 0 5 2.5 2.5 0 0 0 0-5z" fill="currentColor"/></svg></button>
        <button type="button" class="tool-icon" :title="label('Ребро · 3', 'Edge · 3')" :aria-label="label('Ребро · 3', 'Edge · 3')" :aria-pressed="selectMode==='edge'" @click="selectMode='edge'"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4zM4 7v10l8 4 8-4V7"/><path d="M12 11v10" stroke-width="3.5"/></svg></button>
        <button type="button" class="tool-icon" :title="label('Грань · 4', 'Face · 4')" :aria-label="label('Грань · 4', 'Face · 4')" :aria-pressed="selectMode==='face'" @click="selectMode='face'"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linejoin="round" aria-hidden="true"><path d="M4 7l8-4 8 4-8 4z" fill="currentColor" fill-opacity="0.45"/><path d="M4 7v10l8 4 8-4V7M12 11v10"/></svg></button>
        <span class="tool-divider" aria-hidden="true"></span>
        <button type="button" class="tool-icon" :title="label('Сбросить вид', 'Reset view')" :aria-label="label('Сбросить вид', 'Reset view')" @click="resetView()"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 12a9 9 0 1 0 3-6.7M3 4v5h5"/></svg></button>
        <button type="button" class="tool-icon" :title="label('Сетка', 'Grid')" :aria-label="label('Сетка', 'Grid')" :aria-pressed="floorVisible" @click="floorVisible = !floorVisible"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M3 9h18M3 15h18M9 3v18M15 3v18M3 3h18v18H3z"/></svg></button>
        <span class="tool-divider" aria-hidden="true"></span>
        <button type="button" class="tool-icon" :title="label('Панель свойств', 'Properties panel')" :aria-label="label('Панель свойств', 'Properties panel')" :aria-pressed="propsOpen" @click="propsOpen = !propsOpen"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round" aria-hidden="true"><path d="M4 8h10M18 8h2M4 16h4M12 16h8"/><circle cx="16" cy="8" r="2"/><circle cx="10" cy="16" r="2"/></svg></button>
        <span class="subtle">{{ stats ? `${stats.vertices}v · ${stats.edges}e · ${stats.faces}f · ${stats.closed ? label('замкнут', 'closed') : label('открыт', 'open')}` : label('ЛКМ: вращение · Shift: панорама · колесо: масштаб', 'LMB: orbit · Shift: pan · wheel: zoom') }}</span>
        <span v-if="manifoldTaskPending" class="manifold-progress" role="status" aria-label="manifold-inspection"><progress :value="manifoldTaskProgress?.completed ?? 0" :max="manifoldTaskProgress?.total ?? 1" :aria-label="label('Прогресс проверки manifold','Manifold inspection progress')"></progress>{{ label('Проверка manifold', 'Checking manifold topology') }} · {{ manifoldTaskProgress?.completed ?? 0 }}/{{ manifoldTaskProgress?.total ?? 1 }}<button type="button" :aria-label="label('Отменить проверку manifold','Cancel manifold inspection')" @click="cancelManifoldTask">{{ label('Отмена', 'Cancel') }}</button></span>
        <p v-if="error" class="error" role="alert">{{ error }}</p>
        <span v-else-if="!kernelReady" role="status">{{ label('Подготовка геометрии…', 'Preparing geometry…') }}</span>
        <span v-else-if="meshTaskPending" :class="meshTaskPresentation.className" role="status"><progress :value="meshTaskProgress?.completed ?? 0" :max="meshTaskProgress?.total ?? meshTaskPresentation.total" :aria-label="meshTaskPresentation.progressLabel"></progress>{{ meshTaskStage }}<button type="button" :aria-label="meshTaskPresentation.cancelLabel" @click="cancelMeshTask">{{ label('Отмена', 'Cancel') }}</button></span>
        <p v-else-if="importNotice" class="import-report" role="status">{{ importNotice }} <button type="button" :aria-label="label('Закрыть отчёт импорта', 'Dismiss import report')" @click="importNotice = ''">×</button></p>
        <p v-else-if="booleanNotice" class="boolean-report" role="status">{{ booleanNotice }} <button type="button" :aria-label="label('Закрыть отчёт булевой операции', 'Dismiss Boolean report')" @click="booleanNotice = ''">×</button></p>
        <p v-else-if="repairNotice" class="repair-report" role="status">{{ repairNotice }} <button type="button" :aria-label="label('Закрыть отчёт repair', 'Dismiss repair report')" @click="repairNotice = ''">×</button></p>
      </div>
      <ModelingGridControls :locale="locale" />
      <WorkspaceActionBar v-if="selected" :label="label('Действия над объектом', 'Object actions')">
        <span class="ws-name">{{ selected.name }}</span>
        <span class="ws-sep" aria-hidden="true"></span>
        <template v-if="selectMode === 'face'">
          <button type="button" @click="applyExtrude">{{ label('Выдавить', 'Extrude') }}</button>
          <button type="button" @click="applyInset">Inset</button>
        </template>
        <button type="button" :aria-pressed="propsOpen" @click="propsOpen = !propsOpen">{{ label('Свойства', 'Properties') }}</button>
        <span class="ws-sep" aria-hidden="true"></span>
        <button type="button" class="danger" @click="removeSelected">{{ label('Удалить', 'Delete') }}</button>
      </WorkspaceActionBar>
      <div class="mesh-stage">
      <div class="mesh-stage-view">
      <div class="mesh-view-wrap">
      <div v-if="!scene.length" class="mesh-empty"><strong>{{ label('Объектов пока нет', 'No objects yet') }}</strong><span>{{ label('Добавьте куб или сферу, импортируйте сетку или соберите модель в Code: сцена переносится сюда автоматически.', 'Add a cube or sphere, import a mesh, or build a model in Code: the scene carries over here automatically.') }}</span></div>
      <svg
        class="mesh-view"
        viewBox="-100 -100 200 200"
        @pointerdown="onPointerDown"
        @pointermove="onPointerMove"
        @pointerup="onPointerUp"
        @pointercancel="onPointerUp"
        @click.capture="onClickCapture"
        @contextmenu.prevent
        @wheel.prevent="onWheel"
      >
        <g :transform="`translate(${center[0]},${center[1]}) scale(${view / 100})`">
          <ModelingFloorGrid v-if="floorVisible" :camera="camera" :size="20000 / view" :center="[-center[0] * 100 / view, -center[1] * 100 / view]" flip-y />
          <g>
            <polygon
              v-for="tri in sceneFaces"
              :key="`${tri.objectId}-${tri.face}`"
              :points="tri.points"
              :class="{ face: true, selected: tri.objectId === selection && selectedFaces.includes(tri.face) }"
              @pointerdown="startElementDrag($event, tri.objectId, 'face', tri.face)"
              @click.stop="selection = tri.objectId; selectMode === 'face' && togglePick('face', tri.face)"
            />
          </g>
          <g v-for="object in scene" :key="object.id">
            <line
              v-for="edge in object.edges"
              :key="`${object.id}-e-${edge.id}`"
              :x1="edge.x1"
              :y1="edge.y1"
              :x2="edge.x2"
              :y2="edge.y2"
              :class="{ edge: true, selected: object.id === selection && selectedEdges.includes(edge.id) }"
              @pointerdown="startElementDrag($event, object.id, 'edge', edge.id)"
              @click.stop="selection = object.id; togglePick('edge', edge.id)"
            />
            <circle
              v-for="vert in object.verts"
              :key="`${object.id}-v-${vert.id}`"
              :cx="vert.x"
              :cy="vert.y"
              r="0.6"
              :class="{ vert: true, selected: selectedVerts.includes(vert.id) }"
              @pointerdown="startElementDrag($event, object.id, 'vertex', vert.id)"
              @click.stop="selection = object.id; togglePick('vertex', vert.id)"
            />
          </g>
          <template v-for="object in scene" :key="`${object.id}-manifold-defects`">
            <g v-if="object.defects.boundaryEdges.length || object.defects.nonManifoldEdges.length || object.defects.nonManifoldVertices.length" class="manifold-overlay" :data-object="object.id" pointer-events="none">
              <line v-for="edge in object.defects.boundaryEdges" :key="`boundary-${edge.id}`" data-defect="boundary" class="manifold-boundary" :x1="edge.x1" :y1="edge.y1" :x2="edge.x2" :y2="edge.y2" />
              <line v-for="edge in object.defects.nonManifoldEdges" :key="`non-manifold-${edge.id}`" data-defect="non-manifold" class="manifold-non-manifold" :x1="edge.x1" :y1="edge.y1" :x2="edge.x2" :y2="edge.y2" />
              <circle v-for="vertex in object.defects.nonManifoldVertices" :key="`pinched-${vertex.id}`" data-defect="pinched-vertex" class="manifold-pinched" :cx="vertex.x" :cy="vertex.y" r="1.2" />
            </g>
          </template>
        </g>
      </svg>
      <div v-if="scene.some(object => object.defects.boundaryEdges.length || object.defects.nonManifoldEdges.length || object.defects.nonManifoldVertices.length)" class="manifold-legend" role="note">
        <span v-if="scene.some(object => object.defects.boundaryEdges.length)"><i class="boundary"></i>{{ label('Граница', 'Boundary') }}</span>
        <span v-if="scene.some(object => object.defects.nonManifoldEdges.length)"><i class="non-manifold"></i>{{ label('Не manifold', 'Non-manifold') }}</span>
        <span v-if="scene.some(object => object.defects.nonManifoldVertices.length)"><i class="pinched"></i>{{ label('Pinched-вершины', 'Pinched vertices') }}</span>
      </div>
      </div>
      </div>
      <aside class="side-dock" :class="{ collapsed: !dockOpen }" :aria-label="label('Панель', 'Panel')">
        <div class="dock-rail" role="tablist" aria-orientation="vertical">
          <button type="button" role="tab" :aria-selected="dockOpen && dockTab === 'scene'" :aria-label="label('Сцена', 'Scene')" :title="label('Сцена', 'Scene')" @click="dockTab = 'scene'; dockOpen = true"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M4 6h16M4 12h16M4 18h16"/></svg></button>
          <button type="button" role="tab" :aria-selected="dockOpen && dockTab === 'props'" :aria-label="label('Свойства', 'Properties')" :title="label('Свойства', 'Properties')" @click="dockTab = 'props'; dockOpen = true"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" aria-hidden="true"><path d="M4 8h10M18 8h2M4 16h4M12 16h8M16 6a2 2 0 1 0 0 4 2 2 0 0 0 0-4zM10 14a2 2 0 1 0 0 4 2 2 0 0 0 0-4z"/></svg></button>
          <span class="dock-rail-spacer"></span>
          <button type="button" :aria-label="dockOpen ? label('Свернуть панель', 'Collapse panel') : label('Развернуть панель', 'Expand panel')" :title="dockOpen ? label('Свернуть панель', 'Collapse panel') : label('Развернуть панель', 'Expand panel')" @click="dockOpen = !dockOpen"><svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" aria-hidden="true"><path :d="dockOpen ? 'M9 6l6 6-6 6' : 'M15 6l-6 6 6 6'"/></svg></button>
        </div>
        <div v-if="dockOpen" class="dock-body">
          <template v-if="dockTab === 'scene'">
            <div class="dock-heading">{{ label('Сцена', 'Scene') }} <span>{{ document.objects.length }}</span></div>
            <ul class="scene-list">
              <li v-for="object in document.objects" :key="object.id">
                <div class="scene-object">
                  <button type="button" class="scene-object-select" :aria-label="object.name" :aria-pressed="selection === object.id" @click="selection = object.id; selectedVerts = []; selectedFaces = []; selectedEdges = []"><span class="dot body"></span>{{ object.name }}<small>{{ object.mesh.indices.length / 3 }} △</small></button>
                  <button v-if="manifoldReports.get(object.id) && !manifoldReports.get(object.id)!.isManifold" type="button" class="manifold-warning" :aria-label="`${label('Дефекты manifold', 'Manifold defects')}: ${manifoldDefects(object.id).join(', ')}`" :title="manifoldDefects(object.id).join(', ')" :aria-expanded="expandedManifoldObject === object.id" @click.stop="expandedManifoldObject = expandedManifoldObject === object.id ? '' : object.id">⚠</button>
                </div>
                <ul v-if="expandedManifoldObject === object.id && manifoldReports.get(object.id) && !manifoldReports.get(object.id)!.isManifold" class="manifold-defects" :aria-label="label('Категории дефектов manifold', 'Manifold defect categories')">
                  <li v-for="defect in manifoldDefects(object.id)" :key="defect">{{ defect }}</li>
                </ul>
              </li>
              <li v-if="!document.objects.length" class="scene-empty">{{ label('Объектов нет. Добавьте куб или сферу, импортируйте сетку или соберите модель в Code.', 'No objects. Add a cube or sphere, import a mesh, or build a model in Code.') }}</li>
            </ul>
          </template>
          <template v-else>
            <div class="dock-heading">{{ selected?.name || label('Свойства', 'Properties') }}</div>
            <div class="dock-props">
        <section v-if="selected">
          <h3>{{ label('Трансформ', 'Transform') }} G / R / S</h3>
          <label>ΔX <input v-model.number="dx" type="number" step="0.1" /></label>
          <label>ΔY <input v-model.number="dy" type="number" step="0.1" /></label>
          <label>ΔZ <input v-model.number="dz" type="number" step="0.1" /></label>
          <label>{{ label('Угол', 'Angle') }} <input v-model.number="angle" type="number" step="1" /></label>
          <label>{{ label('Масштаб', 'Scale') }} <input v-model.number="scale" type="number" step="0.05" min="0.01" /></label>
          <template v-if="selectMode === 'vertex'">
            <label><span>O · {{ label('Пропорционально', 'Proportional') }}</span><input v-model="proportionalEdit" type="checkbox" /></label>
            <label v-if="proportionalEdit">{{ label('Радиус влияния', 'Influence radius') }} <input v-model.number="proportionalRadius" type="number" step="0.5" min="0.01" /></label>
            <p v-if="proportionalEdit" class="tool-hint">{{ label('Плавный спад по расстоянию в пространстве.', 'Smooth falloff by spatial distance.') }}</p>
          </template>
          <button type="button" @click="transformSelected">{{ label('Применить', 'Apply') }}</button>
        </section>
        <section v-if="selected && selectMode === 'face'">
          <h3>Edit · Face</h3>
          <label>E {{ label('Выдавить', 'Extrude') }} <input v-model.number="extrudeDistance" type="number" step="0.1" /></label>
          <button type="button" @click="applyExtrude">Extrude</button>
          <label>I {{ label('Inset', 'Inset') }} <input v-model.number="insetAmount" type="number" step="0.05" min="0.01" /></label>
          <button type="button" @click="applyInset">Inset</button>
          <button type="button" @click="applySubdivide">Subdivide</button>
          <button type="button" @click="applyDeleteFaces">Delete faces</button>
          <button type="button" @click="applyFlip">Flip normals</button>
        </section>
        <section v-if="selected && selectMode === 'edge'">
          <h3>Edit · Edge</h3>
          <p class="tool-hint">{{ label('Клик выбирает рёбра; Shift вращает вид.', 'Click edges to select; Shift orbits the view.') }}</p>
          <button type="button" :disabled="!selectedEdges.length" @click="applyKnife">K · {{ label('Разрез по середине', 'Knife at midpoint') }}</button>
        </section>
        <section v-if="selected">
          <h3>{{ label('Топология', 'Topology') }}</h3>
          <label>{{ label('Слияние', 'Merge by distance') }} <input v-model.number="mergeDistance" type="number" step="0.001" min="0.0001" /></label>
          <button type="button" @click="applyMerge">Merge</button>
          <button type="button" :disabled="meshTaskPending" @click="applyRepair">{{ label('Починить меш · Full', 'Repair mesh · Full') }}</button>
          <button type="button" @click="applySeparate">Separate faces</button>
          <button type="button" @click="applyJoin">Join objects</button>
          <button type="button" @click="applySymmetrize">Symmetrize X</button>
          <h3>{{ label('Скульптинг', 'Sculpt') }}</h3>
          <label>{{ label('Кисть', 'Brush') }}
            <select v-model="brushKind">
              <option v-for="k in SCULPT_KINDS" :key="k" :value="k">{{ k }}</option>
            </select>
          </label>
          <label>Falloff
            <select v-model="brushFalloff">
              <option v-for="f in SCULPT_FALLOFFS" :key="f" :value="f">{{ f }}</option>
            </select>
          </label>
          <label>R <input v-model.number="brushRadius" type="number" step="0.5" min="0.1" /></label>
          <label>{{ brushIsFraction ? label('Доля', 'Amount') : label('Сила', 'Strength') }}
            <input v-model.number="brushStrength" type="number" :step="brushIsFraction ? 0.05 : 0.1" :min="brushIsFraction ? 0 : undefined" :max="brushIsFraction ? 1 : undefined" />
          </label>
          <label title="Mirror X"><input v-model="brushMirror[0]" type="checkbox" /> X</label>
          <label title="Mirror Y"><input v-model="brushMirror[1]" type="checkbox" /> Y</label>
          <label title="Mirror Z"><input v-model="brushMirror[2]" type="checkbox" /> Z</label>
          <button type="button" @click="applyBrush">{{ label('Мазок', 'Stroke') }}</button>
          <label>Twist <input v-model.number="twistAmount" type="number" step="0.05" /></label>
          <button type="button" @click="applyTwist">Twist deform</button>
          <button type="button" @click="applySubdivide">Subdivide all</button>
          <button type="button" @click="applyDuplicate">Duplicate</button>
          <button type="button" @click="removeSelected">Delete object</button>
        </section>
        <section v-if="document.objects.length > 1">
          <h3>Boolean</h3>
          <select v-model="booleanOp" :disabled="meshTaskPending">
            <option value="union">Union</option>
            <option value="difference">Difference</option>
            <option value="intersection">Intersection</option>
          </select>
          <select v-model="booleanTarget" :disabled="meshTaskPending">
            <option value="">{{ label('Второй объект…', 'Second object…') }}</option>
            <option v-for="object in document.objects.filter(o => o.id !== selection)" :key="object.id" :value="object.id">{{ object.name }}</option>
          </select>
          <button type="button" :disabled="meshTaskPending" @click="applyBoolean">{{ label('Выполнить', 'Run') }}</button>
        </section>
        <p v-if="!selected" class="tool-hint">{{ label('Выберите объект в полосе снизу или кликом по холсту.', 'Select an object in the strip below or by clicking the canvas.') }}</p>
            </div>
          </template>
        </div>
      </aside>
      </div>
    </div>
  </div>
</template>

<style scoped src="./styles/mesh-modeler.css"></style>
