import { createDisplayLevelWorkerHandler, type DisplayLevelRequest } from '../kernel/displayLevelWorkerRuntime'

const handle = createDisplayLevelWorkerHandler((message, transfer) => self.postMessage(message, { transfer }))
self.addEventListener('message', (event: MessageEvent<DisplayLevelRequest>) => handle(event.data))
