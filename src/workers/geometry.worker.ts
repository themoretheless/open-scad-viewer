import { parseOpenSCAD, OpenSCADParseError } from '../services/openscadParser'
import type { GeometryRequest, GeometryResponse } from '../services/geometryWorkerProtocol'

self.addEventListener('message', async (event: MessageEvent<GeometryRequest>) => {
  const { id, source, quality } = event.data
  // Validate the id and ALWAYS respond; the main thread discards stale
  // responses by comparing against its own latest id. The previous
  // Math.max(latestRequest, id) tracker was NaN-poisonable (one malformed
  // message muted every future response) and suppressed responses that a
  // per-id waiter on the main side would wait on forever.
  if (!Number.isInteger(id) || typeof source !== 'string') return
  const started = performance.now()

  try {
    const result = await parseOpenSCAD(source, { quality })
    const response: GeometryResponse = {
      id,
      ok: true,
      ...result,
      durationMs: performance.now() - started,
    }
    const transfer = result.meshes.flatMap(mesh => [
      mesh.vertices.buffer,
      mesh.indices.buffer,
      mesh.edgeIndices.buffer,
      mesh.faceIds.buffer,
      mesh.bvh.bounds.buffer,
      mesh.bvh.nodes.buffer,
      mesh.bvh.triangles.buffer,
    ])
    self.postMessage(response, { transfer })
  } catch (error) {
    const response: GeometryResponse = {
      id,
      ok: false,
      error: {
        name: error instanceof Error ? error.name : 'Error',
        message: error instanceof Error ? error.message : String(error),
        line: error instanceof OpenSCADParseError ? error.line : undefined,
        column: error instanceof OpenSCADParseError ? error.column : undefined,
      },
      durationMs: performance.now() - started,
    }
    self.postMessage(response)
  }
})
