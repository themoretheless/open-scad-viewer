import { invert, type Mat4, type Vec3 } from '../services/math3d'
import type {
  DisplayMode,
  PickHit,
  RenderTheme,
  SelectionMode,
  ShadingModel,
} from '../services/rendererContracts'
import { resolveMeshShaderId } from '../services/rendererContracts'
import type { TransparentSortBuffer } from '../services/transparentOrdering'
import type { MeshDrawBundle } from '../services/meshDrawBundle'
import type { ViewFrustum } from '../services/viewFrustum'
import type { MeshInstances } from '../services/meshInstances'
import { isTransparentAlpha } from '../services/backendQuality'
import {
  OBJECT_UNIFORM_LAYOUT,
  SCENE_UNIFORM_LAYOUT,
} from '../services/shaders'
import {
  DEEP_DEPTH,
  type PipelineFactory,
  TRANSPARENT_DEPTH,
  type PipelineFlavor,
} from '../services/pipelineFactory'
import type { TextureResources } from '../services/textureResources'
import { fillSceneUniforms } from '../services/uniformFill'
import { GpuCsgRenderer, type GpuCsgTree } from '../services/gpuCsgPreview'
import type { GpuWboitRenderer } from '../services/gpuWboit'
import {
  NO_GHOST_MESHES,
  type Bounds,
  type GMesh,
  type OverlaySlot,
} from './types'
import { sortEdgeDraws, sortOpaqueDraws } from './drawSorting'

/**
 * Groups draws by shading model, preserving first-seen order. Used only when
 * a scene mixes materials; the all-default path skips the allocation.
 */
export function groupByShadingModel(draws: readonly GMesh[]): GMesh[][] {
  const groups = new Map<ShadingModel, GMesh[]>()
  for (const mesh of draws) {
    const group = groups.get(mesh.shadingModel)
    if (group) group.push(mesh)
    else groups.set(mesh.shadingModel, [mesh])
  }
  return [...groups.values()]
}

export function setObjectStyleImmediate(
  pass: GPURenderPassEncoder,
  mesh: GMesh,
  immediateObjectStyle: boolean,
  styleScratch: Float32Array<ArrayBuffer>,
) {
  if (!immediateObjectStyle) return
  styleScratch[0] = mesh.styleAlpha
  styleScratch[1] = mesh.styleSelected
  styleScratch[2] = mesh.styleEdge
  styleScratch[3] = mesh.styleHovered
  pass.setImmediates(0, styleScratch)
}

export interface AdvanceGeometryAnimationContext {
  readonly dev: GPUDevice | null
  readonly meshes: readonly GMesh[]
  readonly geometryFade: { started: number; progress: number } | null
  readonly geometryGhosts: Array<{ meshes: GMesh[]; started: number; alphas: number[] }>
  readonly morphDummyVB: GPUBuffer | null
  readonly styleScratch: Float32Array
  readonly objectUniformScratch: Float32Array
  readonly morphScratch: Float32Array
  readonly setGeometryFade: (fade: { started: number; progress: number } | null) => void
  readonly setGeometryGhosts: (ghosts: Array<{ meshes: GMesh[]; started: number; alphas: number[] }>) => void
  readonly invalidateShadowMap: () => void
  readonly updateMeshStyles: () => void
  readonly destroyMeshes: (meshes: GMesh[]) => void
  readonly clearDrawCaches: () => void
}

