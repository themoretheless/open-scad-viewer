import { currentRenderSeam, setViewportProbe } from '../dev/testSeam'
import { RecordingRenderer } from './recordingRenderer'
import { WebGPURenderer } from './webgpuRenderer'

/**
 * Creates the main viewport renderer. Under `?renderSeam=recording` it returns
 * the CPU-only RecordingRenderer, which implements the public surface of
 * WebGPURenderer (checked by `implements` in recordingRenderer.ts).
 */
export function createViewportRenderer(): WebGPURenderer {
  if (currentRenderSeam() !== 'recording') return new WebGPURenderer()
  const recorder = new RecordingRenderer()
  setViewportProbe(() => recorder.snapshot())
  return recorder as unknown as WebGPURenderer
}
