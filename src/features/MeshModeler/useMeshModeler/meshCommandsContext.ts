import type {MeshObject} from '../../../services/meshEditing'
import type {MainSolidJob,MainSolidResults} from '../../../services/mainSolidProtocol'
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
import type {Ref,ComputedRef,ShallowRef} from 'vue'
export interface MeshCommandsContext {
  run: (fn: () => void) => void
  history: MeshHistory
  commit: (next: MeshWorkspaceDocument, operation?: string) => void
  selection: Ref<string, string>
  selectMode: Ref<MeshSelectMode, MeshSelectMode>
  selected: ComputedRef<MeshObject | undefined>
  selectedVerts: Ref<number[], number[]>
  selectedEdges: Ref<number[], number[]>
  getEdgeList: (mesh: PolygonMesh) => Array<[number, number]>
  proportionalEdit: Ref<boolean, boolean>
  dx: Ref<number, number>
  dy: Ref<number, number>
  dz: Ref<number, number>
  proportionalRadius: Ref<number, number>
  angle: Ref<number, number>
  scale: Ref<number, number>
  selectedFaces: Ref<number[], number[]>
  label: (a: string, b: string) => string
  extrudeDistance: Ref<number, number>
  insetAmount: Ref<number, number>
  mergeDistance: Ref<number, number>
  meshTaskPending: Ref<boolean, boolean>
  error: Ref<string, string>
  repairNotice: Ref<string, string>
  importNotice: Ref<string, string>
  booleanNotice: Ref<string, string>
  document: ShallowRef<MeshWorkspaceDocument, MeshWorkspaceDocument>
  runMeshTask: <K extends MainSolidJob["kind"]>(job: Extract<MainSolidJob, { kind: K; }>, accept: (result: MainSolidResults[K]) => void, reject: (error: unknown) => void) => Promise<boolean>
  ru: ComputedRef<boolean>
  booleanTarget: Ref<string, string>
  brushKind: Ref<SculptKind, SculptKind>
  brushRadius: Ref<number, number>
  brushStrength: Ref<number, number>
  brushFalloff: Ref<SculptFalloff, SculptFalloff>
  brushMirror: Ref<[boolean, boolean, boolean], [boolean, boolean, boolean]>
  twistAmount: Ref<number, number>
  booleanOp: Ref<"union" | "difference" | "intersection", "union" | "difference" | "intersection">
  exportFormat: Ref<"stl" | "stl_binary" | "3mf" | "obj" | "ply" | "off" | "amf", "stl" | "stl_binary" | "3mf" | "obj" | "ply" | "off" | "amf">
  props: MeshModelerProps
}
