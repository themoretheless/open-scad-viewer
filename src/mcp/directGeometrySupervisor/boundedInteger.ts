import { MAX_DRAINED_CHILD_OUTPUT_BYTES } from '../directGeometrySupervisor'
import { Worker } from 'node:worker_threads'

export function boundedInteger(
  value: number,
  label: string,
  minimum: number,
  maximum: number,
): number {
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) {
    throw new RangeError(`${label} must be an integer between ${minimum} and ${maximum}`)
  }
  return value
}

export function defaultWorkerFactory(_workerEpoch: number): Worker {
  return new Worker(new URL('../directGeometry.worker.mjs', import.meta.url), {
    // The bootstrap owns its loader; do not inherit a second --import tsx realm.
    execArgv: [],
    // Geometry source never needs host credentials or configuration. An empty
    // environment keeps accidental dependency reads from inheriting secrets.
    env: {},
    stdout: true,
    stderr: true,
    resourceLimits: {
      maxOldGenerationSizeMb: 256,
      maxYoungGenerationSizeMb: 32,
      stackSizeMb: 8,
    },
  })
}

export function attachBoundedOutputDrain(stream: Worker['stdout']): () => void {
  let countedBytes = 0
  const onData = (chunk: unknown) => {
    const byteLength = typeof chunk === 'string'
      ? Buffer.byteLength(chunk)
      : ArrayBuffer.isView(chunk) ? chunk.byteLength : 0
    countedBytes = Math.min(MAX_DRAINED_CHILD_OUTPUT_BYTES, countedBytes + byteLength)
  }
  try {
    stream.unpipe()
    stream.on('data', onData)
    stream.resume()
    return () => {
      try {
        stream.off('data', onData)
      } catch {
        // A joined child owns no further output; cleanup is best effort.
      }
    }
  } catch {
    // A real Worker pipe supports the operations above. Keep an injected test
    // double from violating the more important terminate-and-join invariant.
    try { stream.resume() } catch { /* no-op */ }
    return () => undefined
  }
}

export function abortError(message: string): DOMException {
  return new DOMException(message, 'AbortError')
}
