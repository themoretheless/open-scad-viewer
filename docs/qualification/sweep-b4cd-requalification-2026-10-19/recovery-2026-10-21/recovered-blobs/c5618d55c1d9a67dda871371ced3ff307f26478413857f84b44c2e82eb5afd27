import type {
  JSONRPCMessage,
  MessageExtraInfo,
  RequestId,
  Transport,
  TransportSendOptions,
} from '@modelcontextprotocol/server'

export const DEFAULT_MAX_IN_FLIGHT_REQUESTS = 8
export const DEFAULT_MAX_PENDING_OUTBOUND_MESSAGES = 24
export const DEFAULT_MAX_PENDING_CONTROL_MESSAGES = 64
export const MAX_MCP_SUBSCRIPTIONS = 8
export const MAX_MCP_STRING_REQUEST_ID_LENGTH = 128
export const TRANSPORT_BUSY_RETRY_AFTER_MS = 250

export class OutboundBackpressureError extends Error {
  constructor(readonly maxPending: number) {
    super(`MCP outbound queue exceeded ${maxPending} pending messages`)
    this.name = 'OutboundBackpressureError'
  }
}

interface ActiveRequest {
  kind: 'request' | 'initialize' | 'subscription'
  handlerSettled: boolean
  cancelled: boolean
  cancelForwarded: boolean
  responseStarted: boolean
}

function admissibleRequestId(requestId: RequestId): boolean {
  return typeof requestId === 'number'
    ? Number.isSafeInteger(requestId)
    : requestId.length <= MAX_MCP_STRING_REQUEST_ID_LENGTH
}

function cancelledRequestId(message: JSONRPCMessage): RequestId | null {
  if (!('method' in message) || message.method !== 'notifications/cancelled'
    || !('params' in message) || !message.params || typeof message.params !== 'object'
    || !('requestId' in message.params)) return null
  const id = message.params.requestId
  if (typeof id !== 'string' && typeof id !== 'number') return null
  return admissibleRequestId(id) ? id : null
}

function inboundRequestId(message: JSONRPCMessage): RequestId | null {
  return 'method' in message && 'id' in message && message.id !== undefined
    ? message.id
    : null
}

function outboundResponseId(message: JSONRPCMessage): RequestId | null {
  return 'id' in message && message.id !== undefined && ('result' in message || 'error' in message)
    ? message.id
    : null
}

function acknowledgedSubscriptionId(message: JSONRPCMessage): RequestId | null {
  if (!('method' in message) || message.method !== 'notifications/subscriptions/acknowledged'
    || !('params' in message) || !message.params || typeof message.params !== 'object'
    || !('_meta' in message.params) || !message.params._meta
    || typeof message.params._meta !== 'object') return null
  const id = message.params._meta['io.modelcontextprotocol/subscriptionId']
  return typeof id === 'string' || typeof id === 'number' ? id : null
}

/**
 * Adds process-local admission control and serialized writes to a shared-channel
 * MCP transport. It rejects excess requests immediately and closes the normal
 * transport error path if a client stops draining too many large responses.
 * Small overload responses have a separate bounded reserve so a normal burst
 * cannot exhaust the data queue and tear down the process.
 */
export class BoundedTransport implements Transport {
  onclose?: () => void
  onerror?: (error: Error) => void
  onmessage?: <T extends JSONRPCMessage>(message: T, extra?: MessageExtraInfo) => void

  private readonly activeRequests = new Map<RequestId, ActiveRequest>()
  private readonly activeSubscriptions = new Set<RequestId>()
  private outboundTail: Promise<void> = Promise.resolve()
  private pendingOutbound = 0
  private pendingControl = 0
  private started = false
  private initializationResponseStarted = false
  private initializedNotificationSeen = false

  constructor(
    private readonly inner: Transport,
    private readonly limits: {
      maxInFlightRequests?: number
      maxPendingOutboundMessages?: number
      maxPendingControlMessages?: number
    } = {},
  ) {
    const maxInFlight = this.maxInFlightRequests
    const maxOutbound = this.maxPendingOutboundMessages
    const maxControl = this.maxPendingControlMessages
    if (!Number.isSafeInteger(maxInFlight) || maxInFlight < 1) {
      throw new RangeError('maxInFlightRequests must be a positive safe integer')
    }
    if (!Number.isSafeInteger(maxOutbound) || maxOutbound < 1) {
      throw new RangeError('maxPendingOutboundMessages must be a positive safe integer')
    }
    if (maxOutbound < maxInFlight) {
      throw new RangeError('maxPendingOutboundMessages must cover every in-flight request')
    }
    if (!Number.isSafeInteger(maxControl) || maxControl < 1) {
      throw new RangeError('maxPendingControlMessages must be a positive safe integer')
    }
  }

  get sessionId() {
    return this.inner.sessionId
  }

  get hasPerRequestStream() {
    return this.inner.hasPerRequestStream
  }

  readonly setProtocolVersion = (version: string) => {
    this.inner.setProtocolVersion?.(version)
  }

  readonly setSupportedProtocolVersions = (versions: string[]) => {
    this.inner.setSupportedProtocolVersions?.(versions)
  }

  async start(): Promise<void> {
    if (this.started) throw new Error('BoundedTransport already started')
    this.started = true
    this.inner.onclose = () => {
      this.activeRequests.clear()
      this.activeSubscriptions.clear()
      this.onclose?.()
    }
    this.inner.onerror = error => this.onerror?.(error)
    this.inner.onmessage = (message, extra) => this.receive(message, extra)
    await this.inner.start()
  }

