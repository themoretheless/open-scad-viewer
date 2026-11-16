import { readFileSync } from 'node:fs'
import { resolve } from 'node:path'
import { describe, expect, it } from 'vitest'
import {
  nonColorCueKey,
  resolveSelectionState,
  SELECTION_STATE_ORDER,
  SELECTION_STATES,
  selectionStateRgb,
  type SelectionState,
  type ThemeName,
} from '../src/ui/selectionStates'
import { THEME_PRESETS } from '../src/render/rendererContracts'

const css = readFileSync(resolve('src/features/styles/direct-modeler.css'), 'utf8')
const states = Object.keys(SELECTION_STATES) as SelectionState[]
const themes: ThemeName[] = ['light', 'dark']

/** Variables declared for a theme block in the stylesheet. */
function themeVars(theme: ThemeName): Record<string, string> {
  const selector = theme === 'light' ? '[data-theme="light"] .direct-workspace{' : '.direct-workspace{--sel-'
  const start = css.indexOf(selector)
  expect(start).toBeGreaterThanOrEqual(0)
  const block = css.slice(start, css.indexOf('}', start))
  return Object.fromEntries([...block.matchAll(/--sel-(\w+):(#[0-9a-f]+)/g)].map(m => [m[1], m[2]]))
}

describe('selection state table', () => {
  it('covers the six states in priority order', () => {
    expect([...SELECTION_STATE_ORDER].sort()).toEqual(['hover', 'invalid', 'locked', 'operand', 'preview', 'selected'])
    expect(resolveSelectionState({})).toBeUndefined()
    expect(resolveSelectionState({ hover: true, selected: true })).toBe('selected')
    expect(resolveSelectionState({ selected: true, operand: true })).toBe('operand')
    expect(resolveSelectionState({ operand: true, locked: true, invalid: true })).toBe('invalid')
  })

  it.each(themes)('gives every state a distinct non-color cue in the %s theme', theme => {
    const vars = themeVars(theme)
    const keys = states.map(state => nonColorCueKey(state))
    expect(new Set(keys).size).toBe(states.length)
    expect(new Set(states.map(state => SELECTION_STATES[state].glyph)).size).toBe(states.length)
    expect(new Set(states.map(state => SELECTION_STATES[state].dash)).size).toBe(states.length)
    for (const state of states) {
      const cue = SELECTION_STATES[state]
      expect(cue.glyph.trim()).not.toBe('')
      // The stylesheet colors match the table for this theme...
      expect(vars[state]).toBe(cue.color[theme])
      // ...and the outline rule carries the non-color part, independent of theme.
      expect(css).toContain(
        `[data-sel-state=${state}]{outline:${cue.outlineWidth}px ${cue.outlineStyle} var(--sel-${state})`,
      )
    }
  })

  it('is applied to tree rows, the legend and the inspector', () => {
    const source = readFileSync(resolve('src/features/DirectModeler.svt'), 'utf8')
    expect(source).toContain(':data-sel-state="sceneRowState(row.item.id)"')
    expect(source).toContain('v-for="state in SELECTION_STATE_ORDER"')
    expect(source).toMatch(/class="dock-props" :data-sel-state=/)
    expect(css).toMatch(/input\[aria-invalid=true\][^{]*\{[^}]*border-style:dashed/)
  })
})

describe('GPU selection colours', () => {
  const toHex = (rgb: readonly number[]) =>
    '#' +
    rgb
      .map(c =>
        Math.round(c * 255)
          .toString(16)
          .padStart(2, '0'),
      )
      .join('')

  it('converts table colours to 0..1 RGB without losing the 8-bit value', () => {
    for (const state of states)
      for (const theme of themes)
        expect(toHex(selectionStateRgb(state, theme))).toBe(SELECTION_STATES[state].color[theme])
  })

  it('takes selectionColor and hoverColor of every render theme from the table', () => {
    const themeOf: Record<string, ThemeName> = { default: 'dark', 'dark-contrast': 'dark', light: 'light' }
    expect(THEME_PRESETS.map(t => t.id).sort()).toEqual(Object.keys(themeOf).sort())
    for (const preset of THEME_PRESETS) {
      expect(toHex(preset.selectionColor), preset.id).toBe(SELECTION_STATES.selected.color[themeOf[preset.id]])
      expect(toHex(preset.hoverColor), preset.id).toBe(SELECTION_STATES.hover.color[themeOf[preset.id]])
    }
  })
})
