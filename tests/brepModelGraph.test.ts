import {expect,it} from 'vitest'
import {parseOpenSCAD} from '../src/services/openscadParser'
import {buildOwnNurbs} from '../src/services/modelGraphNurbsKernel'
import {compileModelGraphNurbs,modelGraphNurbsSchema} from '../src/services/modelGraphNurbs'
import {readFileSync} from 'node:fs'
import {z} from 'zod/v4'

it('keeps the public B-rep schema identical for Rust and MCP clients',()=>{
 expect(JSON.parse(readFileSync('docs/languages/modelgraph-nurbs-1.schema.json','utf8'))).toEqual(z.toJSONSchema(modelGraphNurbsSchema))
 const document={language:'modelgraph/nurbs-1',units:'mm',parameters:[{id:'r',value:3}],nodes:[{id:'body',op:'brep_tube',outer_radius:{param:'r'},inner_radius:1,height:5},{id:'display',op:'brep_tessellate',input:'body',segments:8}],root:'display'}
 expect(compileModelGraphNurbs(document).resolved_document.nodes[0]).toMatchObject({outer_radius:3})
 const result=buildOwnNurbs(document,{action:'build'})
 expect(result).toBeDefined()
 expect(()=>compileModelGraphNurbs({...document,nodes:[{id:'body',op:'brep_frustum',bottom_radius:3,height:5}],root:'body'})).toThrow()
})

it.each([
 'show polygon_profile([[0mm,0mm],[3mm,0mm],[3mm,4mm],[0mm,4mm]]).brep_revolve(137deg).brep_tessellate(8)',
 'show brep_sphere(3mm).transform([[-1,0,0,10],[0,2,0,0],[0,0,1,0],[0,0,0,1]]).brep_tessellate(8)',
 'show brep_sphere(3mm).brep_tessellate(8)',
 'show brep_torus(8mm,2mm).brep_tessellate(8)',
 'show brep_frustum(3mm,0mm,5mm).brep_tessellate(8)',
 'show brep_cylinder(3mm,5mm).brep_tessellate(8)',
 'show brep_frustum(3mm,1mm,5mm).brep_tessellate(8)',
 'show brep_tube(3mm,1mm,5mm).brep_tessellate(8)',
 'show polygon_profile([[1mm,0mm],[3mm,0mm],[3mm,2mm],[2mm,2mm],[2mm,5mm],[1mm,5mm]]).brep_revolve().brep_tessellate(8)',
 'show polygon_profile([[0mm,0mm],[5mm,0mm],[5mm,2mm],[3mm,2mm],[3mm,5mm],[0mm,5mm]],[[[1mm,1mm],[1mm,2mm],[2mm,2mm],[2mm,1mm]]]).brep_extrude(2mm).brep_tessellate(2)',
 'a=brep_box([0mm,0mm,0mm],[3mm,3mm,3mm])\nb=brep_box([1mm,1mm,1mm],[4mm,4mm,4mm])\nshow a.brep_subtract(b).brep_tessellate(2)',
 'show brep_box([0mm,0mm,0mm],[3mm,3mm,3mm]).brep_chamfer([0],0.2mm).brep_tessellate(2)',
 'show brep_box([0mm,0mm,0mm],[3mm,3mm,3mm]).brep_fillet([0],0.2mm,8).brep_tessellate(2)',
])('runs authored B-rep through the text compiler: %s',async source=>{
 const scene=await parseOpenSCAD('// @modelgraph-text/1\n'+source)
 expect(scene.meshes).toHaveLength(1)
 expect(scene.meshes[0].faceIdsAuthoritative).toBe(true)
 expect(scene.meshes[0].indices.length).toBeGreaterThan(0)
})
