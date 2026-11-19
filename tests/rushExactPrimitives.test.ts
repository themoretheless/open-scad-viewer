import { expect, it } from 'vitest'
import { compileRushFrontend } from '../src/language/rushFrontend'
import { buildOwnNurbsAsync } from '../src/language/rushGraphNurbsKernel'

const axes = '[[1,0,0],[0,1,0],[0,0,1]]'
const shapes = [
  ['exact_ellipse', 'axis_u:[2mm,0,0],axis_v:[0,1mm,0]'],
  ['exact_ellipsoid','axes:[[2mm,0,0],[0,3mm,0],[0,0,4mm]]'],
  ['exact_ring_torus',`axes:${axes},major_radius:3mm,minor_radius:1mm`],
  ['exact_frustum_side','axes:[[1,0,0],[0,1,0],[0,0,4mm]],radii:[2mm,1mm]'],
  ['exact_planar_region','axes:[[1,0,0],[0,1,0]],radii:[1mm,3mm]'],
]
it.each(shapes)('builds %s only when the exact Rust constructor admits the source', async (name,args) => {
  const curve = name === 'exact_ellipse'
  const program = `// @rush/1\ns = ${name}(center:[0,0,0],${args},max_work:1000000)\nshow s${curve?'.surface_extrude(vector:[0,0,2mm])':''}.tessellate(8,8)`
  const {document} = compileRushFrontend(program)
  const node = document.nodes[0]!.id
  const sample = {node,u:0.31,...(curve?{}:{v:0.43})}
  const result = await buildOwnNurbsAsync(document,{action:'evaluate',evaluations:[sample]})
  const [x,y,z] = result.evaluations[0]!.point as [number,number,number]
  switch (name) {
    case 'exact_ellipse': expect((x/2)**2+y*y).toBeCloseTo(1,11); expect(z).toBe(0); break
    case 'exact_ellipsoid': expect((x/2)**2+(y/3)**2+(z/4)**2).toBeCloseTo(1,11); break
    case 'exact_ring_torus': expect((Math.hypot(x,y)-3)**2+z*z).toBeCloseTo(1,11); break
    case 'exact_frustum_side': expect(Math.hypot(x,y)).toBeCloseTo(2-z/4,11); break
    case 'exact_planar_region': expect(Math.hypot(x,y)).toBeCloseTo(1+2*0.43,11); expect(z).toBe(0); break
  }
  expect((await buildOwnNurbsAsync(document,{action:'build'})).report.mesh!.triangleCount).toBeGreaterThan(0)
  const refused = compileRushFrontend(program.replace('max_work:1000000','max_work:1')).document
  await expect(buildOwnNurbsAsync(refused,{action:'build'})).rejects.toThrow(/resource_limit/)
})
