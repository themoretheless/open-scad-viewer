import { onMounted, onUnmounted } from 'svet'
import type { OpenScadHandoffDocument } from '../core/handoffBundle'
import { LOCAL_EDITOR_HEADER, LOCAL_EDITOR_PATH } from '../core/localEditorProtocol'

export function useLocalEditorBridge(open: (document: OpenScadHandoffDocument) => Promise<boolean>) {
  let timer: ReturnType<typeof setTimeout> | undefined
  const lifecycle = new AbortController()

  async function poll() {
    try {
      if (document.visibilityState !== 'visible') return
      const response = await fetch(LOCAL_EDITOR_PATH, {
        cache: 'no-store',
        headers: { [LOCAL_EDITOR_HEADER]: 'poll' },
        signal: lifecycle.signal,
      })
      if (response.headers.get(LOCAL_EDITOR_HEADER) !== 'bridge') {
        lifecycle.abort()
        return
      }
      if (response.status === 204) return
      // Static hosts can return the SPA HTML for an unknown URL.
      if (!response.ok || !response.headers.get('content-type')?.startsWith('application/json')) {
        lifecycle.abort()
        return
      }
      const { id, bundle } = await response.json()
      if (typeof id !== 'string' || !/^[a-f0-9-]{36}$/.test(id) || lifecycle.signal.aborted) return
      let opened = false
      try {
        // The verifier is needed only once the local bridge delivers a bundle.
        const { verifyOpenScadHandoffBundle } = await import('../core/handoffBundle')
        const verified = verifyOpenScadHandoffBundle(bundle)
        if (verified.origin === 'mcp-catalog') opened = await open(verified.document)
      } finally {
        await fetch(LOCAL_EDITOR_PATH, {
          method: 'POST',
          headers: { [LOCAL_EDITOR_HEADER]: `${opened ? 'opened' : 'blocked'}:${id}` },
          signal: lifecycle.signal,
        })
      }
    } catch {
      // A local server restart can interrupt a poll. The next poll reconnects.
    } finally {
      if (!lifecycle.signal.aborted) timer = setTimeout(poll, 800)
    }
  }

  onMounted(() => {
    if (['localhost', '127.0.0.1', '[::1]'].includes(location.hostname)) void poll()
  })
  onUnmounted(() => {
    lifecycle.abort()
    clearTimeout(timer)
  })
}
