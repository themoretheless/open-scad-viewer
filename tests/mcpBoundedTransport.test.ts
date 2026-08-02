import { describe, expect, it, vi } from 'vitest'
import type {
  JSONRPCMessage,
  MessageExtraInfo,
  Transport,
  TransportSendOptions,
} from '@modelcontextprotocol/server'
import { BoundedTransport, OutboundBackpressureError } from '../src/mcp/boundedTransport'

class ControlledTransport implements Transport {
  onclose?: () => void
  onerror?: (error: Error) => void
  onmessage?: <T extends JSONRPCMessage>(message: T, extra?: MessageExtraInfo) => void
  readonly sent: JSONRPCMessage[] = []
  private readonly releases: Array<() => void> = []

  async start() {}

  send(message: JSONRPCMessage, _options?: TransportSendOptions): Promise<void> {
    this.sent.push(message)
    return new Promise(resolve => this.releases.push(resolve))
  }

  async close() {
    this.onclose?.()
  }

  receive(message: JSONRPCMessage) {
    this.onmessage?.(message)
  }

  releaseNext() {
    this.releases.shift()?.()
  }
}

class ImmediateTransport implements Transport {
  onclose?: () => void
  onerror?: (error: Error) => void
  onmessage?: <T extends JSONRPCMessage>(message: T, extra?: MessageExtraInfo) => void
  readonly sent: JSONRPCMessage[] = []

  async start() {}

  async send(message: JSONRPCMessage): Promise<void> {
    this.sent.push(message)
  }

  async close() {
    this.onclose?.()
  }

  receive(message: JSONRPCMessage) {
    this.onmessage?.(message)
  }
}

