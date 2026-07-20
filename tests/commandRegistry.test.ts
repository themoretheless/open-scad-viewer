import { describe, expect, it } from 'vitest'
import {
  COMMAND_REGISTRY,
  buildPaletteDescriptors,
  isEditableTarget,
  matchesKeyboardBinding,
  normalizeKeyboardEvent,
  resolveKeyboardCommand,
  type CommandId,
} from '../src/services/commandRegistry'

function key(
  value: string,
  options: Partial<Parameters<typeof normalizeKeyboardEvent>[0]> = {},
) {
  return { key: value, code: '', ...options }
}

describe('command registry', () => {
  it('is the unique typed inventory for every current palette and keyboard command', () => {
    const ids = COMMAND_REGISTRY.map(command => command.id)
    expect(new Set(ids).size).toBe(ids.length)
    expect(ids).toEqual(expect.arrayContaining([
      'render', 'open', 'save', 'export-stl', 'export-obj', 'share',
      'focus', 'fit', 'isolate', 'deselect', 'previous-view', 'projection',
      'grid', 'shaded', 'edges', 'xray', 'select-point', 'select-face',
      'select-object', 'measure', 'section', 'sidebar', 'iso', 'front',
      'right', 'top', 'back', 'left', 'bottom', 'theme',
      'command-palette', 'cancel-measure', 'flip-section', 'hide-selected',
      'cycle-selection-mode',
    ] satisfies CommandId[]))
  })

  it('builds localized palette descriptors with dynamic state and MRU ranks', () => {
    const translated: Record<string, string> = {
      projection: 'Projection', orthographic: 'Orthographic', perspective: 'Perspective',
      isolate: 'Isolate', unisolate: 'Show all', exportStl: 'Export STL',
    }
    const russian: Record<string, string> = {
      orthographic: 'Ортография', perspective: 'Перспектива',
      isolate: 'Изолировать', unisolate: 'Показать всё', exportStl: 'Экспорт STL',
    }
    const resolveEnglish = (labelKey: string) => translated[labelKey] ?? `en:${labelKey}`
    const resolveRussian = (labelKey: string) => russian[labelKey] ?? `ru:${labelKey}`

    const commands = buildPaletteDescriptors({
      resolveLabel: resolveEnglish,
      aliasResolvers: [resolveEnglish, resolveRussian],
      state: {
        projection: { label: 'Switch to orthographic' },
        isolate: { label: 'Show all' },
        'export-stl': { enabled: false, disabledReason: 'Full build required' },
      },
      mru: ['grid', 'projection'],
    })

    const projection = commands.find(command => command.id === 'projection')
    expect(projection).toMatchObject({
      label: 'Switch to orthographic',
      shortcut: '5',
      mruRank: 1,
      keywords: ['camera', 'projection'],
    })
    expect(projection?.aliases).toEqual([
      'Perspective', 'Перспектива', 'Orthographic', 'Ортография',
    ])
    expect(commands.find(command => command.id === 'grid')?.mruRank).toBe(0)
    expect(commands.find(command => command.id === 'export-stl')).toMatchObject({
      enabled: false,
      disabledReason: 'Full build required',
    })
    expect(commands.some(command => command.id === ('command-palette' as string))).toBe(false)
  })
})

