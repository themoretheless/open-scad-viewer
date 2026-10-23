import { computed, ref } from 'vue'
import type { Language } from '../i18n/appStrings'
import { storageGet, storageGetEnum, storageSet } from '../services/safeStorage'
import { customizeTokens, normalizeHex } from '../services/themeCustomization'
import { resolveTheme, themeCanvasColor, THEME_SELECTIONS, type ThemeSelection } from '../services/themePreferences'

export interface AppearanceOptions {
  /** Called after every application of the preferences with the effective canvas colour. */
  onApplied(canvasColor: ReturnType<typeof themeCanvasColor>): void
}

/**
 * Interface language, colour theme and custom accent/base colours of the application shell:
 * persisted preferences, the document-level tokens they produce and the system theme listener.
 */
export function useAppearance(options: AppearanceOptions) {
  const lang = ref<Language>(storageGetEnum<Language>('scad-lang', ['ru', 'en'], 'ru'))
  const legacyThemeRaw = storageGet('scad-theme')
  const legacyTheme: ThemeSelection = legacyThemeRaw === 'dark' || legacyThemeRaw === 'light' ? legacyThemeRaw : 'system'
  const preVersionedTheme = storageGetEnum<ThemeSelection>('scad-theme-selection', THEME_SELECTIONS, legacyTheme)
  const themeSelection = ref(storageGetEnum<ThemeSelection>('scad-theme-v1', THEME_SELECTIONS, preVersionedTheme))
  const themeMediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
  const systemPrefersDark = ref(themeMediaQuery.matches)
  const customAccent = ref<string | null>(normalizeHex(storageGet('scad-accent-v1')))
  const customBase = ref<string | null>(normalizeHex(storageGet('scad-base-v1')))
  const effectiveTokens = computed(() => resolveTheme(themeSelection.value, systemPrefersDark.value).tokens)
  const isDark = computed(() => resolveTheme(themeSelection.value, systemPrefersDark.value).scheme === 'dark')

  function effectiveCanvasColor() {
    const theme = resolveTheme(themeSelection.value, systemPrefersDark.value)
    const tokens = { ...theme.tokens, ...customizeTokens(theme.tokens, { accent: customAccent.value, base: customBase.value }) }
    return themeCanvasColor({ ...theme, tokens })
  }

  function applyPreferences() {
    const theme = resolveTheme(themeSelection.value, systemPrefersDark.value)
    document.documentElement.dataset.theme = theme.scheme
    document.documentElement.dataset.themePreset = themeSelection.value
    document.documentElement.lang = lang.value
    document.documentElement.style.colorScheme = theme.scheme
    const tokens = { ...theme.tokens, ...customizeTokens(theme.tokens, { accent: customAccent.value, base: customBase.value }) }
    for (const [token, value] of Object.entries(tokens)) {
      document.documentElement.style.setProperty(token, value)
    }
    if (customAccent.value) storageSet('scad-accent-v1', customAccent.value)
    else storageSet('scad-accent-v1', '')
    if (customBase.value) storageSet('scad-base-v1', customBase.value)
    else storageSet('scad-base-v1', '')
    storageSet('scad-theme-v1', themeSelection.value)
    storageSet('scad-theme', theme.scheme)
    options.onApplied(effectiveCanvasColor())
  }

  function setCustomColor(kind: 'accent' | 'base', event: Event) {
    const value = normalizeHex((event.target as HTMLInputElement).value)
    if (kind === 'accent') customAccent.value = value
    else customBase.value = value
    applyPreferences()
  }

  function resetCustomColors() {
    customAccent.value = null
    customBase.value = null
    applyPreferences()
  }

  function toggleLang() {
    lang.value = lang.value === 'ru' ? 'en' : 'ru'
    storageSet('scad-lang', lang.value)
    document.documentElement.lang = lang.value
  }

  function toggleTheme() {
    themeSelection.value = isDark.value ? 'light' : 'dark'
    applyPreferences()
  }

  function handleSystemThemeChange(event: MediaQueryListEvent) {
    systemPrefersDark.value = event.matches
    if (themeSelection.value === 'system') applyPreferences()
  }

  /** Starts following the system colour scheme and applies the stored preferences. */
  function attachSystemTheme() {
    themeMediaQuery.addEventListener('change', handleSystemThemeChange)
    applyPreferences()
  }

  function detachSystemTheme() {
    themeMediaQuery.removeEventListener('change', handleSystemThemeChange)
  }

  return {
    lang, themeSelection, customAccent, customBase, effectiveTokens, isDark,
    setCustomColor, effectiveCanvasColor, resetCustomColors,
    toggleLang, toggleTheme, applyPreferences, attachSystemTheme, detachSystemTheme,
  }
}
