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

it('checks a counterclockwise sector and relative derivatives against a moving reference',async()=>{
  for(const name of ['orientation-sector','relative-orientation-sector']){
    const r=await join(name)
    expect(r.cells[0].status).toBe('Within')
    expect(r.cells.at(-1).status).toBe('Exceeds')
  }
  const r=await join('relative-angular-limits')
  expect(r.cells[0].statuses).toEqual(['Exceeds','Within','Within'])
  expect(r.cells.at(-1).statuses[0]).toBe('Within')
  await expect(join('orientation-sector',s=>s.replace('[[1,-1],[1,0.5]]','[[1,0],[1,0]]'))).rejects.toThrow()
})
it('encloses signed and absolute travel in radians in fixed and counter-rotating frames',async()=>{
  for(const [name,angle] of [['angular-travel',Math.PI/4],['relative-angular-travel',Math.PI/2]] as const){
    const r=await join(name)
    for(const bounds of [r.net,r.total]){
      expect(bounds[0]).toBeLessThanOrEqual(angle)
      expect(bounds[1]).toBeGreaterThanOrEqual(angle)
      expect(bounds[1]-bounds[0]).toBeLessThan(0.05)
    }
  }
})
