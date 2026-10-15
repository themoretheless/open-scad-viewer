import type { ShadingModel } from '../rendererContracts'

export interface EntityDraw {
  shadingModel: ShadingModel
  bg: GPUBindGroup
  vb: GPUBuffer
  ib: GPUBuffer
  ic: number
  morphSlot?: GPUBuffer | null
}

/**
 * Groups draws by shading model, preserving first-seen order. Used only when
 * a scene mixes materials; the all-default path skips the allocation.
 */
export function groupByShadingModel<T extends { shadingModel: ShadingModel }>(draws: readonly T[]): T[][] {
  const groups = new Map<ShadingModel, T[]>()
  for (const mesh of draws) {
    const group = groups.get(mesh.shadingModel)
    if (group) group.push(mesh)
    else groups.set(mesh.shadingModel, [mesh])
  }
  return [...groups.values()]
}

/**
 * Per-entity transparent draws. Without `pipelineFor` the caller has bound the
 * default phong pipeline and every draw sets its style immediates; with it the
 * pipeline switches per shading model and immediates serve phong only.
 */
export function drawTransparentEntities<T extends EntityDraw>(
  pass: GPURenderPassEncoder,
  draws: readonly T[],
  setStyle: (pass: GPURenderPassEncoder, draw: T) => void,
  pipelineFor?: (model: ShadingModel) => [GPURenderPipeline, GPUBindGroup | undefined],
) {
  let activeModel: ShadingModel | null = pipelineFor ? null : 'phong'
  for (const g of draws) {
    if (pipelineFor && g.shadingModel !== activeModel) {
      activeModel = g.shadingModel
      const [pipeline, extraGroup] = pipelineFor(activeModel)
      pass.setPipeline(pipeline)
      if (extraGroup) pass.setBindGroup(2, extraGroup)
    }
    pass.setBindGroup(1, g.bg)
    if (activeModel === 'phong') setStyle(pass, g)
    pass.setVertexBuffer(0, g.vb)
    pass.setVertexBuffer(1, g.morphSlot!)
    pass.setIndexBuffer(g.ib, 'uint32')
    pass.drawIndexed(g.ic)
  }
}
