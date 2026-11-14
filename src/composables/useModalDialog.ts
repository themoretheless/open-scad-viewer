import { nextTick, onBeforeUnmount, watch, type Ref } from 'svet'

/** Elements that can take keyboard focus; `focusableElements` drops disabled, hidden and `tabindex="-1"` ones. */
const FOCUSABLE =
  'a[href], area[href], button, input:not([type="hidden"]), select, textarea, summary, iframe, [contenteditable="true"], [tabindex]'

type Focusable = HTMLElement

function isDisabled(element: Focusable): boolean {
  try {
    return element.matches(':disabled')
  } catch {
    return false
  }
}

function isRendered(element: Focusable): boolean {
  return typeof element.getClientRects !== 'function' || element.getClientRects().length > 0
}

/** Tab stops inside `root` in document order. */
export function focusableElements(root: Focusable): Focusable[] {
  return [...root.querySelectorAll<Focusable>(FOCUSABLE)].filter(
    element => element.tabIndex >= 0 && !isDisabled(element) && isRendered(element),
  )
}

/** Index of the next tab stop when focus cycles inside a dialog; `current` is -1 when focus is outside. */
export function nextTrappedIndex(current: number, count: number, backwards: boolean): number {
  if (backwards) return current <= 0 ? count - 1 : current - 1
  return current < 0 || current === count - 1 ? 0 : current + 1
}

let scrollLocks = 0
let savedOverflow = ''

/** Background scroll stays locked while at least one modal surface is open. */
function lockBackgroundScroll(): void {
  if (typeof document === 'undefined' || !document.body) return
  if (scrollLocks++ === 0) {
    savedOverflow = document.body.style.overflow
    document.body.style.overflow = 'hidden'
  }
}

function unlockBackgroundScroll(): void {
  if (typeof document === 'undefined' || !document.body || scrollLocks === 0) return
  if (--scrollLocks === 0) document.body.style.overflow = savedOverflow
}

function activeElement(): Focusable | null {
  if (typeof document === 'undefined') return null
  const active = document.activeElement as Focusable | null
  return active && typeof active.focus === 'function' ? active : null
}

export interface ModalDialogOptions {
  open: () => boolean
  dialog: Ref<Focusable | null>
  /** Element that receives focus on open; defaults to the first tab stop, then the dialog itself. */
  initialFocus?: () => Focusable | null | undefined
  /** Whether focus returns to the opener on close; evaluated at close time. */
  restoreFocus?: () => boolean
  onClose: () => void
  /** Keys other than Escape and Tab. */
  onKeydown?: (event: KeyboardEvent) => void
}

/**
 * Shared modal behavior: moves focus in on open, keeps Tab and Shift+Tab inside the
 * dialog, closes on Escape, locks background scroll and returns focus to the opener.
 */
export function useModalDialog(options: ModalDialogOptions) {
  let opener: Focusable | null = null
  let locked = false

  /** `unmounting` restores focus even though the owner still reports the dialog as open. */
  function release(unmounting = false): void {
    if (locked) unlockBackgroundScroll()
    locked = false
    const target = opener
    opener = null
    if (!target) return
    const dialogAtClose = options.dialog.value
    void nextTick(() => {
      if ((!unmounting && options.open()) || options.restoreFocus?.() === false || target.isConnected === false) return
      const active = activeElement()
      // Focus moved somewhere on purpose (for example a command focused the editor): keep it there.
      const stillInDialog = Boolean(active && dialogAtClose?.contains(active))
      if (active && active !== document.body && !stillInDialog) return
      target.focus({ preventScroll: true })
    })
  }

  async function engage(): Promise<void> {
    opener = activeElement()
    if (!locked) lockBackgroundScroll()
    locked = true
    await nextTick()
    const dialog = options.dialog.value
    if (!options.open() || !dialog) return
    const target = options.initialFocus?.() ?? focusableElements(dialog)[0] ?? dialog
    target.focus()
  }

  watch(
    options.open,
    open => {
      if (open) void engage()
      else release()
    },
    { immediate: true },
  )
  onBeforeUnmount(() => release(true))

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      options.onClose()
      return
    }
    if (event.key !== 'Tab') {
      options.onKeydown?.(event)
      return
    }
    const dialog = options.dialog.value
    if (!dialog) return
    event.preventDefault()
    const stops = focusableElements(dialog)
    if (!stops.length) {
      dialog.focus()
      return
    }
    const active = activeElement()
    const current = active ? stops.indexOf(active) : -1
    stops[nextTrappedIndex(current, stops.length, event.shiftKey)].focus()
  }

  return { handleKeydown }
}
