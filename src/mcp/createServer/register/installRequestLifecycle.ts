import type { RequestId } from '@modelcontextprotocol/server'
import type { McpServerContext } from './context'

/** Request lifecycle hooks around every finalized dispatch handler. */
export function installRequestLifecycle(ctx: McpServerContext): void {
  const { options, server } = ctx
  if (options.onRequestSettled) {
    type DispatchHandler = (...args: unknown[]) => unknown
    const protocolServer = server.server as unknown as {
      setRequestHandler: (...args: unknown[]) => void
      _requestHandlers: Map<string, DispatchHandler>
    }
    const wrappedHandlers = new WeakSet<DispatchHandler>()
    const wrapDispatchHandler = (method: string) => {
      const handler = protocolServer._requestHandlers.get(method)
      if (!handler || wrappedHandlers.has(handler)) return
      const wrapped: DispatchHandler = async (...args: unknown[]) => {
        const context = args.at(-1) as {
          mcpReq?: { id?: RequestId; signal?: AbortSignal }
        } | undefined
        const requestId = context?.mcpReq?.id
        const signal = context?.mcpReq?.signal
        if (requestId !== undefined && signal) {
          const cancelled = () => options.onRequestCancelled?.(requestId)
          if (signal.aborted) cancelled()
          else signal.addEventListener('abort', cancelled, { once: true })
        }
        try {
          return await handler(...args)
        } finally {
          if (requestId !== undefined) options.onRequestSettled?.(requestId)
        }
      }
      wrappedHandlers.add(wrapped)
      protocolServer._requestHandlers.set(method, wrapped)
    }

    // MCP SDK 2.0 has no public outer-dispatch middleware. Wrap finalized
    // registry entries so lifecycle covers SDK validation and application
    // callbacks. serveStdio installs a fresh modern discover handler after the
    // factory returns, so future registrations must pass through the same seam.
    const originalSetRequestHandler = protocolServer.setRequestHandler.bind(protocolServer)
    protocolServer.setRequestHandler = (...registration: unknown[]) => {
      originalSetRequestHandler(...registration)
      if (typeof registration[0] === 'string') wrapDispatchHandler(registration[0])
    }
    for (const method of protocolServer._requestHandlers.keys()) wrapDispatchHandler(method)
  }
}
