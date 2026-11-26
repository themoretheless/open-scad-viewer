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

it('separates collision and clearance for all four obstacle shapes',async()=>{
  for(const name of ['sphere-path-clearance','capsule-path-clearance','box-path-clearance','oriented-box-path-clearance']){
    const r=await join(name)
    const at=(t:number)=>r.cells.find((c:{domain:number[]})=>c.domain[0]<=t && t<=c.domain[1])
    expect(at(0.2).status).toBe('Within')
    expect(at(0.49).status).toBe('Exceeds')
    expect(at(0.8).status).toBe('Within')
    expect(at(0.49).gaps[0][1]).toBeLessThanOrEqual(0)
  }
})
it('bounds the first sphere contact and retains the minimum-gap witness',async()=>{
  const {contact}=await join('first-sphere-contact')
  expect(contact).toMatchObject({kind:'Confirmed',obstacle:0})
  expect(contact.domain[0]).toBeLessThanOrEqual(0.35)
  expect(contact.domain[1]).toBeGreaterThanOrEqual(0.35)
  const {minimum}=await join('minimum-sphere-partition-gap')
  expect(minimum.gap[0]).toBeLessThanOrEqual(-0.15)
  expect(minimum.gap[1]).toBeGreaterThanOrEqual(-0.15)
  expect(minimum.witness[1]).toBe(0)
  expect(minimum.domains.some(([a,b]:number[])=>a!<=0.5 && 0.5<=b!)).toBe(true)
})