describe('keyboard command routing', () => {
  it('normalizes letter casing, legacy Escape, and the platform primary modifier', () => {
    expect(normalizeKeyboardEvent(key('S', { ctrlKey: true }))).toMatchObject({
      key: 's', primary: true, ctrl: true, meta: false,
    })
    expect(normalizeKeyboardEvent(key('Esc', { metaKey: true })).key).toBe('escape')
    expect(resolveKeyboardCommand(key('K', { metaKey: true }), 'global')).toBe('command-palette')
    expect(resolveKeyboardCommand(key('k', { ctrlKey: true }), 'global')).toBe('command-palette')
  })

  it('matches exact modifiers and layout-independent codes', () => {
    const plainGrid = normalizeKeyboardEvent(key('g', { code: 'KeyG' }))
    const modifiedGrid = normalizeKeyboardEvent(key('g', { code: 'KeyG', ctrlKey: true }))
    expect(matchesKeyboardBinding(plainGrid, { key: 'G', scope: 'viewport' })).toBe(true)
    expect(matchesKeyboardBinding(modifiedGrid, { key: 'G', scope: 'viewport' })).toBe(false)

    expect(resolveKeyboardCommand(key('!', { code: 'Digit1', shiftKey: true }), 'viewport')).toBeNull()
    expect(resolveKeyboardCommand(key('1', { code: 'Digit1' }), 'viewport')).toBe('select-point')
    expect(resolveKeyboardCommand(key('0', { code: 'Numpad0' }), 'viewport')).toBe('iso')
    expect(resolveKeyboardCommand(key('/', { shiftKey: true }), 'viewport')).toBe('focus')
  })

  it('resolves intentional overlap by exact modifiers and runtime precedence', () => {
    expect(resolveKeyboardCommand(key('f', { code: 'KeyF' }), 'viewport')).toBe('focus')
    expect(resolveKeyboardCommand(key('F', { code: 'KeyF', shiftKey: true }), 'viewport', {
      isEnabled: id => id === 'flip-section' || id === 'focus',
    })).toBe('flip-section')

    expect(resolveKeyboardCommand(key('Escape'), 'viewport', {
      isEnabled: id => id === 'cancel-measure' || id === 'deselect',
    })).toBe('cancel-measure')
    expect(resolveKeyboardCommand(key('Escape'), 'viewport', {
      isEnabled: id => id === 'deselect',
    })).toBe('deselect')
  })

  it('routes the existing editor, viewport, and global shortcuts', () => {
    expect(resolveKeyboardCommand(key('Enter', { ctrlKey: true }), 'editor')).toBe('render')
    expect(resolveKeyboardCommand(key('s', { metaKey: true }), 'editor')).toBe('save')
    expect(resolveKeyboardCommand(key('=', { code: 'Equal', ctrlKey: true }), 'viewport')).toBe('measure')
    expect(resolveKeyboardCommand(key('b', { ctrlKey: true, shiftKey: true }), 'global')).toBe('sidebar')
    expect(resolveKeyboardCommand(key('[', { code: 'BracketLeft' }), 'viewport')).toBe('previous-view')
    expect(resolveKeyboardCommand(key('m', { code: 'KeyM', shiftKey: true }), 'viewport')).toBe('cycle-selection-mode')
    expect(resolveKeyboardCommand(key('h', { code: 'KeyH' }), 'viewport')).toBe('hide-selected')
    expect(resolveKeyboardCommand(key('5', { code: 'Digit5' }), 'viewport')).toBe('projection')
  })

  it('guards editable targets while preserving explicit application shortcuts', () => {
    const textarea = { tagName: 'TEXTAREA' }
    const nestedInEditor = {
      tagName: 'SPAN',
      parentElement: { tagName: 'DIV', isContentEditable: true },
    }
    expect(isEditableTarget(textarea)).toBe(true)
    expect(isEditableTarget(nestedInEditor)).toBe(true)
    expect(isEditableTarget({ tagName: 'CANVAS' })).toBe(false)

    expect(resolveKeyboardCommand(key('f', { target: textarea }), 'viewport')).toBeNull()
    expect(resolveKeyboardCommand(key('s', { ctrlKey: true, target: textarea }), 'editor')).toBe('save')
    expect(resolveKeyboardCommand(key('Enter', { ctrlKey: true, target: textarea }), 'editor')).toBe('render')
  })

  it('ignores prevented and composing keyboard events', () => {
    expect(resolveKeyboardCommand(key('g', { defaultPrevented: true }), 'viewport')).toBeNull()
    expect(resolveKeyboardCommand(key('g', { isComposing: true }), 'viewport')).toBeNull()
  })
})
