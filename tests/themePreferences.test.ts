import { describe, expect, it } from 'vitest'
import {
  assertThemeCatalog,
  contrastRatio,
  resolveTheme,
  THEME_CATALOG,
  THEME_SELECTIONS,
  themeCanvasColor,
} from '../src/services/themePreferences'

describe('theme preferences', () => {
  it('ships a unique localized and contrast-checked token catalog', () => {
    expect(() => assertThemeCatalog(THEME_CATALOG)).not.toThrow()
    expect(THEME_CATALOG.map(theme => theme.id)).toEqual(['dark', 'light', 'nord', 'solarized'])
    expect(THEME_SELECTIONS).toEqual(['system', 'dark', 'light', 'nord', 'solarized'])
  })

  it('resolves system preference without changing explicit presets', () => {
    expect(resolveTheme('system', true).id).toBe('dark')
    expect(resolveTheme('system', false).id).toBe('light')
    expect(resolveTheme('nord', false).id).toBe('nord')
    expect(themeCanvasColor(resolveTheme('dark', false))).toEqual([
      0x14 / 255, 0x12 / 255, 0x10 / 255,
    ])
  })

  it('computes WCAG contrast deterministically and rejects malformed tokens', () => {
    expect(contrastRatio('#000000', '#ffffff')).toBeCloseTo(21, 6)
    expect(() => contrastRatio('red', '#ffffff')).toThrow(TypeError)
    expect(() => assertThemeCatalog([{ ...THEME_CATALOG[0], id: 'bad', tokens: { ...THEME_CATALOG[0].tokens, '--text': '#111216' } }]))
      .toThrow(/contrast/)
  })
})
