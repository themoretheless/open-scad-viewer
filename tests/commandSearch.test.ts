import { describe, expect, it } from 'vitest'
import {
  isPaletteCommandEnabled,
  nextEnabledCommandIndex,
  nextPaletteCommandIndex,
  normalizeCommandText,
  rankPaletteCommands,
  type PaletteCommand,
} from '../src/services/commandSearch'

function command(id: string, label: string, options: Partial<PaletteCommand> = {}): PaletteCommand {
  return { id, label, ...options }
}

describe('command search', () => {
  it('ranks exact, prefix, token, subsequence and typo matches deterministically', () => {
    const commands = [
      command('typo', 'rendur'),
      command('subsequence', 'r-e-n-d-e-r'),
      command('token', 'Quick render'),
      command('prefix', 'Render model'),
      command('exact', 'Render'),
    ]

    expect(rankPaletteCommands(commands, 'render').map(item => item.id)).toEqual([
      'exact',
      'prefix',
      'token',
      'subsequence',
      'typo',
    ])
  })

  it('searches localized aliases, keywords and detail', () => {
    const commands = [
      command('measure', 'Измерить', { aliases: ['Measure', 'Distance'] }),
      command('section', 'Section', { aliases: ['Сечение'] }),
      command('grid', 'Grid', { keywords: ['сетка', 'overlay'] }),
      command('export', 'Export', { detail: 'Сохранить модель' }),
    ]

    expect(rankPaletteCommands(commands, 'measure')[0]?.id).toBe('measure')
    expect(rankPaletteCommands(commands, 'сечение')[0]?.id).toBe('section')
    expect(rankPaletteCommands(commands, 'сетка')[0]?.id).toBe('grid')
    expect(rankPaletteCommands(commands, 'сохранить')[0]?.id).toBe('export')
  })

  it('handles multi-token queries and bounded Russian or English typos', () => {
    const commands = [
      command('measure', 'Measure distance', { aliases: ['Измерить расстояние'] }),
      command('render', 'Render model'),
      command('unrelated', 'Open file'),
    ]

    expect(rankPaletteCommands(commands, 'measure dist').map(item => item.id)).toEqual(['measure'])
    expect(rankPaletteCommands(commands, 'измирить').map(item => item.id)).toEqual(['measure'])
    expect(rankPaletteCommands(commands, 'redner').map(item => item.id)).toEqual(['render'])
    expect(rankPaletteCommands(commands, 'xyz').map(item => item.id)).toEqual([])
  })

  it('uses MRU only to break equal textual ranks', () => {
    const commands = [
      command('exact-old', 'Render', { mruRank: 20 }),
      command('prefix-recent', 'Render model', { mruRank: 0 }),
      command('exact-recent', 'Render', { mruRank: 1 }),
    ]

    expect(rankPaletteCommands(commands, 'render').map(item => item.id)).toEqual([
      'exact-recent',
      'exact-old',
      'prefix-recent',
    ])
  })

  it('keeps all commands for an empty query, with enabled MRU commands first', () => {
    const commands = [
      command('normal', 'Normal'),
      command('disabled-recent', 'Disabled', { enabled: false, mruRank: 0 }),
      command('recent', 'Recent', { mruRank: 2 }),
      command('older', 'Older', { mruRank: 10 }),
    ]

    expect(rankPaletteCommands(commands, '').map(item => item.id)).toEqual([
      'recent',
      'older',
      'normal',
      'disabled-recent',
    ])
    expect(commands.map(item => item.id)).toEqual(['normal', 'disabled-recent', 'recent', 'older'])
  })

  it('keeps matching disabled commands visible and skips them in navigation', () => {
    const commands = [
      command('disabled', 'Export STL', { enabled: false, disabledReason: 'Build the full model first' }),
      command('first', 'Export OBJ'),
      command('also-disabled', 'Export source', { enabled: false }),
      command('last', 'Export image'),
    ]
    const results = rankPaletteCommands(commands, 'export')

    expect(results.map(item => item.id)).toContain('disabled')
    expect(isPaletteCommandEnabled(results.find(item => item.id === 'disabled'))).toBe(false)
    expect(nextEnabledCommandIndex(commands, -1, 1)).toBe(1)
    expect(nextEnabledCommandIndex(commands, 1, 1)).toBe(3)
    expect(nextEnabledCommandIndex(commands, 3, 1)).toBe(1)
    expect(nextEnabledCommandIndex(commands, 1, -1)).toBe(3)
    expect(nextEnabledCommandIndex(commands.filter(item => item.enabled === false), -1, 1)).toBe(-1)
  })

  it('lets keyboard navigation land on disabled options to expose their reasons', () => {
    const commands = [
      command('enabled', 'Enabled'),
      command('disabled', 'Disabled', { enabled: false, disabledReason: 'Needs a model' }),
      command('last', 'Last'),
    ]

    expect(nextPaletteCommandIndex(commands, -1, 1)).toBe(0)
    expect(nextPaletteCommandIndex(commands, 0, 1)).toBe(1)
    expect(nextPaletteCommandIndex(commands, 1, 1)).toBe(2)
    expect(nextPaletteCommandIndex(commands, 2, 1)).toBe(0)
    expect(nextPaletteCommandIndex(commands, 0, -1)).toBe(2)
    expect(nextPaletteCommandIndex([], 0, 1)).toBe(-1)
  })

  it('normalizes accents, punctuation and case consistently', () => {
    expect(normalizeCommandText('  RÉNDER_scene / Ёлка  ')).toBe('render scene елка')
  })
})
