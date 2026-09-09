import type { Mat4 } from './math3d'
import type { BundleMesh } from './meshDrawBundle'

export interface InstanceMesh extends BundleMesh {
  readonly transform: Mat4
  readonly inverseTransform: Mat4
  readonly color: readonly number[]
  readonly styleAlpha: number
  readonly styleSelected: number
  readonly styleEdge: number
  readonly styleHovered: number
}

/** Derive the storage variant from the same shading expressions as single objects. */
export function instancedObjectShader(source: string, output: 'V' | 'EdgeV') {
  const replacements = [
    ['var<uniform> ob: Obj;', 'var<storage, read> objects: array<Obj>;'],
    [`struct ${output} {`, `struct ${output} { @location(2) @interpolate(flat) instance: u32,`],
    ['@vertex fn vs(', '@vertex fn vs(@builtin(instance_index) instance: u32, '],
    [` -> ${output} {`, ` -> ${output} {\n  let ob = objects[instance];`],
    [`return ${output}(`, `return ${output}(instance, `],
    [`@fragment fn fs(v: ${output}) -> @location(0) vec4f {`, `@fragment fn fs(v: ${output}) -> @location(0) vec4f {\n  let ob = objects[v.instance];`],
  ]
  for (const [from, to] of replacements) {
    if (source.split(from).length !== 2) throw new Error(`Instanced shader contract changed: ${from}`)
    source = source.replace(from, to)
  }
  return source
}

interface Group { mesh: InstanceMesh; start: number; count: number; ib: GPUBuffer; ic: number }

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
    if (meshes.length < 16 || meshes.length * 160 > device.limits.maxStorageBufferBindingSize) {
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
    const floats = meshes.length * 40
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
        const mesh = meshes[i], offset = i * 40
        const alpha = Math.fround(mesh.styleAlpha), selected = Math.fround(mesh.styleSelected)
        const edge = Math.fround(mesh.styleEdge), hovered = Math.fround(mesh.styleHovered)
        if (this.data[offset + 36] !== alpha || this.data[offset + 37] !== selected
          || this.data[offset + 38] !== edge || this.data[offset + 39] !== hovered) {
          this.data[offset + 36] = alpha; this.data[offset + 37] = selected
          this.data[offset + 38] = edge; this.data[offset + 39] = hovered
          try {
            device.queue.writeBuffer(this.buffer!, i * 160 + 144, this.data, offset + 36, 4)
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
        const mesh = meshes[i], offset = i * 40
        for (let row = 0; row < 4; row++) for (let column = 0; column < 4; column++) this.data[offset + column * 4 + row] = mesh.transform[row * 4 + column]
        this.data.set(mesh.inverseTransform, offset + 16)
        this.data.set(mesh.color, offset + 32)
        this.data[offset + 36] = mesh.styleAlpha; this.data[offset + 37] = mesh.styleSelected
        this.data[offset + 38] = mesh.styleEdge; this.data[offset + 39] = mesh.styleHovered
      }
      device.queue.writeBuffer(this.buffer!, 0, this.data, 0, floats)
      this.uploadPending = false
    }
    pass.setPipeline(pipeline)
    pass.setBindGroup(0, scene)
    pass.setBindGroup(1, this.group!)
    for (const group of this.groups) {
      pass.setVertexBuffer(0, group.mesh.vb)
      pass.setIndexBuffer(group.ib, 'uint32')
      pass.drawIndexed(group.ic, group.count, 0, 0, group.start)
    }
    return true
  }
}