describe('BoundedTransport', () => {
  it('rejects excess in-flight requests before forwarding them to the server', async () => {
    const inner = new ControlledTransport()
    const transport = new BoundedTransport(inner, {
      maxInFlightRequests: 2,
      maxPendingOutboundMessages: 2,
    })
    const received: JSONRPCMessage[] = []
    transport.onmessage = message => received.push(message)
    await transport.start()

    inner.receive({ jsonrpc: '2.0', id: 1, method: 'tools/list', params: {} })
    inner.receive({ jsonrpc: '2.0', id: 2, method: 'resources/list', params: {} })
    inner.receive({ jsonrpc: '2.0', id: 3, method: 'prompts/list', params: {} })

    expect(received.map(message => 'id' in message ? message.id : null)).toEqual([1, 2])
    await vi.waitFor(() => expect(inner.sent).toHaveLength(1))
    expect(inner.sent[0]).toMatchObject({
      id: 3,
      error: { code: -32000, data: { code: 'server_busy' } },
    })
    inner.releaseNext()

    const response = transport.send({ jsonrpc: '2.0', id: 1, result: {} })
    await vi.waitFor(() => expect(inner.sent).toHaveLength(2))
    inner.releaseNext()
    await response

    inner.receive({ jsonrpc: '2.0', id: 4, method: 'tools/list', params: {} })
    expect(received.map(message => 'id' in message ? message.id : null)).toEqual([1, 2, 4])
    await transport.close()
  })

  it('serializes outbound writes and caps messages waiting behind backpressure', async () => {
    const inner = new ControlledTransport()
    const transport = new BoundedTransport(inner, {
      maxInFlightRequests: 2,
      maxPendingOutboundMessages: 2,
    })
    await transport.start()

    const first = transport.send({ jsonrpc: '2.0', method: 'notifications/tools/list_changed' })
    const second = transport.send({ jsonrpc: '2.0', method: 'notifications/resources/list_changed' })
    await expect(transport.send({
      jsonrpc: '2.0',
      method: 'notifications/prompts/list_changed',
    })).rejects.toBeInstanceOf(OutboundBackpressureError)

    await vi.waitFor(() => expect(inner.sent).toHaveLength(1))
    inner.releaseNext()
    await first
    await vi.waitFor(() => expect(inner.sent).toHaveLength(2))
    inner.releaseNext()
    await second
    await transport.close()
  })

  it('does not forward duplicate active request ids', async () => {
    const inner = new ImmediateTransport()
    const transport = new BoundedTransport(inner, {
      maxInFlightRequests: 2,
      maxPendingOutboundMessages: 2,
    })
    const received: JSONRPCMessage[] = []
    transport.onmessage = message => received.push(message)
    await transport.start()

    for (let index = 0; index < 100; index++) {
      inner.receive({ jsonrpc: '2.0', id: 1, method: 'tools/list', params: {} })
    }

    expect(received).toHaveLength(1)
    expect(inner.sent).toHaveLength(0)
    await transport.close()
  })

  it('drops oversized string request ids instead of retaining and echoing them', async () => {
    const inner = new ImmediateTransport()
    const transport = new BoundedTransport(inner, {
      maxInFlightRequests: 2,
      maxPendingOutboundMessages: 2,
    })
    const received: JSONRPCMessage[] = []
    transport.onmessage = message => received.push(message)
    await transport.start()

    const oversizedId = 'x'.repeat(1024 * 1024)
    for (let index = 0; index < 100; index++) {
      inner.receive({ jsonrpc: '2.0', id: oversizedId, method: 'tools/list', params: {} })
    }

    expect(received).toEqual([])
    expect(inner.sent).toEqual([])
    await transport.close()
  })

  it('keeps a cancelled request admitted until its handler actually settles', async () => {
    const inner = new ControlledTransport()
    const transport = new BoundedTransport(inner, {
      maxInFlightRequests: 1,
      maxPendingOutboundMessages: 1,
    })
    const received: JSONRPCMessage[] = []
    transport.onmessage = message => received.push(message)
    await transport.start()

    inner.receive({ jsonrpc: '2.0', id: 1, method: 'resources/read', params: {} })
    inner.receive({
      jsonrpc: '2.0',
      method: 'notifications/cancelled',
      params: { requestId: 1, reason: 'no longer needed' },
    })
    inner.receive({
      jsonrpc: '2.0',
      method: 'notifications/cancelled',
      params: { requestId: 1, reason: 'duplicate cancellation' },
    })
    transport.cancelRequest(1)
    inner.receive({ jsonrpc: '2.0', id: 2, method: 'resources/read', params: {} })

    expect(received.map(message => 'id' in message ? message.id : null)).toEqual([1, null])
    await vi.waitFor(() => expect(inner.sent).toHaveLength(1))
    expect(inner.sent[0]).toMatchObject({ id: 2, error: { data: { code: 'server_busy' } } })
    inner.releaseNext()
    transport.settleRequest(1)

    inner.receive({ jsonrpc: '2.0', id: 3, method: 'resources/read', params: {} })
    expect(received.map(message => 'id' in message ? message.id : null)).toEqual([1, null, 3])
    await transport.close()
  })

  it('survives a draining burst without treating busy control responses as fatal backpressure', async () => {
    const inner = new ImmediateTransport()
    const transport = new BoundedTransport(inner, {
      maxInFlightRequests: 2,
      maxPendingOutboundMessages: 2,
      maxPendingControlMessages: 64,
    })
    const received: JSONRPCMessage[] = []
    const errors: Error[] = []
    transport.onmessage = message => received.push(message)
    transport.onerror = error => errors.push(error)
    await transport.start()

    for (let id = 1; id <= 24; id++) {
      inner.receive({ jsonrpc: '2.0', id, method: 'tools/list', params: {} })
    }

    await vi.waitFor(() => expect(inner.sent).toHaveLength(22))
    expect(received.map(message => 'id' in message ? message.id : null)).toEqual([1, 2])
    expect(inner.sent.every(message => 'error' in message
      && message.error.data && typeof message.error.data === 'object'
      && 'code' in message.error.data && message.error.data.code === 'server_busy')).toBe(true)
    expect(errors).toEqual([])
    await transport.close()
  })

  it('does not retire a settled normal request until its own response write completes', async () => {
    const inner = new ControlledTransport()
    const transport = new BoundedTransport(inner, {
      maxInFlightRequests: 1,
      maxPendingOutboundMessages: 1,
    })
    const received: JSONRPCMessage[] = []
    transport.onmessage = message => received.push(message)
    await transport.start()

    inner.receive({ jsonrpc: '2.0', id: 1, method: 'tools/list', params: {} })
    transport.settleRequest(1)
    const response = transport.send({ jsonrpc: '2.0', id: 1, result: {} })
    transport.cancelRequest(1)
    inner.receive({ jsonrpc: '2.0', id: 1, method: 'resources/list', params: {} })

    expect(received).toHaveLength(1)
    await vi.waitFor(() => expect(inner.sent).toHaveLength(1))
    inner.releaseNext()
    await response

    inner.receive({ jsonrpc: '2.0', id: 1, method: 'resources/list', params: {} })
    expect(received).toHaveLength(2)
    await transport.close()
  })

  it('releases a cancelled request after the outer SDK handler reports abort and settlement', async () => {
    const inner = new ControlledTransport()
    const transport = new BoundedTransport(inner, {
      maxInFlightRequests: 1,
      maxPendingOutboundMessages: 1,
    })
    const received: JSONRPCMessage[] = []
    transport.onmessage = message => received.push(message)
    await transport.start()

    inner.receive({ jsonrpc: '2.0', id: 1, method: 'ping', params: {} })
    inner.receive({
      jsonrpc: '2.0',
      method: 'notifications/cancelled',
      params: { requestId: 1 },
    })
    transport.cancelRequest(1)
    transport.settleRequest(1)
    inner.receive({ jsonrpc: '2.0', id: 2, method: 'tools/list', params: {} })
    expect(received.map(message => 'id' in message ? message.id : null)).toEqual([1, null, 2])

    await transport.close()
  })

  it('bounds inbound notifications while a subscription acknowledgement is backpressured', async () => {
    const inner = new ControlledTransport()
    const transport = new BoundedTransport(inner, {
      maxInFlightRequests: 2,
      maxPendingOutboundMessages: 2,
    })
    const received: JSONRPCMessage[] = []
    transport.onmessage = message => received.push(message)
    await transport.start()

    inner.receive({ jsonrpc: '2.0', id: 1, method: 'subscriptions/listen', params: {} })
    const acknowledgement = transport.send({
      jsonrpc: '2.0',
      method: 'notifications/subscriptions/acknowledged',
      params: {
        notifications: {},
        _meta: { 'io.modelcontextprotocol/subscriptionId': 1 },
      },
    })
    for (let index = 0; index < 1_000; index++) {
      inner.receive({
        jsonrpc: '2.0',
        method: 'notifications/cancelled',
        params: { requestId: index === 0 ? 1 : 10_000 + index, reason: 'x'.repeat(1_024) },
      })
      inner.receive({ jsonrpc: '2.0', method: 'notifications/progress', params: { progressToken: index } })
      inner.receive({ jsonrpc: '2.0', id: 20_000 + index, result: {} })
    }

    expect(received.map(message => 'id' in message ? message.id : null)).toEqual([1, null])
    await vi.waitFor(() => expect(inner.sent).toHaveLength(1))
    inner.releaseNext()
    await acknowledgement
    await transport.close()
  })

  it('forwards initialized once and only after the initialize response starts', async () => {
    const inner = new ControlledTransport()
    const transport = new BoundedTransport(inner, {
      maxInFlightRequests: 1,
      maxPendingOutboundMessages: 1,
    })
    const received: JSONRPCMessage[] = []
    transport.onmessage = message => received.push(message)
    await transport.start()

    inner.receive({ jsonrpc: '2.0', method: 'notifications/initialized' })
    inner.receive({ jsonrpc: '2.0', id: 1, method: 'initialize', params: {} })
    const response = transport.send({ jsonrpc: '2.0', id: 1, result: {} })
    inner.receive({ jsonrpc: '2.0', method: 'notifications/initialized' })
    inner.receive({ jsonrpc: '2.0', method: 'notifications/initialized' })

    expect(received.map(message => 'id' in message ? message.id : null)).toEqual([1, null])
    await vi.waitFor(() => expect(inner.sent).toHaveLength(1))
    inner.releaseNext()
    await response
    await transport.close()
  })
})
