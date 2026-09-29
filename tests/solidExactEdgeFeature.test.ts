import {expect,it} from 'vitest'
import {solidExactEdgeFeature} from '../src/services/solidExactEdgeFeature'
import {solidTopology} from '../src/services/directSolidTools'
import {createBrepBox,tessellateNurbsBrep,analyzeNurbsBrep,inspectNurbsBrep} from '../src/services/geometry/brep'
import {DirectHistory,parseDirectDocument} from '../src/services/directModeling'
import {stringifyMeshJson} from '../src/services/meshJson'

function fixture(){
 const brep=createBrepBox([0,0,0],[10,8,6]),mesh=tessellateNurbsBrep(brep)
 const body={id:'part',name:'Part',brep,mesh,material:{name:'Copper',color:'#cc7744'}}
 const vertical=solidTopology(mesh).edges.flatMap((e,i)=>{
  const a=mesh.positions.slice(e.a*3,e.a*3+3),b=mesh.positions.slice(e.b*3,e.b*3+3)
  return Math.abs(a[0]-b[0])<1e-8&&Math.abs(a[1]-b[1])<1e-8?[i]:[]
 })
 return {body,vertical}
}
it('authors exact cylindrical fillets from display selection, with analytic volume and restorable identity',()=>{
 const {body,vertical}=fixture(),before=structuredClone(body)
 expect(vertical).toHaveLength(4)
 const result=solidExactEdgeFeature(body,vertical,1,'fillet')
 expect(result.evidence.certificate.capability).toBe('exact-convex-prism-edge-fillet/1')
 expect(result.body.brep.faces.some(face=>face.surface.degreeU===2||face.surface.degreeV===2)).toBe(true)
 expect(inspectNurbsBrep(result.body.brep).topologyValid).toBe(true)
 expect(analyzeNurbsBrep(result.body.brep).signedVolumeMm3).toBeCloseTo(480-(4-Math.PI)*6,5)
 expect(result.body.material).toEqual(body.material)
 expect(body).toEqual(before)
 const history=new DirectHistory({version:1,sketches:[],bodies:[body]})
 history.commit({...history.document,bodies:[result.body]})
 expect(history.undo().bodies[0].brep).toEqual(body.brep)
 const restored=parseDirectDocument(stringifyMeshJson(history.redo()))
 expect(restored.bodies[0].brep?.topologyIds).toEqual(result.body.brep.topologyIds)
})
it('authors a chamfer of the requested distance',()=>{
 const {body,vertical}=fixture()
 const result=solidExactEdgeFeature(body,[vertical[0]],1,'chamfer')
 expect(result.evidence.certificate.capability).toBe('exact-convex-straight-edge-chamfer/1')
 expect(analyzeNurbsBrep(result.body.brep).signedVolumeMm3).toBeCloseTo(477,5)
})
it('refuses invalid radii and edge selections without mutating the body',()=>{
 const {body,vertical}=fixture(),before=structuredClone(body)
 for(const radius of [-1,0,100,NaN])expect(()=>solidExactEdgeFeature(body,vertical,radius,'fillet')).toThrow()
 expect(()=>solidExactEdgeFeature(body,[-1],1,'fillet')).toThrow('existing straight edge')
 expect(()=>solidExactEdgeFeature(body,[vertical[0],vertical[0]],1,'fillet')).toThrow()
 expect(body).toEqual(before)
})
it('authors a linear radius law and a three-edge spherical corner',()=>{
 const {body}=fixture()
 const vertical=body.brep.edges.findIndex(edge=>{
  const [a,b]=edge.vertices.map(i=>body.brep.vertices[i].point)
  return a[0]===b[0]&&a[1]===b[1]
 })
 const variable=solidExactEdgeFeature(body,[vertical],.5,'fillet','brep',{mode:'variable',endRadius:1.5})
 expect(variable.evidence.certificate.capability).toBe('exact-variable-radius-fillet/1')
 expect(analyzeNurbsBrep(variable.body.brep).signedVolumeMm3).toBeCloseTo(480-(1-Math.PI/4)*6*(.25+.75+2.25)/3,5)
 const corner=body.brep.vertices.findIndex(v=>v.point.every((x,i)=>x===[10,8,6][i]))
 const edges=body.brep.edges.flatMap((edge,i)=>edge.vertices.includes(corner)?[i]:[])
 const result=solidExactEdgeFeature(body,edges,1,'fillet','brep',{mode:'corner',endRadius:1})
 expect(result.evidence.certificate.capability).toBe('exact-valence3-corner-blend/1')
 expect(result.evidence.audit.ok).toBe(true)
 expect(analyzeNurbsBrep(result.body.brep).signedVolumeMm3).toBeCloseTo(480-(24-3)*(1-Math.PI/4)-(1-Math.PI/6),5)
 expect(()=>solidExactEdgeFeature(body,[vertical],1,'fillet','brep',{mode:'variable',endRadius:1})).toThrow()
 expect(()=>solidExactEdgeFeature(body,[vertical],1,'fillet','brep',{mode:'corner',endRadius:1})).toThrow()
})
