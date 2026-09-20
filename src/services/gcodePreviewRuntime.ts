import { emitPolygonMeshGcode, emitPolygonMeshGcodeJob, inspectGcode } from './geometry/polygon'
import { flattenGroupGeometry } from './meshFlatten'
import { warmGeometryKernel } from './geometry/kernel'
import { checkGcodePreviewJob, type GcodePreviewJob, type GcodePreviewRequest, type GcodePreviewResponse } from './gcodePreviewProtocol'

function requestId(request: GcodePreviewRequest): number {
  return request && typeof request === 'object' && Number.isSafeInteger(request.id) ? request.id : 0
}

function checkedJob(request: GcodePreviewRequest): GcodePreviewJob {
  if (!request || request.version !== 1 || !Number.isSafeInteger(request.id) || request.id < 1) throw new Error('Invalid G-code worker request.')
  return checkGcodePreviewJob(request.job)
}

function failure(id: number, error: unknown): GcodePreviewResponse {
  return { version: 1, id, ok: false, error: error instanceof Error ? error.message : String(error) }
}

function executeJob(id: number, job: GcodePreviewJob): GcodePreviewResponse {
  if (job.kind === 'parse') {
    const inspected = inspectGcode(job.gcode)
    return {
      version: 1,
      id,
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
      id,
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
  return { version: 1, id, ok: true, result }
}

export function executeGcodePreview(request: GcodePreviewRequest): GcodePreviewResponse {
  const id = requestId(request)
  try {
    return executeJob(id, checkedJob(request))
  } catch (error) {
    return failure(id, error)
  }
}

/** The worker owns the structured-cloned request; warm before synchronous geometry work. */
export async function executeGcodePreviewAsync(request: GcodePreviewRequest): Promise<GcodePreviewResponse> {
  const id = requestId(request)
  try {
    const job = checkedJob(request)
    await warmGeometryKernel()
    return executeJob(id, job)
  } catch (error) {
    return failure(id, error)
  }
}
