/**
 * Texture resources for the WebGPU renderer: matcap capture, environment
 * map, and key-light shadow map — each with sampler, 1×1 fallback dummy,
 * and bind group — plus their async load lifecycles. Load tokens make a
 * slow fetch unable to overwrite a newer selection; teardown invalidates
 * in-flight loads and destroys every texture. The renderer keeps only the
 * public API forwarding.
 */
import {
  DEFAULT_ENV,
  DEFAULT_MATCAP,
  getEnvPreset,
  getMatcapPreset,
} from './rendererContracts'

/** Key-light shadow map resolution (depth32float, depth-only pass). */
export const SHADOW_MAP_SIZE = 1024

/**
 * Renderer callbacks the resource manager needs: the current device (null
 * after teardown, so late loads die), a rebind notification (draw bundles
 * bake bind groups and must be re-encoded), and an error sink for failed
 * texture fetches.
 */
export interface TextureResourceHooks {
  getDevice(): GPUDevice | null
  onTexturesRebound(): void
  onLoadError(error: unknown): void
}

export class TextureResources {
  /** group(2): matcap capture texture + sampler, shared by all matcap draws. */
  matcapBGL!: GPUBindGroupLayout
  matcapBG: GPUBindGroup | null = null
  matcapSampler: GPUSampler | null = null
  /** Currently bound capture; the 1×1 dummy selects the procedural path. */
  matcapTexture: GPUTexture | null = null
  matcapDummyTexture: GPUTexture | null = null
  matcapId = DEFAULT_MATCAP.id
  /** Monotonic token so a slow fetch cannot overwrite a newer selection. */
  private matcapLoadToken = 0
  /** group(2): equirect environment map + sampler, shared by all PBR draws. */
  envBGL!: GPUBindGroupLayout
  envBG: GPUBindGroup | null = null
  envSampler: GPUSampler | null = null
  /** Currently bound env map; the 1×1 dummy selects the analytic-light path. */
  envTexture: GPUTexture | null = null
  envDummyTexture: GPUTexture | null = null
  envId = DEFAULT_ENV.id
  /** Monotonic token so a slow fetch cannot overwrite a newer selection. */
  private envLoadToken = 0
  /**
   * Key-light contact shadow: depth map + comparison sampler. Bound by the
   * mesh-surface shaders at group(2) (mesh/meshToon bindings 0/1; the matcap
   * and env groups carry it at bindings 2/3) and by the grid at group(1).
   * A 1×1 dummy plus shadowParams.enabled == 0 keeps the default look exact.
   */
  shadowBGL!: GPUBindGroupLayout
  shadowBG: GPUBindGroup | null = null
  shadowSampler: GPUSampler | null = null
  shadowTexture: GPUTexture | null = null
  shadowDummyTexture: GPUTexture | null = null
  shadowsEnabled = false

  constructor(private readonly hooks: TextureResourceHooks) {}

  /**
   * (Re)creates samplers, 1×1 dummies, and the default bind groups against
   * the current device. Shadow first: the matcap/env groups embed the shadow
   * view at bindings 2/3, so it must exist before those groups are built.
   */
  init(dev: GPUDevice, layouts: { matcapBGL: GPUBindGroupLayout; envBGL: GPUBindGroupLayout; shadowBGL: GPUBindGroupLayout }) {
    this.matcapBGL = layouts.matcapBGL
    this.envBGL = layouts.envBGL
    this.shadowBGL = layouts.shadowBGL
    // Shadow fallback: a 1×1 depth dummy plus shadowParams.enabled == 0 keeps
    // the sampling shaders on their unshadowed path (default look exact).
    this.shadowTexture?.destroy()
    this.shadowDummyTexture?.destroy()
    this.shadowSampler = dev.createSampler({ compare: 'less-equal' })
    this.shadowDummyTexture = dev.createTexture({ size: [1, 1], format: 'depth32float', usage: GPUTextureUsage.TEXTURE_BINDING })
    // The enable flag outlives teardown (device-loss recovery re-inits the
    // same instance, and callers may enable before init), so an enabled
    // renderer gets its real map back instead of silently rendering unshadowed.
    this.shadowTexture = this.shadowsEnabled ? this.createShadowMap(dev) : this.shadowDummyTexture
    this.shadowBG = this.createShadowBindGroup(this.shadowTexture)
    // Matcap fallback: a 1×1 white capture makes the shader take its
    // procedural path (textureDimensions == 1), so the default look is exact.
    this.matcapTexture?.destroy()
    this.matcapDummyTexture?.destroy()
    this.matcapSampler = dev.createSampler({ magFilter: 'linear', minFilter: 'linear' })
    this.matcapDummyTexture = dev.createTexture({ size: [1, 1], format: 'rgba8unorm', usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST })
    dev.queue.writeTexture({ texture: this.matcapDummyTexture }, new Uint8Array([255, 255, 255, 255]), {}, [1, 1])
    this.matcapTexture = this.matcapDummyTexture
    this.matcapBG = this.createMatcapBindGroup(this.matcapTexture)
    // Env fallback: a 1×1 dummy makes mesh_pbr take its analytic-lighting path
    // (textureDimensions == 1), so the default look is exact.
    this.envTexture?.destroy()
    this.envDummyTexture?.destroy()
    this.envSampler = dev.createSampler({ magFilter: 'linear', minFilter: 'linear', addressModeU: 'repeat', addressModeV: 'clamp-to-edge' })
    this.envDummyTexture = dev.createTexture({ size: [1, 1], format: 'rgba8unorm', usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST })
    dev.queue.writeTexture({ texture: this.envDummyTexture }, new Uint8Array([255, 255, 255, 255]), {}, [1, 1])
    this.envTexture = this.envDummyTexture
    this.envBG = this.createEnvBindGroup(this.envTexture)
  }

