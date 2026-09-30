import {expect,it} from 'vitest'
import {sceneListWindow,sceneRowOffsets} from '../src/services/sceneListWindow'
it('bounds rendered rows throughout a thousand-object list and preserves its pixel extent',()=>{
 const offsets=sceneRowOffsets([44,...Array(1000).fill(32)])
 for(let top=0;top<offsets.at(-1)!;top+=137){
  const w=sceneListWindow(offsets,top,600)
  expect(w.end-w.start).toBeLessThanOrEqual(36)
  expect(w.before+offsets[w.end]-offsets[w.start]+w.after).toBe(w.total)
  expect(offsets[w.start]).toBeLessThanOrEqual(top)
  expect(offsets[w.end]).toBeGreaterThanOrEqual(Math.min(top+600,w.total))
 }
 const last=sceneListWindow(offsets,offsets.at(-1)!-600,600)
 expect(last.end).toBe(1001)
})
it('includes variable group headers, clamps stale scroll positions after replacement and handles empty scenes',()=>{
 const offsets=sceneRowOffsets([44,32,32,44,32])
 expect(sceneListWindow(offsets,80,80,0)).toMatchObject({start:2,end:5})
 expect(sceneListWindow(offsets,-500,33,0)).toMatchObject({start:0,end:1})
 expect(sceneListWindow(offsets,100000,600,0)).toMatchObject({start:5,end:5,after:0})
 expect(sceneListWindow(sceneRowOffsets([]),0,600)).toEqual({start:0,end:0,before:0,after:0,total:0})
})
it('rejects invalid row metrics and viewport inputs',()=>{
 for(const height of [0,-1,Infinity,NaN])expect(()=>sceneRowOffsets([height])).toThrow()
 expect(()=>sceneRowOffsets([32],-1)).toThrow()
 expect(()=>sceneListWindow([0,33],0,NaN)).toThrow()
 expect(()=>sceneListWindow([0,33],0,600,1.5)).toThrow()
})
