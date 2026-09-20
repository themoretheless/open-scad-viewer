import {expect, it} from 'vitest'
import {MeshHistory, type MeshWorkspaceDocument} from '../src/services/meshEditing'

function document(name: string): MeshWorkspaceDocument {
  return {version: 1, objects: [{id: 'mesh', name, visible: true,
    mesh: {positions: [0,0,0, 1,0,0, 0,1,0], indices: [0,1,2]}}]}
}

it('keeps independent snapshots and preserves redo on a no-op commit', () => {
  const input = document('initial')
  const history = new MeshHistory(input)
  input.objects[0].mesh.positions[0] = 42
  expect(history.document.objects[0].mesh.positions[0]).toBe(0)
  const next = history.document
  next.objects[0].name = 'edited'
  history.commit(next)
  next.objects[0].mesh.positions[0] = 99
  expect(history.document.objects[0].mesh.positions[0]).toBe(0)
  history.undo()
  history.commit(history.document)
  expect(history.canRedo).toBe(true)
  expect(history.redo().objects[0].name).toBe('edited')
  history.undo()
  history.commit(document('branched'))
  expect(history.canRedo).toBe(false)
  expect(history.undo().objects[0].name).toBe('initial')
})

it('retains exactly 80 past snapshots and moves sizes with undo/redo', () => {
  const history = new MeshHistory(document('0'))
  for (let i=1;i<=90;i++) history.commit(document(String(i)))
  for (let i=89;i>=10;i--) expect(history.undo().objects[0].name).toBe(String(i))
  expect(history.canUndo).toBe(false)
  for (let i=11;i<=90;i++) expect(history.redo().objects[0].name).toBe(String(i))
  expect(history.canRedo).toBe(false)
  history.commit(document('91'))
  for (let i=90;i>=11;i--) expect(history.undo().objects[0].name).toBe(String(i))
  expect(history.canUndo).toBe(false)
})

it.each([[5_999_998, 4], [5_999_999, 3]])('preserves the exact JSON-array limit for %i-character snapshots', (characters, retained) => {
  const padded = (name: string) => {
    // Unknown document fields are preserved by the existing parser. Padding
    // isolates serialized-size arithmetic without generating complex geometry.
    const value = {...document(name), padding: ''}
    value.padding = 'x'.repeat(characters - JSON.stringify(value).length)
    expect(JSON.stringify(value).length).toBe(characters)
    return value
  }
  const history = new MeshHistory(padded('0'))
  for (let i=1;i<=4;i++) history.commit(padded(String(i)))
  let count = 0
  while (history.canUndo) {
    expect(history.undo().objects[0].name).toBe(String(3-count))
    count++
  }
  expect(count).toBe(retained)
}, 15_000)
