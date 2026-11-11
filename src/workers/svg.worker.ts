import { createSvgWorkerHandler } from '../exchange/svgWorkerRuntime'
import type { SvgWorkerResponse } from '../exchange/svgWorkerProtocol'
const handle = createSvgWorkerHandler((message: SvgWorkerResponse) => self.postMessage(message))
self.addEventListener('message', event => { void handle(event.data) })
