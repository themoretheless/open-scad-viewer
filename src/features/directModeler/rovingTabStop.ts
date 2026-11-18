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

/**
 * One Tab stop per scene row: the row's secondary buttons (isolate, hide, lock) leave the Tab order and
 * ArrowLeft/ArrowRight move focus between all enabled buttons of the row (the handler sits on the row).
 */
export function rowButtonArrowKey(event: KeyboardEvent) {
  const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0
  const row = event.currentTarget as HTMLElement | null
  if (!step || !row?.querySelectorAll) return
  const buttons = [...row.querySelectorAll<HTMLButtonElement>('button:not(:disabled)')]
  const at = buttons.indexOf(event.target as HTMLButtonElement)
  if (at < 0 || !buttons.length) return
  event.preventDefault()
  buttons[(at + step + buttons.length) % buttons.length]?.focus()
}
