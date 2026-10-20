import {readFileSync} from 'node:fs'
import {afterEach,expect,it,vi} from 'vitest'
import {IDBFactory} from 'fake-indexeddb'
import {DirectHistory,parseDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
import {transformSelection} from '../src/services/directSolidTools'
import {readSolidDraftHead,writeSolidDraftHead} from '../src/services/solidDraftHeadStore'

afterEach(()=>vi.unstubAllGlobals())

it.each(['bracket','flange','enclosure'])('preserves actual %s geometry and history across twenty competing draft publications',async(name)=>{
 vi.stubGlobal('indexedDB',new IDBFactory())
 const original=parseDirectDocument(readFileSync(`docs/qualification/cad-ui-history-2026-10-09/${name}.json`,'utf8'))
 const history=new DirectHistory(original)
 const states=[serializeDirectDocument(original)]
 let head=await writeSolidDraftHead(name,null,states[0])
 for(let step=0;step<20;step++){
  const next=transformSelection(history.document,[name],[0,0,0.1],[0,0,1],0,1)
  history.commit(next)
  const text=serializeDirectDocument(history.document)
  const competitor=serializeDirectDocument(transformSelection(history.document,[name],[1,0,0],[0,0,1],0,1))
  const results=await Promise.allSettled([
   writeSolidDraftHead(name,head.revision,text),
   writeSolidDraftHead(name,head.revision,competitor),
  ])
  expect(results.filter(result=>result.status==='fulfilled')).toHaveLength(1)
  expect(results.find(result=>result.status==='rejected')).toMatchObject({reason:{message:'DRAFT_CONFLICT'}})
  head=(await readSolidDraftHead(name))!
  // A conflict must be explicit; resolving it publishes the intended CAD state.
  if(head.text!==text) head=await writeSolidDraftHead(name,head.revision,text)
  expect(serializeDirectDocument(parseDirectDocument(head.text))).toBe(text)
  await expect(writeSolidDraftHead(name,head.revision,competitor,()=>false)).rejects.toThrow('DRAFT_SUPERSEDED')
  expect(await readSolidDraftHead(name)).toEqual(head)
  states.push(text)
 }
 for(let step=19;step>=0;step--){
  const text=serializeDirectDocument(history.undo())
  expect(text).toBe(states[step])
  head=await writeSolidDraftHead(name,head.revision,text)
  expect(serializeDirectDocument(parseDirectDocument((await readSolidDraftHead(name))!.text))).toBe(states[step])
 }
 for(let step=1;step<=20;step++){
  const text=serializeDirectDocument(history.redo())
  expect(text).toBe(states[step])
  head=await writeSolidDraftHead(name,head.revision,text)
 }
 expect(serializeDirectDocument(parseDirectDocument((await readSolidDraftHead(name))!.text))).toBe(states[20])
},120_000)
