/** Native command reuse; buffer contents (camera, clipping, styles) stay live. */
export interface BundleMesh {
  vb: GPUBuffer
  ib: GPUBuffer
  ic: number
  bg: GPUBindGroup
  edgeIB: GPUBuffer | null
  edgeIC: number
}

export class MeshDrawBundle {
  private bundle: GPURenderBundle | null = null
  private meshes: BundleMesh[] = []
  private indices: Array<GPUBuffer | null> = []
  private counts: number[] = []
  private pipeline: GPURenderPipeline | null = null
  private scene: GPUBindGroup | null = null

  clear() {
    this.bundle = null
    this.meshes.length = this.indices.length = this.counts.length = 0
    this.pipeline = null
    this.scene = null
  }

  draw(pass: GPURenderPassEncoder, device: GPUDevice, format: GPUTextureFormat,
    pipeline: GPURenderPipeline, scene: GPUBindGroup, meshes: readonly BundleMesh[], edges = false) {
    const encode = (encoder: GPURenderPassEncoder | GPURenderBundleEncoder) => {
      encoder.setPipeline(pipeline)
      encoder.setBindGroup(0, scene)
      for (const mesh of meshes) {
        encoder.setBindGroup(1, mesh.bg)
        encoder.setVertexBuffer(0, mesh.vb)
        encoder.setIndexBuffer(edges ? mesh.edgeIB! : mesh.ib, 'uint32')
        encoder.drawIndexed(edges ? mesh.edgeIC : mesh.ic)
      }
    }
    // Small lists cost less to encode directly; do not retain an old large scene.
    if (meshes.length < 16) {
      this.clear()
      if (meshes.length) encode(pass)
      return
    }
    let changed = !this.bundle || this.pipeline !== pipeline || this.scene !== scene || this.meshes.length !== meshes.length
    for (let i = 0; !changed && i < meshes.length; i++) {
      const mesh = meshes[i]
      changed = this.meshes[i] !== mesh || this.indices[i] !== (edges ? mesh.edgeIB : mesh.ib)
        || this.counts[i] !== (edges ? mesh.edgeIC : mesh.ic)
    }
    if (changed) {
      const encoder = device.createRenderBundleEncoder({ colorFormats: [format], depthStencilFormat: 'depth24plus' })
      encode(encoder)
      const bundle = encoder.finish()
      this.meshes.length = this.indices.length = this.counts.length = meshes.length
      for (let i = 0; i < meshes.length; i++) {
        const mesh = meshes[i]
        this.meshes[i] = mesh
        this.indices[i] = edges ? mesh.edgeIB : mesh.ib
        this.counts[i] = edges ? mesh.edgeIC : mesh.ic
      }
      this.pipeline = pipeline
      this.scene = scene
      this.bundle = bundle
    }
    pass.executeBundles([this.bundle!])
  }
}
