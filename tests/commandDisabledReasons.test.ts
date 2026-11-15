import { readFileSync } from 'node:fs'
import { ref } from 'svet'
import { describe, expect, it } from 'vitest'
import { translate, type Language } from '../src/i18n/appStrings'
import { usePaletteCommands } from '../src/composables/useAppCommands'

/** Sources that declare palette and modeler commands through a local `cmd(...)` helper. */
const COMMAND_SOURCES = [
  'src/features/directModeler/useCommandPalette.ts',
  'src/features/MainModelingTools.svt',
  'src/features/MeshModeler/useMeshModeler.ts',
]

/** Each `cmd(...)` call with balanced parentheses, skipping string contents. */
function commandCalls(source: string): string[] {
  const calls: string[] = []
  for (const match of source.matchAll(/\bcmd\(/g)) {
    let depth = 0,
      quote = ''
    for (let i = match.index! + 3; i < source.length; i++) {
      const ch = source[i]
      if (quote) {
        if (ch === '\\') i++
        else if (ch === quote) quote = ''
        continue
      }
      if (ch === "'" || ch === '"' || ch === '`') quote = ch
      else if (ch === '(') depth++
      else if (ch === ')' && --depth === 0) {
        calls.push(source.slice(match.index!, i + 1))
        break
      }
    }
  }
  return calls
}

describe('disabled command reasons', () => {
  it.each(COMMAND_SOURCES)('every conditionally enabled command in %s explains why it is disabled', file => {
    const calls = commandCalls(readFileSync(file, 'utf8')).filter(call => /\benabled\s*:/.test(call))
    expect(calls.length).toBeGreaterThan(0)
    const missing = calls.filter(call => !/\bdisabledReason\s*:/.test(call)).map(call => call.slice(0, 80))
    expect(missing).toEqual([])
  })

  it.each(['ru', 'en'] as const)(
    'gives every disabled app palette command a localized reason (%s)',
    (lang: Language) => {
      const { paletteCommands } = usePaletteCommands({
        t: key => translate(lang, key),
        sceneMeshes: ref([]),
        meshVisibility: ref([]),
        selectedMesh: ref(null),
        isolated: ref(false),
        rendering: ref(true),
        measureActive: ref(false),
        canExport: () => false,
        canPreviousView: ref(false),
        editorOpen: ref(false),
        mainTools: () => null,
        viewport: { projection: ref('perspective') },
        section: { sectionAvailable: ref(false), sectionEnabled: ref(false) },
      } as unknown as Parameters<typeof usePaletteCommands>[0])
      const disabled = paletteCommands.value.filter(command => command.enabled === false)
      expect(disabled.length).toBeGreaterThan(5)
      for (const command of disabled) {
        const reason = command.disabledReason ?? ''
        expect(reason.trim(), command.id).not.toBe('')
        expect(reason, command.id).not.toMatch(/^[a-z][A-Za-z]+$/)
        if (lang === 'ru') expect(reason, command.id).toMatch(/[а-яё]/i)
      }
    },
  )
})
