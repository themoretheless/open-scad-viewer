import { emitPolygonMeshGcode, emitPolygonMeshGcodeJob, inspectGcode } from './geometry/polygon'
import { flattenGroupGeometry } from './meshFlatten'
import { checkGcodePreviewJob, type GcodePreviewRequest, type GcodePreviewResponse } from './gcodePreviewProtocol'

export function executeGcodePreview(request: GcodePreviewRequest): GcodePreviewResponse {
  const id = request && typeof request === 'object' && Number.isSafeInteger(request.id) ? request.id : 0
  try {
    if (!request || request.version !== 1 || !Number.isSafeInteger(request.id) || request.id < 1) throw new Error('Invalid G-code worker request.')
    const job = checkGcodePreviewJob(request.job)
    if (job.kind === 'parse') {
      const inspected = inspectGcode(job.gcode)
      return {
        version: 1,
        id: request.id,
        ok: true,
        result: {
          gcode: job.gcode,
          preview: inspected.preview,
          dialect: inspected.dialect,
          native: inspected.native,
          generator: inspected.generator,
          flavor: inspected.flavor,
        },
      }
    }
    const mesh = flattenGroupGeometry([job.mesh])
    if (job.kind === 'job') {
      const result = emitPolygonMeshGcodeJob(mesh, job.zMin, job.zMax, job.settings)
      return {
        version: 1,
        id: request.id,
        ok: true,
        result: {
          gcode: result.gcode,
          preview: result.preview,
          dialect: result.dialect,
          gcode3mfBase64: result.gcode3mfBase64,
          native: true,
          flavor: result.flavor,
        },
      }
    }
    const result = emitPolygonMeshGcode(mesh, job.zMin, job.zMax, job.settings)
    return { version: 1, id: request.id, ok: true, result }
  } catch (error) {
    return { version: 1, id, ok: false, error: error instanceof Error ? error.message : String(error) }
  }
}
