import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const initial=()=>readFileSync('examples/rush/style-step-occurrences.r','utf8').split('\nshow ')[0]!
async function run(operation:string,args:string,preview=0){
  const source=initial()+'\nresult = styled.'+operation+'('+args+',preview_index:'+preview+')\nshow result.brep_tessellate(3)'
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  const result=scene.measurements!.find(row=>row.operation===operation)!
  return {report:JSON.parse(result.report),step:result.step}
}
const branch='product_path:["Fixture","Left"]'

it('reports counts, bounds and density-weighted mass by STEP product path',async()=>{
  const {report:parts}=await run('step_branch_parts',branch)
  expect(parts.parts).toEqual([{name:'Pin',definition:0,inside:1,outside:2}])
  expect(parts.paths).toContainEqual(['Fixture','Left','Pin'])
  const {report:bounds}=await run('step_branch_bounds',branch)
  expect(bounds.boundsMm.flat()).toEqual([0,0,0,2,2,2].map(value=>expect.closeTo(value,8)))
  const {report:mass}=await run('step_branch_mass',branch+',densities:[2],relative_tolerance:0.000001,max_evaluations:100000')
  expect(mass.massKg).toBeCloseTo(16)
  expect(mass.centroidMm).toEqual([1,1,1].map(value=>expect.closeTo(value,8)))
  expect(mass.inertiaKgMm2[0][0]).toBeCloseTo(32/3)
},180000)

it('exports, replaces or moves the selected named branch and retains occurrence colours',async()=>{
  const exported=await run('export_step_branch',branch)
  expect(exported.report.paths).toEqual([['Left','Pin']])
  expect(exported.report.colours).toEqual([[0,0,1]])
  expect(exported.step).toContain('NEXT_ASSEMBLY_USAGE_OCCURRENCE(')
  const moved=await run('move_step_branch',branch+',placement:[[1,0,0,5],[0,1,0,0],[0,0,1,0],[0,0,0,1]]')
  const left=moved.report.paths.findIndex((path:string[])=>path[1]==='Left')
  expect(moved.report.placements[left][0][3]).toBeCloseTo(5)
  expect(moved.report.colours[left]).toEqual([0,0,1])
  const replaced=await run('replace_step_branch_leaf',branch.replace('"Left"]','"Left","Pin"]')+',target_geometry:box,product_name:"Updated"')
  expect(replaced.report.paths).toContainEqual(['Fixture','Left','Updated'])
  expect(replaced.report.paths).toContainEqual(['Fixture','Right','Pin'])
},180000)
