import {expect,it,vi} from 'vitest'
import * as kernel from '../src/services/languages/kernel'
import {resolveOpenScadRotate,resolveOpenScadMirror,resolveOpenScadMultmatrix,resolveOpenScadTranslate,resolveOpenScadScale} from '../src/services/openScadStableTransformSemantics'

it('returns matrix classification with construction instead of a second analysis request',()=>{
 const spy=vi.spyOn(kernel,'languageRequest')
 try {
  resolveOpenScadRotate({a:[20,30,40]})
  expect(spy.mock.calls.map(call=>call[0])).toEqual([24])
  spy.mockClear()
  resolveOpenScadRotate({a:90,v:[1,2,3]})
  expect(spy.mock.calls.map(call=>call[0])).toEqual([27])
  spy.mockClear()
  resolveOpenScadMultmatrix([[1,0,0,2],[0,1,0,3],[0,0,1,4]])
  expect(spy.mock.calls.map(call=>call[0])).toEqual([25])
  spy.mockClear()
  resolveOpenScadMirror([1,2,3])
  expect(spy.mock.calls.map(call=>call[0])).toEqual([26])
  for(const resolve of [resolveOpenScadTranslate,resolveOpenScadScale]){
   spy.mockClear();resolve([1,2,3]);expect(spy.mock.calls.map(call=>call[0])).toEqual([26])
  }
 } finally {spy.mockRestore()}
})
