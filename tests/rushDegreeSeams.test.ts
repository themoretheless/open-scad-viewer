import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

async function join(name:string, transform=(s:string)=>s){
  const source=transform(readFileSync('examples/rush/'+name+'.r','utf8'))
  const scene=await buildTextNurbsScene(compileRushFrontend(source).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}

it('joins mixed degrees and removes a redundant quadratic seam',async()=>{
  expect(await join('join-mixed-degree')).toMatchObject({degree:2,domain:[-2,5]})
  expect(await join('compact-join-seam')).toMatchObject({seamMultiplicity:0,removalRefusal:null,knots:[2,2,2,5,5,5]})
})
it('distinguishes a speed change from a geometric corner',async()=>{
  expect(await join('compare-parametric-seam')).toMatchObject({order:0,residuals:[['0','0','0'],['-1','0','0'],['0','0','0']]})
  expect(await join('compare-geometric-seam')).toMatchObject({order:2,speedScale:'2',accelerationScale:'0'})
  expect(await join('compare-geometric-seam',s=>s.replace('[3mm,0,0]','[1mm,2mm,0]'))).toMatchObject({order:0})
})
