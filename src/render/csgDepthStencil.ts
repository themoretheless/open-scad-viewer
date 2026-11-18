/**
 * Depth-stencil attachment for the CSG preview pass, split out of the
 * renderer so the size-keyed reallocation rules can be tested with a fake
 * device.
 */
export class CsgDepthStencil {
  private texture: GPUTexture | null = null
  private view: GPUTextureView | null = null

  /** View of a depth24plus-stencil8 texture of the given size, reallocated on size change. */
  ensure(dev: GPUDevice, width: number, height: number): GPUTextureView | null {
    if (!this.texture || this.texture.width !== width || this.texture.height !== height) {
      this.texture?.destroy()
      this.texture = dev.createTexture({
        size: [width, height],
        format: 'depth24plus-stencil8',
        usage: GPUTextureUsage.RENDER_ATTACHMENT,
      })
      this.view = this.texture.createView()
    }
    return this.view
  }

  /** Destroys the texture; the next `ensure` allocates a fresh one. */
  release(): void {
    this.texture?.destroy()
    this.texture = null
    this.view = null
  }
}
