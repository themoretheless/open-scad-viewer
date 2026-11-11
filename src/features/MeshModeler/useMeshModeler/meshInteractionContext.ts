import type {OrbitCamera} from '../../../services/directModelingTools'
import {createMeshCommands} from '.././useMeshModeler/meshCommands'
import VrControls from '../../../components/VrControls.svt'
import { prepareVrPolygons } from '../../../render/vrScene'
import { computed, nextTick, onUnmounted, ref, shallowRef, watch } from 'svet'
import { stringifyMeshJson } from '../../../exchange/meshJson'
import ModelingFloorGrid from '../../../components/ModelingFloorGrid.svt'
import ModelingGridControls from '../../../components/ModelingGridControls.svt'
import WorkspaceActionBar from '../../../components/WorkspaceActionBar.svt'
import CommandPalette from '../../../components/CommandPalette.svt'
import type { PaletteCommand } from '../../../services/commandSearch'
import { SCULPT_FALLOFFS, SCULPT_KINDS, buildSculptBrush, isFractionalSculptKind, type SculptFalloff, type SculptKind } from '../../../kernel/geometryEditing'
import { meshCentroid, sculptMesh } from '../../../kernel/meshEditing'
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
} from '../../../kernel/meshEditing'
import { createSolidPreviewWorker } from '../../../kernel/solidPreviewWorker'
import { useMainSolidTask } from '../.././useMainSolidTask'
import { defaultDirectCamera, projectDirectPoint } from '../../../services/directModelingTools'
import { projectMeshModelerObject,cachedMeshModelerStats } from '../../../services/meshModelerProjection'
import { isGeometryKernelReady, warmGeometryKernel } from '../../../kernel/geometry/kernel'
import { storageGet, storageSet } from '../../../services/safeStorage'
import { MESH_IMPORT_ACCEPT, stripMeshExtension } from '../../../exchange/meshImport'
import { manifoldPreferenceSnapshot } from '../../../kernel/manifoldPreferences'
import ManifoldSettings from '../../../components/ManifoldSettings.svt'
import { formatMeshImportNotice } from '../../../exchange/meshImportReport'
import { formatMeshBooleanNotice } from '../../../kernel/meshBooleanReport'
import { formatMeshRepairNotice } from '../../../kernel/meshRepairReport'
import { meshManifoldDefectLabels } from '../../../kernel/meshManifoldDefects'
import { MESH_EXPORT_FORMATS, MESH_FORMAT_LABELS, type MeshExportFormat } from '../../../exchange/meshConvert'
import { downloadBytes } from '../../../exchange/downloadArtifact'
import type { PolygonMesh } from '../../../kernel/geometry/polygon'
import type { KernelManifoldReport } from '../../../kernel/geometry/manifoldAnalysis'
import type {MeshModelerProps,MeshModelerEmit} from '../useMeshModeler'
import type {ShallowRef,Ref} from 'svet'
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
