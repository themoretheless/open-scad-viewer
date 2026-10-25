import {describe,it,expect} from 'vitest'
import {packHistoryText,unpackHistoryText,sweepHistoryTextPool} from '../src/services/historyText'
describe('history text retention',()=>{
 it('roundtrips escaped JSON and shares unchanged geometry without losing snapshots',()=>{
  const pool=new Map<string,string>(),states=[]
  for(let edit=0;edit<21;edit++){
   const text=JSON.stringify({edit,label:'quotes " braces } commas , backslash \\',geometry:Array.from({length:10000},(_,i)=>({id:i,point:[i,i+1,i+2]}))})
   const snapshot=packHistoryText(text,pool);states.push(snapshot)
   expect(unpackHistoryText(snapshot)).toBe(text)
  }
  const retained=sweepHistoryTextPool(pool,states)
  expect(retained).toBeLessThan(states[0]!.characters*2)
  for(let edit=0;edit<21;edit++)expect(JSON.parse(unpackHistoryText(states[edit]!)).edit).toBe(edit)
  const after=sweepHistoryTextPool(pool,[states.at(-1)!]);expect(after).toBeLessThan(retained)
 })
 it('preserves arbitrary text, empty text, and long strings exactly',()=>{
  const pool=new Map<string,string>()
  for(const text of ['', 'plain', JSON.stringify(['x'.repeat(10000),'\\"',null,true]),'{"x":[]}'])expect(unpackHistoryText(packHistoryText(text,pool))).toBe(text)
  expect(sweepHistoryTextPool(pool,[])).toBe(0);expect(pool.size).toBe(0)
 })
})
