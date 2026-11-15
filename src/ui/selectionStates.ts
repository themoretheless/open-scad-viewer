/**
 * Selection state table for the direct modeler (roadmap C5).
 *
 * Every state carries a cue that does not depend on color: an outline style and
 * width, a glyph and a text label. Colors only reinforce the cue and are defined
 * per theme. The CSS in `src/features/styles/direct-modeler.css` mirrors this
 * table through `[data-sel-state=...]` selectors; `tests/selectionStates.test.ts`
 * keeps both in sync.
 */
export type SelectionState = 'hover' | 'selected' | 'operand' | 'preview' | 'invalid' | 'locked'
export type ThemeName = 'light' | 'dark'

export interface SelectionStateCue {
  /** CSS outline-style; the primary non-color cue. */
  outlineStyle: 'solid' | 'dashed' | 'dotted' | 'double'
  /** Outline width in CSS pixels. */
  outlineWidth: number
  /** Stroke dash pattern for SVG overlays in the viewport (`''` means a solid line). */
  dash: string
  /** Glyph shown next to the item; never empty. */
  glyph: string
  label: { ru: string; en: string }
  color: Record<ThemeName, string>
}

export const SELECTION_STATE_ORDER: readonly SelectionState[] = ['invalid', 'locked', 'operand', 'selected', 'preview', 'hover']

export const SELECTION_STATES: Readonly<Record<SelectionState, SelectionStateCue>> = {
  hover: { outlineStyle: 'dotted', outlineWidth: 1, dash: '2 3', glyph: '○', label: { ru: 'Наведение', en: 'Hover' }, color: { light: '#2f6fd1', dark: '#89baff' } },
  selected: { outlineStyle: 'solid', outlineWidth: 2, dash: '', glyph: '●', label: { ru: 'Выбрано', en: 'Selected' }, color: { light: '#6a3fd0', dark: '#b496ff' } },
  operand: { outlineStyle: 'double', outlineWidth: 3, dash: '8 2 2 2', glyph: '◆', label: { ru: 'Операнд', en: 'Operand' }, color: { light: '#a65c00', dark: '#ffb35c' } },
  preview: { outlineStyle: 'dashed', outlineWidth: 2, dash: '6 3', glyph: '◌', label: { ru: 'Предпросмотр', en: 'Preview' }, color: { light: '#08795a', dark: '#77eac5' } },
  invalid: { outlineStyle: 'dashed', outlineWidth: 3, dash: '1 3', glyph: '!', label: { ru: 'Ошибка', en: 'Invalid' }, color: { light: '#c02626', dark: '#ff7a7a' } },
  locked: { outlineStyle: 'dotted', outlineWidth: 2, dash: '1 5', glyph: '⊘', label: { ru: 'Заблокировано', en: 'Locked' }, color: { light: '#5c6470', dark: '#a7afba' } },
}

export interface SelectionFlags {
  hover?: boolean
  selected?: boolean
  operand?: boolean
  preview?: boolean
  invalid?: boolean
  locked?: boolean
}

/** Highest-priority state for an item, or `undefined` for a plain item. */
export function resolveSelectionState(flags: SelectionFlags): SelectionState | undefined {
  return SELECTION_STATE_ORDER.find(state => flags[state])
}

/** The non-color part of a cue as one comparable key. */
export function nonColorCueKey(state: SelectionState): string {
  const cue = SELECTION_STATES[state]
  return `${cue.outlineStyle}/${cue.outlineWidth}/${cue.glyph}`
}

export function selectionStateLabel(state: SelectionState, locale: string): string {
  return locale === 'ru' ? SELECTION_STATES[state].label.ru : SELECTION_STATES[state].label.en
}

/**
 * A state's theme color as 0..1 RGB (each 8-bit channel / 255) for the GPU render theme
 * (`selectionColor`/`hoverColor` in `src/render/rendererContracts.ts`), so the
 * viewport highlight uses the same table as the DOM and SVG cues.
 */
export function selectionStateRgb(state: SelectionState, theme: ThemeName): [number, number, number] {
  const hex = SELECTION_STATES[state].color[theme]
  return [1, 3, 5].map(i => Math.round((parseInt(hex.slice(i, i + 2), 16) / 255) * 1000) / 1000) as [number, number, number]
}
