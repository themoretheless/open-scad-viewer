import { describe, expect, it } from 'vitest'
import { applyDirectExtrusion, circularDirectCopies, defaultDirectCamera, directExtrusionTool, projectDirectPoint, snapDirectPoint, unprojectDirectXY } from '../src/services/directModelingTools'
import { DirectHistory, emptyDirectDocument, type DirectSketch } from '../src/services/directModeling'
import { inspectPolygonMesh } from '../src/services/geometry/polygon'
const box = (id:string,x:number,y:number,w:number):DirectSketch => ({id,name:id,closed:true,points:[[x,y],[x+w,y],[x+w,y+w],[x,y+w]]})
describe('direct modeling canvas tools', () => {
  it('projects and unprojects XY at different orbit angles', () => {
    for (const yaw of [0,.8,2,4]) for (const pitch of [-.8,.2,1.3]) {
      const p=projectDirectPoint([12,-7,0],{yaw,pitch}), q=unprojectDirectXY([p[0],p[1]],{yaw,pitch})
      expect(q[0]).toBeCloseTo(12); expect(q[1]).toBeCloseTo(-7)
    }
    expect(projectDirectPoint([0,0,10],defaultDirectCamera())[1]).toBeLessThan(0)
    expect(()=>unprojectDirectXY([0,0],{yaw:0,pitch:0})).toThrow()
  })
  it('prefers close vertices over grid and supports free positioning', () => {
    expect(snapDirectPoint([1.12,2.1],[[1.1,2.2]],.5,1)).toEqual({point:[1.1,2.2],kind:'vertex'})
    expect(snapDirectPoint([1.12,2.1],[],.5,1)).toEqual({point:[1,2],kind:'grid'})
    expect(snapDirectPoint([1.12,2.1],[],.5,0).point).toEqual([1.12,2.1])
  })
  it('previews a signed extrusion at a base plane without editing the source', () => {
    const s=box('profile',0,0,10), before=JSON.stringify(s)
    const b=directExtrusionTool(s,-5,12)
    const zs=b.mesh.positions.filter((_,i)=>i%3===2)
    expect(Math.min(...zs)).toBe(7);expect(Math.max(...zs)).toBe(12)
    expect(inspectPolygonMesh(b.mesh).signedVolumeMm3).toBeCloseTo(500)
    expect(JSON.stringify(s)).toBe(before)
  })
  it('cuts a through hole atomically and supports undo of the confirmed operation', () => {
    const d=emptyDirectDocument();d.sketches.push(box('base',0,0,10),box('hole',2,2,4))
    const solid=applyDirectExtrusion(d,'base',10,0,'new','','body'), h=new DirectHistory(solid)
    const cut=applyDirectExtrusion(solid,'hole',12,-1,'difference','body','unused')
    expect(inspectPolygonMesh(cut.bodies[0].mesh).signedVolumeMm3).toBeCloseTo(840)
    expect(inspectPolygonMesh(cut.bodies[0].mesh).closed).toBe(true)
    expect(inspectPolygonMesh(solid.bodies[0].mesh).signedVolumeMm3).toBeCloseTo(1000)
    h.commit(cut); expect(inspectPolygonMesh(h.undo().bodies[0].mesh).signedVolumeMm3).toBeCloseTo(1000)
    expect(h.redo()).toEqual(cut);expect(cut.sketches).toEqual(solid.sketches)
    expect(()=>applyDirectExtrusion(solid,'hole',5,0,'difference','missing','x')).toThrow()
  })
  it('adds extrusion to a target without growing an operation tree', () => {
    const d=emptyDirectDocument();d.sketches.push(box('base',0,0,10))
    const a=applyDirectExtrusion(d,'base',10,0,'new','','body')
    const b=applyDirectExtrusion(a,'base',10,5,'union','body','unused')
    expect(b.bodies).toHaveLength(1);expect(inspectPolygonMesh(b.bodies[0].mesh).signedVolumeMm3).toBeCloseTo(1500)
  })
  it('copies around a specified center without duplicating the final full-circle position', () => {
    const s=box('s',10,0,2);let i=0
    const copies=circularDirectCopies(s,4,[0,0],360,()=>String(i++))
    expect(copies).toHaveLength(3);expect(copies[0].points[0][0]).toBeCloseTo(0);expect(copies[0].points[0][1]).toBeCloseTo(10)
    expect(copies[2].points[0][1]).toBeCloseTo(-10)
    copies[0].points[0][0]=99;expect(s.points[0][0]).toBe(10)
    expect(()=>circularDirectCopies(s,65,[0,0],360,()=>String(i++))).toThrow()
  })
})
