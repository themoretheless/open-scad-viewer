import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
const program=readFileSync('examples/rush/parallel-plane-sections.r','utf8')
const build=(code:string)=>buildTextNurbsScene(compileRushFrontend(code).document)

it('distinguishes empty levels from complete section curves and selects an original-domain piece',async()=>{
  const scene=await build(program)
  expect(scene.meshes).toHaveLength(1)
  const report=JSON.parse(scene.measurements![0]!.report)
  expect(report.complete).toBe(true)
  expect(report.sections.map((s:{status:string})=>s.status)).toEqual(['Approximate','Approximate','Empty'])
  for(const section of report.sections.slice(0,2)){
    expect(section.errorUpperMm).toBeLessThanOrEqual(0.05)
    expect(section.pieces).toBeGreaterThan(0)
  }
  await expect(build(program.replace('section_index:1','section_index:2'))).rejects.toThrow(/Empty/)
})
it('reports the unresolved domain when shared work is exhausted',async()=>{
  const query=program.slice(0,program.indexOf('section ='))+'show levels.tessellate(8,8)'
  const scene=await build(query.replace('max_work:100000','max_work:1'))
  const report=JSON.parse(scene.measurements![0]!.report)
  expect(report.complete).toBe(false)
  expect(report.work).toBeLessThanOrEqual(1)
  expect(report.sections.some((s:{unresolvedDomain:unknown})=>s.unresolvedDomain!==null)).toBe(true)
})
