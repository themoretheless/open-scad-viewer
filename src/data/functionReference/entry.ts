export type ReferenceLanguage = 'openscad' | 'rush'
export type ReferenceLocale = 'ru' | 'en'
export type ReferenceCategory =
  'transforms' | 'primitives' | 'booleans' | 'profiles' | 'patterns' | 'checks' | 'math' | 'lists'
export type ReferenceText = Readonly<Record<ReferenceLocale, string>>

export interface ReferenceVariant {
  readonly signature: string
  readonly parameters: readonly { readonly name: string; readonly description: ReferenceText }[]
  /** A complete scene, including the routing header for Rush. */
  readonly example: string
  readonly notes?: ReferenceText
}

export interface ReferenceEntry {
  readonly id: string
  readonly name: string
  readonly category: ReferenceCategory
  readonly summary: ReferenceText
  readonly keywords?: readonly string[]
  readonly variants: Readonly<Partial<Record<ReferenceLanguage, ReferenceVariant>>>
}

export const text = (ru: string, en: string): ReferenceText => ({ ru, en })
export const parameter = (name: string, ru: string, en: string) => ({ name, description: text(ru, en) })
export const scad = (
  signature: string,
  parameters: ReferenceVariant['parameters'],
  example: string,
  notes?: ReferenceText,
): ReferenceVariant => ({ signature, parameters, example, ...(notes ? { notes } : {}) })
export const graph = (
  signature: string,
  parameters: ReferenceVariant['parameters'],
  example: string,
  notes?: ReferenceText,
): ReferenceVariant => scad(signature, parameters, `// @rush/1\n${example}`, notes)
