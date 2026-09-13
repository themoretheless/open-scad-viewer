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
const PROGRAMS:Record<string,string>={
 difference:'difference(){cube(3);translate([2,0,0]) cube(1);}',
 unionTransform:'union(){cube(2);translate([1,0,0]) cube(1);}',
 hull:'hull(){cube(1);translate([2,0,0]) cube(1);}',
 children:'module pair(){children(0);children(1);} pair(){cube(1);translate([2,0,0]) cube(1);}',
}
const lower=(body:string)=>lowerOpenSCADToSemanticProgram('// @language openscad-viewer/brep-1\n'+body)
/** Rebuilds a trusted lowering with exactly one production component tampered. */
const forge=(source:string,mutate:(program:Record<string,any>)=>void):SemanticLoweringSuccess=>{
 const base=lower(source)
 const program=structuredClone(base.program) as Record<string,any>
 mutate(program)
 return {...base,program:program as SemanticLoweringSuccess['program']}
}
for(const [name,source] of Object.entries(PROGRAMS)){
 if(name==='hull'){
  // The native executor does not evaluate hull nodes yet (pre-existing gap,
  // see brepNativeExecutor.test.ts); the lowered hull program must still pass
  // native production admission and be refused downstream by evaluation.
  it('admits a real lowered hull program natively (evaluation remains unsupported)',async()=>{
   await expect(executeBrepNativeProgram(lower(source))).rejects.toMatchObject({
    code:'E_SEMANTIC_BACKEND_FAILURE',
    backendCause:{code:'E_BREP_SEMANTIC_UNSUPPORTED'},
   })
  })
  continue
 }
 it(`executes a real lowered ${name} program through native production admission`,async()=>{
  const base=lower(source)
  expect(base.program.core.occurrences.length).toBeGreaterThan(0)
  const result=await executeBrepNativeProgram(base)
  try{
   expect(result.outputs.length).toBeGreaterThan(0)
   for(const output of result.outputs) expect(output.value.tag).toBe('value')
  }finally{await result.dispose()}
 })
}
const tampers:[string,string,(program:Record<string,any>)=>void][]=[
 ['removed occurrence row',PROGRAMS.difference,p=>{p.core.occurrences.splice(2,1)}],
 ['duplicated occurrence row',PROGRAMS.difference,p=>{p.core.occurrences.push(structuredClone(p.core.occurrences[2]))}],
 ['broken transform-map chain',PROGRAMS.difference,p=>{
  const transform=p.core.nodes.find((node:any)=>node.kind==='transform')
  transform.input=0
 }],
 ['bad boolean operand structure',PROGRAMS.unionTransform,p=>{
  const union=p.core.nodes.find((node:any)=>node.kind==='boolean'&&node.operation==='union')
  union.inputs=[...union.inputs].reverse()
 }],
 ['broken hull reduction',PROGRAMS.hull,p=>{
  const hull=p.core.nodes.find((node:any)=>node.kind==='hull')
  hull.inputs=[...hull.inputs].reverse()
 }],
 ['misplaced production node reference',PROGRAMS.difference,p=>{
  p.core.occurrences[2].node=p.core.nodes.length-1
 }],
 ['broken $expansion continuation anchor',PROGRAMS.children,p=>{
  const expansion=p.core.occurrences.find((row:any)=>p.core.operations[row.operation].name==='$expansion')
  expansion.staticParent=0
 }],
 ['removed $expansion continuation row',PROGRAMS.children,p=>{
  const index=p.core.occurrences.findIndex((row:any)=>p.core.operations[row.operation].name==='$expansion')
  p.core.occurrences.splice(index,1)
 }],
 ['broken result producer row',PROGRAMS.difference,p=>{
  p.core.result.item.producerOccurrence=1
 }],
]
for(const [name,source,mutate] of tampers){
 it(`refuses a tampered ${name} before any geometry executes`,async()=>{
  let progressed=0
  await expect(executeBrepNativeProgram(forge(source,mutate),{onNode:()=>progressed++})).rejects.toMatchObject({
   code:'E_SEMANTIC_BACKEND_FAILURE',
   node:null,
   backendCause:{code:'E_BREP_SEMANTIC_CONTRACT'},
  })
  expect(progressed).toBe(0)
 })
}
