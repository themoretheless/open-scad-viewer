import { expect, it, vi } from 'vitest'
import { DirectHistory, emptyDirectDocument, parseDirectDocument } from '../src/services/directModeling'

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

it('projects detached object identities without copying mesh geometry',()=>{
 const d=emptyDirectDocument()
 d.sketches.push({id:'outline',name:'Outline',points:[[0,0],[1,0],[0,1]],closed:true})
 d.bodies.push({id:'body',name:'Body',mesh:{positions:new Float64Array([0,0,0,1,0,0,0,1,0]),indices:new Uint32Array([0,1,2])}})
 const history=new DirectHistory(d)
 const copying=vi.spyOn(globalThis,'structuredClone')
 const ids=history.objectIds
 expect(copying).not.toHaveBeenCalled();copying.mockRestore()
 expect(ids).toEqual(['body','outline'])
 ids.splice(0,ids.length,'corrupted')
 expect(history.objectIds).toEqual(['body','outline'])
 history.commit(emptyDirectDocument());expect(history.objectIds).toEqual([])
 history.undo();expect(history.objectIds).toEqual(['body','outline'])
 history.redo();expect(history.objectIds).toEqual([])
})


it('reading identities after async restore leaves compact history unmaterialized',async()=>{
 const d=emptyDirectDocument()
 d.sketches.push({id:'outline',name:'Outline',points:[[0,0],[1,0],[0,1]],closed:true})
 const history=new DirectHistory(d)
 history.commit(emptyDirectDocument())
 expect(await history.restoreAsync('undo',async text=>parseDirectDocument(text))).toBe(true)
 expect(history.storageStats.materializedStates).toBe(0)
 expect(history.objectIds).toEqual(['outline'])
 expect(history.storageStats.materializedStates).toBe(0)
 expect(await history.restoreAsync('redo',async text=>parseDirectDocument(text))).toBe(true)
 expect(history.objectIds).toEqual([])
 expect(history.storageStats.materializedStates).toBe(0)
})
