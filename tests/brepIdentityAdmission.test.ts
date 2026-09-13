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
/** Rebuilds a trusted lowering with exactly one identity component tampered. */
const forge=(mutate:(program:Record<string,any>)=>void):SemanticLoweringSuccess=>{
 const base=lower(SOURCE)
 const program=structuredClone(base.program) as Record<string,any>
 mutate(program)
 return {...base,program:program as SemanticLoweringSuccess['program']}
}
it('executes a real lowered program with operations transported',async()=>{
 const base=lower(SOURCE)
 expect(base.program.core.operations.length).toBeGreaterThan(0)
 expect(base.program.core.occurrences.length).toBeGreaterThan(0)
 const result=await executeBrepNativeProgram(base)
 try{
  expect(result.outputs).toHaveLength(1)
  expect(result.outputs[0].value.tag).toBe('value')
 }finally{await result.dispose()}
})
const tampers:[string,(program:Record<string,any>)=>void][]=[
 ['operationId',p=>{p.core.operations[1].operationId=p.core.operations[1].operationId.replace('opv1:','opv9:')}],
 ['operation structural path',p=>{p.core.operations[1].structuralPath=[...p.core.operations[1].structuralPath.slice(0,-1),{...p.core.operations[1].structuralPath.at(-1),ordinal:1}]}],
 ['operation childOrdinal',p=>{p.core.operations[1].childOrdinal=(p.core.operations[1].childOrdinal??0)+1}],
 ['operation ambiguity group membership',p=>{p.core.operations[1].identityEvidence='same-name-positional';p.core.operations[1].ambiguityGroup='ambv1:'+'0'.repeat(64)}],
 ['operation preorder',p=>{const ops=p.core.operations;const tail=ops.slice(1);tail.reverse();p.core.operations=[ops[0],...tail].map((op:any,id:number)=>({...op,id}))}],
 ['occurrenceId',p=>{p.core.occurrences[1].occurrenceId=p.core.occurrences[1].occurrenceId.replace('occv1:','occv9:')}],
 ['sceneEntityId',p=>{
  const row=p.core.occurrences.find((o:any)=>o.sceneEntityId!==null)
  row.sceneEntityId=row.sceneEntityId.replace('entity:v2:','entity:v9:')
 }],
 ['occurrence operation reference',p=>{
  const row=p.core.occurrences[p.core.occurrences.length-1]
  row.operation=(row.operation+1)%p.core.operations.length
 }],
 ['occurrence dynamic slot value',p=>{
  p.core.occurrences[1].dynamicSlots=[{name:'forged',value:{tag:'number',value:1},duplicateOrdinal:0}]
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
