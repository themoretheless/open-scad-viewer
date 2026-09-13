import { executeGcodePreview } from '../services/gcodePreviewRuntime'
self.onmessage = event => { self.postMessage(executeGcodePreview(event.data)) }
