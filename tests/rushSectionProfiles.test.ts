import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const source=(name:string)=>readFileSync('examples/rush/'+name+'.r','utf8')
async function inspect(text:string){
  const s=await buildTextNurbsScene(compileRushFrontend(text).document)
  expect(s.meshes).toHaveLength(1)
  return JSON.parse(s.measurements![0]!.report)
}
it('constructs named NACA and asymmetric CST splines and rejects an unsupported NACA mean line',async()=>{
  const naca=source('naca-five-digit')
  expect(await inspect(naca)).toMatchObject({profileCode:23012,chordMm:2,fitted:true})
  expect(await inspect(source('cst-spline-section'))).toMatchObject({degree:3,controlCount:24,chordMm:2,fitted:true})
  await expect(inspect(naca.replace('23012','99999'))).rejects.toThrow()
})
it('encloses independent integrals of a symmetric parabolic section at a two millimetre chord',async()=>{
  const text=source('cst-section-integrals')
  const r=await inspect(text)
  const encloses=(interval:number[],expected:number)=>{
    expect(interval[0]).toBeLessThanOrEqual(expected)
    expect(interval[1]).toBeGreaterThanOrEqual(expected)
    expect(interval[1]!-interval[0]!).toBeLessThan(1e-8)
  }
  encloses(r.areaMm2,1/3)
  encloses(r.centroidMm[0],1)
  encloses(r.centroidMm[1],0)
  encloses(r.momentsAtCentroidMm4[0],1/840)
  encloses(r.momentsAtCentroidMm4[1],1/15)
  encloses(r.momentsAtCentroidMm4[2],0)
  await expect(inspect(text.replace('max_work:1000000','max_work:1'))).rejects.toThrow()
})
