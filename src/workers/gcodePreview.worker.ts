import { executeGcodePreviewAsync } from '../services/gcodePreviewRuntime'
self.addEventListener('message', async event => { self.postMessage(await executeGcodePreviewAsync(event.data)) })
