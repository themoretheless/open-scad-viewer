import type { GpuCsgPrimitive } from './gpuCsgTypes'
import { generateUnitCube, generateUnitCylinder, generateUnitSphere, type GeometryData } from './gpuCsgPrimitives'


/* ── WebGPU Geometry Cache ────────────────────────────── */

export interface CachedGpuMesh {
  vb: GPUBuffer
  ib: GPUBuffer
  indexCount: number
}

export class GpuCsgGeometryCache {
  private cube: CachedGpuMesh | null = null
  private cylinder: CachedGpuMesh | null = null
  private sphere: CachedGpuMesh | null = null
  private customMeshes = new Map<string, CachedGpuMesh>()

  private uploadMesh(dev: GPUDevice, data: GeometryData): CachedGpuMesh {
    const vb = dev.createBuffer({
      size: data.vertices.byteLength,
      usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
      mappedAtCreation: true,
    })
    new Float32Array(vb.getMappedRange()).set(data.vertices)
    vb.unmap()

    const ib = dev.createBuffer({
      size: data.indices.byteLength,
      usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST,
      mappedAtCreation: true,
    })
    new Uint32Array(ib.getMappedRange()).set(data.indices)
    ib.unmap()

    return { vb, ib, indexCount: data.indices.length }
  }

  getPrimitiveMesh(dev: GPUDevice, prim: GpuCsgPrimitive): CachedGpuMesh {
    if (prim.kind === 'cube') {
      if (!this.cube) this.cube = this.uploadMesh(dev, generateUnitCube(true))
      return this.cube
    }
    if (prim.kind === 'cylinder') {
      if (!this.cylinder) this.cylinder = this.uploadMesh(dev, generateUnitCylinder(32, true))
      return this.cylinder
    }
    if (prim.kind === 'sphere') {
      if (!this.sphere) this.sphere = this.uploadMesh(dev, generateUnitSphere(32, 16))
      return this.sphere
    }

    // Polyhedron / custom mesh
    let cached = this.customMeshes.get(prim.id)
    if (!cached && prim.vertices && prim.indices) {
      cached = this.uploadMesh(dev, { vertices: prim.vertices, indices: prim.indices })
      this.customMeshes.set(prim.id, cached)
    }
    if (cached) return cached

    // Fallback to cube
    if (!this.cube) this.cube = this.uploadMesh(dev, generateUnitCube(true))
    return this.cube
  }

  destroy() {
    this.cube?.vb.destroy()
    this.cube?.ib.destroy()
    this.cylinder?.vb.destroy()
    this.cylinder?.ib.destroy()
    this.sphere?.vb.destroy()
    this.sphere?.ib.destroy()
    for (const mesh of this.customMeshes.values()) {
      mesh.vb.destroy()
      mesh.ib.destroy()
    }
    this.customMeshes.clear()
    this.cube = null
    this.cylinder = null
    this.sphere = null
  }
}
