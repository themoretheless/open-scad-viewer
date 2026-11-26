import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function run(name:string){
  const scene=await buildTextNurbsScene(compileRushFrontend(readFileSync('examples/rush/'+name+'.r','utf8')).document)
  const measurement=scene.measurements!.at(-1)!
  expect(measurement.step!.match(/=MANIFOLD_SOLID_BREP\(/g)).toHaveLength(1)
  return JSON.parse(measurement.report)
}
it('shares the same solid across two groups and the assembly root',async()=>{
  const r=await run('group-step-parts')
  expect(r.definitionCount).toBe(1)
  expect(r.paths).toHaveLength(3)
  expect(r.paths.filter((p:string[])=>p.includes('Left'))).toHaveLength(1)
  expect(r.paths.filter((p:string[])=>p.includes('Right'))).toHaveLength(1)
},180000)
it('recovers occurrence colours by placement for instances of one shared solid',async()=>{
  for(const name of ['style-step-occurrences','read-occurrence-styles']){
    const r=await run(name)
    const placements=r.origins.map((p:number[],i:number)=>[p[0],r.colours[i]]).sort((a:[number,unknown],b:[number,unknown])=>a[0]-b[0])
    expect(placements).toEqual([[0,[0,0,1]],[1,[1,0,0]],[2,null]])
  }
},180000)
