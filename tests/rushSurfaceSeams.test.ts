import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'

const source=readFileSync('examples/rush/surface-seam-continuity.r','utf8')
async function inspect(code:string) {
  const scene=await buildTextNurbsScene(compileRushFrontend(code).document)
  expect(scene.meshes).toHaveLength(1)
  return new Map(scene.measurements!.map(m=>[m.operation,JSON.parse(m.report)]))
}

it('qualifies adjacent planes and retains the inferred G2 correspondence and signed branches',async()=>{
  const r=await inspect(source)
  expect(r.get('surface_seam_g0').decision).toBe('WithinTolerance')
  expect(r.get('surface_seam_g1')).toMatchObject({decision:'WithinTolerance',regularityProven:true})
  const g2=r.get('surface_seam_g2')
  expect(g2.candidateMonotone).toBe(true)
  expect(g2.nodes[0]).toEqual([0,0])
  expect(g2.nodes.at(-1)).toEqual([1,1])
  expect(g2.verification).toMatchObject({decision:'WithinTolerance',regularityProven:true})
  expect(g2.verification.branches).toHaveLength(2)
})

it('finds a tangent-plane break across a positional join, and a gap on the wrong boundary',async()=>{
  const tilted=await inspect(source.replace('end:[1mm,0mm,0mm]', 'end:[1mm,0mm,1mm]'))
  expect(tilted.get('surface_seam_g0').decision).toBe('WithinTolerance')
  expect(tilted.get('surface_seam_g1').decision).toBe('CorrespondenceExceedsTolerance')
  const gap=await inspect(source.replaceAll('target_edge:"u_min"','target_edge:"u_max"'))
  expect(gap.get('surface_seam_g0').decision).toBe('CorrespondenceExceedsTolerance')
})
