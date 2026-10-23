/** Structured clone copies whole distinct backing buffers; shared memory is not a snapshot. */
export function fitsClonedBufferBudget(views: readonly ArrayBufferView[], maxBytes: number): boolean {
  let bytes = 0
  for (const buffer of new Set(views.map(view => view.buffer))) {
    if (!(buffer instanceof ArrayBuffer)) return false
    bytes += buffer.byteLength
    if (bytes > maxBytes) return false
  }
  return Number.isSafeInteger(maxBytes) && maxBytes >= 0
}
