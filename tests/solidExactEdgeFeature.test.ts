import {expect,it} from 'vitest'
import {solidExactEdgeFeature} from '../src/services/solidExactEdgeFeature'
import {solidTopology} from '../src/services/directSolidTools'
import {createBrepBox,tessellateNurbsBrep,analyzeNurbsBrep,inspectNurbsBrep,transformNurbsBrep} from '../src/services/geometry/brep'
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
it('preserves endpoint ordering, dimensions and identity for variable-radius rounds on all rotated cuboid edges',()=>{
 const source=createBrepBox([-7,3,-2],[3,11,4]),a=.37,b=-.61
 const brep=transformNurbsBrep(source,[
  [Math.cos(a)*Math.cos(b),-Math.sin(a),Math.cos(a)*Math.sin(b),17],
  [Math.sin(a)*Math.cos(b),Math.cos(a),Math.sin(a)*Math.sin(b),-9],
  [-Math.sin(b),0,Math.cos(b),23],[0,0,0,1],
 ])
 const body={id:'rotated',name:'Rotated part',brep,mesh:tessellateNurbsBrep(brep),material:{name:'Copper',color:'#cc7744'}}
 const before=stringifyMeshJson(body)
 for(let edge=0;edge<brep.edges.length;edge++)for(const [start,end] of [[.5,1.5],[1.5,.5]]){
  const [p,q]=source.edges[edge].vertices.map(i=>source.vertices[i].point)
  const length=Math.hypot(...p.map((x,i)=>x-q[i]))
  const result=solidExactEdgeFeature(body,[edge],start,'fillet','brep',{mode:'variable',endRadius:end})
  expect(result.evidence.audit.ok&&result.evidence.namingComplete).toBe(true)
  expect(result.body.id).toBe(body.id);expect(result.body.material).toEqual(body.material)
  expect(analyzeNurbsBrep(result.body.brep).signedVolumeMm3).toBeCloseTo(480-(1-Math.PI/4)*length*(start**2+start*end+end**2)/3,4)
  const history=new DirectHistory({version:1,sketches:[],bodies:[body]})
  history.commit({...history.document,bodies:[result.body]})
  expect(history.undo().bodies[0].brep).toEqual(body.brep)
  expect(parseDirectDocument(stringifyMeshJson(history.redo())).bodies[0].brep).toEqual(result.body.brep)
 }
 expect(stringifyMeshJson(body)).toBe(before)
},60_000)
it('authors exact cylindrical fillets from display selection, with analytic volume and restorable identity',()=>{
 const {body,vertical}=fixture(),before=structuredClone(body)
 expect(vertical).toHaveLength(4)
 const result=solidExactEdgeFeature(body,vertical,1,'fillet')
 expect(result.evidence.certificate.capability).toBe('exact-simple-prism-convex-edge-fillet/1')
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

it('exposes audited concave-prism convex-edge rounds through the WASM bridge without widening the old capability',async()=>{
 const {extrudeSketchProfile}=await import('../src/services/directExtrusion')
 const {exactSimplePrismFillet,exactConvexPrismFillet,transformNurbsBrep}=await import('../src/services/geometry/brep')
 const brep=extrudeSketchProfile([{id:'profile',name:'Bracket',closed:true,points:[[0,0],[40,0],[40,5],[5,5],[5,30],[0,30]]}],20)
 const before=stringifyMeshJson(brep)
 const edgeAt=(x:number,y:number)=>brep.edges.findIndex(e=>{const [a,b]=e.vertices.map(i=>brep.vertices[i].point);return a[0]===x&&b[0]===x&&a[1]===y&&b[1]===y&&Math.abs(a[2]-b[2])>19})
 const outer=edgeAt(0,0)
 expect(()=>exactConvexPrismFillet(brep,[outer],1)).toThrow('strictly convex')
 for(const radius of [.25,1,16]){
  const result=exactSimplePrismFillet(brep,[outer],radius)
  expect(result.certificate.capability).toBe('exact-simple-prism-convex-edge-fillet/1')
  expect(result.certificate.complete&&result.audit.ok&&result.namingComplete).toBe(true)
  expect(analyzeNurbsBrep(result.model).signedVolumeMm3).toBeCloseTo(6500-radius**2*(1-Math.PI/4)*20,4)
  expect(result.model.topologyIds).toBeDefined()
  const document={version:1 as const,sketches:[],bodies:[{id:'bracket',name:'Bracket',brep:result.model,mesh:tessellateNurbsBrep(result.model,8)}]}
  expect(parseDirectDocument(stringifyMeshJson(document)).bodies[0].brep).toEqual(result.model)
 }
 expect(()=>exactSimplePrismFillet(brep,[outer],20)).toThrow('nonadjacent span')
 expect(()=>exactSimplePrismFillet(brep,[edgeAt(5,5)],1)).toThrow('must be convex')
 const selected=[[0,0],[40,0],[40,5],[5,30],[0,30]].map(([x,y])=>edgeAt(x,y))
 const rounded=exactSimplePrismFillet(brep,selected,1)
 expect(rounded.audit.ok&&rounded.namingComplete).toBe(true)
 expect(analyzeNurbsBrep(rounded.model).signedVolumeMm3).toBeCloseTo(6500-5*(1-Math.PI/4)*20,4)
 const placed=transformNurbsBrep(brep,[[0,0,1,11],[1,0,0,-7],[0,1,0,3],[0,0,0,1]])
 const placedResult=exactSimplePrismFillet(placed,selected,1)
 expect(placedResult.audit.ok&&placedResult.namingComplete).toBe(true)
 expect(analyzeNurbsBrep(placedResult.model).signedVolumeMm3).toBeCloseTo(6500-5*(1-Math.PI/4)*20,4)
 for(const radius of [0,-1,100])expect(()=>exactSimplePrismFillet(brep,[outer],radius)).toThrow()
 for(const edges of [[],[outer,outer],[-1],[brep.edges.length]])expect(()=>exactSimplePrismFillet(brep,edges,1)).toThrow()
 const cap=brep.edges.findIndex(e=>{const [a,b]=e.vertices.map(i=>brep.vertices[i].point);return a[2]===b[2]})
 expect(()=>exactSimplePrismFillet(brep,[cap],1)).toThrow()
 expect(stringifyMeshJson(brep)).toBe(before)
},30000)

it('rounds a C-profile outer corner without crossing its notch',async()=>{
 const {extrudeSketchProfile}=await import('../src/services/directExtrusion')
 const {exactSimplePrismFillet}=await import('../src/services/geometry/brep')
 const brep=extrudeSketchProfile([{id:'c',name:'C profile',closed:true,points:[[0,0],[10,0],[10,2],[2,2],[2,8],[10,8],[10,10],[0,10]]}],5)
 const before=stringifyMeshJson(brep)
 const edge=brep.edges.findIndex(e=>e.vertices.every(i=>brep.vertices[i].point[0]===0&&brep.vertices[i].point[1]===0))
 const result=exactSimplePrismFillet(brep,[edge],.5)
 expect(result.audit.ok&&result.namingComplete&&result.certificate.complete).toBe(true)
 expect(analyzeNurbsBrep(result.model).signedVolumeMm3).toBeCloseTo(260-.25*(1-Math.PI/4)*5,4)
 expect(()=>exactSimplePrismFillet(brep,[edge],9)).toThrow()
 expect(stringifyMeshJson(brep)).toBe(before)
})

it('refuses holed and disconnected prisms atomically',async()=>{
 const {extrudeSketchProfile}=await import('../src/services/directExtrusion')
 const {exactSimplePrismFillet,booleanNurbsBrep}=await import('../src/services/geometry/brep')
 const holed=extrudeSketchProfile([
  {id:'outer',name:'Outer',closed:true,points:[[0,0],[20,0],[20,20],[0,20]]},
  {id:'hole',name:'Hole',closed:true,points:[[5,5],[5,15],[15,15],[15,5]]},
 ],5)
 const disconnected=booleanNurbsBrep(createBrepBox([0,0,0],[10,10,5]),createBrepBox([20,0,0],[30,10,5]),'union')
 for(const model of [holed,disconnected]){
  const before=stringifyMeshJson(model)
  const edge=model.edges.findIndex(e=>{const [a,b]=e.vertices.map(i=>model.vertices[i].point);return a[0]===0&&b[0]===0&&a[1]===0&&b[1]===0&&a[2]!==b[2]})
  expect(edge).toBeGreaterThanOrEqual(0)
  expect(()=>exactSimplePrismFillet(model,[edge],.5)).toThrow()
  expect(stringifyMeshJson(model)).toBe(before)
 }
})

it('rounds complete circular rims of the roadmap flange through the annular WASM API',async()=>{
 const {cadRoadmapParts}=await import('../benchmarks/cad-roadmap-fixtures')
 const {exactAnnularFillet,transformNurbsBrep}=await import('../src/services/geometry/brep')
 const body=cadRoadmapParts().find(p=>p.body.id==='flange')!.body,model=body.brep!
 const before=stringifyMeshJson(model)
 const outerTop=model.edges.flatMap((e,i)=>e.curve.degree===2&&e.vertices.every(v=>{const p=model.vertices[v].point;return Math.abs(p[2]-6)<1e-8&&Math.abs(Math.hypot(p[0],p[1])-20)<1e-8})?[i]:[])
 expect(outerTop).toHaveLength(4)
 const command=solidExactEdgeFeature(body,outerTop,1,'fillet','brep')
 expect(command.evidence.certificate.capability).toBe('exact-annular-circular-edge-fillet/1')
 expect(command.body.id).toBe(body.id)
 expect(()=>solidExactEdgeFeature(body,outerTop.slice(0,1),1,'fillet','brep')).toThrow('Partial circular rim')
 for(const radius of [.25,1,2.5]){
  const result=exactAnnularFillet(model,outerTop,radius)
  expect(result.certificate.capability).toBe('exact-annular-circular-edge-fillet/1')
  expect(result.certificate.complete&&result.audit.ok&&result.namingComplete).toBe(true)
  const moment=(20-radius)*(1-Math.PI/4)*radius**2+radius**3/6
  expect(analyzeNurbsBrep(result.model).signedVolumeMm3).toBeCloseTo(2250*Math.PI-2*Math.PI*moment,4)
  const history=new DirectHistory({version:1,sketches:[],bodies:[body]})
  history.commit({...history.document,bodies:[{...body,brep:result.model,mesh:tessellateNurbsBrep(result.model,8)}]})
  expect(history.undo().bodies[0].brep).toEqual(model)
  expect(parseDirectDocument(stringifyMeshJson(history.redo())).bodies[0].brep).toEqual(result.model)
 }
 expect(()=>exactAnnularFillet(model,outerTop.slice(0,1),1)).toThrow('Partial circular rim')
 expect(()=>exactAnnularFillet(model,outerTop,6)).toThrow()
 const placed=transformNurbsBrep(model,[[0,0,1,7],[1,0,0,-3],[0,1,0,11],[0,0,0,1]])
 expect(exactAnnularFillet(placed,outerTop,1).audit.ok).toBe(true)
 expect(stringifyMeshJson(model)).toBe(before)
},30000)

it('rounds complete enclosure chains without opening its cavity through the WASM API',async()=>{
 const {cadRoadmapParts}=await import('../benchmarks/cad-roadmap-fixtures')
 const {exactLayeredPrismFillet,transformNurbsBrep}=await import('../src/services/geometry/brep')
 const body=cadRoadmapParts().find(p=>p.body.id==='enclosure')!.body,model=body.brep!
 const before=stringifyMeshJson(model)
 const edges=model.edges.flatMap((e,i)=>e.vertices.every(v=>model.vertices[v].point[0]===0&&model.vertices[v].point[1]===0)?[i]:[])
 expect(edges.length).toBeGreaterThan(1)
 expect(solidExactEdgeFeature(body,edges,1,'fillet','brep').evidence.certificate.capability).toBe('exact-layered-prism-edge-fillet/1')
 for(const radius of [1,4]){
  const result=exactLayeredPrismFillet(model,edges,radius)
  expect(result.certificate.capability).toBe('exact-layered-prism-edge-fillet/1')
  expect(result.audit.ok).toBe(true)
  expect(result.namingComplete).toBe(true)
  expect(analyzeNurbsBrep(result.model).signedVolumeMm3).toBeCloseTo(7152-(1-Math.PI/4)*radius**2*20,4)
  const history=new DirectHistory({version:1,sketches:[],bodies:[body]})
  const next={...body,brep:result.model,mesh:tessellateNurbsBrep(result.model)}
  history.commit({...history.document,bodies:[next]})
  expect(stringifyMeshJson(history.undo().bodies[0].brep)).toBe(before)
  expect(parseDirectDocument(stringifyMeshJson(history.redo())).bodies[0].brep?.topologyIds).toEqual(result.model.topologyIds)
 }
 const angle=.37,c=Math.cos(angle),s=Math.sin(angle)
 const placed=transformNurbsBrep(model,[[c,-s,0,7],[0,0,-1,11],[s,c,0,-3],[0,0,0,1]])
 expect(analyzeNurbsBrep(exactLayeredPrismFillet(placed,edges,1).model).signedVolumeMm3).toBeCloseTo(7152-(1-Math.PI/4)*20,4)
 expect(()=>exactLayeredPrismFillet(model,edges,8)).toThrow()
 expect(()=>exactLayeredPrismFillet(model,[edges[0]],1)).toThrow()
 expect(stringifyMeshJson(model)).toBe(before)
})
