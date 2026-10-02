import { describe, expect, it } from 'vitest'
import { customizeTokens, normalizeHex } from '../src/services/themeCustomization'
import { contrastRatio, THEME_CATALOG } from '../src/services/themePreferences'

describe('theme customization', () => {
  it('accepts only 6-digit hex colors', () => {
    expect(normalizeHex('#A0B1C2')).toBe('#a0b1c2')
    expect(normalizeHex('red')).toBeNull()
    expect(normalizeHex(null)).toBeNull()
  })

  it('leaves the theme untouched without a choice', () => {
    expect(customizeTokens(THEME_CATALOG[0].tokens, { accent: null, base: null })).toEqual({})
  })

  it.each(['#101010', '#2a1f14', '#ffffff', '#f2e8d5', '#808080', '#757575', '#707070', '#6e6e6e', '#7a7a7a', '#777777'])('keeps core contrast for base %s with an awkward accent', base => {
    const tokens = customizeTokens(THEME_CATALOG[0].tokens, { accent: '#3a3a3a', base })
    expect(contrastRatio(tokens['--text'], tokens['--bg'])).toBeGreaterThanOrEqual(4.5)
    expect(contrastRatio(tokens['--text-dim'], tokens['--surface'])).toBeGreaterThanOrEqual(4.5)
    expect(contrastRatio(tokens['--border'], tokens['--surface'])).toBeGreaterThanOrEqual(3)
    expect(contrastRatio(tokens['--accent'], tokens['--surface'])).toBeGreaterThanOrEqual(3)
    expect(contrastRatio('#ffffff', tokens['--accent-strong'])).toBeGreaterThanOrEqual(4.5)
  })
})

describe('theme customization sweep', () => {
  it('keeps core contrast for every gray', () => {
    for (let level = 0; level <= 255; level += 5) {
      const hex = level.toString(16).padStart(2, '0')
      const base = `#${hex}${hex}${hex}`
      const tokens = customizeTokens(THEME_CATALOG[0].tokens, { accent: '#e29a5a', base })
      expect(contrastRatio(tokens['--text'], tokens['--bg']), base).toBeGreaterThanOrEqual(4.5)
      expect(contrastRatio(tokens['--text'], tokens['--surface']), base).toBeGreaterThanOrEqual(4.5)
      expect(contrastRatio(tokens['--text-dim'], tokens['--surface']), base).toBeGreaterThanOrEqual(4.5)
      expect(contrastRatio(tokens['--border'], tokens['--surface']), base).toBeGreaterThanOrEqual(3)
    }
  })
})
