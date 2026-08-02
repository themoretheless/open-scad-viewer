import { parentPort, workerData } from 'node:worker_threads'

if (parentPort === null) throw new Error('Web Worker harness requires a parent port')
if (workerData === null || typeof workerData !== 'object'
  || typeof (workerData as { entryUrl?: unknown }).entryUrl !== 'string') {
  throw new TypeError('Web Worker harness requires an entryUrl')
}

type Listener = EventListenerOrEventListenerObject
const listeners = new Map<string, Set<Listener>>()
const queued: unknown[] = []
let imported = false

const dispatch = (data: unknown) => {
  const event = { data } as MessageEvent<unknown>
  for (const listener of listeners.get('message') ?? []) {
    if (typeof listener === 'function') listener(event)
    else listener.handleEvent(event)
  }
}

const scope = globalThis as typeof globalThis & {
  postMessage(value: unknown, options?: StructuredSerializeOptions | Transferable[]): void
  close(): void
  addEventListener(type: string, listener: Listener): void
  removeEventListener(type: string, listener: Listener): void
}

Object.defineProperties(scope, {
  postMessage: { value(value: unknown, options?: StructuredSerializeOptions | Transferable[]) {
    const transfer = Array.isArray(options) ? options : options?.transfer ?? []
    parentPort!.postMessage(value, transfer)
  }, configurable: true },
  close: { value() {
    parentPort!.close()
  }, configurable: true },
  addEventListener: { value(type: string, listener: Listener) {
    const registered = listeners.get(type) ?? new Set<Listener>()
    registered.add(listener)
    listeners.set(type, registered)
  }, configurable: true },
  removeEventListener: { value(type: string, listener: Listener) {
    listeners.get(type)?.delete(listener)
  }, configurable: true },
})

Object.defineProperty(globalThis, 'self', {
  value: scope,
  configurable: false,
  enumerable: true,
  writable: false,
})

parentPort.on('message', data => {
  if (imported) dispatch(data)
  else queued.push(data)
})

await import((workerData as { entryUrl: string }).entryUrl)
imported = true
if ((workerData as { announceReady?: unknown }).announceReady !== false) {
  parentPort.postMessage({ __webWorkerHarness: 'ready' })
}
for (const data of queued.splice(0)) dispatch(data)
