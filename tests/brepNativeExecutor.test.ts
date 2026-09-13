import {expect,it} from 'vitest'
import {executeBrepNativeProgram} from '../src/services/brepNativeExecutor'
import {lowerOpenSCADToSemanticProgram} from '../src/services/semanticProgramLowerer'
const lower=(body:string)=>lowerOpenSCADToSemanticProgram('// @language openscad-viewer/brep-1\n'+body)
it('executes polygonal analytic revolutions through the Rust graph',async()=>{
 for(const angle of [90,360]){
  const result=await executeBrepNativeProgram(lower(`rotate_extrude(angle=${angle}) square([2,3]);`))
  try{
   expect(result.outputs).toHaveLength(1)
   expect(result.outputs[0].value.tag).not.toBe('empty')
  }finally{await result.dispose()}
 }
})
it('executes a retained circular profile revolution through Rust',async()=>{
 for(const angle of [90,210,360]){
  const result=await executeBrepNativeProgram(lower(`rotate_extrude(angle=${angle}) translate([3,0,0]) circle(1);`))
  try{expect(result.outputs[0].value.tag).not.toBe('empty')}finally{await result.dispose()}
 }
})
it('preserves empty-set semantics across geometric feature boundaries',async()=>{
 for(const source of [
  'translate([0,0,1]) difference(){square(1);square(1);}',
  'linear_extrude(height=2,twist=30) difference(){square(1);square(1);}',
  'rotate_extrude() difference(){square(1);square(1);}',
  'projection() difference(){cube(1);cube(1);}',
  'offset(1) difference(){square(1);square(1);}',
  'hull(){difference(){cube(1);cube(1);}difference(){cube(2);cube(2);}}',
 ]){
  const result=await executeBrepNativeProgram(lower(source))
  try{expect(result.outputs.map(output=>output.value.tag)).toEqual(['empty'])}finally{await result.dispose()}
 }
})
it('reports each native step and disposes immutable output snapshots',async()=>{
 const input=lower('translate([2,0,0])cube(1);')
 const progress:number[]=[]
 const result=await executeBrepNativeProgram(input,{onNode:completed=>progress.push(completed)})
 expect(progress).toEqual(input.program.core.nodes.map((_,index)=>index+1))
 expect(result.outputs).toHaveLength(1)
 expect(Object.isFrozen(result.outputs[0].value)).toBe(true)
 await result.dispose();await result.dispose()
 expect(result.disposed).toBe(true)
 expect(result.outputs).toHaveLength(0)
})
it('cleans up native ownership on cancellation and callback failure',async()=>{
 const input=lower('cube(1);translate([2,0,0])cube(1);')
 for(const callbackFailure of [false,true]){
  const controller=new AbortController()
  await expect(executeBrepNativeProgram(input,{signal:controller.signal,onNode:()=>{
   if(callbackFailure)throw new Error('callback failed')
   controller.abort()
  }})).rejects.toMatchObject({code:callbackFailure?'E_SEMANTIC_CALLBACK':'E_SEMANTIC_ABORTED',node:0})
  const recovered=await executeBrepNativeProgram(input)
  expect(recovered.outputs).toHaveLength(2)
  await recovered.dispose()
 }
})
it('keeps budgets, deadlines and native failure locations',async()=>{
 const input=lower('cube(1);hull(){sphere(1);sphere(2);}')
 await expect(executeBrepNativeProgram(input,{maxNodes:0})).rejects.toMatchObject({code:'E_SEMANTIC_BUDGET'})
 await expect(executeBrepNativeProgram(input,{deadlineAt:1,now:()=>1})).rejects.toMatchObject({code:'E_SEMANTIC_DEADLINE'})
 await expect(executeBrepNativeProgram(input)).rejects.toMatchObject({code:'E_SEMANTIC_BACKEND_FAILURE',backendCause:{code:'E_BREP_SEMANTIC_UNSUPPORTED'}})
})
