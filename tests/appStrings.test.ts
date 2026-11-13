import { describe, expect, it } from 'vitest'
import { APP_STRINGS, baseKeys, pluralCategory, translate } from '../src/i18n/appStrings'

const placeholders = (text: string) => [...text.matchAll(/\{(\w+)\}/g)].map(m => m[1]).sort()

describe('app strings', () => {
  it('keeps the same keys in Russian and English', () => {
    expect([...baseKeys('ru')].sort()).toEqual([...baseKeys('en')].sort())
  })

  it('uses the same placeholders in both languages', () => {
    for (const key of Object.keys(APP_STRINGS.en)) {
      const ru = APP_STRINGS.ru[key]
      if (ru !== undefined) expect(placeholders(ru), key).toEqual(placeholders(APP_STRINGS.en[key]))
    }
  })

  it('has no empty translations', () => {
    for (const lang of ['ru', 'en'] as const)
      for (const [key, value] of Object.entries(APP_STRINGS[lang])) expect(value.trim(), `${lang}.${key}`).not.toBe('')
  })

  it('falls back to English and then to the key', () => {
    expect(translate('ru', 'render')).toBe('Собрать')
    expect(translate('en', 'render')).toBe('Render')
    expect(translate('ru', '__missing__')).toBe('__missing__')
  })

  it('interpolates named parameters and keeps unknown placeholders', () => {
    const table = APP_STRINGS.en as Record<string, string>
    table.__probe = '{a} and {b}'
    try {
      expect(translate('en', '__probe', { a: 1 })).toBe('1 and {b}')
    } finally {
      delete table.__probe
    }
  })

  it('selects CLDR plural forms by count', () => {
    expect([1, 2, 5, 21].map(n => pluralCategory('ru', n))).toEqual(['one', 'few', 'many', 'one'])
    const ru = APP_STRINGS.ru as Record<string, string>
    Object.assign(ru, {
      __items_one: '{count} деталь',
      __items_few: '{count} детали',
      __items_many: '{count} деталей',
      __items_other: '{count} детали',
    })
    try {
      expect([1, 3, 11, 1.5].map(count => translate('ru', '__items', { count }))).toEqual([
        '1 деталь',
        '3 детали',
        '11 деталей',
        '1.5 детали',
      ])
    } finally {
      for (const key of ['__items_one', '__items_few', '__items_many', '__items_other']) delete ru[key]
    }
  })
})
