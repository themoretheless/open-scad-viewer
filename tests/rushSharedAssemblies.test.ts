import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function evaluate(source:string) {
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  const measurement=scene.measurements!.at(-1)!
  return {report:JSON.parse(measurement.report),step:measurement.step!}
}
const source=(name:string)=>readFileSync('examples/rush/'+name+'.r','utf8')
it('reimports STEP exchange text and edits shared definitions separately from individual occurrences',async()=>{
  const imported=await evaluate(source('read-shared-step'))
  expect(imported.report.occurrences.map((o:{definition:number})=>o.definition)).toEqual([0,0])
  const external=await evaluate('// @rush/1\na = open_step_file(step_text:'+JSON.stringify(imported.step)+',preview_index:1)\nshow a.brep_tessellate(3)')
  expect(external.report.occurrenceCount).toBe(2)
  const shared=await evaluate(source('replace-step-part'))
  expect(shared.report.definitionCount).toBe(1)
  const variant=await evaluate(source('replace-step-instance'))
  expect(variant.report.definitionCount).toBe(2)
  expect(variant.report.occurrences[0].definition).not.toBe(variant.report.occurrences[1].definition)
  const placed=await evaluate(source('place-step-instance'))
  expect(placed.report.occurrences[0].placement[0][3]).toBe(7)
  await expect(evaluate(source('place-step-instance').replace('index:0,placement','index:2,placement'))).rejects.toThrow()
},180000)
it('preserves sharing and occurrence identities through add, remove, append, deduplicate and extract',async()=>{
  for (const [op,definitions,occurrences] of [
    ['remove-step-instance',1,1],['add-step-instance',1,3],['insert-step-solid',2,3],
    ['append-step-assembly',2,4],['deduplicate-step-parts',1,4],['extract-step-instances',1,1],
  ] as const){
    const {report,step}=await evaluate(source(op))
    expect(report.definitionCount).toBe(definitions)
    expect(report.occurrenceCount).toBe(occurrences)
    expect(step.match(/MANIFOLD_SOLID_BREP\(/g)).toHaveLength(definitions)
    expect(step.match(/NEXT_ASSEMBLY_USAGE_OCCURRENCE\(/g)).toHaveLength(occurrences)
  }
},180000)
