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

it('finds the rotated simple eigenvector and withholds a repeated axis',async()=>{
  const r=await join('find-principal-axis')
  const [x,y,z]=r.direction
  expect(Math.abs(x-y)/Math.sqrt(2*(x*x+y*y+z*z))).toBeCloseTo(1,12)
  expect(r.sineUpper).toBeLessThan(1e-8)
  const repeated=await join('find-principal-axis',s=>s.replaceAll('[1, 1]','[0, 0]'))
  expect(repeated.direction).toBeNull()
})
it('bounds the sign-independent rank-one projector and withholds an ambiguous projector',async()=>{
  const r=await join('bound-axis-projector')
  const expected=[[0.5,-0.5,0],[-0.5,0.5,0],[0,0,0]]
  expected.forEach((row,i)=>row.forEach((v,j)=>{
    const [lo,hi]=r.projectorBounds[i][j]
    expect(lo).toBeLessThanOrEqual(v)
    expect(hi).toBeGreaterThanOrEqual(v)
    expect(hi-lo).toBeLessThan(1e-8)
  }))
  expect((await join('bound-axis-projector',s=>s.replaceAll('[1, 1]','[0, 0]'))).projectorBounds).toBeNull()
})
