import {expect,it} from 'vitest'
import {parseOpenSCAD} from '../src/language/openscadParser'
import {buildOwnNurbs} from '../src/language/rushGraphNurbsKernel'
import {compileRushGraphNurbs,rushGraphNurbsSchema} from '../src/language/rushGraphNurbs'
import {readFileSync} from 'node:fs'
import {z} from 'zod/v4'
import {createBrepBox,extrudeBrepPolygon} from '../src/kernel/geometry/brep'
import {analyzeBrepSelection,selectBrepEdges,selectBrepFaces} from '../src/language/rushGraphNurbsKernel/selectBrepEdges'

it('keeps the public B-rep schema identical for Rust and MCP clients',()=>{
 expect(JSON.parse(readFileSync('docs/languages/rush-nurbs-1.schema.json','utf8'))).toEqual(z.toJSONSchema(rushGraphNurbsSchema))
 const document={language:'rush/nurbs-1',units:'mm',parameters:[{id:'r',value:3}],nodes:[{id:'body',op:'brep_tube',outer_radius:{param:'r'},inner_radius:1,height:5},{id:'display',op:'brep_tessellate',input:'body',segments:8}],root:'display'}
 expect(compileRushGraphNurbs(document).resolved_document.nodes[0]).toMatchObject({outer_radius:3})
 const result=buildOwnNurbs(document,{action:'build'})
 expect(result).toBeDefined()
 expect(()=>compileRushGraphNurbs({...document,nodes:[{id:'body',op:'brep_frustum',bottom_radius:3,height:5}],root:'body'})).toThrow()
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
 'show brep_box([0mm,0mm,0mm],[3mm,3mm,3mm]).brep_chamfer({face:{normal:[0,0,1]}},0.2mm).brep_tessellate(2)',
 'show brep_box([0mm,0mm,0mm],[3mm,3mm,3mm]).brep_fillet({face:{normal:[0,0,-1]},convexity:"convex",min_length:1mm},0.2mm,8).brep_tessellate(2)',
])('runs authored B-rep through the text compiler: %s',async source=>{
 const scene=await parseOpenSCAD('// @rush/1\n'+source)
 expect(scene.meshes).toHaveLength(1)
 expect(scene.meshes[0].faceIdsAuthoritative).toBe(true)
 expect(scene.meshes[0].indices.length).toBeGreaterThan(0)
})

it('answers edge and face queries on an exact box',()=>{
 const box=createBrepBox([0,0,0],[4,3,2])
 const facts=analyzeBrepSelection(box)
 expect(facts.edges).toHaveLength(12)
 expect(facts.edges.every(e=>e.convexity==='convex'&&e.faces.length===2)).toBe(true)
 expect(selectBrepEdges(box,{direction:[0,0,1]}).map(i=>facts.edges[i]!.length)).toEqual([2,2,2,2].map(()=>expect.closeTo(2,9)))
 expect(selectBrepEdges(box,{min_length:3.5})).toHaveLength(4)
 const top=selectBrepFaces(box,{normal:[0,0,1]})
 expect(top).toHaveLength(1)
 expect(facts.faces[top[0]!]!.area).toBeCloseTo(12,9)
 expect(selectBrepFaces(box,{type:'planar',min_area:7})).toHaveLength(4)
 const topEdges=selectBrepEdges(box,{face:{normal:[0,0,1]}})
 expect(topEdges).toHaveLength(4)
 expect(topEdges.every(i=>facts.edges[i]!.faces.includes(top[0]!))).toBe(true)
 expect(selectBrepEdges(box,[3,5])).toEqual([3,5])
 expect(()=>selectBrepEdges(box,{min_length:10})).toThrow('matched no')
})

it('delivers native exact Full Round to rush/nurbs-1 text and JSON as a display mesh',async()=>{
 const scene=await parseOpenSCAD('// @rush/1\nshow brep_box([0mm,0mm,0mm],[4mm,8mm,6mm]).brep_full_round_exact([5,4,3],4)')
 expect(scene.meshes).toHaveLength(1)
 expect(scene.meshes[0].indices.length).toBeGreaterThan(0)
 const document={language:'rush/nurbs-1',units:'mm',parameters:[],nodes:[{id:'box',op:'brep_box',min:[0,0,0],max:[4,8,6]},{id:'round',op:'brep_full_round_exact',input:'box',faces:[5,4,3]}],root:'round'}
 expect(compileRushGraphNurbs(document).resolved_document.nodes[1]).toMatchObject({faces:[5,4,3],segments:4})
 expect(buildOwnNurbs(document,{action:'build'})).toBeDefined()
 expect(()=>compileRushGraphNurbs({...document,nodes:[document.nodes[0],{...document.nodes[1],faces:[5,4]}]})).toThrow()
 expect(()=>buildOwnNurbs({...document,nodes:[document.nodes[0],{...document.nodes[1],faces:[5,5,3]}]},{action:'build'})).toThrow()
},60000)

it('classifies concave edges of an L-shaped prism',()=>{
 const l=extrudeBrepPolygon([[0,0],[4,0],[4,1],[1,1],[1,4],[0,4]],0,2,[])
 const concave=selectBrepEdges(l,{convexity:'concave'})
 expect(concave).toHaveLength(1)
 const facts=analyzeBrepSelection(l)
 expect(facts.edges[concave[0]!]!.direction!.map(Math.abs)).toEqual([0,0,1])
})

it('resolves edge queries inside rush/nurbs-1 chamfer and fillet nodes',()=>{
 const document={language:'rush/nurbs-1',units:'mm',parameters:[],nodes:[{id:'box',op:'brep_box',min:[0,0,0],max:[3,3,3]},{id:'cut',op:'brep_chamfer',input:'box',edges:{face:{normal:[0,0,1]}},size:0.2},{id:'display',op:'brep_tessellate',input:'cut',segments:2}],root:'display'}
 expect(compileRushGraphNurbs(document).resolved_document.nodes[1]).toMatchObject({edges:{face:{normal:[0,0,1],angle_tolerance:1}}})
 expect(buildOwnNurbs(document,{action:'build'})).toBeDefined()
 expect(()=>buildOwnNurbs({...document,nodes:[document.nodes[0],{...document.nodes[1],edges:{min_length:9}},document.nodes[2]]},{action:'build'})).toThrow('matched no')
})
