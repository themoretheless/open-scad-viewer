import { describe, expect, it } from 'vitest'
import { DirectHistory, emptyDirectDocument, type DirectDocument } from '../src/app/directModeling'
import { createSelectionHistory } from '../src/features/directModeler/selectionHistory'

const sketch = (id: string) => ({
  id,
  name: id,
  points: [
    [0, 0],
    [1, 0],
    [0, 1],
  ],
  closed: true,
})
const withSketches = (...ids: string[]): DirectDocument => ({ ...emptyDirectDocument(), sketches: ids.map(sketch) })

describe('selection history', () => {
  it('returns undefined for unknown states and filters missing identities', () => {
    const memory = createSelectionHistory()
    const state = {}
    expect(memory.recall(state, () => true)).toBeUndefined()
    memory.remember(state, { primary: 'a', extra: ['b', 'a', 'c'] })
    expect(memory.recall(state, id => id !== 'a')).toEqual({ primary: 'b', extra: ['c'] })
    expect(memory.recall(state, () => false)).toEqual({ primary: '', extra: [] })
  })

  it('copies the recorded selection', () => {
    const memory = createSelectionHistory()
    const state = {}
    const extra = ['b']
    memory.remember(state, { primary: 'a', extra })
    extra.push('x')
    expect(memory.recall(state, () => true)).toEqual({ primary: 'a', extra: ['b'] })
  })

  it('follows DirectHistory states through Undo and Redo', () => {
    const history = new DirectHistory()
    const memory = createSelectionHistory()
    const exists = (id: string) => history.objectIds.includes(id)
    const empty = history.stateToken
    memory.remember(empty, { primary: '', extra: [] })
    history.commit(withSketches('s1'))
    const first = history.stateToken
    expect(first).not.toBe(empty)
    memory.remember(first, { primary: 's1', extra: [] })
    history.commit(withSketches('s1', 's2'))
    const second = history.stateToken
    memory.remember(second, { primary: 's2', extra: ['s1'] })
    history.undo()
    expect(history.stateToken).toBe(first)
    expect(memory.recall(history.stateToken, exists)).toEqual({ primary: 's1', extra: [] })
    history.redo()
    expect(history.stateToken).toBe(second)
    expect(memory.recall(history.stateToken, exists)).toEqual({ primary: 's2', extra: ['s1'] })
    history.undo()
    history.undo()
    expect(history.stateToken).toBe(empty)
    expect(memory.recall(history.stateToken, exists)).toEqual({ primary: '', extra: [] })
  })

  it('keeps the token for an unchanged commit and for asynchronous restore', async () => {
    const history = new DirectHistory()
    history.commit(withSketches('s1'))
    const first = history.stateToken
    history.commit(withSketches('s1'))
    expect(history.stateToken).toBe(first)
    history.commit(withSketches('s1', 's2'))
    expect(await history.restoreAsync('undo', async () => withSketches('s1'))).toBe(true)
    expect(history.stateToken).toBe(first)
  })
})
