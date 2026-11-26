import {readFileSync} from 'node:fs'
import {expect,it} from 'vitest'
import {compileRushFrontend} from '../src/language/rushFrontend'
import {buildTextNurbsScene} from '../src/language/rushFrontendScene'
async function run(name:string,transform=(s:string)=>s){
  const scene=await buildTextNurbsScene(compileRushFrontend(transform(readFileSync('examples/rush/'+name+'.r','utf8'))).document)
  expect(scene.meshes).toHaveLength(1)
  return JSON.parse(scene.measurements!.at(-1)!.report)
}
it('encloses patch radii and reconstructs oriented coedges on an affine cap',async()=>{
  const r=await run('piecewise-axis-radius')
  expect(r.radiusBoundsMm[0]).toBeLessThanOrEqual(0)
  expect(r.radiusBoundsMm[1]).toBeGreaterThanOrEqual(Math.sqrt(8))
  const trim=await run('cap-edge-pcurve')
  expect(trim.pcurve.controlPoints).toHaveLength(2)
  expect(trim.pcurve.weights).toEqual([1,1])
  const wire=await run('transfer-cap-loop')
  expect(wire.coedges).toHaveLength(4)
  expect(new Set(wire.coedges.map((e:{edge:number})=>e.edge)).size).toBe(4)
},180000)
it('normalizes radius units and keeps the enlarged bore sewn to the tooth shell',async()=>{
  for(const name of ['replace-bore-wall','cut-bore-cylinder']){
    const source=readFileSync('examples/rush/'+name+'.r','utf8')
    const operation=name.replaceAll('-','_')
    const compile=(text:string)=>compileRushFrontend(text).document.nodes.find(node=>node.op===operation)
    const metric=compile(source)
    expect(metric).toMatchObject({radius_mm:22,height_mm:10})
    expect(compile(source.replace('radius_mm:22mm','radius_mm:2.2cm'))).toMatchObject({radius_mm:22,height_mm:10})
  }
  expect((await run('bore-wall-faces')).wallFaces).toHaveLength(32)
  for(const name of ['replace-bore-wall','cut-bore-cylinder']){
    const r=await run(name)
    expect(r.bodyCount).toBe(1)
    expect(r.boundaryEdges).toBe(0)
    expect(r.faceCount).toBe(r.sourceFaceCount-32+4)
  }
  await expect(run('replace-bore-wall',s=>s.replace('radius_mm:22mm','radius_mm:24mm'))).rejects.toThrow()
},240000)
