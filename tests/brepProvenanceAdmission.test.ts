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
const intent=(occurrence:number)=>({occurrence,chordTolerance:0.1,angularToleranceDegrees:null as null,minSegments:3,maxSegments:64})
it('executes a real lowered program with all envelope components transported',async()=>{
 const base=lower(SOURCE)
 expect(base.program.provenance.length).toBe(base.program.core.operations.length)
 expect(base.program.diagnostics.length).toBe(base.program.core.diagnosticTemplates.length)
 const result=await executeBrepNativeProgram(base)
 try{
  expect(result.outputs).toHaveLength(1)
  expect(result.outputs[0].value.tag).toBe('value')
 }finally{await result.dispose()}
})
const tampers:[string,(program:Record<string,any>)=>void][]=[
 ['dropped provenance entry',p=>{p.provenance=p.provenance.slice(1)}],
 ['reordered provenance entries',p=>{p.provenance=[p.provenance[1],p.provenance[0],...p.provenance.slice(2)]}],
 ['provenance span beyond the source',p=>{p.provenance[0]={...p.provenance[0],span:{start:0,end:p.source.utf16CodeUnitLength+1}}}],
 ['empty provenance span',p=>{p.provenance[0]={...p.provenance[0],span:{start:1,end:1}}}],
 ['duplicate tessellation intent occurrence',p=>{p.tessellationIntents=[intent(0),intent(0)]}],
 ['intent with maxSegments below minSegments',p=>{
  const producing=p.core.occurrences.findIndex((occurrence:any)=>occurrence.node!==null)
  p.tessellationIntents=[{...intent(producing),minSegments:10,maxSegments:3}]
 }],
 ['non-positive intent chord tolerance',p=>{
  const producing=p.core.occurrences.findIndex((occurrence:any)=>occurrence.node!==null)
  p.tessellationIntents=[{...intent(producing),chordTolerance:0}]
 }],
 ['diagnostic without a template',p=>{p.diagnostics=[...p.diagnostics,{template:p.diagnostics.length,message:'forged',span:null}]}],
 ['reordered diagnostics',p=>{
  if(p.diagnostics.length>1)p.diagnostics=[p.diagnostics[1],p.diagnostics[0],...p.diagnostics.slice(2)]
  else p.diagnostics=[{template:0,message:'forged',span:null}]
 }],
 ['diagnostic template code',p=>{
  if(p.core.diagnosticTemplates.length>0)p.core.diagnosticTemplates[0]={...p.core.diagnosticTemplates[0],code:'forged-code'}
  else p.core.diagnosticTemplates=[{id:0,code:'forged-code',severity:'info',operation:null,arguments:[]}]
 }],
]
// The legacy/current intent refusal cannot be reached through the executor
// (the host requires brep-1 before graph-begin); it is covered natively.
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
