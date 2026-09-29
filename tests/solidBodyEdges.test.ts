import {expect,it} from 'vitest'
import {solidBodyEdges} from '../src/services/solidBodyEdges'
import {solidExactEdgeFeature} from '../src/services/solidExactEdgeFeature'
import {createBrepBox,tessellateNurbsBrep} from '../src/services/geometry/brep'
import {solidTopology} from '../src/services/directSolidTools'
it('keeps exact edge identities and samples independent of display tessellation',()=>{
 const brep=createBrepBox([0,0,0],[10,8,6])
 const body={id:'b',name:'B',brep,mesh:tessellateNurbsBrep(brep)}
 const original=solidBodyEdges(body)
 expect(original).toHaveLength(12)
 expect(original.map(edge=>edge.id)).toEqual(brep.topologyIds!.edges)
 const rounded=solidExactEdgeFeature(body,[0],.5,'fillet','brep').body
 const edges=solidBodyEdges(rounded)
 expect(edges).toHaveLength(rounded.brep.edges.length)
 expect(edges.length).toBeLessThan(solidTopology(rounded.mesh).edges.length)
 expect(edges.some(edge=>edge.points.length>2)).toBe(true)
 expect(solidBodyEdges({...rounded,mesh:tessellateNurbsBrep(rounded.brep,3)})).toEqual(edges)
 for(const edge of edges){
  expect(edge.points[0]).toEqual(rounded.brep.edges[edge.index].curve.controlPoints[0])
 }
})
it('keeps polygon feature edges for mesh-only bodies',()=>{
 const brep=createBrepBox([0,0,0],[10,8,6]),mesh=tessellateNurbsBrep(brep)
 expect(solidBodyEdges({id:'m',name:'Mesh',mesh})).toHaveLength(solidTopology(mesh).edges.length)
})