export function advanceSceneGeometryAnimation(
  ctx: AdvanceGeometryAnimationContext,
  now: number,
  finish = false,
): boolean {
  let active = false
  if (ctx.geometryFade) {
    // Fade progress reclassifies opacity per frame; the final restyle lands
    // after the flag clears, so finish needs one last shadow re-render.
    if (finish || now - ctx.geometryFade.started >= 180) {
      ctx.setGeometryFade(null)
      ctx.invalidateShadowMap()
    } else {
      ctx.geometryFade.progress = Math.max(0, (now - ctx.geometryFade.started) / 180)
      active = true
    }
    ctx.updateMeshStyles()
  }
  for (const ghost of ctx.geometryGhosts) {
    const t = finish ? 1 : Math.min(1, Math.max(0, (now - ghost.started) / 180))
    if (t === 1) ctx.destroyMeshes(ghost.meshes)
    else {
      active = true
      ghost.meshes.forEach((mesh, i) => {
        mesh.alpha = ghost.alphas[i] * (1 - t * t * (3 - 2 * t))
        ctx.styleScratch[0] = mesh.alpha
        ctx.styleScratch[1] = 0
        ctx.styleScratch[2] = 0
        ctx.styleScratch[3] = 0
        ctx.dev?.queue.writeBuffer(mesh.ub, OBJECT_UNIFORM_LAYOUT.styleByteOffset, ctx.styleScratch)
      })
    }
  }
  ctx.setGeometryGhosts(ctx.geometryGhosts.filter(ghost => !finish && now - ghost.started < 180))
  let morphFinished = false
  for (const mesh of ctx.meshes) {
    const morph = mesh.morph
    if (!morph) continue
    const t = finish ? 1 : Math.min(1, Math.max(0, (now - morph.started) / 180))
    const eased = t * t * (3 - 2 * t)
    if (morph.matrix) {
      morph.currentMatrix = morph.matrix(eased)
      const uniform = ctx.objectUniformScratch
      for (let row = 0; row < 4; row++) {
        for (let column = 0; column < 4; column++) {
          uniform[column * 4 + row] = morph.currentMatrix[row * 4 + column]
        }
      }
      invert(morph.currentMatrix, uniform.subarray(16, 32))
      ctx.dev?.queue.writeBuffer(mesh.ub, 0, uniform.subarray(0, 32))
    }
    if (t === 1) {
      // The vertex buffer already holds the destination; retire the morph
      // and restore the slot-1 dummy with the weight back at rest (1), so
      // the stale blend state is inert: mix(dummy, pos, 1) = pos.
      mesh.morph = undefined
      mesh.morphSlot = ctx.morphDummyVB
      ctx.morphScratch[0] = 1; ctx.morphScratch[1] = 0; ctx.morphScratch[2] = 0; ctx.morphScratch[3] = 0
      ctx.dev?.queue.writeBuffer(mesh.ub, OBJECT_UNIFORM_LAYOUT.morphByteOffset, ctx.morphScratch)
      morphFinished = true
    } else {
      // The vertex shader interpolates from the morph source; only the
      // blend weight travels to the GPU each frame.
      ctx.morphScratch[0] = eased; ctx.morphScratch[1] = 0; ctx.morphScratch[2] = 0; ctx.morphScratch[3] = 0
      ctx.dev?.queue.writeBuffer(mesh.ub, OBJECT_UNIFORM_LAYOUT.morphByteOffset, ctx.morphScratch)
      active = true
    }
  }
  // Finished morphs flip slot 1 back to the dummy; retained draw bundles and
  // instance caches key on the binding and must be rebuilt. The cached shadow
  // map holds mid-morph depth, so the final geometry needs one re-render.
  if (morphFinished) {
    ctx.clearDrawCaches()
    ctx.invalidateShadowMap()
  }
  return active
}

