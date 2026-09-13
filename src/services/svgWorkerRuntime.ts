import { svgPreview, svgProfile, contoursSvg, contoursExtrusion, meshSvgContours } from './svgGeometry'
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
  let active = false
  return async (value: unknown) => {
    const request = value as Partial<SvgWorkerRequest> | null
    if (!request || request.version !== 1 || !Number.isSafeInteger(request.id) || !request.job) return
    const id = request.id!
    if (active) { post({ version: 1, id, ok: false, error: { name: 'Error', message: 'SVG worker is busy.' } }); return }
    active = true
    try {
      const result = await executeSvgJob(request.job)
      post({ version: 1, id, ok: true, result })
    } catch (cause) {
      const error = cause instanceof Error ? cause : new Error(String(cause))
      post({ version: 1, id, ok: false, error: { name: error.name, message: error.message, ...('code' in error && typeof error.code === 'string' ? { code: error.code } : {}) } })
    } finally { active = false }
  }
}
