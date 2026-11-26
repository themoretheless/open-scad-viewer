import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function run(name:string){
  const source=readFileSync('examples/rush/'+name+'.r','utf8')
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  const measurement=scene.measurements!.at(-1)!
  expect(measurement.step).toContain('NEXT_ASSEMBLY_USAGE_OCCURRENCE')
  return JSON.parse(measurement.report)
}
it('preserves names and quantities across an exchange and selected parts export',async()=>{
  for(const name of ['name-step-parts','read-step-names','step-parts-list']){
    const r=await run(name)
    expect(r.assemblyName).toBe('Fixture')
    expect(r.parts.map((p:{name:string;quantity:number})=>[p.name,p.quantity])).toEqual([['Pin',2],['Mount',1]])
  }
  const r=await run('select-step-parts')
  expect(r.assemblyName).toBe('Pins')
  expect(r.definitionNames).toEqual(['Pin'])
  expect(r.occurrenceCount).toBe(2)
},180000)
it('tracks named variants, merged assemblies, deduplication and parts list revisions',async()=>{
  const v=await run('name-step-variant')
  expect(v.parts.find((p:{name:string})=>p.name==='Wide pin').quantity).toBe(1)
  expect((await run('append-named-parts')).occurrenceCount).toBe(6)
  const compact=await run('consolidate-named-parts')
  expect(compact.occurrenceCount).toBe(6)
  expect(compact.parts).toHaveLength(2)
  expect(compact.removedDefinitions).toBe(2)
  const changes=await run('compare-parts-lists')
  expect(changes.changes).toContainEqual(expect.objectContaining({name:'Pin',before:2,after:0}))
},180000)

it('reports stored and expanded NURBS faces and poles by shared definition',async()=>{
  const {surfaceUsage}=await run('step-parts-list')
  expect(surfaceUsage).toEqual([
    {name:'Pin',definition:0,quantity:2,storedFaces:6,placedFaces:12,storedPoles:24,placedPoles:48},
    {name:'Mount',definition:1,quantity:1,storedFaces:6,placedFaces:6,storedPoles:24,placedPoles:24},
  ])
},180000)
