import { contrastRatio } from './themePreferences'

export interface ThemeCustomization {
  readonly accent: string | null
  readonly base: string | null
}

const HEX = /^#[0-9a-f]{6}$/i

export function normalizeHex(value: unknown): string | null {
  return typeof value === 'string' && HEX.test(value) ? value.toLowerCase() : null
}

function channels(hex: string): number[] {
  return [1, 3, 5].map(offset => Number.parseInt(hex.slice(offset, offset + 2), 16))
}

function mix(from: string, to: string, amount: number): string {
  const a = channels(from)
  const b = channels(to)
  return `#${a.map((value, index) => Math.round(value + (b[index] - value) * amount).toString(16).padStart(2, '0')).join('')}`
}

// Moves `color` toward `target` until it reaches `ratio` against `against`; stops at the target.
function reach(color: string, target: string, against: string, ratio: number): string {
  let result = color
  for (let step = 1; step <= 20 && contrastRatio(result, against) < ratio; step++) result = mix(color, target, step / 20)
  return result
}

/** Derives a complete, contrast-checked token set from one base tone and one accent. */
export function customizeTokens(tokens: Readonly<Record<string, string>>, custom: ThemeCustomization): Record<string, string> {
  const result: Record<string, string> = {}
  let surface = tokens['--surface']
  let text = tokens['--text']
  if (custom.base) {
    const base = custom.base
    const dark = contrastRatio('#ffffff', base) > contrastRatio('#000000', base)
    const toward = dark ? '#ffffff' : '#000000'
    surface = dark ? mix(base, toward, 0.04) : mix(base, '#ffffff', 0.7)
    text = reach(dark ? '#ece9e4' : '#1d1b18', dark ? '#ffffff' : '#000000', base, 4.5)
    const edge = dark ? 0.1 : 0.05
    Object.assign(result, {
      '--bg': base, '--surface': surface, '--surface-raised': mix(base, toward, edge),
      '--hover': mix(base, toward, edge * 0.7), '--hairline': mix(base, toward, edge + 0.04),
      '--canvas-bg': mix(base, toward, dark ? 0.08 : 0.07), '--text': text,
      '--text-dim': reach(mix(text, base, 0.35), text, surface, 4.5),
      '--border': reach(mix(base, toward, 0.4), toward, surface, 3),
    })
  }
  if (custom.accent) {
    const accent = reach(custom.accent, text, surface, 3)
    const strong = reach(mix(accent, '#000000', 0.35), '#000000', '#ffffff', 4.5)
    Object.assign(result, { '--accent': accent, '--accent-strong': strong, '--focus': accent })
  }
  return result
}
