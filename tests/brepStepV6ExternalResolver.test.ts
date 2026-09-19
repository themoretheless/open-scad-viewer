import {createHash} from 'node:crypto'
import {describe,expect,it} from 'vitest'
import {createBrepBox} from '../src/services/geometry/brep'
import {exportDirectStepV6} from '../src/services/cadNurbsStep'
import {importStepV6WithResolver,type StepExternalResolver} from '../src/services/cadStepExternalResolver'
import {importStepForWorkbench} from '../src/services/cadStepRouting'

const sha=(text:string)=>createHash('sha256').update(text).digest('hex')
const reference=(text:string,uri:string)=>text.replace('ENDSEC;\nEND-ISO',`#62000=DOCUMENT_FILE('${uri}','','',(),(),(),());\nENDSEC;\nEND-ISO`)

describe('STEP /6 explicit external resolver',()=>{
  it('resolves allowlisted digest-bound documents without ambient IO',async()=>{
    const child=exportDirectStepV6(createBrepBox([0,0,0],[1,1,1])).text
    const root=reference(exportDirectStepV6(createBrepBox([2,0,0],[3,1,1])).text,'urn:part:child')
    const calls:string[]=[]
    const resolver:StepExternalResolver={resolve:async uri=>{calls.push(uri);return{text:child,sha256:sha(child)}}}
    const bundle=await importStepV6WithResolver(root,resolver,new AbortController().signal,{
      allowedUris:new Set(['urn:part:child']),expectedSha256:{'urn:part:child':sha(child)},
    })
    expect(calls).toEqual(['urn:part:child'])
    expect(bundle.root.model.bodies).toHaveLength(1)
    expect(bundle.documents[0].imported.model.bodies).toHaveLength(1)
    expect(bundle.authoritative.model.bodies).toHaveLength(2)
    expect(bundle.authoritative.occurrences).toHaveLength(2)
    expect(new Set(bundle.authoritative.occurrences.map(row=>row.topoId)).size).toBe(2)
    expect(bundle.authoritative.documents.map(row=>row.digest)).toEqual([sha(root),sha(child)])
    expect(bundle.report.resolved).toEqual(['urn:part:child'])
    expect(()=>importStepForWorkbench(root)).toThrow(/explicitly authorized resolver/)
  })

  it('typed-refuses untrusted, mismatched, cancelled, cyclic and over-depth references',async()=>{
    const child=exportDirectStepV6(createBrepBox([0,0,0],[1,1,1])).text
    const root=reference(child,'urn:a')
    const resolver:StepExternalResolver={resolve:async uri=>({
      text:uri==='urn:a'?reference(child,'urn:b'):reference(child,'urn:a'),sha256:sha(uri==='urn:a'?reference(child,'urn:b'):reference(child,'urn:a')),
    })}
    await expect(importStepV6WithResolver(root,resolver,new AbortController().signal)).rejects.toThrow(/Untrusted/)
    await expect(importStepV6WithResolver(root,{resolve:async()=>({text:child,sha256:'0'.repeat(64)})},new AbortController().signal,{
      allowedUris:new Set(['urn:a']),expectedSha256:{'urn:a':sha(child)},
    })).rejects.toThrow(/digest mismatch/)
    const cancelled=new AbortController();cancelled.abort()
    await expect(importStepV6WithResolver(root,resolver,cancelled.signal,{allowedUris:new Set(['urn:a'])})).rejects.toMatchObject({name:'AbortError'})
    const a=reference(child,'urn:b'),b=reference(child,'urn:a')
    const cyclic:StepExternalResolver={resolve:async uri=>{const text=uri==='urn:a'?a:b;return{text,sha256:sha(text)}}}
    await expect(importStepV6WithResolver(root,cyclic,new AbortController().signal,{
      allowedUris:new Set(['urn:a','urn:b']),expectedSha256:{'urn:a':sha(a),'urn:b':sha(b)},
    })).rejects.toThrow(/cycle/)
    await expect(importStepV6WithResolver(root,cyclic,new AbortController().signal,{
      allowedUris:new Set(['urn:a','urn:b']),expectedSha256:{'urn:a':sha(a),'urn:b':sha(b)},maxDepth:1,
    })).rejects.toThrow(/depth/)
  })

  it('uses parsed references and enforces exact plus-one byte and document limits',async()=>{
    const child=exportDirectStepV6(createBrepBox([0,0,0],[1,1,1])).text.replace('DATA;','DATA;/*é*/')
    const root=reference(exportDirectStepV6(createBrepBox([2,0,0],[3,1,1])).text,'urn:part:child')
      .replace('DATA;',"DATA;\n/* DOCUMENT_FILE('urn:comment') */")
    const resolver:StepExternalResolver={resolve:async()=>({text:child,sha256:sha(child)})}
    const rootBytes=new TextEncoder().encode(root).byteLength
    const childBytes=new TextEncoder().encode(child).byteLength
    const policy={allowedUris:new Set(['urn:part:child']),expectedSha256:{'urn:part:child':sha(child)},
      maxDocuments:1,maxDocumentBytes:childBytes,maxTotalBytes:rootBytes+childBytes}
    await expect(importStepV6WithResolver(root,resolver,new AbortController().signal,policy)).resolves.toMatchObject({
      report:{resolved:['urn:part:child'],totalBytes:rootBytes+childBytes},
    })
    await expect(importStepV6WithResolver(root,resolver,new AbortController().signal,{...policy,maxDocuments:0})).rejects.toThrow(/count/)
    await expect(importStepV6WithResolver(root,resolver,new AbortController().signal,{...policy,maxDocumentBytes:childBytes-1})).rejects.toThrow(/byte limit/)
    await expect(importStepV6WithResolver(root,resolver,new AbortController().signal,{...policy,maxTotalBytes:rootBytes+childBytes-1})).rejects.toThrow(/total byte/)
  })
})
