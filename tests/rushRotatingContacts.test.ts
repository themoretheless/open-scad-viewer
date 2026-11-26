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

it('certifies contact at faces, corners and interior chord points in seven geometric fixtures',async()=>{
  for(const kind of ['face','corner','asymmetric','skew','chord','clipped-chord','exact-chord']){
    const r=await join('rotating-'+kind+'-obstacles')
    expect(r.cells.length).toBeGreaterThan(0)
    expect(r.cells.every((c:{obstacles:string[]})=>c.obstacles[0]==='Exceeds')).toBe(true)
    expect(r.workUsed).toBeLessThanOrEqual(1000)
  }
})
it('separates exact chord tangency from contradictory half-spaces and enforces the work budget',async()=>{
  const name='rotating-exact-chord-obstacles'
  const r=await join(name,s=>s.replace('[[[1,0,1],[-1,0,-1]]]','[[[1,0,1],[-1,0,-1]],[[1,0,1],[-1,0,-1.0000000000000002]]]'))
  expect(r.cells.every((c:{obstacles:string[]})=>c.obstacles[0]==='Exceeds' && c.obstacles[1]!=='Exceeds')).toBe(true)
  await expect(join(name,s=>s.replace('max_cells:1000','max_cells:1'))).rejects.toThrow()
})
