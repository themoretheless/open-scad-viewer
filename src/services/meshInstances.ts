import type { Mat4 } from './math3d'
import type { BundleMesh } from './meshDrawBundle'
import { OBJECT_UNIFORM_LAYOUT, instancedObjectShader } from './shaders'

export interface InstanceMesh extends BundleMesh {
  readonly transform: Mat4
  readonly inverseTransform: Mat4
  readonly color: readonly number[]
  readonly styleAlpha: number
  readonly styleSelected: number
  readonly styleEdge: number
  readonly styleHovered: number
}

interface Group { mesh: InstanceMesh; start: number; count: number; ib: GPUBuffer; ic: number }

/** Per-instance Obj record: model (16) + nmat (16) + color (4) + style (4) + morph (4) floats. */
const INSTANCE_FLOATS = OBJECT_UNIFORM_LAYOUT.floats
const INSTANCE_BYTES = OBJECT_UNIFORM_LAYOUT.bytes
const STYLE_FLOAT_OFFSET = OBJECT_UNIFORM_LAYOUT.styleFloatOffset
const MORPH_FLOAT_OFFSET = OBJECT_UNIFORM_LAYOUT.morphFloatOffset

/** Consecutive equal geometry only: preserves source order and transparency order. */
export class MeshInstances {
  private buffer: GPUBuffer | null = null
  private group: GPUBindGroup | null = null
  private data = new Float32Array(0)
  private meshes: InstanceMesh[] = []
  private groups: Group[] = []
  private edges = false
  private eligible = false
  private uploadPending = false
  private edgeBuffers: Array<GPUBuffer | null> = []
  private edgeCounts: number[] = []
  private morphDummy: GPUBuffer | null = null

  clear() {
    this.buffer?.destroy()
    this.buffer = null
    this.group = null
    this.data = new Float32Array(0)
    this.meshes.length = this.groups.length = 0
    this.eligible = false
    this.uploadPending = false
    this.edgeBuffers.length = this.edgeCounts.length = 0
  }

