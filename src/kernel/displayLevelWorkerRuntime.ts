/**
 * Message handling of the display level worker. Requests are queued and run
 * one per macrotask so a `cancel` that arrives while earlier work runs still
 * drops a queued request before its native evaluation starts.
 */
import { DisplayLevelService, type DisplayLevelPayload, type DisplayLevelSummaryEntry } from './displayLevelService'

export type DisplayLevelRequest =
  | { readonly type: 'summary'; readonly id: number; readonly geometry: string; readonly geometryJson: string }
  | {
      readonly type: 'level'
      readonly id: number
      readonly geometry: string
      readonly geometryJson: string
      readonly index: number
    }
  | { readonly type: 'cancel'; readonly id: number }
  | { readonly type: 'forget'; readonly geometry: string }

export type DisplayLevelResponse =
  | { readonly id: number; readonly ok: true; readonly summary: DisplayLevelSummaryEntry[] | null }
  | { readonly id: number; readonly ok: true; readonly level: DisplayLevelPayload }
  | { readonly id: number; readonly ok: false; readonly error: string }

type Post = (message: DisplayLevelResponse, transfer: Transferable[]) => void
type Defer = (task: () => void) => void

const levelTransferables = (level: DisplayLevelPayload): Transferable[] => {
  const buffers = new Set<ArrayBuffer>()
  for (const view of [
    level.vertices,
    level.indices,
    level.edgeIndices,
    level.faceIds,
    level.bvh.bounds,
    level.bvh.nodes,
    level.bvh.triangles,
  ]) {
    if (view.buffer instanceof ArrayBuffer) buffers.add(view.buffer)
  }
  return [...buffers]
}

/** Copy a cached level so transferring it does not detach the worker's cache. */
const cloneLevel = (level: DisplayLevelPayload): DisplayLevelPayload => ({
  vertices: level.vertices.slice(),
  indices: level.indices.slice(),
  edgeIndices: level.edgeIndices.slice(),
  faceIds: level.faceIds.slice(),
  bvh: {
    ...level.bvh,
    bounds: level.bvh.bounds.slice(),
    nodes: level.bvh.nodes.slice(),
    triangles: level.bvh.triangles.slice(),
  },
})

export function createDisplayLevelWorkerHandler(
  post: Post,
  service = new DisplayLevelService(),
  defer: Defer = task => setTimeout(task, 0),
) {
  const queue: Exclude<DisplayLevelRequest, { type: 'cancel' } | { type: 'forget' }>[] = []
  let scheduled = false
  const run = () => {
    scheduled = false
    const request = queue.shift()
    if (!request) return
    try {
      if (request.type === 'summary') {
        post({ id: request.id, ok: true, summary: service.summary(request.geometry, request.geometryJson) }, [])
      } else {
        const level = cloneLevel(service.level(request.geometry, request.geometryJson, request.index))
        post({ id: request.id, ok: true, level }, levelTransferables(level))
      }
    } catch (error) {
      post({ id: request.id, ok: false, error: error instanceof Error ? error.message : String(error) }, [])
    }
    schedule()
  }
  const schedule = () => {
    if (scheduled || !queue.length) return
    scheduled = true
    defer(run)
  }
  return (message: DisplayLevelRequest) => {
    if (message.type === 'cancel') {
      const index = queue.findIndex(request => request.id === message.id)
      if (index >= 0) queue.splice(index, 1)
      return
    }
    if (message.type === 'forget') {
      service.forget(message.geometry)
      return
    }
    queue.push(message)
    schedule()
  }
}
