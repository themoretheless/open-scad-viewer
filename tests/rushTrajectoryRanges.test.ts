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

it('covers every parameter and preserves unresolved boundaries under refinement',async()=>{
  for(const name of ['partition-axis-position','partition-axis-positions']){
    const r=await join(name)
    expect(r.workUsed).toBeLessThanOrEqual(256)
    let end=0
    const states=new Set<string>()
    for(const c of r.cells){
      expect(c.domain[0]).toBe(end)
      end=c.domain[1]
      states.add(c.axes?c.axes[0].status:c.status)
      if(c.axes)expect(c.axes[1].status).toBe('Within')
    }
    expect(end).toBe(1)
    expect(states).toEqual(new Set(['Within','Exceeds','Undetermined']))
  }
})
it('classifies speed for each axis independently of the position range',async()=>{
  const r=await join('partition-trajectory-bounds')
  expect(r.cells.length).toBeGreaterThan(0)
  for(const c of r.cells){
    expect(c.axes[0].motion).toEqual(['Exceeds','Within','Within'])
    expect(c.axes[1].motion).toEqual(['Within','Within','Within'])
  }
  await expect(join('partition-trajectory-bounds',s=>s.replace('max_cells:256','max_cells:1'))).rejects.toThrow()
})
