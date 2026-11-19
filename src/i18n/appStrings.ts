import { ru } from './appStrings/ru'
import { en } from './appStrings/en'

/** Interface strings of the main application shell, per language. */
export type Language = 'ru' | 'en'

export const APP_STRINGS: Record<Language, Record<string, string>> = {
  ru,
  en,
}

export type TranslateParams = Readonly<Record<string, string | number>>

const PLURAL_SUFFIXES = ['zero', 'one', 'two', 'few', 'many', 'other'] as const
const pluralRules = new Map<Language, Intl.PluralRules>()

/** Plural category for `count` under the language's CLDR rules. */
export function pluralCategory(lang: Language, count: number): Intl.LDMLPluralRule {
  let rules = pluralRules.get(lang)
  if (!rules) pluralRules.set(lang, (rules = new Intl.PluralRules(lang)))
  return rules.select(count)
}

/**
 * Looks up `key` for `lang`, falling back to English and then to the key itself.
 * A numeric `count` parameter selects `key_<category>` (CLDR plural category,
 * then `key_other`) when such a form exists. `{name}` placeholders are replaced
 * by matching parameters; unknown placeholders are left untouched.
 */
export function translate(lang: Language, key: string, params?: TranslateParams): string {
  const table = APP_STRINGS[lang]
  let template: string | undefined
  const count = params?.count
  if (typeof count === 'number') {
    template = table[`${key}_${pluralCategory(lang, count)}`] ?? table[`${key}_other`]
  }
  template ??= table[key] ?? APP_STRINGS.en[key] ?? key
  if (!params) return template
  return template.replace(/\{(\w+)\}/g, (match, name: string) =>
    Object.prototype.hasOwnProperty.call(params, name) ? String(params[name]) : match,
  )
}

/** Base keys with plural suffixes stripped, for parity checks between languages. */
export function baseKeys(lang: Language): Set<string> {
  const suffix = new RegExp(`_(${PLURAL_SUFFIXES.join('|')})$`)
  return new Set(Object.keys(APP_STRINGS[lang]).map(key => key.replace(suffix, '')))
}