export interface EncodeFramePassContext {
  readonly canvas: HTMLCanvasElement
  readonly dev: GPUDevice
  readonly ctx: GPUCanvasContext
  readonly fmt: GPUTextureFormat
  readonly depth: GPUTexture
  readonly depthView: GPUTextureView | null
  readonly sceneUB: GPUBuffer
  readonly sceneBG: GPUBindGroup
  readonly pipelines: PipelineFactory
  readonly textures: TextureResources
  readonly backgroundColor: readonly [number, number, number]
  readonly eye: Vec3
  readonly viewProjection: Mat4
  readonly tx: number
  readonly ty: number
  readonly tz: number
  readonly sectionNormal: Vec3
  readonly sectionOffset: number
  readonly sectionEnabled: boolean
  readonly gridVisible: boolean
  readonly gridStep: number
  readonly gridExtent: number
  readonly gridFadeDistance: number
  readonly gridQuadVB: GPUBuffer | null
  readonly gridVB: GPUBuffer | null
  readonly gridVC: number
  readonly measurementVB: GPUBuffer | null
  readonly measurementVC: number
  readonly theme: RenderTheme
  readonly bounds: Bounds | null
  readonly displayMode: DisplayMode
  readonly transparencyMode: 'sorted' | 'wboit'
  readonly selectionMode: SelectionMode
  readonly selected: number | null
  readonly selectedHit: PickHit | null
  readonly meshes: readonly GMesh[]
  readonly geometryFade: { started: number; progress: number } | null
  readonly geometryGhosts: ReadonlyArray<{ meshes: GMesh[]; started: number; alphas: number[] }>
  readonly sceneUniformScratch: Float32Array
  readonly lastSceneUniforms: Float32Array
  readonly lastSceneUniformsValid: boolean
  readonly setLastSceneUniformsValid: (valid: boolean) => void
  readonly shadowMapEpoch: number
  readonly shadowMapRenderedEpoch: number
  readonly setShadowMapRenderedEpoch: (epoch: number) => void
  readonly shadowMapVP: Float32Array
  readonly shadowMapVPValid: boolean
  readonly setShadowMapVPValid: (valid: boolean) => void
  readonly viewFrustum: ViewFrustum
  readonly opaqueDraws: GMesh[]
  readonly edgeDraws: GMesh[]
  readonly transparentSort: TransparentSortBuffer
  readonly transparentDraws: GMesh[]
  readonly opaqueInstances: MeshInstances
  readonly transparentInstances: MeshInstances
  readonly edgeInstances: MeshInstances
  readonly opaqueBundle: MeshDrawBundle
  readonly sectionCapBundle: MeshDrawBundle
  readonly edgeBundle: MeshDrawBundle
  readonly sourceFaceSlot: OverlaySlot
  readonly selectionFaceSlot: OverlaySlot
  readonly sourceLineSlot: OverlaySlot
  readonly selectionLineSlot: OverlaySlot
  readonly deepSelectionLineSlot: OverlaySlot
  readonly styleScratch: Float32Array<ArrayBuffer>
  readonly csgPreviewTree: GpuCsgTree | null
  readonly ensureCsgDepthStencil: (dev: GPUDevice, width: number, height: number) => GPUTextureView | null
  readonly getCsgRenderer: () => GpuCsgRenderer
  readonly getWboitRenderer: () => GpuWboitRenderer
  readonly isMeshVisible: (index: number) => boolean
  readonly usesObjectSelectionStyle: (index: number) => boolean
  readonly usesObjectHoverStyle: (index: number) => boolean
}

