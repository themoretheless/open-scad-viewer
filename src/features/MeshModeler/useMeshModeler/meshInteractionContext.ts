import type {OrbitCamera} from '../../../app/directModelingTools'
import {
  MeshHistory,
  type MeshSelectMode,
  type MeshWorkspaceDocument,
} from '../../../kernel/meshEditing'
import type { PolygonMesh } from '../../../kernel/geometry/polygon'
import type { KernelManifoldReport } from '../../../kernel/geometry/manifoldAnalysis'
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
