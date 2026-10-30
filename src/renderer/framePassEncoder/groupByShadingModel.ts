// framePassEncoder: groupByShadingModel.
import { invert, type Mat4, type Vec3 } from '../../services/math3d'

import type {
  DisplayMode,
  PickHit,
  RenderTheme,
  SelectionMode,
  ShadingModel,
} from '../../services/rendererContracts'

import { resolveMeshShaderId } from '../../services/rendererContracts'

import type { TransparentSortBuffer } from '../../services/transparentOrdering'

import type { MeshDrawBundle } from '../../services/meshDrawBundle'

import type { ViewFrustum } from '../../services/viewFrustum'

import type { MeshInstances } from '../../services/meshInstances'

import { isTransparentAlpha } from '../../services/backendQuality'

import {
  OBJECT_UNIFORM_LAYOUT,
  SCENE_UNIFORM_LAYOUT,
} from '../../services/shaders'

import {
  DEEP_DEPTH,
  type PipelineFactory,
  TRANSPARENT_DEPTH,
  type PipelineFlavor,
} from '../../services/pipelineFactory'

import type { TextureResources } from '../../services/textureResources'

import { fillSceneUniforms } from '../../services/uniformFill'

import { GpuCsgRenderer, type GpuCsgTree } from '../../services/gpuCsgPreview'

import type { GpuWboitRenderer } from '../../services/gpuWboit'

import {
  NO_GHOST_MESHES,
  type Bounds,
  type GMesh,
  type OverlaySlot,
} from '.././types'

import { sortEdgeDraws, sortOpaqueDraws } from '.././drawSorting'



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
