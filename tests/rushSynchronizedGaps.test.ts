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

function encloses(bounds:number[],value:number) {
  expect(bounds[0]).toBeLessThanOrEqual(value)
  expect(bounds[1]).toBeGreaterThanOrEqual(value)
  expect(bounds[1]!-bounds[0]!).toBeLessThan(0.02)
}
it('encloses the signed minimum and returns its shared parameter or normalized progress',async()=>{
  for(const name of ['minimum-synchronized-gap','minimum-affine-gap']){
    const r=await join(name)
    encloses(r.gap,-0.15)
    expect(r.witnessParameter??r.witnessProgress).toBeCloseTo(0.5)
    expect(r.stopReason).toBe('Tolerance')
    expect(r.workUsed).toBeLessThanOrEqual(1000)
  }
  await expect(join('minimum-affine-gap',s=>s.replace('[0.25,0.75]','[0.25,2]'))).rejects.toThrow()
})
it('classifies contact and separation on shared and independently traversed parameter windows',async()=>{
  for(const name of ['synchronized-contact-intervals','affine-contact-intervals']){
    const r=await join(name)
    const key=name.startsWith('affine')?'progress':'domain'
    expect(r.cells[0][key][0]).toBe(0)
    expect(r.cells.at(-1)[key][1]).toBe(1)
    expect(r.cells[0].status).toBe('Within')
    const center=r.cells.find((c:Record<string,[number,number]>)=>c[key]![0]<=0.5 && c[key]![1]>=0.5)
    expect(center.status).toBe('Exceeds')
    if(key==='domain')expect(center.spans).toHaveLength(2)
  }
})
