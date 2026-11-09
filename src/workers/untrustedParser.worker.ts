import { executeUntrustedParserJob, parserWorkerError } from '../services/untrustedParserWorkerRuntime'
import type { UntrustedParserRequest, UntrustedParserResponse } from '../services/untrustedParserWorker'

self.addEventListener('message', event => {
  const request = event.data as UntrustedParserRequest
  if (!request || request.version !== 1 || !Number.isSafeInteger(request.id) || !request.job) return
  let response: UntrustedParserResponse
  try { response = { version: 1, id: request.id, ok: true, result: executeUntrustedParserJob(request.job) } }
  catch (error) { response = { version: 1, id: request.id, ok: false, error: parserWorkerError(error) } }
  self.postMessage(response)
})
