import type {OrbitCamera} from '../../../services/directModelingTools'
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
import type {ShallowRef,Ref} from 'vue'
export interface MeshInteractionContext {
  document: ShallowRef<MeshWorkspaceDocument, MeshWorkspaceDocument>
  selectedVerts: Ref<number[], number[]>
  selectedFaces: Ref<number[], number[]>
  selectedEdges: Ref<number[], number[]>
  edgeListCache: WeakMap<PolygonMesh, [number, number][]>
  camera: Ref<{ yaw: number; pitch: number; }, OrbitCamera | { yaw: number; pitch: number; }>
  selectMode: Ref<MeshSelectMode, MeshSelectMode>
  manifoldReports: ShallowRef<ReadonlyMap<string, KernelManifoldReport>, ReadonlyMap<string, KernelManifoldReport>>
  view: Ref<number, number>
  selection: Ref<string, string>
  elementDrag: { objectId: string; vertexIds: number[]; base: PolygonMesh; x: number; y: number; moved: boolean; svg: SVGSVGElement; } | null
  dragSwallowsClick: boolean
  proportionalEdit: Ref<boolean, boolean>
  proportionalRadius: Ref<number, number>
  history: MeshHistory
  error: Ref<string, string>
  run: (fn: () => void) => void
  commit: (next: MeshWorkspaceDocument, operation?: string) => void
  orbit: { x: number; y: number; yaw: number; pitch: number; pan: boolean; center: [number, number]; moved: boolean; } | null
  center: Ref<[number, number], [number, number]>
}
