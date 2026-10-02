export type ThemeSelection = 'system' | 'dark' | 'light' | 'nord' | 'solarized'
export type ThemeScheme = 'dark' | 'light'

export interface ThemeDefinition {
  readonly id: Exclude<ThemeSelection, 'system'>
  readonly name: Readonly<Record<'ru' | 'en', string>>
  readonly scheme: ThemeScheme
  readonly tokens: Readonly<Record<string, string>>
}

// Neutral graphite with a single cool-blue accent; every value is checked by assertThemeCatalog.
// --hairline is the quiet divider for chrome; --border stays contrast-checked for controls.
const darkTokens = {
  '--bg': '#1d1d1f', '--surface': '#252527', '--surface-raised': '#303033',
  '--border': '#707076', '--hairline': '#333336', '--text': '#ececee', '--text-dim': '#a2a2a8',
  '--accent': '#5c9dff', '--accent-strong': '#2a63c8', '--hover': '#2d2d30',
  '--danger': '#ff8d84', '--warning': '#e6b24e', '--canvas-bg': '#2b2b2e', '--focus': '#8dbaff',
} as const

export const THEME_CATALOG: readonly ThemeDefinition[] = Object.freeze([
  { id: 'dark', name: { ru: 'Тёмная', en: 'Dark' }, scheme: 'dark', tokens: darkTokens },
  {
    id: 'light', name: { ru: 'Светлая', en: 'Light' }, scheme: 'light', tokens: {
      '--bg': '#f6f6f7', '--surface': '#ffffff', '--surface-raised': '#ececee',
      '--border': '#85858b', '--hairline': '#e2e2e5', '--text': '#1b1b1d', '--text-dim': '#5a5a60',
      '--accent': '#1f5fd6', '--accent-strong': '#1b4fb0', '--hover': '#efeff1',
      '--danger': '#b3261e', '--warning': '#8a5b00', '--canvas-bg': '#e3e3e6', '--focus': '#1b4fb0',
    } },
  {
    id: 'nord', name: { ru: 'Nord', en: 'Nord' }, scheme: 'dark', tokens: {
      '--bg': '#242933', '--surface': '#2e3440', '--surface-raised': '#3b4252',
      '--border': '#7d899f', '--hairline': '#3b4252', '--text': '#eceff4', '--text-dim': '#c0c8d6',
      '--accent': '#88c0d0', '--accent-strong': '#466985', '--hover': '#434c5e',
      '--danger': '#ef8c8c', '--warning': '#ebcb8b', '--canvas-bg': '#20242d', '--focus': '#9bd8e8',
    } },
  {
    id: 'solarized', name: { ru: 'Solarized', en: 'Solarized' }, scheme: 'dark', tokens: {
      '--bg': '#002b36', '--surface': '#073642', '--surface-raised': '#104653',
      '--border': '#93a1a1', '--hairline': '#0d4653', '--text': '#eee8d5', '--text-dim': '#b8c2c0',
      '--accent': '#2aa8d8', '--accent-strong': '#17658c', '--hover': '#0d4653',
      '--danger': '#ff7d6f', '--warning': '#e0bd4f', '--canvas-bg': '#001f27', '--focus': '#7fd8f5',
    } },
])

export const THEME_SELECTIONS: readonly ThemeSelection[] = ['system', 'dark', 'light', 'nord', 'solarized']

export function resolveTheme(selection: ThemeSelection, prefersDark: boolean): ThemeDefinition {
  const resolvedId = selection === 'system' ? (prefersDark ? 'dark' : 'light') : selection
  return THEME_CATALOG.find(theme => theme.id === resolvedId) ?? THEME_CATALOG[0]
}

export function contrastRatio(foreground: string, background: string): number {
  const luminance = (hex: string) => {
    if (!/^#[0-9a-f]{6}$/i.test(hex)) throw new TypeError(`Invalid theme color: ${hex}`)
    const channels = [1, 3, 5].map(offset => Number.parseInt(hex.slice(offset, offset + 2), 16) / 255)
      .map(value => value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4)
    return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2]
  }
  const first = luminance(foreground)
  const second = luminance(background)
  return (Math.max(first, second) + 0.05) / (Math.min(first, second) + 0.05)
}

export function themeCanvasColor(theme: ThemeDefinition): [number, number, number] {
  const value = theme.tokens['--canvas-bg']
  if (!/^#[0-9a-f]{6}$/i.test(value)) throw new TypeError(`Invalid canvas color: ${value}`)
  return [1, 3, 5].map(offset => Number.parseInt(value.slice(offset, offset + 2), 16) / 255) as [number, number, number]
}

export function assertThemeCatalog(themes: readonly ThemeDefinition[]): void {
  const ids = new Set<string>()
  for (const theme of themes) {
    if (!/^[a-z][a-z0-9-]{0,31}$/.test(theme.id) || ids.has(theme.id)) throw new TypeError(`Invalid theme id: ${theme.id}`)
    if (!theme.name.ru.trim() || !theme.name.en.trim()) throw new TypeError(`Theme ${theme.id} is not localized`)
    for (const token of Object.keys(darkTokens)) {
      if (!/^#[0-9a-f]{6}$/i.test(theme.tokens[token] ?? '')) throw new TypeError(`Theme ${theme.id} has invalid ${token}`)
    }
    if (contrastRatio(theme.tokens['--text'], theme.tokens['--bg']) < 4.5
      || contrastRatio(theme.tokens['--text'], theme.tokens['--surface']) < 4.5
      || contrastRatio(theme.tokens['--text-dim'], theme.tokens['--surface']) < 4.5
      || contrastRatio(theme.tokens['--border'], theme.tokens['--surface']) < 3
      || contrastRatio(theme.tokens['--focus'], theme.tokens['--surface']) < 3
      || contrastRatio('#ffffff', theme.tokens['--accent-strong']) < 4.5) {
      throw new RangeError(`Theme ${theme.id} does not meet core text contrast`)
    }
    ids.add(theme.id)
  }
}
