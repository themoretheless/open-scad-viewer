export type RovingFocusKey = 'ArrowLeft' | 'ArrowRight' | 'Home' | 'End'

/** Resolve the next item for a horizontal, wrapping roving-tabindex control. */
export function nextRovingIndex(current: number, count: number, key: RovingFocusKey): number {
  if (count <= 0) return -1
  const safeCurrent = Math.max(0, Math.min(count - 1, current))
  if (key === 'Home') return 0
  if (key === 'End') return count - 1
  return (safeCurrent + (key === 'ArrowRight' ? 1 : -1) + count) % count
}
