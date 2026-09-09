import { describe, expect, it } from 'vitest'
import {
  FUNCTION_REFERENCE,
  REFERENCE_CATEGORIES,
  searchFunctionReference,
  type ReferenceLanguage,
} from '../src/data/functionReference'
import { compileModelGraphText } from '../src/services/modelGraphText'
import { parseOpenSCAD } from '../src/services/openscadParser'
import { lowerOpenSCADToSemanticProgram } from '../src/services/semanticProgramLowerer'

const languages: ReferenceLanguage[] = ['openscad', 'modelgraph']
const examples = FUNCTION_REFERENCE.flatMap(entry => languages.flatMap(language => {
  const variant = entry.variants[language]
  return variant ? [{ id: entry.id, language, example: variant.example }] : []
}))

describe('function reference examples', () => {
  it.each(examples)('$language: $id compiles and produces browser geometry', async ({ language, example }) => {
    if (language === 'modelgraph') {
      expect(compileModelGraphText(example).document.nodes.length).toBeGreaterThan(0)
    } else {
      const lowered = lowerOpenSCADToSemanticProgram(example)
      expect(lowered.program.core.nodes.length).toBeGreaterThan(0)
      expect(lowered.warnings).toEqual([])
    }
    // The same source routing entry point used by the viewer also checks construction,
    // catching examples that parse but produce an empty or invalid solid.
    const scene = await parseOpenSCAD(example)
    expect(scene.meshes.length).toBeGreaterThan(0)
    expect(scene.warnings).toEqual([])
  })
})

describe('function reference discovery', () => {
  const ids = (query: string, language: ReferenceLanguage = 'openscad') => searchFunctionReference(query, language).map(entry => entry.id)

  it('finds functions by names, both description languages and normalized Cyrillic', () => {
    expect(ids('TRANSLATE')).toContain('translate')
    expect(ids('  перемещает   осям ')).toContain('translate')
    expect(ids('moves geometry')).toContain('translate')
    expect(ids('создает сферу')).toContain('sphere')
    expect(ids('extrude', 'modelgraph')).toContain('extrude')
    expect(ids('subtract', 'modelgraph')).toContain('difference')
  })

  it.each([
    ['поворот', 'rotate'], ['rotation', 'rotate'],
    ['перемещение', 'translate'], ['сдвиг', 'translate'], ['translation', 'translate'],
    ['масштаб', 'scale'], ['scaling', 'scale'],
    ['отражение', 'mirror'], ['reflection', 'mirror'],
  ])('finds the transform for the natural-language query %s', (query, expectedId) => {
    for (const language of languages) expect(ids(query, language)).toContain(expectedId)
  })

  it('filters by the selected language and category together', () => {
    expect(ids('sin', 'modelgraph')).not.toContain('sin')
    expect(ids('select', 'openscad')).not.toContain('select')
    const transforms = searchFunctionReference('', 'modelgraph', 'transforms')
    expect(transforms.map(entry => entry.id)).toEqual(['translate', 'rotate', 'scale', 'mirror'])
    expect(searchFunctionReference('sphere', 'openscad', 'transforms')).toEqual([])
    expect(ids('this-function-does-not-exist')).toEqual([])
  })

  it('has unique ids and complete bilingual content for every advertised variant', () => {
    expect(new Set(FUNCTION_REFERENCE.map(entry => entry.id)).size).toBe(FUNCTION_REFERENCE.length)
    const categoryIds = new Set(REFERENCE_CATEGORIES.map(category => category.id))
    for (const entry of FUNCTION_REFERENCE) {
      expect(categoryIds.has(entry.category)).toBe(true)
      expect(entry.summary.ru.trim()).not.toBe('')
      expect(entry.summary.en.trim()).not.toBe('')
      for (const language of languages) {
        const variant = entry.variants[language]
        if (!variant) continue
        expect(variant.signature.trim()).not.toBe('')
        expect(variant.example.startsWith('// @modelgraph-text/1')).toBe(language === 'modelgraph')
        for (const parameter of variant.parameters) {
          expect(parameter.description.ru.trim()).not.toBe('')
          expect(parameter.description.en.trim()).not.toBe('')
        }
      }
    }
  })
})
