import {describe,it,expect} from 'vitest'
import {extrudeDirectSketch,parseDirectDocument,emptyDirectDocument,DirectHistory,type DirectSketch} from '../src/services/directModeling'
import {solidTopology,pushPullFace,bevelSolidEdge,shellSolid,splitSolid,facePlane,transformSelection} from '../src/services/directSolidTools'
import {sampleCurve,offsetSketch,trimSketch,extendSketch,transformSketch,worldPoint} from '../src/services/directSketchGeometry'
import {directExtrusionTool} from '../src/services/directModelingTools'
import {inspectPolygonMesh} from '../src/services/geometry/polygon'
const s:DirectSketch={id:'s',name:'s',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]}
const box=()=>extrudeDirectSketch(s,10,'b')
const volume=(b:ReturnType<typeof box>)=>{const r=inspectPolygonMesh(b.mesh);expect(r.closed).toBe(true);expect(r.degenerateTriangles).toBe(0);return r.signedVolumeMm3}
describe('advanced direct modeling',()=>{
 it('merges triangle faces, omits diagonal edges and pushes/pulls one support plane',()=>{
  const b=box(),t=solidTopology(b.mesh),top=t.faces.findIndex(f=>f.normal[2]>.99)
  expect(t.faces).toHaveLength(6);expect(t.edges).toHaveLength(12)
  expect(volume(pushPullFace(b,top,3))).toBeCloseTo(1300)
  expect(volume(pushPullFace(b,top,-3))).toBeCloseTo(700)
  expect(()=>pushPullFace(b,top,-11)).toThrow()
 })
 it('creates an edge chamfer and a tangent-radius fillet',()=>{
  const b=box();expect(volume(bevelSolidEdge(b,0,2,'chamfer'))).toBeCloseTo(980)
  expect(volume(bevelSolidEdge(b,0,2,'fillet'))).toBeCloseTo(1000-10*(4-Math.PI),0)
  expect(()=>bevelSolidEdge(b,0,50,'fillet')).toThrow()
 })
 it('shells a cube through selected openings at an exact wall thickness',()=>{
  const b=box(),t=solidTopology(b.mesh),top=t.faces.findIndex(f=>f.normal[2]>.99)
  expect(volume(shellSolid(b,[top],1))).toBeCloseTo(1000-8*8*9)
  expect(()=>shellSolid(b,[top],6)).toThrow()
 })
 it('splits closed solids along axis-aligned and oblique planes preserving total volume',()=>{
  const b=box();for(const [n,d] of [[[0,0,1],4],[[1,1,1],8]] as const){const [a,c]=splitSolid(b,[...n],d);expect(volume(a)+volume(c)).toBeCloseTo(1000)}
  const [a,b2]=splitSolid(b,[0,0,1],4);expect(volume(a)).toBeCloseTo(600);expect(volume(b2)).toBeCloseTo(400)
 })
 it('draws and extrudes in the basis of a vertical face',()=>{
  const b=box(),face=solidTopology(b.mesh).faces.find(f=>f.normal[0]>.99)!,plane=facePlane(b,face)
  const sketch={...s,plane},body=directExtrusionTool(sketch,3,0)
  expect(volume(body)).toBeCloseTo(300)
  const xs=body.mesh.positions.filter((_,i)=>i%3===0);expect(Math.min(...xs)).toBeCloseTo(10);expect(Math.max(...xs)).toBeCloseTo(13)
  expect(worldPoint([0,0],plane)).toEqual(plane.origin)
 })
 it('keeps curves analytic across JSON, translation, scale and offsets',()=>{
  const analytic={kind:'circle' as const,center:[2,3] as [number,number],radius:5,start:0,sweep:360},d=emptyDirectDocument();d.sketches.push({...s,analytic,points:sampleCurve(analytic)})
  const round=transformSketch(d.sketches[0],[10,0],30,2)
  expect(round.analytic?.radius).toBe(10);expect(round.analytic?.center).toEqual([12,3]);expect(offsetSketch(round,2).analytic?.radius).toBe(12)
  d.sketches=[round];expect(parseDirectDocument(JSON.stringify(d))).toEqual(d)
  const arc={...analytic,kind:'arc' as const,sweep:90};expect(sampleCurve(arc)[0]).toEqual([7,3]);expect(sampleCurve(arc).at(-1)![1]).toBeCloseTo(8)
 })
 it('offsets polygons and rejects collapsed insets',()=>{
  const out=offsetSketch(s,1);expect(volume(extrudeDirectSketch(out,1,'x'))).toBeCloseTo(144)
  expect(volume(extrudeDirectSketch(offsetSketch(s,-1),1,'x'))).toBeCloseTo(64)
  expect(()=>offsetSketch(s,-6)).toThrow()
 })
 it('trims at intersections, keeps two chains and extends endpoints to boundaries',()=>{
  const line:DirectSketch={...s,closed:false,points:[[0,5],[10,5]]},a:DirectSketch={...line,id:'a',points:[[3,0],[3,10]]},b:DirectSketch={...a,id:'c',points:[[7,0],[7,10]]}
  const pieces=trimSketch(line,0,.5,[a,b]);expect(pieces.map(s=>s.points)).toEqual([[[0,5],[3,5]],[[7,5],[10,5]]])
  expect(extendSketch({...line,points:[[0,5],[2,5]]},'end',[a,b]).points).toEqual([[0,5],[3,5]])
  expect(()=>extendSketch(line,'end',[a])).toThrow()
 })
 it('transforms multiple bodies about a shared pivot in one history transaction',()=>{
  const d=emptyDirectDocument();d.bodies=[box(),{...box(),id:'c',mesh:{...box().mesh,positions:box().mesh.positions.map((v,i)=>i%3===0?v+20:v)}}]
  const result=transformSelection(d,['b','c'],[5,0,0],[0,0,1],180,1),h=new DirectHistory(d)
  const min=(i:number)=>Math.min(...result.bodies[i].mesh.positions.filter((_,i)=>i%3===0))
  expect(min(0)).toBeCloseTo(25);expect(min(1)).toBeCloseTo(5);h.commit(result);expect(h.undo()).toEqual(d);expect(h.redo()).toEqual(result)
 })
})
