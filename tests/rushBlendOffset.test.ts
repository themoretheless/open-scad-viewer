import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function run(example:string,replace=(s:string)=>s){
  const source=replace(readFileSync('examples/rush/'+example+'.r','utf8'))
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  return {report:JSON.parse(scene.measurements!.at(-1)!.report),positions:Array.from(scene.meshes[0]!.vertices).filter((_,index)=>index%6<3)}
}

it('retains both support boundaries for G1 and G2 Hermite blends',async()=>{
  for(const [kind,order] of [['G1','C1'],['G2','C2']] as const){
    const {report,positions}=await run('surface-boundary-blend',s=>s.replace('continuity_class:"G2"','continuity_class:"'+kind+'"'))
    expect(report).toMatchObject({patchCount:1,realArithmeticOrder:order,roundingCertified:false})
    const points=positions.filter((_,i)=>i%3===0).map((_,i)=>positions.slice(i*3,i*3+3))
    for(const expected of [[0,1,0],[2,1,0],[0,2,1],[2,2,1]]){
      expect(points.some(point=>point.every((value,i)=>Math.abs(value-expected[i]!)<1e-8))).toBe(true)
    }
  }
},180000)

it('reports sampled assurance for a nonzero offset and identity for zero distance',async()=>{
  const moved=await run('offset-nurbs-panel',s=>s.replace('distance:2mm','distance:0.2cm'))
  expect(moved.report).toMatchObject({assurance:'sampled_deviation',degenerateRegions:[],exceedsFoldFreeBound:false})
  expect(moved.positions.filter((_,i)=>i%3===2).every(z=>Math.abs(z-2)<1e-8)).toBe(true)
  const same=await run('offset-nurbs-panel',s=>s.replace('distance:2mm','distance:0mm'))
  expect(same.report).toMatchObject({assurance:'exact_identity',maxSampledDeviationMm:0})
  expect(same.positions.filter((_,i)=>i%3===2).every(z=>z===0)).toBe(true)
},180000)