  draw(pass: GPURenderPassEncoder, device: GPUDevice, layout: GPUBindGroupLayout,
    pipeline: GPURenderPipeline, scene: GPUBindGroup, meshes: readonly InstanceMesh[], edges = false): boolean {
    if (meshes.length < 16 || meshes.length * INSTANCE_BYTES > device.limits.maxStorageBufferBindingSize) {
      if (this.meshes.length) this.clear()
      return false
    }
    let changed = this.meshes.length !== meshes.length || this.edges !== edges
    for (let i = 0; !changed && i < meshes.length; i++) {
      changed = this.meshes[i] !== meshes[i] || (edges && (this.edgeBuffers[i] !== meshes[i].edgeIB || this.edgeCounts[i] !== meshes[i].edgeIC))
    }
    // Edge buffers can become available after publication. Recheck their identity.
    for (const group of this.groups) {
      if (group.ib !== (edges ? group.mesh.edgeIB : group.mesh.ib) || group.ic !== (edges ? group.mesh.edgeIC : group.mesh.ic)) changed = true
    }
    if (changed) {
      this.groups.length = 0
      for (let start = 0; start < meshes.length;) {
        const mesh = meshes[start], ib = (edges ? mesh.edgeIB : mesh.ib)!, ic = edges ? mesh.edgeIC : mesh.ic
        let end = start + 1
        while (end < meshes.length && meshes[end].vb === mesh.vb
          && (edges ? meshes[end].edgeIB : meshes[end].ib) === ib && (edges ? meshes[end].edgeIC : meshes[end].ic) === ic) end++
        this.groups.push({ mesh, start, count: end - start, ib, ic })
        start = end
      }
      this.meshes = meshes.slice()
      this.edgeBuffers = edges ? meshes.map(mesh => mesh.edgeIB) : []
      this.edgeCounts = edges ? meshes.map(mesh => mesh.edgeIC) : []
      this.edges = edges
      this.eligible = this.groups.length * 4 <= meshes.length
    }
    if (!this.eligible) return false
    const floats = meshes.length * INSTANCE_FLOATS
    if (this.data.length < floats) {
      const capacity = Math.min(device.limits.maxStorageBufferBindingSize, Math.max(floats * 4, this.data.byteLength * 2))
      const buffer = device.createBuffer({ size: capacity, usage: GPUBufferUsage.STORAGE | GPUBufferUsage.COPY_DST })
      let group: GPUBindGroup
      try { group = device.createBindGroup({ layout, entries: [{ binding: 0, resource: { buffer } }] }) }
      catch (error) { buffer.destroy(); throw error }
      this.buffer?.destroy()
      this.buffer = buffer
      this.group = group
      this.data = new Float32Array(capacity / 4)
      changed = true
    }
    if (!changed) {
      // Hover/selection touches only the 4 style floats of a few instances;
      // write those 16-byte slices instead of the whole buffer.
      for (let i = 0; i < meshes.length; i++) {
        const mesh = meshes[i], offset = i * INSTANCE_FLOATS
        const alpha = Math.fround(mesh.styleAlpha), selected = Math.fround(mesh.styleSelected)
        const edge = Math.fround(mesh.styleEdge), hovered = Math.fround(mesh.styleHovered)
        if (this.data[offset + STYLE_FLOAT_OFFSET] !== alpha || this.data[offset + STYLE_FLOAT_OFFSET + 1] !== selected
          || this.data[offset + STYLE_FLOAT_OFFSET + 2] !== edge || this.data[offset + STYLE_FLOAT_OFFSET + 3] !== hovered) {
          this.data[offset + STYLE_FLOAT_OFFSET] = alpha; this.data[offset + STYLE_FLOAT_OFFSET + 1] = selected
          this.data[offset + STYLE_FLOAT_OFFSET + 2] = edge; this.data[offset + STYLE_FLOAT_OFFSET + 3] = hovered
          try {
            device.queue.writeBuffer(this.buffer!, i * INSTANCE_BYTES + OBJECT_UNIFORM_LAYOUT.styleByteOffset, this.data, offset + STYLE_FLOAT_OFFSET, 4)
          } catch (error) {
            // Retry as a full upload on the next frame, matching the old contract.
            this.uploadPending = true
            throw error
          }
        }
      }
    }
    if (changed) this.uploadPending = true
    if (this.uploadPending) {
      for (let i = 0; i < meshes.length; i++) {
        const mesh = meshes[i], offset = i * INSTANCE_FLOATS
        for (let row = 0; row < 4; row++) for (let column = 0; column < 4; column++) this.data[offset + column * 4 + row] = mesh.transform[row * 4 + column]
        this.data.set(mesh.inverseTransform, offset + 16)
        this.data.set(mesh.color, offset + 32)
        this.data[offset + STYLE_FLOAT_OFFSET] = mesh.styleAlpha; this.data[offset + STYLE_FLOAT_OFFSET + 1] = mesh.styleSelected
        this.data[offset + STYLE_FLOAT_OFFSET + 2] = mesh.styleEdge; this.data[offset + STYLE_FLOAT_OFFSET + 3] = mesh.styleHovered
        // Instances never morph; keep the blend weight at rest.
        this.data[offset + MORPH_FLOAT_OFFSET] = 0; this.data[offset + MORPH_FLOAT_OFFSET + 1] = 0
        this.data[offset + MORPH_FLOAT_OFFSET + 2] = 0; this.data[offset + MORPH_FLOAT_OFFSET + 3] = 0
      }
      device.queue.writeBuffer(this.buffer!, 0, this.data, 0, floats)
      this.uploadPending = false
    }
    // Instanced pipelines declare a second vertex buffer (morph source);
    // morphing meshes never reach this path, so a zero dummy always suffices.
    if (!this.morphDummy) this.morphDummy = device.createBuffer({ size: 12, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
    pass.setPipeline(pipeline)
    pass.setBindGroup(0, scene)
    pass.setBindGroup(1, this.group!)
    pass.setVertexBuffer(1, this.morphDummy)
    for (const group of this.groups) {
      pass.setVertexBuffer(0, group.mesh.vb)
      pass.setIndexBuffer(group.ib, 'uint32')
      pass.drawIndexed(group.ic, group.count, 0, 0, group.start)
    }
    return true
  }
}
