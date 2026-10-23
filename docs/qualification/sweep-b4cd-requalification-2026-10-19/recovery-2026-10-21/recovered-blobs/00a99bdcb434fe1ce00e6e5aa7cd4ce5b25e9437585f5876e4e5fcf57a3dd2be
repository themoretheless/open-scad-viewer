import { createSvgWorkerHandler } from '../services/svgWorkerRuntime'
import type { SvgWorkerResponse } from '../services/svgWorkerProtocol'
const handle = createSvgWorkerHandler((message: SvgWorkerResponse) => self.postMessage(message))
self.addEventListener('message', event => { void handle(event.data) })
