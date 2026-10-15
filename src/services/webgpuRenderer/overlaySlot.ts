/** Overlay vertex slots shared by the renderer's selection/source highlight passes. */
const MAX_OVERLAY_BUFFER_BYTES = 16 * 1024 * 1024

/** Persistent capacity-grown vertex buffer for transient overlay geometry. */
export interface OverlaySlot {
  buffer: GPUBuffer | null
  /** Allocated size in bytes; may exceed the currently written data. */
  capacity: number
  /** Vertices to draw this frame (stride 28 bytes). */
  count: number
}

/**
 * Uploads overlay vertices into a persistent buffer, growing its capacity
 * with headroom only when the current allocation is too small. Per-frame
 * hover updates therefore reuse the same GPU buffer via queue.writeBuffer
 * instead of a destroy/create pair.
 */
export function writeOverlaySlot(dev: GPUDevice | null, slot: OverlaySlot, values: number[]) {
  slot.count = 0
  if (!dev || !values.length) return
  const data = new Float32Array(values)
  const maxBytes = Math.min(MAX_OVERLAY_BUFFER_BYTES, dev.limits.maxBufferSize)
  if (data.byteLength > maxBytes) return
  if (!slot.buffer || slot.capacity < data.byteLength) {
    const capacity = Math.min(maxBytes, Math.max(4096, data.byteLength * 2))
    let replacement: GPUBuffer | null = null
    try {
      replacement = dev.createBuffer({ size: capacity, usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST })
      dev.queue.writeBuffer(replacement, 0, data)
    } catch {
      replacement?.destroy()
      return
    }
    const previous = slot.buffer
    slot.buffer = replacement
    slot.capacity = capacity
    slot.count = values.length / 7
    previous?.destroy()
    return
  }
  try {
    dev.queue.writeBuffer(slot.buffer, 0, data)
    slot.count = values.length / 7
  } catch {
    slot.count = 0
  }
}