  send(message: JSONRPCMessage, options?: TransportSendOptions): Promise<void> {
    if (this.pendingOutbound >= this.maxPendingOutboundMessages) {
      return Promise.reject(new OutboundBackpressureError(this.maxPendingOutboundMessages))
    }
    this.pendingOutbound++
    const responseId = outboundResponseId(message)
    const subscriptionId = acknowledgedSubscriptionId(message)
    const settledId = responseId ?? subscriptionId
    const settledRequest = settledId === null ? undefined : this.activeRequests.get(settledId)
    if (settledRequest) {
      settledRequest.responseStarted = true
      if (responseId !== null && settledRequest.kind === 'initialize') {
        this.initializationResponseStarted = true
      }
    }
    const sent = this.enqueue(message, options)
    const finish = (succeeded: boolean) => {
      this.pendingOutbound--
      if (succeeded && subscriptionId !== null && settledRequest?.kind === 'subscription'
        && !settledRequest.cancelForwarded) {
        this.activeSubscriptions.add(subscriptionId)
      }
      if (settledId !== null && this.activeRequests.get(settledId) === settledRequest) {
        this.activeRequests.delete(settledId)
      }
    }
    return sent.then(
      () => finish(true),
      error => {
        finish(false)
        throw error
      },
    )
  }

  async close(): Promise<void> {
    this.activeRequests.clear()
    this.activeSubscriptions.clear()
    await this.inner.close()
  }

  /** Called by the server's request-handler wrapper after actual handler settlement. */
  settleRequest(requestId: RequestId): void {
    const request = this.activeRequests.get(requestId)
    if (!request) return
    request.handlerSettled = true
    if (request.cancelled && !request.responseStarted) this.activeRequests.delete(requestId)
  }

  /** Called from the SDK handler's AbortSignal, not from an unvalidated wire notification. */
  cancelRequest(requestId: RequestId): void {
    const request = this.activeRequests.get(requestId)
    if (!request) return
    request.cancelled = true
    if (request.handlerSettled && !request.responseStarted) this.activeRequests.delete(requestId)
  }

  private get maxInFlightRequests(): number {
    return this.limits.maxInFlightRequests ?? DEFAULT_MAX_IN_FLIGHT_REQUESTS
  }

  private get maxPendingOutboundMessages(): number {
    return this.limits.maxPendingOutboundMessages ?? DEFAULT_MAX_PENDING_OUTBOUND_MESSAGES
  }

  private get maxPendingControlMessages(): number {
    return this.limits.maxPendingControlMessages ?? DEFAULT_MAX_PENDING_CONTROL_MESSAGES
  }

  private enqueue(message: JSONRPCMessage, options?: TransportSendOptions): Promise<void> {
    const sent = this.outboundTail.then(() => this.inner.send(message, options))
    this.outboundTail = sent.catch(() => undefined)
    return sent
  }

  private sendBusy(requestId: RequestId): void {
    // A hostile client can keep sending requests after admission closes. Keep a
    // finite reserve for small control responses; once full, dropping another
    // overload response is safer than growing memory or terminating the server.
    if (this.pendingControl >= this.maxPendingControlMessages) return
    this.pendingControl++
    void this.enqueue({
      jsonrpc: '2.0',
      id: requestId,
      error: {
        code: -32000,
        message: 'MCP server is busy; retry later',
        data: {
          code: 'server_busy',
          retry_after_ms: TRANSPORT_BUSY_RETRY_AFTER_MS,
        },
      },
    }).catch(error => this.onerror?.(error)).finally(() => {
      this.pendingControl--
    })
  }

  private receive<T extends JSONRPCMessage>(message: T, extra?: MessageExtraInfo): void {
    const requestId = inboundRequestId(message)
    if (requestId === null) {
      this.receiveNonRequest(message, extra)
      return
    }
    // Never echo an attacker-sized string id into the bounded busy-response
    // reserve. The raw frame is already capped by the SDK; dropping it here
    // prevents one large id from being retained dozens of times.
    if (!admissibleRequestId(requestId)) return
    // Reusing an active JSON-RPC id is a protocol violation. Do not forward it
    // and do not emit a second response with the same id, which could cause the
    // client to associate the original request with the wrong response.
    if (this.activeRequests.has(requestId)) return
    if (this.activeRequests.size >= this.maxInFlightRequests) {
      this.sendBusy(requestId)
      return
    }
    const method = 'method' in message ? message.method : ''
    this.activeRequests.set(requestId, {
      kind: method === 'initialize'
        ? 'initialize'
        : method === 'subscriptions/listen' ? 'subscription' : 'request',
      handlerSettled: false,
      cancelled: false,
      cancelForwarded: false,
      responseStarted: false,
    })
    this.onmessage?.(message, extra)
  }

  private receiveNonRequest<T extends JSONRPCMessage>(message: T, extra?: MessageExtraInfo): void {
    // This server never issues client-bound requests, so inbound JSON-RPC
    // responses have no consumer and are dropped before serveStdio's queue.
    if (!('method' in message)) return

    if (message.method === 'notifications/initialized') {
      if (!this.initializationResponseStarted || this.initializedNotificationSeen) return
      this.initializedNotificationSeen = true
      this.onmessage?.(message, extra)
      return
    }

    if (message.method !== 'notifications/cancelled') return
    const requestId = cancelledRequestId(message)
    if (requestId === null) return
    const request = this.activeRequests.get(requestId)
    if (request) {
      if (request.cancelForwarded) return
      request.cancelForwarded = true
      this.onmessage?.(message, extra)
      return
    }
    if (this.activeSubscriptions.delete(requestId)) this.onmessage?.(message, extra)
  }
}