  createMatcapBindGroup(texture: GPUTexture): GPUBindGroup {
    return this.hooks.getDevice()!.createBindGroup({
      layout: this.matcapBGL,
      entries: [
        { binding: 0, resource: texture.createView() },
        { binding: 1, resource: this.matcapSampler! },
        { binding: 2, resource: this.shadowTexture!.createView() },
        { binding: 3, resource: this.shadowSampler! },
      ],
    })
  }

  createEnvBindGroup(texture: GPUTexture): GPUBindGroup {
    return this.hooks.getDevice()!.createBindGroup({
      layout: this.envBGL,
      entries: [
        { binding: 0, resource: texture.createView() },
        { binding: 1, resource: this.envSampler! },
        { binding: 2, resource: this.shadowTexture!.createView() },
        { binding: 3, resource: this.shadowSampler! },
      ],
    })
  }

  private createShadowMap(dev: GPUDevice): GPUTexture {
    return dev.createTexture({
      size: [SHADOW_MAP_SIZE, SHADOW_MAP_SIZE],
      format: 'depth32float',
      usage: GPUTextureUsage.RENDER_ATTACHMENT | GPUTextureUsage.TEXTURE_BINDING,
    })
  }

  /** Shadow-map bind group: key-light depth view + comparison sampler. */
  createShadowBindGroup(texture: GPUTexture): GPUBindGroup {
    return this.hooks.getDevice()!.createBindGroup({
      layout: this.shadowBGL,
      entries: [
        { binding: 0, resource: texture.createView() },
        { binding: 1, resource: this.shadowSampler! },
      ],
    })
  }

  /**
   * Selects the matcap capture for the meshMatcap shader. 'procedural' binds
   * the 1×1 dummy (the shader's procedural fallback, pixel-identical to the
   * pre-texture look); other presets fetch their PNG and upload it as a
   * GPUTexture. Async: the frame is requested once the texture is bound.
   */
  setMatcapTexture(id: string): Promise<void> {
    const preset = getMatcapPreset(id)
    if (!preset) return Promise.reject(new Error(`Renderer: unknown matcap '${id}'`))
    this.matcapId = id
    const token = ++this.matcapLoadToken
    const bind = (texture: GPUTexture) => {
      if (!this.hooks.getDevice() || token !== this.matcapLoadToken) { texture.destroy(); return }
      const previous = this.matcapTexture
      this.matcapTexture = texture
      this.matcapBG = this.createMatcapBindGroup(texture)
      if (previous && previous !== this.matcapDummyTexture) previous.destroy()
      // Draw bundles bake bind group 2; force re-encoding with the new one.
      this.hooks.onTexturesRebound()
    }
    if (!preset.url) {
      if (this.matcapDummyTexture) bind(this.matcapDummyTexture)
      return Promise.resolve()
    }
    const dev = this.hooks.getDevice()
    if (!dev || typeof fetch !== 'function' || typeof createImageBitmap !== 'function') return Promise.resolve()
    return (async () => {
      const response = await fetch(preset.url!)
      if (!response.ok) throw new Error(`Renderer: matcap fetch failed (${response.status})`)
      const bitmap = await createImageBitmap(await response.blob(), { imageOrientation: 'flipY' })
      const texture = dev.createTexture({
        size: [bitmap.width, bitmap.height],
        format: 'rgba8unorm',
        usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | GPUTextureUsage.RENDER_ATTACHMENT,
      })
      dev.queue.copyExternalImageToTexture({ source: bitmap }, { texture }, [bitmap.width, bitmap.height])
      bitmap.close()
      bind(texture)
    })().catch(error => {
      // A missing capture must not break rendering: stay on the current matcap.
      // A superseded (or torn-down) load is no longer the user's selection.
      if (token === this.matcapLoadToken) this.hooks.onLoadError(error)
    })
  }

