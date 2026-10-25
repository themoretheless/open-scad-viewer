import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {DirectHistory,parseDirectDocument} from '../src/services/directModeling'
import {transformSelection} from '../src/services/directSolidTools'
import {serializeDirectDocument} from '../src/services/directModeling'
it('retains all twenty translations of the actual enclosure that exhausted full-text history',()=>{
 const d=parseDirectDocument(readFileSync('docs/qualification/cad-ui-history-2026-10-09/enclosure.json','utf8'))
 const h=new DirectHistory(d),states=[serializeDirectDocument(h.document)]
 for(let i=0;i<20;i++){h.commit(transformSelection(h.document,['enclosure'],[0,0,0.1],[0,0,1],0,1));states.push(serializeDirectDocument(h.document))}
 expect(h.storageStats.undoStates).toBe(20)
 expect(h.storageStats.retainedCharacters).toBeLessThanOrEqual(16_000_000)
 for(let i=19;i>=0;i--)expect(serializeDirectDocument(h.undo())).toBe(states[i])
 expect(h.canUndo).toBe(false)
 for(let i=1;i<=20;i++)expect(serializeDirectDocument(h.redo())).toBe(states[i])
 expect(h.canRedo).toBe(false)
})
