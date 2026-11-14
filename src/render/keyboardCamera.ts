/** One keyboard camera step, expressed in the same pixel units as pointer gestures. */
export type KeyboardCameraStep =
  { kind: 'orbit'; dx: number; dy: number } | { kind: 'pan'; dx: number; dy: number } | { kind: 'zoom'; delta: number }

/** Pointer-equivalent pixels per arrow press; orbit uses the drag sensitivity of the renderer. */
export const KEYBOARD_ORBIT_PIXELS = 24
export const KEYBOARD_PAN_PIXELS = 40
/** Wheel-equivalent pixels per +/- press. */
export const KEYBOARD_ZOOM_PIXELS = 120

export interface KeyboardCameraEventLike {
  key: string
  code?: string
  shiftKey?: boolean
  ctrlKey?: boolean
  metaKey?: boolean
  altKey?: boolean
}

const ARROWS: Record<string, [number, number]> = {
  ArrowLeft: [-1, 0],
  ArrowRight: [1, 0],
  ArrowUp: [0, -1],
  ArrowDown: [0, 1],
}

/**
 * Keyboard orbit, pan and zoom of a focused viewport: arrows orbit, Shift+arrows
 * pan, +/- (main row or numpad) zoom. Ctrl, Meta and Alt combinations are left to
 * the command registry (Ctrl/⌘+= is a registered command).
 */
export function resolveKeyboardCameraStep(event: KeyboardCameraEventLike): KeyboardCameraStep | null {
  if (event.ctrlKey || event.metaKey || event.altKey) return null
  const arrow = ARROWS[event.key]
  if (arrow) {
    const [x, y] = arrow
    return event.shiftKey
      ? { kind: 'pan', dx: x * KEYBOARD_PAN_PIXELS, dy: y * KEYBOARD_PAN_PIXELS }
      : { kind: 'orbit', dx: x * KEYBOARD_ORBIT_PIXELS, dy: y * KEYBOARD_ORBIT_PIXELS }
  }
  if (event.key === '+' || event.key === '=' || event.code === 'NumpadAdd')
    return { kind: 'zoom', delta: -KEYBOARD_ZOOM_PIXELS }
  if (event.key === '-' || event.key === '_' || event.code === 'NumpadSubtract')
    return { kind: 'zoom', delta: KEYBOARD_ZOOM_PIXELS }
  return null
}