  /**
   * Selects the equirect environment map for the meshPbr shader (IBL).
   * 'none' binds the 1×1 dummy (the shader's analytic-lighting fallback,
   * pixel-identical to the pre-IBL look); other presets fetch their PNG and
   * upload it as a GPUTexture. Async: the frame is requested once bound.
   */
  setEnvMap(id: string): Promise<void> {
    const preset = getEnvPreset(id)
    if (!preset) return Promise.reject(new Error(`Renderer: unknown environment map '${id}'`))
    this.envId = id
    const token = ++this.envLoadToken
    const bind = (texture: GPUTexture) => {
      if (!this.hooks.getDevice() || token !== this.envLoadToken) { texture.destroy(); return }
      const previous = this.envTexture
      this.envTexture = texture
      this.envBG = this.createEnvBindGroup(texture)
      if (previous && previous !== this.envDummyTexture) previous.destroy()
      // Draw bundles bake bind group 2; force re-encoding with the new one.
      this.hooks.onTexturesRebound()
    }
    if (!preset.url) {
      if (this.envDummyTexture) bind(this.envDummyTexture)
      return Promise.resolve()
    }
    const dev = this.hooks.getDevice()
    if (!dev || typeof fetch !== 'function' || typeof createImageBitmap !== 'function') return Promise.resolve()
    return (async () => {
      const response = await fetch(preset.url!)
      if (!response.ok) throw new Error(`Renderer: environment map fetch failed (${response.status})`)
      // No flipY: equirect v=0 is the +Y pole, which is the PNG's top row.
      const bitmap = await createImageBitmap(await response.blob())
      const texture = dev.createTexture({
        size: [bitmap.width, bitmap.height],
        format: 'rgba8unorm',
        usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST | GPUTextureUsage.RENDER_ATTACHMENT,
      })
      dev.queue.copyExternalImageToTexture({ source: bitmap }, { texture }, [bitmap.width, bitmap.height])
      bitmap.close()
      bind(texture)
    })().catch(error => {
      // A missing map must not break rendering: stay on the current env.
      // A superseded (or torn-down) load is no longer the user's selection.
      if (token === this.envLoadToken) this.hooks.onLoadError(error)
    })
  }

  /**
   * Toggles the contact-shadow map allocation. Enabling lazily allocates the
   * real 1024² depth map and rebinds every group that embeds its view;
   * disabling rebinds the 1×1 dummy. Returns true when the device-side
   * resources were rebuilt (the renderer then drops its shadow-map cache and
   * re-encodes draw bundles).
   */
  setShadowsEnabled(enabled: boolean, dev: GPUDevice | null): boolean {
    this.shadowsEnabled = enabled
    if (dev && this.shadowDummyTexture) {
      if (enabled) {
        if (this.shadowTexture && this.shadowTexture !== this.shadowDummyTexture) this.shadowTexture.destroy()
        this.shadowTexture = this.createShadowMap(dev)
      } else {
        if (this.shadowTexture && this.shadowTexture !== this.shadowDummyTexture) this.shadowTexture.destroy()
        this.shadowTexture = this.shadowDummyTexture
      }
      this.shadowBG = this.createShadowBindGroup(this.shadowTexture!)
      // The matcap/env groups embed the shadow view at bindings 2/3.
      if (this.matcapTexture) this.matcapBG = this.createMatcapBindGroup(this.matcapTexture)
      if (this.envTexture) this.envBG = this.createEnvBindGroup(this.envTexture)
      return true
    }
    return false
  }

  /** Destroys every texture, invalidates in-flight loads, and drops groups. */
  teardown() {
    this.matcapLoadToken++
    if (this.matcapTexture && this.matcapTexture !== this.matcapDummyTexture) this.matcapTexture.destroy()
    this.matcapTexture = null
    this.matcapDummyTexture?.destroy()
    this.matcapDummyTexture = null
    this.matcapBG = null
    this.matcapSampler = null
    this.envLoadToken++
    if (this.envTexture && this.envTexture !== this.envDummyTexture) this.envTexture.destroy()
    this.envTexture = null
    this.envDummyTexture?.destroy()
    this.envDummyTexture = null
    this.envBG = null
    this.envSampler = null
    if (this.shadowTexture && this.shadowTexture !== this.shadowDummyTexture) this.shadowTexture.destroy()
    this.shadowTexture = null
    this.shadowDummyTexture?.destroy()
    this.shadowDummyTexture = null
    this.shadowBG = null
    this.shadowSampler = null
  }
}
