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

it('accounts for tool support before accepting a path inside a half-space',async()=>{
  for(const [name,edge] of [['point-workspace',0.75],['ball-workspace',0.70],['capsule-workspace',0.65],['hull-workspace',0.65]] as const){
    const r=await join(name)
    const at=(t:number)=>r.cells.find((c:{domain:number[]})=>c.domain[0]<=t && t<=c.domain[1])
    expect(at(edge-0.02).status).toBe('Within')
    expect(at(edge+0.02).status).toBe('Exceeds')
    expect(r.workUsed).toBeLessThanOrEqual(1000)
  }
})
it('detects intrusion and clearance for a point, a ball and a rounded hull',async()=>{
  for(const name of ['point-obstacles','ball-obstacles','hull-obstacles']){
    const r=await join(name)
    const at=(t:number)=>r.cells.find((c:{domain:number[]})=>c.domain[0]<=t && t<=c.domain[1])
    expect(at(0.1).obstacles).toEqual(['Within'])
    expect(at(0.5).obstacles).toEqual(['Exceeds'])
    expect(at(0.9).status).toBe('Within')
  }
})
