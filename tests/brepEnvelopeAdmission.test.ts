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
const SOURCE='difference(){cube(3);translate([2,0,0]) cube(1);}'
const lower=(body:string)=>lowerOpenSCADToSemanticProgram('// @language openscad-viewer/brep-1\n'+body)
/** Rebuilds a trusted lowering with exactly one envelope component tampered. */
const forge=(mutate:(program:Record<string,any>)=>void):SemanticLoweringSuccess=>{
 const base=lower(SOURCE)
 const program=structuredClone(base.program) as Record<string,any>
 mutate(program)
 return {...base,program:program as SemanticLoweringSuccess['program']}
}
it('admits the lowered envelope components on the native graph path',async()=>{
 const result=await executeBrepNativeProgram(lower(SOURCE))
 try{
  expect(result.outputs).toHaveLength(1)
  expect(result.outputs[0].value.tag).toBe('value')
 }finally{await result.dispose()}
})
const tampers:[string,(program:Record<string,any>)=>void][]=[
 ['source digest letter case',p=>{p.source={...p.source,sha256:p.source.sha256.toUpperCase()}}],
 ['source UTF-8 length beyond the limit',p=>{p.source={...p.source,utf8ByteLength:4_000_001}}],
 ['source UTF-16 length beyond the limit',p=>{p.source={...p.source,utf16CodeUnitLength:250_001}}],
 ['core schema literal',p=>{p.core={...p.core,schema:'semantic-program-core-v2'}}],
 ['schema version 1.0 requiring re-lowering',p=>{p.core={...p.core,schemaVersion:{major:1,minor:0}}}],
 ['schema version 1.1 requiring re-lowering',p=>{p.core={...p.core,schemaVersion:{major:1,minor:1}}}],
 ['empty required features',p=>{p.core={...p.core,requiredFeatures:[]}}],
 ['identity version',p=>{p.core={...p.core,identityVersion:'semantic-program-core-v0'}}],
 ['language semantics revision',p=>{p.core={...p.core,language:{...p.core.language,semanticsRevision:'brep-0.0.0'}}}],
 ['capability graph version',p=>{p.core={...p.core,language:{...p.core.language,capabilityGraphVersion:'semantic-capabilities-v0'}}}],
 ['units convention',p=>{p.core={...p.core,units:{...p.core.units,length:'meter'}}}],
 ['unsorted declared capabilities',p=>{p.core={...p.core,declaredCapabilities:['b.cap','a.cap']}}],
 ['duplicate declared capabilities',p=>{p.core={...p.core,declaredCapabilities:['a.cap','a.cap']}}],
 ['declared capability missing from the closure',p=>{p.core={...p.core,declaredCapabilities:[...p.core.declaredCapabilities,'zzz.declared']}}],
 ['capability closure missing one entry',p=>{p.core={...p.core,capabilityClosure:p.core.capabilityClosure.slice(1)}}],
 ['capability closure with a changed entry',p=>{
  const closure=[...p.core.capabilityClosure]
  closure[closure.length-1]='zzz.forged'
  p.core={...p.core,capabilityClosure:closure}
 }],
]
for(const [name,mutate] of tampers){
 it(`refuses a tampered ${name} before any geometry executes`,async()=>{
  let progressed=0
  await expect(executeBrepNativeProgram(forge(mutate),{onNode:()=>progressed++})).rejects.toMatchObject({
   code:'E_SEMANTIC_BACKEND_FAILURE',
   node:null,
   backendCause:{code:'E_BREP_SEMANTIC_CONTRACT'},
  })
  expect(progressed).toBe(0)
 })
}
