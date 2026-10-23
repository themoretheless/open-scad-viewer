import { executeGcodePreviewAsync } from '../services/gcodePreviewRuntime'
import { prepareGcodePreviewTransfer } from '../services/gcodePreviewWorkerTransport'
self.addEventListener('message', async event => {
  const format = event.data?.responseFormat
  const result = await executeGcodePreviewAsync(event.data)
  if (format !== 'f64-moves-v1' || !result.ok) { self.postMessage(result); return }
  try {
    const {response, transfer} = prepareGcodePreviewTransfer(result)
    self.postMessage(response, {transfer})
  } catch (error) {
    self.postMessage({version: 1, id: result.id, ok: false, error: error instanceof Error ? error.message : String(error)})
  }
})
