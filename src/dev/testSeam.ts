import { auditAccessibility, type AccessibilityIssue } from './accessibilityAudit'
import { operationCounts, resetOperationCounts } from './operationCounters'

/**
 * Browser test seam. `?renderSeam=recording` replaces the WebGPU viewport
 * renderer with a CPU-only recorder (no adapter, no device, no draw) and
 * exposes debug hooks on `window.__osvTest`. Without the flag nothing changes.
 */
export type RenderSeam = 'webgpu' | 'recording'

export function renderSeamFromSearch(search: string): RenderSeam {
  return new URLSearchParams(search).get('renderSeam') === 'recording' ? 'recording' : 'webgpu'
}

export function currentRenderSeam(): RenderSeam {
  return typeof location === 'undefined' ? 'webgpu' : renderSeamFromSearch(location.search)
}

export interface TestSeamHooks {
  readonly renderSeam: RenderSeam
  operations(): ReturnType<typeof operationCounts>
  resetOperations(): void
  /** Scene state of the most recently created recording renderer, if any. */
  viewport(): unknown
  /** Accessibility issues in the current document (`auditAccessibility`). */
  accessibility(): AccessibilityIssue[]
}

let viewportProbe: () => unknown = () => null

export function setViewportProbe(probe: () => unknown): void {
  viewportProbe = probe
}

export function installTestSeamHooks(target: { __osvTest?: TestSeamHooks } = globalThis as never): void {
  const renderSeam = currentRenderSeam()
  if (renderSeam !== 'recording') return
  target.__osvTest = Object.freeze({
    renderSeam,
    operations: operationCounts,
    resetOperations: resetOperationCounts,
    viewport: () => viewportProbe(),
    accessibility: () => auditAccessibility(document.body),
  })
}
