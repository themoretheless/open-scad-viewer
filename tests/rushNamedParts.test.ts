import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
import {readNativeGeometryData} from '../src/kernel/nativeGeometrySnapshot'
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

it('compresses a face on a shared definition and preserves both placed instances',async()=>{
  const text=readFileSync('tests/fixtures/rush-step/refined-shared-definition.step','utf8')
  const head='// @rush/1\nassembly = open_step_file(step_text:'+JSON.stringify(text)+',preview_index:0)\n'
  const query=async (lines:string, preview:string)=>{
    const scene=await buildTextNurbsScene(compileRushFrontend(head+lines+'\nshow '+preview+'.brep_tessellate(3)\n').document)
    return scene
  }
  const before=await query('','assembly')
  const model=readNativeGeometryData(before.meshes[0]!.nativeGeometry!).geometry
  const face=model.faces.findIndex(face=>face.surface.controlPoints.length===3)
  expect(face).toBeGreaterThanOrEqual(0)
  const edit=(budget:number)=>'edited = assembly.compress_step_definition(definition:0,faces:['+face+'],max_work:'+budget+',preview_index:0)'
  const refused=(await query(edit(1),'edited')).measurements!.at(-1)!
  expect(JSON.parse(refused.report).accepted).toBe(false)
  expect(refused.step).toBe(text)
  const updated=(await query(edit(1000000),'edited')).measurements!.at(-1)!
  expect(JSON.parse(updated.report)).toMatchObject({accepted:true,assemblyName:'Mount',occurrenceCount:3})
  expect(updated.step!.length).toBeLessThan(text.length)
  const usage=await query(edit(1000000)+'\nparts = edited.step_parts_list(preview_index:2)','parts')
  expect(JSON.parse(usage.measurements!.at(-1)!.report).surfaceUsage[0]).toMatchObject({
    name:'Bracket',quantity:2,storedPoles:24,placedPoles:48,
  })
  const rotated=readNativeGeometryData(usage.meshes[0]!.nativeGeometry!).geometry
  expect(Math.min(...rotated.vertices.map(vertex=>vertex.point[0]))).toBeCloseTo(36)
  expect(Math.max(...rotated.vertices.map(vertex=>vertex.point[1]))).toBeCloseTo(8)
},180000)

it('compresses individual B-rep carriers without detaching either edge use',async()=>{
  const text=readFileSync('tests/fixtures/rush-step/refined-edge.step','utf8')
  const head='// @rush/1\nsource = open_step_file(step_text:'+JSON.stringify(text)+',preview_index:0)\n'
  const read=async (script:string, output:string)=>{
    const doc=compileRushFrontend(head+script+'\nshow '+output+'.brep_tessellate(3)\n').document
    const scene=await buildTextNurbsScene(doc)
    return readNativeGeometryData(scene.meshes[0]!.nativeGeometry!).geometry
  }
  const before=await read('','source')
  const face=before.faces.findIndex(face=>face.surface.controlPoints.length===3)
  const edge=before.edges.findIndex(edge=>edge.curve.controlPoints.length===3)
  expect(face).toBeGreaterThanOrEqual(0)
  expect(edge).toBeGreaterThanOrEqual(0)
  const after=await read('face = source.compress_brep_faces(faces:['+face+'],max_work:1000000)\nresult = face.compress_brep_edge(edge_index:'+edge+',max_work:1000000)','result')
  expect(after.faces[face]!.surface.controlPoints).toHaveLength(2)
  expect(after.edges[edge]!.curve.controlPoints).toHaveLength(2)
  const uses=after.loops.flatMap(loop=>loop.coedges).filter(coedge=>coedge.edge===edge)
  expect(uses.map(use=>use.reversed).sort()).toEqual([false,true])
  expect(uses.every(use=>use.pcurve.controlPoints.length===2)).toBe(true)
  expect(after.vertices).toEqual(before.vertices)
  expect(after.shells).toEqual(before.shells)
},180000)
