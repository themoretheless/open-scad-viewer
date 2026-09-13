import {expect,it,vi} from 'vitest'
import type {SemanticLoweringSuccess} from '../src/services/semanticProgramLowerer'
vi.mock('../src/services/semanticProgramLowerer',async importOriginal=>{
 const original=await importOriginal<typeof import('../src/services/semanticProgramLowerer')>()
 // Defense in depth: a tampered program must be refused by native admission
 // even if host-side lowering trust were bypassed, so admission cannot rely
 // on the process-local trust set. Trust itself is covered elsewhere.
 return {...original,requireTrustedSemanticLowering:(value:unknown)=>value as SemanticLoweringSuccess}
})
import {executeBrepNativeProgram} from '../src/services/brepNativeExecutor'
import {lowerOpenSCADToSemanticProgram} from '../src/services/semanticProgramLowerer'
const HEADER='// @language openscad-viewer/brep-1\n'
const SOURCE='difference(){cube(3);translate([2,0,0]) cube(1);}'
const lower=(text:string)=>lowerOpenSCADToSemanticProgram(text)
/** Rebuilds a trusted lowering with the program and/or source text tampered. */
const forge=(mutate:(draft:{program:Record<string,any>;sourceText:string})=>void):SemanticLoweringSuccess=>{
 const base=lower(HEADER+SOURCE)
 const draft={program:structuredClone(base.program) as Record<string,any>,sourceText:base.sourceText}
 mutate(draft)
 return {...base,program:draft.program as SemanticLoweringSuccess['program'],sourceText:draft.sourceText}
}
const expectContractRefusal=async(input:SemanticLoweringSuccess)=>{
 let progressed=0
 await expect(executeBrepNativeProgram(input,{onNode:()=>progressed++})).rejects.toMatchObject({
  code:'E_SEMANTIC_BACKEND_FAILURE',
  node:null,
  backendCause:{code:'E_BREP_SEMANTIC_CONTRACT'},
 })
 expect(progressed).toBe(0)
}
it('executes a real lowered program with the envelope object and source text transported',async()=>{
 const base=lower(HEADER+SOURCE)
 expect(base.sourceText).toBe(HEADER+SOURCE)
 const result=await executeBrepNativeProgram(base)
 try{
  expect(result.outputs).toHaveLength(1)
  expect(result.outputs[0].value.tag).toBe('value')
 }finally{await result.dispose()}
})
it('refuses a tampered transported source text before any node runs',async()=>{
 await expectContractRefusal(forge(draft=>{draft.sourceText=draft.sourceText+' '}))
})
it('refuses a tampered source descriptor before any node runs',async()=>{
 await expectContractRefusal(forge(draft=>{draft.program.source={...draft.program.source,utf16CodeUnitLength:draft.program.source.utf16CodeUnitLength+1}}))
 await expectContractRefusal(forge(draft=>{
  const sha=draft.program.source.sha256 as string
  draft.program.source={...draft.program.source,sha256:(sha[0]==='a'?'b':'a')+sha.slice(1)}
 }))
})
it('refuses a provenance span endpoint inside a surrogate pair',async()=>{
 const text=HEADER+'// 😀\n'+SOURCE
 const emoji=text.indexOf('😀')
 expect(emoji).toBeGreaterThan(0)
 const base=lower(text)
 expect(base.program.source.utf16CodeUnitLength).toBe(text.length)
 const draft={program:structuredClone(base.program) as Record<string,any>,sourceText:base.sourceText}
 // The emoji occupies UTF-16 offsets emoji (high) and emoji+1 (low):
 // a boundary at emoji+1 splits the pair.
 draft.program.provenance[0]={...draft.program.provenance[0],span:{start:emoji+1,end:emoji+2}}
 await expectContractRefusal({...base,program:draft.program as SemanticLoweringSuccess['program'],sourceText:draft.sourceText})
})
it('admits span endpoints at surrogate pair edges',async()=>{
 const text=HEADER+'// 😀\n'+SOURCE
 const emoji=text.indexOf('😀')
 const base=lower(text)
 const draft={program:structuredClone(base.program) as Record<string,any>,sourceText:base.sourceText}
 draft.program.provenance[0]={...draft.program.provenance[0],span:{start:emoji,end:emoji+2}}
 const result=await executeBrepNativeProgram({...base,program:draft.program as SemanticLoweringSuccess['program'],sourceText:draft.sourceText})
 try{
  expect(result.outputs).toHaveLength(1)
  expect(result.outputs[0].value.tag).toBe('value')
 }finally{await result.dispose()}
})
