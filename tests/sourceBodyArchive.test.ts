import {expect,it} from 'vitest'
import {readFileSync} from 'node:fs'
import {gunzipSync} from 'node:zlib'
import {createSourceBodyRecord,sourceBodyRecordOptions} from '../src/services/sourceBodyArchive'
import {DirectHistory,emptyDirectDocument,parseDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
import {exportSolidStepAssembly} from '../src/services/solidStepAssembly'
import {exportSolidBlenderSnapshot} from '../src/services/solidBlenderExchange'
import type {SourceBodyOptions} from '../src/services/sourceBody'
const fixture=JSON.parse(gunzipSync(readFileSync(new URL('../docs/qualification/rolling-ball-offset-foundation-2026-10-05/source-body-request.json.gz',import.meta.url))).toString())
const options=():SourceBodyOptions=>{const {op:_op,...o}=structuredClone(fixture);return o}
it('retains exact source input bytes across document reload and metadata Undo/Redo',()=>{
 const record=createSourceBodyRecord('source-canal','Native canal',options())
 const document={...emptyDirectDocument(),sourceBodies:[record]},text=serializeDirectDocument(document)
 const loaded=parseDirectDocument(text);expect(loaded.sourceBodies).toEqual([record])
 const history=new DirectHistory(loaded);expect(history.objectIds).toEqual(['source-canal'])
 const renamed=history.document;renamed.sourceBodies![0]!.name='Renamed canal';history.commit(renamed)
 expect(history.undo().sourceBodies).toEqual([record])
 expect(history.redo().sourceBodies![0]!.source).toEqual(record.source)
 expect(parseDirectDocument(serializeDirectDocument(history.document)).sourceBodies![0]!.source).toEqual(record.source)
},30000)
it('preserves signed zero in the opaque source payload',()=>{
 const o=options();o.limits.volume.origin=-0
 const record=createSourceBodyRecord('signed-zero','Signed zero',o)
 const archived=JSON.parse(JSON.stringify(record))
 expect(Object.is(sourceBodyRecordOptions(archived).limits.volume.origin,-0)).toBe(true)
})
it('rechecks native admission rather than trusting a saved success flag',()=>{
 const o=options();o.limits.volume.cells=1
 const record={...createSourceBodyRecord('refused-source','Refused',o),admitted:true}
 expect(()=>parseDirectDocument(serializeDirectDocument({...emptyDirectDocument(),sourceBodies:[record]}))).toThrow('source-volume-initial-work-limit')
})
it('refuses damaged bytes and duplicate identities',()=>{
 const record=createSourceBodyRecord('duplicate','Native',options())
 expect(()=>sourceBodyRecordOptions({...record,source:{encoding:'MGV1/base64',data:'AAAA'}})).toThrow()
 const damaged={...record,source:{encoding:'MGV1/base64' as const,data:'AAAA'}}
 expect(()=>parseDirectDocument(serializeDirectDocument({...emptyDirectDocument(),sourceBodies:[damaged]}))).toThrow('Source body duplicate:')
 const document={...emptyDirectDocument(),sourceBodies:[record,{...record}]}
 expect(()=>parseDirectDocument(serializeDirectDocument(document))).toThrow('Invalid object identity')
})
it('refuses unsupported whole-scene exchange rather than omitting native source bodies',async()=>{
 const document={...emptyDirectDocument(),sourceBodies:[createSourceBodyRecord('source','Source',options())]}
 await expect(exportSolidStepAssembly(document)).rejects.toThrow('exact source Body restrictions')
 expect(()=>exportSolidBlenderSnapshot(document,'project')).toThrow('exact source bodies')
})
