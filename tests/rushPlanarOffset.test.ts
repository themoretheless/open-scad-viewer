import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source=(name:string)=>readFileSync('examples/rush/'+name+'.r','utf8')
async function inspect(text:string) {
  const scene=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements![0]!.report)
}

it('bounds the offset across the entire quadratic and checks the selected piece',async()=>{
  const text=source('bounded-planar-offset')
  const r=await inspect(text)
  expect(r).toMatchObject({accepted:true,wholeCurve:true})
  expect(r.cells.length).toBeGreaterThan(1)
  expect(r.errorUpperMm).toBeLessThanOrEqual(0.05)
  expect(r.pieces).toBeGreaterThan(0)
  await expect(inspect(text.replace('piece:0','piece:999'))).rejects.toThrow(/piece/i)
})

it('constructs all three signed miter vertices on a weighted nonunit domain',async()=>{
  const text=source('planar-knot-miter')
  for(const d of [0.25,-0.25]){
    const r=await inspect(text.replace('distance_mm:0.25','distance_mm:'+d))
    expect(r.errorUpperMm).toBeLessThanOrEqual(0.00001)
    const expected=[[10,d],[10-d,d],[10-d,0]]
    expect(r.points).toHaveLength(3)
    r.points.forEach((p:number[],i:number)=>expect(Math.hypot(p[0]!-expected[i]![0]!,p[1]!-expected[i]![1]!)).toBeLessThanOrEqual(r.errorUpperMm))
  }
  await expect(inspect(text.replace('knot:4','knot:3'))).rejects.toThrow(/knot/i)
})
