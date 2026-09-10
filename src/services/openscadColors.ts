/**
 * Color state and tables for the legacy OpenSCAD evaluator: the auto-assign
 * palette, CSS color names and channel clamping. Pure data + the palette
 * counter; evaluation errors stay with the parser.
 */

export type RGBA = [number, number, number, number]

const PALETTE: RGBA[] = [
  [0.26, 0.52, 0.96, 1], [0.96, 0.52, 0.26, 1],
  [0.26, 0.86, 0.56, 1], [0.86, 0.26, 0.66, 1],
  [0.96, 0.86, 0.26, 1], [0.46, 0.76, 0.86, 1],
  [0.76, 0.56, 0.96, 1], [0.56, 0.86, 0.36, 1],
]

let paletteIndex = 0

export function nextColor(): RGBA { return [...PALETTE[paletteIndex++ % PALETTE.length]] as RGBA }

export function resetPalette(): void { paletteIndex = 0 }

export const CSS_COLORS: Record<string, RGBA> = {
  red: [1, 0, 0, 1], green: [0, 0.5, 0, 1], blue: [0, 0, 1, 1],
  yellow: [1, 1, 0, 1], cyan: [0, 1, 1, 1], magenta: [1, 0, 1, 1],
  white: [1, 1, 1, 1], black: [0, 0, 0, 1], orange: [1, 0.65, 0, 1],
  gray: [0.5, 0.5, 0.5, 1], grey: [0.5, 0.5, 0.5, 1],
  pink: [1, 0.75, 0.8, 1], purple: [0.5, 0, 0.5, 1], brown: [0.65, 0.16, 0.16, 1],
  lime: [0, 1, 0, 1], navy: [0, 0, 0.5, 1], teal: [0, 0.5, 0.5, 1],
}

export function clamp01(value: number): number { return Math.max(0, Math.min(1, value)) }
