/**
 * ResizeObserver wiring for the renderer canvas, split out so the
 * observe/disconnect lifecycle can be tested without a DOM.
 */
export class ResizeWatcher {
  private observer: ResizeObserver | null = null

  /** Starts observing `element`; a no-op when ResizeObserver is unavailable. */
  observe(element: Element | null, onResize: () => void): void {
    if (!element || typeof ResizeObserver === 'undefined') return
    this.observer = new ResizeObserver(() => onResize())
    this.observer.observe(element)
  }

  disconnect(): void {
    this.observer?.disconnect()
    this.observer = null
  }
}