export function encodeAndSubmitFrame(frame: EncodeFramePassContext): void {
  const {
    canvas,
    dev,
    ctx,
    fmt,
    depth,
    sceneUB,
    sceneBG,
    pipelines,
    textures,
    eye,
    viewProjection,
    meshes,
  } = frame

  const getRenderPipeline = (id: string, flavor: PipelineFlavor = {}): GPURenderPipeline =>
    pipelines.getRenderPipeline(dev, id, flavor)
  const extraBindGroupForShader = (id: string): GPUBindGroup | undefined =>
    pipelines.extraBindGroupForShader(id, textures)
  const meshTransparentPipeline = (): GPURenderPipeline => {
    const flavor: PipelineFlavor = { blend: 'alpha', depth: TRANSPARENT_DEPTH }
    return pipelines.immediateObjectLayout
      ? getRenderPipeline('mesh', { variant: 'immediate', ...flavor })
      : getRenderPipeline('mesh', flavor)
  }
  const deepMeshPipeline = (): GPURenderPipeline =>
    pipelines.immediateObjectLayout
      ? getRenderPipeline('deepMesh', { variant: 'immediate' })
      : getRenderPipeline('deepMesh')
  const deepEdgePipeline = (): GPURenderPipeline =>
    pipelines.immediateObjectLayout
      ? getRenderPipeline('edge', { variant: 'immediate', depth: DEEP_DEPTH })
      : getRenderPipeline('edge', { depth: DEEP_DEPTH })

  // Hoisted once per frame; the morph set cannot change mid-render.
  const hasMorph = meshes.some(mesh => mesh.morph)
  const sd = frame.sceneUniformScratch
  // Shadow tail: light VP + (enabled, texel, bias, strength). Enabled 0 keeps
  // every sampling shader on the pixel-identical unshadowed path.
  const shadowActive = textures.shadowsEnabled && !!frame.bounds
    && !!textures.shadowTexture && textures.shadowTexture !== textures.shadowDummyTexture
  fillSceneUniforms(sd, {
    viewProjection,
    inverseVP: invert(viewProjection),
    eye,
    sectionNormal: frame.sectionNormal,
    sectionOffset: frame.sectionOffset,
    sectionEnabled: frame.sectionEnabled,
    gridStep: frame.gridStep,
    gridExtent: frame.gridExtent,
    gridFadeDistance: frame.gridFadeDistance,
    theme: frame.theme,
    shadowActive,
    bounds: frame.bounds,
  })
  let sceneChanged = !frame.lastSceneUniformsValid
  if (!sceneChanged) {
    for (let i = 0; i < sd.length; i++) {
      if (sd[i] !== frame.lastSceneUniforms[i]) {
        sceneChanged = true
        break
      }
    }
  }
  if (sceneChanged) {
    dev.queue.writeBuffer(sceneUB, 0, sd)
    frame.lastSceneUniforms.set(sd)
    frame.setLastSceneUniformsValid(true)
  }

  // Classify draws before encoding: the shadow depth pass runs ahead of the
  // main pass and draws the same opaque set.
  const transitioning = frame.geometryGhosts.length > 0 || hasMorph
  const useWboit = frame.transparencyMode === 'wboit'
  frame.viewFrustum.update(viewProjection)
  frame.opaqueDraws.length = frame.edgeDraws.length = frame.transparentDraws.length = 0
  if (!useWboit) frame.transparentSort.begin()
  for (let index = 0; index < meshes.length; index++) {
    const mesh = meshes[index]
    if (!frame.isMeshVisible(index) || (!mesh.morph && !frame.viewFrustum.intersects(mesh.worldBounds.center, mesh.worldBounds.radius))) continue
    if (isTransparentAlpha(mesh.alpha)) {
      if (useWboit) frame.transparentDraws.push(mesh)
      else frame.transparentSort.add(index, mesh.worldBounds.center)
    } else frame.opaqueDraws.push(mesh)
    const highlighted = frame.usesObjectSelectionStyle(index) || frame.usesObjectHoverStyle(index)
    if (mesh.edgeIB && (frame.displayMode === 'edges' || highlighted)) frame.edgeDraws.push(mesh)
  }
  sortOpaqueDraws(frame.opaqueDraws)
  sortEdgeDraws(frame.edgeDraws)

  const enc = dev.createCommandEncoder()

  // Contact shadow: render opaque meshes' depth from the key light into the
  // shadow map before the main pass samples it (surfaces + grid floor).
  if (shadowActive) {
    const vpOffset = SCENE_UNIFORM_LAYOUT.lightVPFloatOffset
    let vpChanged = !frame.shadowMapVPValid
    if (!vpChanged) {
      for (let i = 0; i < 16; i++) {
        if (sd[vpOffset + i] !== frame.shadowMapVP[i]) { vpChanged = true; break }
      }
    }
    const shadowStale = vpChanged || frame.shadowMapRenderedEpoch !== frame.shadowMapEpoch
      || hasMorph || frame.geometryFade !== null
    if (shadowStale) {
      const shadowPass = enc.beginRenderPass({
        colorAttachments: [],
        depthStencilAttachment: {
          view: textures.shadowTexture!.createView(),
          depthClearValue: 1, depthLoadOp: 'clear', depthStoreOp: 'store',
        },
      })
      shadowPass.setPipeline(getRenderPipeline('meshShadow'))
      shadowPass.setBindGroup(0, sceneBG)
      for (let index = 0; index < meshes.length; index++) {
        const mesh = meshes[index]
        if (!frame.isMeshVisible(index) || isTransparentAlpha(mesh.alpha)) continue
        shadowPass.setBindGroup(1, mesh.bg)
        shadowPass.setVertexBuffer(0, mesh.vb)
        shadowPass.setVertexBuffer(1, mesh.morphSlot!)
        shadowPass.setIndexBuffer(mesh.ib, 'uint32')
        shadowPass.drawIndexed(mesh.ic)
      }
      shadowPass.end()
      frame.setShadowMapRenderedEpoch(frame.shadowMapEpoch)
      frame.shadowMapVP.set(sd.subarray(vpOffset, vpOffset + 16))
      frame.setShadowMapVPValid(true)
    }
  }

  // GPU CSG Preview: evaluate Boolean terms on WebGPU hardware when active
  const isCsgActive = frame.csgPreviewTree !== null && frame.csgPreviewTree.terms.length > 0
  if (isCsgActive) {
    const csgDepthView = frame.ensureCsgDepthStencil(dev, canvas.width, canvas.height)
    if (csgDepthView) {
      const csgRenderer = frame.getCsgRenderer()
      const colorView = ctx.getCurrentTexture().createView()
      csgRenderer.renderCsg(dev, enc, colorView, csgDepthView, frame.csgPreviewTree!, {
        viewProjection,
        eyePosition: [eye[0], eye[1], eye[2]],
        lightDirection: [0.55, 0.75, 0.45],
      })
    }
  }

  const depthView = frame.depthView ?? depth.createView()
  const pass = enc.beginRenderPass({
    colorAttachments: [{
      view: ctx.getCurrentTexture().createView(),
      clearValue: {
        r: frame.backgroundColor[0],
        g: frame.backgroundColor[1],
        b: frame.backgroundColor[2],
        a: 1,
      },
      loadOp: isCsgActive ? 'load' : 'clear', storeOp: 'store',
    }],
    depthStencilAttachment: {
      view: depthView,
      depthClearValue: 1, depthLoadOp: 'clear', depthStoreOp: 'store',
    },
  })

  if (frame.gridVisible && frame.gridQuadVB) {
    pass.setPipeline(getRenderPipeline('grid'))
    pass.setBindGroup(0, sceneBG)
    if (textures.shadowBG) pass.setBindGroup(1, textures.shadowBG)
    pass.setVertexBuffer(0, frame.gridQuadVB)
    pass.draw(6)
  }
  if (frame.gridVisible && frame.gridVB) {
    pass.setPipeline(getRenderPipeline('line'))
    pass.setBindGroup(0, sceneBG)
    pass.setVertexBuffer(0, frame.gridVB)
    pass.draw(frame.gridVC)
  }
  if (frame.measurementVB && frame.measurementVC) {
    pass.setPipeline(getRenderPipeline('line'))
    pass.setBindGroup(0, sceneBG)
    pass.setVertexBuffer(0, frame.measurementVB)
    pass.draw(frame.measurementVC)
  }

  if (!isCsgActive) {
    const customOpaque = frame.opaqueDraws.some(mesh => mesh.shadingModel !== 'phong')
    if (!customOpaque && (transitioning || !frame.opaqueInstances.draw(pass, dev, pipelines.instanceBGL, getRenderPipeline('mesh', { variant: 'instanced' }), sceneBG, frame.opaqueDraws, false, textures.shadowBG ?? undefined))) {
      frame.opaqueBundle.draw(pass, dev, fmt, getRenderPipeline('mesh'), sceneBG, frame.opaqueDraws, false, textures.shadowBG ?? undefined)
    } else if (customOpaque) {
      for (const group of groupByShadingModel(frame.opaqueDraws)) {
        const shaderId = resolveMeshShaderId(group[0].shadingModel)
        frame.opaqueBundle.draw(pass, dev, fmt, getRenderPipeline(shaderId), sceneBG, group,
          false, extraBindGroupForShader(shaderId))
      }
    }

    if (frame.sectionEnabled && frame.displayMode !== 'xray' && frame.opaqueDraws.length) {
      frame.sectionCapBundle.draw(pass, dev, fmt, getRenderPipeline('meshSectionCap'), sceneBG, frame.opaqueDraws,
        false, extraBindGroupForShader('meshSectionCap'))
    }

    if (!useWboit) {
      pass.setPipeline(meshTransparentPipeline())
      pass.setBindGroup(0, sceneBG)
      if (textures.shadowBG) pass.setBindGroup(2, textures.shadowBG)
      const ghostMeshes = frame.geometryGhosts.length
        ? frame.geometryGhosts.flatMap(ghost => ghost.meshes).filter(mesh => mesh.alpha > 0)
        : NO_GHOST_MESHES
      ghostMeshes.forEach((mesh, index) => frame.transparentSort.add(meshes.length + index, mesh.worldBounds.center))
      const transparentOrder = frame.transparentSort.sort(eye, frame.tx, frame.ty, frame.tz)
      frame.transparentDraws.length = 0
      for (const { index } of transparentOrder) frame.transparentDraws.push(index < meshes.length ? meshes[index] : ghostMeshes[index - meshes.length])
      const customTransparent = frame.transparentDraws.some(mesh => mesh.shadingModel !== 'phong')
      if (!customTransparent && (transitioning || !frame.transparentInstances.draw(pass, dev, pipelines.instanceBGL, getRenderPipeline('mesh', { variant: 'instanced', blend: 'alpha', depth: TRANSPARENT_DEPTH }), sceneBG, frame.transparentDraws, false, textures.shadowBG ?? undefined))) {
        for (const g of frame.transparentDraws) {
          pass.setBindGroup(1, g.bg)
          setObjectStyleImmediate(pass, g, pipelines.immediateObjectStyle, frame.styleScratch)
          pass.setVertexBuffer(0, g.vb)
          pass.setVertexBuffer(1, g.morphSlot!)
          pass.setIndexBuffer(g.ib, 'uint32')
          pass.drawIndexed(g.ic)
        }
      } else if (customTransparent) {
        let activeModel: ShadingModel | null = null
        for (const g of frame.transparentDraws) {
          if (g.shadingModel !== activeModel) {
            activeModel = g.shadingModel
            const shaderId = resolveMeshShaderId(activeModel)
            pass.setPipeline(activeModel === 'phong'
              ? meshTransparentPipeline()
              : getRenderPipeline(shaderId, { blend: 'alpha', depth: TRANSPARENT_DEPTH }))
            const extraGroup = extraBindGroupForShader(shaderId)
            if (extraGroup) pass.setBindGroup(2, extraGroup)
          }
          pass.setBindGroup(1, g.bg)
          if (activeModel === 'phong') setObjectStyleImmediate(pass, g, pipelines.immediateObjectStyle, frame.styleScratch)
          pass.setVertexBuffer(0, g.vb)
          pass.setVertexBuffer(1, g.morphSlot!)
          pass.setIndexBuffer(g.ib, 'uint32')
          pass.drawIndexed(g.ic)
        }
      }
    }
  }

  const deepSelectedIndex = frame.selectionMode === 'object'
    && (frame.selectedHit?.cycleIndex ?? 0) > 0
    ? frame.selected
    : null
  if (deepSelectedIndex !== null) {
    const selectedMesh = meshes[deepSelectedIndex]
    if (frame.isMeshVisible(deepSelectedIndex) && selectedMesh) {
      pass.setPipeline(deepMeshPipeline())
      pass.setBindGroup(0, sceneBG)
      pass.setBindGroup(1, selectedMesh.bg)
      const deepGroup = extraBindGroupForShader('deepMesh')
      if (deepGroup) pass.setBindGroup(2, deepGroup)
      setObjectStyleImmediate(pass, selectedMesh, pipelines.immediateObjectStyle, frame.styleScratch)
      pass.setVertexBuffer(0, selectedMesh.vb)
      pass.setVertexBuffer(1, selectedMesh.morphSlot!)
      pass.setIndexBuffer(selectedMesh.ib, 'uint32')
      pass.drawIndexed(selectedMesh.ic)
    }
  }

  if (!frame.geometryFade && !frame.geometryGhosts.length && !hasMorph && frame.sourceFaceSlot.buffer && frame.sourceFaceSlot.count) {
    pass.setPipeline(getRenderPipeline('selectionOverlay'))
    pass.setBindGroup(0, sceneBG)
    pass.setVertexBuffer(0, frame.sourceFaceSlot.buffer)
    pass.draw(frame.sourceFaceSlot.count)
  }

  if (!frame.geometryFade && !frame.geometryGhosts.length && !hasMorph && frame.selectionFaceSlot.buffer && frame.selectionFaceSlot.count) {
    pass.setPipeline(getRenderPipeline('selectionOverlay'))
    pass.setBindGroup(0, sceneBG)
    pass.setVertexBuffer(0, frame.selectionFaceSlot.buffer)
    pass.draw(frame.selectionFaceSlot.count)
  }

  const edgeGroup = extraBindGroupForShader('edge')
  if (transitioning || !frame.edgeInstances.draw(pass, dev, pipelines.instanceBGL, getRenderPipeline('edge', { variant: 'instanced' }), sceneBG, frame.edgeDraws, true, edgeGroup)) {
    frame.edgeBundle.draw(pass, dev, fmt, getRenderPipeline('edge'), sceneBG, frame.edgeDraws, true, edgeGroup)
  }

  if (deepSelectedIndex !== null) {
    const selectedMesh = meshes[deepSelectedIndex]
    if (frame.isMeshVisible(deepSelectedIndex) && selectedMesh?.edgeIB) {
      pass.setPipeline(deepEdgePipeline())
      pass.setBindGroup(0, sceneBG)
      pass.setBindGroup(1, selectedMesh.bg)
      const deepGroup = extraBindGroupForShader('edge')
      if (deepGroup) pass.setBindGroup(2, deepGroup)
      setObjectStyleImmediate(pass, selectedMesh, pipelines.immediateObjectStyle, frame.styleScratch)
      pass.setVertexBuffer(0, selectedMesh.vb)
      pass.setVertexBuffer(1, selectedMesh.morphSlot!)
      pass.setIndexBuffer(selectedMesh.edgeIB, 'uint32')
      pass.drawIndexed(selectedMesh.edgeIC)
    }
  }

  if (!frame.geometryFade && !frame.geometryGhosts.length && !hasMorph && frame.sourceLineSlot.buffer && frame.sourceLineSlot.count) {
    pass.setPipeline(getRenderPipeline('selectionOverlay', { topology: 'line-list' }))
    pass.setBindGroup(0, sceneBG)
    pass.setVertexBuffer(0, frame.sourceLineSlot.buffer)
    pass.draw(frame.sourceLineSlot.count)
  }

  if (!frame.geometryFade && !frame.geometryGhosts.length && !hasMorph && frame.selectionLineSlot.buffer && frame.selectionLineSlot.count) {
    pass.setPipeline(getRenderPipeline('selectionOverlay', { topology: 'line-list' }))
    pass.setBindGroup(0, sceneBG)
    pass.setVertexBuffer(0, frame.selectionLineSlot.buffer)
    pass.draw(frame.selectionLineSlot.count)
  }

  if (!frame.geometryFade && !frame.geometryGhosts.length && !hasMorph && frame.deepSelectionLineSlot.buffer && frame.deepSelectionLineSlot.count) {
    pass.setPipeline(getRenderPipeline('selectionOverlay', { topology: 'line-list', depth: DEEP_DEPTH }))
    pass.setBindGroup(0, sceneBG)
    pass.setVertexBuffer(0, frame.deepSelectionLineSlot.buffer)
    pass.draw(frame.deepSelectionLineSlot.count)
  }

  pass.end()

  // WBOIT runs after the main opaque render pass ends so the command encoder
  // never has two concurrent render passes open and depthView has full opaque depth.
  if (!isCsgActive && useWboit) {
    for (const ghost of frame.geometryGhosts) {
      for (const gMesh of ghost.meshes) {
        if (gMesh.alpha > 0) frame.transparentDraws.push(gMesh)
      }
    }
    if (frame.transparentDraws.length) {
      sortOpaqueDraws(frame.transparentDraws)
      const wboit = frame.getWboitRenderer()
      wboit.ensureTextures(dev, canvas.width, canvas.height)
      const wboitPass = wboit.beginWboitPass(enc, depthView)
      wboitPass.setBindGroup(0, sceneBG)
      if (textures.shadowBG) wboitPass.setBindGroup(2, textures.shadowBG)

      let activeModel: ShadingModel | null = null
      for (const g of frame.transparentDraws) {
        if (g.shadingModel !== activeModel) {
          activeModel = g.shadingModel
          const shaderId = resolveMeshShaderId(activeModel)
          const pipe = wboit.getWboitPipeline(dev, shaderId, pipelines.objectLayout, pipelines.vertexLayouts.mesh)
          wboitPass.setPipeline(pipe)
          const extraGroup = extraBindGroupForShader(shaderId)
          if (extraGroup) wboitPass.setBindGroup(2, extraGroup)
        }
        wboitPass.setBindGroup(1, g.bg)
        wboitPass.setVertexBuffer(0, g.vb)
        wboitPass.setVertexBuffer(1, g.morphSlot!)
        wboitPass.setIndexBuffer(g.ib, 'uint32')
        wboitPass.drawIndexed(g.ic)
      }
      wboitPass.end()

      wboit.composite(dev, enc, ctx.getCurrentTexture().createView(), fmt)
    }
  }

  dev.queue.submit([enc.finish()])
}
