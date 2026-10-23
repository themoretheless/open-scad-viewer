import type { MeshData, MeshProvenanceRun } from '../core/mesh'
import type { GeometryAssetId } from '../core/scene'
import type { Aabb3, Mat4 } from '../services/math3d'
import type { MeshBvh } from '../services/meshBvh'
import type { FaceTriangleIndex } from '../services/meshSelectionOverlay'
import type { MaterialDef, ShadingModel } from '../services/rendererContracts'

/* ── GPU mesh handle ──────────────────────────────── */

export interface GMesh {
  entityId?: MeshData['entityId']
  /**
   * Vertex morph state, blended on the GPU: the vertex shader mixes the
   * position attribute toward `from` (vertex slot 1) by `ob.morph.x`, so the
   * CPU never rewrites vertex buffers per frame.
   */
  morph?: { from: Float32Array; target: Float32Array; started: number; matrix?: (t: number) => Mat4; currentMatrix?: Mat4 }
  /** GPU copy of `morph.from` (positions only, stride 3); capacity-grown. */
  morphVB: GPUBuffer | null
  morphVBCapacity: number
  /** Buffer bound at vertex slot 1: morph source while morphing, else the shared dummy. */
  morphSlot: GPUBuffer | null
  nativeGeometry?: MeshData['nativeGeometry']
  faceIdsAuthoritative?: boolean
  assetId?: GeometryAssetId
  vb: GPUBuffer; ib: GPUBuffer; ic: number
  ub: GPUBuffer; bg: GPUBindGroup
  edgeIB: GPUBuffer | null; edgeIC: number
  vertices: Float32Array; indices: Uint32Array
  edgeIndices: Uint32Array; faceIds: Uint32Array; bvh: MeshBvh
  provenance: MeshProvenanceRun[]
  transform: Mat4; inverseTransform: Mat4
  color: [number, number, number, number]
  alpha: number
  localBounds: Aabb3
  worldBounds: Bounds
  visible: boolean
  /** Lazily built faceId → triangles index; undefined = not built yet, null = unavailable. */
  faceIndex?: FaceTriangleIndex | null
  /** Last style vector written to the uniform buffer (alpha, selected, edge, hovered). */
  styleAlpha: number
  styleSelected: number
  styleEdge: number
  styleHovered: number
  /** Presentation material from MeshData; undefined = legacy Phong defaults. */
  material?: MaterialDef
  /** Resolved shading model; drives mesh pipeline selection at draw time. */
  shadingModel: ShadingModel
}

/** Shared empty list so render() never allocates when there are no ghosts. */
export const NO_GHOST_MESHES: GMesh[] = []

/** Persistent capacity-grown vertex buffer for transient overlay geometry. */
export interface OverlaySlot {
  buffer: GPUBuffer | null
  /** Allocated size in bytes; may exceed the currently written data. */
  capacity: number
  /** Vertices to draw this frame (stride 28 bytes). */
  count: number
}

export interface Bounds {
  center: [number, number, number]
  radius: number
  min: [number, number, number]
  max: [number, number, number]
}

export const FOV_Y = Math.PI / 4
export const DEFAULT_DISTANCE = 50
export const MAX_ORBIT_PITCH = Math.PI / 2 - 0.001
export const MAX_DPR = 2
export const DEFAULT_GRID_STEP = 10
/** Minimum plane half-extent and Z-axis length in model units. */
export const GRID_MIN_EXTENT = 200
/** Grid lines fade out this many orbit distances away from the eye. */
export const GRID_FADE_DISTANCES = 6
export const CLICK_MOVE_THRESHOLD = 3
export const MAX_EDGE_BUFFER_BYTES = 32 * 1024 * 1024
export const MAX_OVERLAY_BUFFER_BYTES = 16 * 1024 * 1024
export const MAX_DEPTH_CANDIDATES = 32
export const MAX_DEPTH_CONTINUATIONS = 256
export const WHEEL_HISTORY_IDLE_MS = 250

export interface WebGPUFrameDiagnostics {
  opaqueMeshCount: number
  transparentMeshCount: number
  edgeMeshCount: number
  totalMeshCount: number
  deviceLost: boolean
}

/**
 * Physical/logical GPU memory subsystem classification:
 * - `'uma'` (Unified Memory Architecture, e.g. Apple Silicon Metal `StorageModeShared`):
 *   CPU and GPU share a single physical coherent DRAM pool; primary buffers can be mapped directly
 *   (`MAP_WRITE | STORAGE`) without intermediate PCIe staging copies.
 * - `'duma'` (Discrete / Decoupled Memory Architecture, e.g. NVIDIA CUDA / PCIe Vulkan / DX12 / WebGPU):
 *   CPU host RAM and GPU device VRAM are separated; uploads use a staging ring buffer (`queue.writeBuffer` / DMA)
 *   into device-local VRAM (`StorageModePrivate`).
 */
