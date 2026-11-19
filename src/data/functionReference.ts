import { text, type ReferenceCategory, type ReferenceEntry, type ReferenceLanguage, type ReferenceText } from './functionReference/entry'
import { language } from './functionReference/language'
import { shapes } from './functionReference/shapes'
import { operations } from './functionReference/operations'
import { expressions } from './functionReference/expressions'
export type { ReferenceLanguage, ReferenceLocale, ReferenceCategory, ReferenceText, ReferenceVariant, ReferenceEntry } from './functionReference/entry'

export const REFERENCE_CATEGORIES: readonly { readonly id: ReferenceCategory; readonly label: ReferenceText }[] = [
  { id: 'transforms', label: text('Преобразования', 'Transforms') },
  { id: 'primitives', label: text('Объёмные фигуры', 'Solid primitives') },
  { id: 'booleans', label: text('Объединение и вычитание', 'Boolean operations') },
  { id: 'profiles', label: text('Контуры и выдавливание', 'Profiles and extrusion') },
  { id: 'patterns', label: text('Повторение и условия', 'Patterns and conditions') },
  { id: 'checks', label: text('Проверки', 'Checks') },
  { id: 'math', label: text('Математика', 'Math') },
  { id: 'lists', label: text('Списки', 'Lists') },
]

/** Browser-supported operations. Text/1 and OpenSCAD intentionally have distinct signatures. */
export const FUNCTION_REFERENCE: readonly ReferenceEntry[] = [
  ...language, ...shapes, ...operations, ...expressions,
]

const normalizeSearch = (value: string) => value.normalize('NFKC').toLocaleLowerCase().replaceAll('ё', 'е')

/** Searches both locales, but only the selected language's callable signature and parameters. */
export function searchFunctionReference(
  query: string,
  language: ReferenceLanguage,
  category: ReferenceCategory | 'all' = 'all',
): ReferenceEntry[] {
  const terms = normalizeSearch(query).trim().split(/\s+/).filter(Boolean)
  return FUNCTION_REFERENCE.filter(entry => {
    const variant = entry.variants[language]
    if (!variant || (category !== 'all' && entry.category !== category)) return false
    const categoryLabel = REFERENCE_CATEGORIES.find(item => item.id === entry.category)!.label
    const haystack = normalizeSearch(
      [
        entry.name,
        entry.id,
        entry.summary.ru,
        entry.summary.en,
        ...(entry.keywords ?? []),
        categoryLabel.ru,
        categoryLabel.en,
        variant.signature,
        ...variant.parameters.flatMap(item => [item.name, item.description.ru, item.description.en]),
        variant.notes?.ru ?? '',
        variant.notes?.en ?? '',
      ].join(' '),
    )
    return terms.every(term => haystack.includes(term))
  })
}
