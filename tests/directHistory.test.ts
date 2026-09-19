import { expect, it } from 'vitest'
import { DirectHistory, emptyDirectDocument } from '../src/services/directModeling'

const document = (name: string) => ({ ...emptyDirectDocument(), groups: [{ name, source: 'cube(1);' }] })

it('retains 80 undo steps through complete undo/redo cycles and branches', () => {
  const history = new DirectHistory(document('0'))
  for (let i = 1; i <= 85; i++) history.commit(document(String(i)))
  for (let i = 84; i >= 5; i--) expect(history.undo().groups![0]!.name).toBe(String(i))
  expect(history.canUndo).toBe(false)
  for (let i = 6; i <= 85; i++) expect(history.redo().groups![0]!.name).toBe(String(i))
  expect(history.canRedo).toBe(false)
  history.undo()
  history.commit(document('branch'))
  expect(history.canRedo).toBe(false)
  expect(history.undo().groups![0]!.name).toBe('84')
})

it('keeps no-op and invalid commits atomic while callers cannot mutate snapshots', () => {
  const history = new DirectHistory(document('first'))
  history.commit(document('second'))
  const undone = history.undo()
  undone.groups![0]!.source = 'mutated'
  history.commit(history.document)
  expect(history.canRedo).toBe(true)
  expect(() => history.commit({ ...document('bad'), version: 2 as never })).toThrow()
  expect(history.canRedo).toBe(true)
  const redone = history.redo()
  redone.groups![0]!.name = 'mutated'
  expect(history.document.groups![0]!.name).toBe('second')
  expect(history.undo().groups![0]!.source).toBe('cube(1);')
})

it('retains the serialized-size bound after undo/redo and a new commit', () => {
  const large = (revision: number) => ({ ...emptyDirectDocument(),
    groups: Array.from({ length: 64 }, (_, index) => ({ name: `${revision}-${index}`, source: ' '.repeat(80_000) })),
  })
  const history = new DirectHistory(large(0))
  for (let revision = 1; revision <= 4; revision++) history.commit(large(revision))
  // Three 5.12M-character snapshots fit, four do not.
  for (let revision = 3; revision >= 1; revision--) expect(history.undo().groups![0]!.name).toBe(`${revision}-0`)
  expect(history.canUndo).toBe(false)
  for (let revision = 2; revision <= 4; revision++) expect(history.redo().groups![0]!.name).toBe(`${revision}-0`)
  history.commit(large(5))
  for (let revision = 4; revision >= 2; revision--) expect(history.undo().groups![0]!.name).toBe(`${revision}-0`)
  expect(history.canUndo).toBe(false)
})