export type GpuMemoryArchitecture = 'uma' | 'duma'

export interface GpuMemoryArchitectureProfile {
  architecture: GpuMemoryArchitecture
  label: string
  unifiedPhysicalRam: boolean
  zeroCopyPrimaryUpload: boolean
  requiresExplicitStagingRing: boolean
  preferredUploadStrategy: 'direct_mapped_primary_buffer' | 'staging_ring_to_device_local_vram'
  preferredReadbackStrategy: 'shared_coherent_map_read' | 'dma_copy_to_staging_then_map'
}

export function getGpuMemoryArchitectureProfile(
  architecture: GpuMemoryArchitecture,
): GpuMemoryArchitectureProfile {
  if (architecture === 'uma') {
    return {
      architecture: 'uma',
      label: 'UMA (Unified Memory Architecture)',
      unifiedPhysicalRam: true,
      zeroCopyPrimaryUpload: true,
      requiresExplicitStagingRing: false,
      preferredUploadStrategy: 'direct_mapped_primary_buffer',
      preferredReadbackStrategy: 'shared_coherent_map_read',
    }
  }
  return {
    architecture: 'duma',
    label: 'dUMA (Discrete Memory Architecture)',
    unifiedPhysicalRam: false,
    zeroCopyPrimaryUpload: false,
    requiresExplicitStagingRing: true,
    preferredUploadStrategy: 'staging_ring_to_device_local_vram',
    preferredReadbackStrategy: 'dma_copy_to_staging_then_map',
  }
}

export interface GpuExecutionTopologyReport {
  backend: 'metal' | 'cuda' | 'webgpu'
  memoryArchitecture: GpuMemoryArchitecture
  memoryProfile: GpuMemoryArchitectureProfile
  scalarUnit: string
  simdWidth: number
  simdGroup: string
  threadBlock: {
    label: string
    maxThreadsPerBlock: number
    optimal1dWorkgroupSize: number
    optimal2dTileSize: [number, number]
  }
  localSram: {
    label: string
    defaultCapacityBytes: number
  }
  matrixCoprocessor: string
  matrixTileShape: [number, number, number]
}

/**
 * Returns the 5-level GPU hardware execution topology mapping + UMA/dUMA memory profile:
 * 1. Scalar execution unit (`ALU / Execution Pipeline` vs `CUDA Core / Streaming Processor`)
 * 2. 32-thread lockstep execution unit (`SIMD-group (32 threads)` vs `Warp (32 threads)`)
 * 3. Thread block / Threadgroup (`Threadgroup` vs `Thread Block`, max 1024 threads)
 * 4. On-chip programmable SRAM (`threadgroup memory` 32 KB vs `__shared__ memory` 48 KB)
 * 5. Matrix Coprocessor (`simdgroup_matrix (8x8)` vs `Tensor Cores (wmma 16x16)`)
 */
export function getGpuExecutionTopologyReport(
  backend: 'metal' | 'cuda' | 'webgpu' = 'metal',
): GpuExecutionTopologyReport {
  if (backend === 'metal') {
    const memoryProfile = getGpuMemoryArchitectureProfile('uma')
    return {
      backend: 'metal',
      memoryArchitecture: 'uma',
      memoryProfile,
      scalarUnit: 'ALU / Execution Pipeline',
      simdWidth: 32,
      simdGroup: 'SIMD-group (32 threads)',
      threadBlock: {
        label: 'Threadgroup',
        maxThreadsPerBlock: 1024,
        optimal1dWorkgroupSize: 128,
        optimal2dTileSize: [32, 8],
      },
      localSram: {
        label: 'threadgroup memory',
        defaultCapacityBytes: 32768,
      },
      matrixCoprocessor: 'simdgroup_matrix (8x8)',
      matrixTileShape: [8, 8, 8],
    }
  }
  const memoryProfile = getGpuMemoryArchitectureProfile('duma')
  return {
    backend,
    memoryArchitecture: 'duma',
    memoryProfile,
    scalarUnit: 'CUDA Core / Streaming Processor',
    simdWidth: 32,
    simdGroup: 'Warp (32 threads)',
    threadBlock: {
      label: backend === 'cuda' ? 'Thread Block (CTA)' : 'Workgroup',
      maxThreadsPerBlock: 1024,
      optimal1dWorkgroupSize: 256,
      optimal2dTileSize: [16, 16],
    },
    localSram: {
      label: backend === 'cuda' ? '__shared__ memory' : 'var<workgroup> shared memory',
      defaultCapacityBytes: backend === 'cuda' ? 49152 : 32768,
    },
    matrixCoprocessor: 'Tensor Cores (wmma 16x16)',
    matrixTileShape: [16, 16, 16],
  }
}
