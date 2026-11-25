import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const program=readFileSync('examples/rush/closed-section-phase.r','utf8')
const build=async(code:string)=>buildTextNurbsScene(compileRushFrontend(code).document)

it('moves the seams of two original sections to the corresponding landmark witnesses',async()=>{
  const scene=await build(program)
  expect(scene.meshes).toHaveLength(1)
  const {phases}=JSON.parse(scene.measurements![0]!.report)
  expect(phases).toHaveLength(2)
  for(const phase of phases){
    expect(phase.status).toBe('Aligned')
    expect(phase.parameter).toBeCloseTo(1.5,8)
    expect(phase.shapeErrorUpperMm).toBeLessThanOrEqual(0.1)
    expect(phase.finalDistanceMm[0]).toBeLessThanOrEqual(1)
    expect(phase.finalDistanceMm[1]).toBeGreaterThanOrEqual(1)
  }
})

it('rejects a missing landmark and withholds the whole result on an unresolved shape',async()=>{
  await expect(build(program.replace(',[2mm,1mm,3mm]',''))).rejects.toThrow(/array length/i)
  await expect(build(program.replace('0.1mm','0.000000000000000000000000000001mm').replace('max_cells:10000','max_cells:5'))).rejects.toThrow(/unresolved/i)
})
