import { readFileSync } from 'node:fs'
import * as nodeModule from 'node:module'

let registered = false

function loadRawSource(url: string, context: unknown, nextLoad: (url: string, context: unknown) => unknown): unknown {
  const source = new URL(url)
  if (source.protocol === 'file:' && source.searchParams.has('raw')) {
    source.search = ''
    return {
      format: 'module',
      shortCircuit: true,
      source: `export default ${JSON.stringify(readFileSync(source, 'utf8'))}`,
    }
  }
  return nextLoad(url, context)
}

/** Register a Node loader hook for Vite `?raw` imports when launched without `--import registerRawSourceImports`. */
export function ensureRawSourceLoader(): void {
  if (registered) return
  registered = true
  const maybeHooks = nodeModule as unknown as {
    registerHooks?: (hooks: { load: typeof loadRawSource }) => void
    register?: (specifier: string | URL, parentURL?: string | URL) => void
  }
  if (typeof maybeHooks.registerHooks === 'function') {
    maybeHooks.registerHooks({ load: loadRawSource })
  } else if (typeof maybeHooks.register === 'function') {
    maybeHooks.register(new URL('./rawSourceLoader.mjs', import.meta.url), import.meta.url)
  }
}
