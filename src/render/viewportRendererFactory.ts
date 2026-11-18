import { currentRenderSeam, setViewportProbe } from '../dev/testSeam'
import { WebGPURenderer } from './webgpuRenderer'

/**
 * Creates the main viewport renderer. Under `?renderSeam=recording` it resolves
 * to the CPU-only RecordingRenderer, which implements the public surface of
 * WebGPURenderer (checked by `implements` in recordingRenderer.ts). Only that
 * seam loads the recorder, so the ordinary path stays synchronous and the
 * startup chunk does not carry test-only code.
 */
export function createViewportRenderer(): WebGPURenderer | Promise<WebGPURenderer> {
  if (currentRenderSeam() !== 'recording') return new WebGPURenderer()
  return import('./recordingRenderer').then(({ RecordingRenderer }) => {
    const recorder = new RecordingRenderer()
    setViewportProbe(() => recorder.snapshot())
    return recorder as unknown as WebGPURenderer
  })
}
