import { svgPreview, svgProfile, contoursSvg, contoursExtrusion, meshSvgContours } from './svgGeometry'
import { createWorkerHandler } from './workerHandlerRuntime'
import type { SvgGeometryResult, SvgJob, SvgWorkerRequest, SvgWorkerResponse } from './svgWorkerProtocol'

/** Shared operation composition for browser workers and Node callers. Core SVG semantics stay in svgGeometry. */
export async function executeSvgJob(job: SvgJob): Promise<SvgGeometryResult> {
  if (job.kind === 'preview') return svgPreview(job.svg, job.options)
  if (job.kind === 'project') {
    const svg = contoursSvg(await meshSvgContours(job.meshes, { axis: job.axis, face: job.face }))
    return { ...await svgPreview(svg, job.options), svg }
  }
  const profile = await svgProfile(job.svg, job.options)
  return { svg: contoursSvg(profile.contours), widthMm: profile.widthMm, heightMm: profile.heightMm, warnings: profile.warnings,
    ...(job.kind === 'extrude' ? { source: contoursExtrusion(profile.contours, job.height) } : {}) }
}

export function createSvgWorkerHandler(post: (value: SvgWorkerResponse) => void) {
  return createWorkerHandler<SvgWorkerRequest, SvgWorkerResponse, SvgGeometryResult>(post, {
    validate: (value) => {
      const request = value as Partial<SvgWorkerRequest> | null
      if (!request || request.version !== 1 || !Number.isSafeInteger(request.id) || !request.job) return null
      return request as SvgWorkerRequest
    },
    busyError: { name: 'Error', message: 'SVG worker is busy.' },
    execute: (request) => executeSvgJob(request.job),
    success: (request, result) => ({ message: { version: 1, id: request.id, ok: true, result } }),
    failure: (request, error) => ({ version: 1, id: request.id, ok: false, error }),
  })
}
