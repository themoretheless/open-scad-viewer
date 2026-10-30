/**
 * Forma Unified Style Guide tokens & theme bridge for open-scad-viewer.
 */
export interface FormaThemeTokens {
  readonly bgCanvas: string
  readonly bgSurface: string
  readonly bgRaised: string
  readonly bgOverlay: string
  readonly borderSubtle: string
  readonly borderStrong: string
  readonly textPrimary: string
  readonly textSecondary: string
  readonly textMuted: string
  readonly accentPrimary: string
  readonly accentStrong: string
  readonly accentSoft: string
  readonly statusSuccess: string
  readonly statusWarning: string
  readonly statusDanger: string
  readonly radiusSm: string
  readonly radiusMd: string
  readonly radiusLg: string
  readonly radiusPill: string
}

export const FORMA_DARK_TOKENS: FormaThemeTokens = {
  bgCanvas: '#1e1d1b',
  bgSurface: '#262523',
  bgRaised: '#32302d',
  bgOverlay: 'rgba(38, 37, 35, 0.92)',
  borderSubtle: '#383530',
  borderStrong: '#76726a',
  textPrimary: '#ece9e4',
  textSecondary: '#a6a199',
  textMuted: '#7c776f',
  accentPrimary: '#e29a5a',
  accentStrong: '#a0511d',
  accentSoft: 'rgba(226, 154, 90, 0.16)',
  statusSuccess: '#9fd0a8',
  statusWarning: '#e6b24e',
  statusDanger: '#ff8d84',
  radiusSm: '5px',
  radiusMd: '6px',
  radiusLg: '8px',
  radiusPill: '999px',
}

export const FORMA_LIGHT_TOKENS: FormaThemeTokens = {
  bgCanvas: '#f6f4f1',
  bgSurface: '#ffffff',
  bgRaised: '#ebe8e3',
  bgOverlay: 'rgba(255, 255, 255, 0.92)',
  borderSubtle: '#e3dfd9',
  borderStrong: '#8a857c',
  textPrimary: '#1d1b18',
  textSecondary: '#5d5850',
  textMuted: '#857f75',
  accentPrimary: '#a5521a',
  accentStrong: '#8a4310',
  accentSoft: 'rgba(165, 82, 26, 0.14)',
  statusSuccess: '#2f7d3f',
  statusWarning: '#8a5b00',
  statusDanger: '#b3261e',
  radiusSm: '5px',
  radiusMd: '6px',
  radiusLg: '8px',
  radiusPill: '999px',
}

export function resolveFormaTokens(
  scheme: 'dark' | 'light',
  hostTokens: Record<string, string> = {},
): Record<string, string> {
  const base = scheme === 'light' ? FORMA_LIGHT_TOKENS : FORMA_DARK_TOKENS
  const bg = hostTokens['--bg'] ?? base.bgCanvas
  const surface = hostTokens['--surface'] ?? base.bgSurface
  const raised = hostTokens['--surface-raised'] ?? base.bgRaised
  const hairline = hostTokens['--hairline'] ?? base.borderSubtle
  const border = hostTokens['--border'] ?? base.borderStrong
  const text = hostTokens['--text'] ?? base.textPrimary
  const textDim = hostTokens['--text-dim'] ?? base.textSecondary
  const accent = hostTokens['--accent'] ?? base.accentPrimary
  const accentStrong = hostTokens['--accent-strong'] ?? base.accentStrong
  const warning = hostTokens['--warning'] ?? base.statusWarning
  const danger = hostTokens['--danger'] ?? base.statusDanger

  return {
    '--forma-bg-canvas': bg,
    '--forma-bg-surface': surface,
    '--forma-bg-raised': raised,
    '--forma-bg-overlay': base.bgOverlay,
    '--forma-border-subtle': hairline,
    '--forma-border-strong': border,
    '--forma-text-primary': text,
    '--forma-text-secondary': textDim,
    '--forma-text-muted': base.textMuted,
    '--forma-accent': accent,
    '--forma-accent-strong': accentStrong,
    '--forma-accent-soft': base.accentSoft,
    '--forma-status-ok': base.statusSuccess,
    '--forma-status-warn': warning,
    '--forma-status-err': danger,
    '--forma-radius-sm': base.radiusSm,
    '--forma-radius-md': base.radiusMd,
    '--forma-radius-lg': base.radiusLg,
    '--forma-radius-pill': base.radiusPill,
  }
}
