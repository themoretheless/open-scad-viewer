import { parseOpenSCAD, OpenSCADParseError } from '../services/openscadParser'
import type { GeometryRequest, GeometryResponse } from '../services/geometryWorkerProtocol'

let latestRequest = 0

self.addEventListener('message', async (event: MessageEvent<GeometryRequest>) => {
  const { id, source, quality } = event.data
  latestRequest = Math.max(latestRequest, id)
  const started = performance.now()

  try {
    const result = await parseOpenSCAD(source, { quality })
    if (id !== latestRequest) return
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
    if (id !== latestRequest) return
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
