import { nextTick } from 'vue'

const ARROW_STEPS: Record<string, number> = { ArrowRight: 1, ArrowDown: 1, ArrowLeft: -1, ArrowUp: -1 }

/**
 * Roving tab stop for a row of tool buttons: only one button takes a Tab stop, arrows move between them.
 * Returns the next value when an arrow key was handled; buttons carry `data-roving="<value>"`.
 */
export function rovingTabStopKey(event: KeyboardEvent, values: readonly string[], current: string): string | undefined {
  const step = ARROW_STEPS[event.key]
  if (!step || !values.length) return undefined
  event.preventDefault()
  event.stopPropagation()
  const next = values[(Math.max(0, values.indexOf(current)) + step + values.length) % values.length]
  const row = (event.currentTarget as HTMLElement | null)?.parentElement
  void nextTick(() => row?.querySelector?.<HTMLElement>(`[data-roving="${next}"]`)?.focus())
  return next
}
