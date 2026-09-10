import { describe, expect, it } from 'vitest'
import { directCornerTool, directRevolveTool, applyDirectRevolve } from '../src/services/directProfileTools'
import { DirectHistory, emptyDirectDocument, extrudeDirectSketch, type DirectSketch } from '../src/services/directModeling'
import { inspectPolygonMesh } from '../src/services/geometry/polygon'
const square:DirectSketch={id:'s',name:'Square',closed:true,points:[[0,0],[10,0],[10,10],[0,10]]}
const area=(s:DirectSketch)=>Math.abs(s.points.reduce((sum,p,i)=>{const q=s.points[(i+1)%s.points.length];return sum+p[0]*q[1]-p[1]*q[0]},0))/2
const opts={axis:'y' as const,offset:0,angle:360,segments:64}
describe('direct corner and revolve operations',()=>{
  it('fillets a corner with the requested radius, tangent endpoints and either winding',()=>{
    for(const s of [square,{...square,points:[...square.points].reverse()}]) {
      const result=directCornerTool(s,0,2,'fillet')
      expect(area(result)).toBeCloseTo(100-4+Math.PI,2)
      const report=inspectPolygonMesh(extrudeDirectSketch(result,5,'body').mesh)
      expect(report.closed).toBe(true);expect(report.signedVolumeMm3).toBeCloseTo(area(result)*5)
    }
    const rounded=directCornerTool(square,0,2,'fillet')
    expect(rounded.points[0][1]).toBeCloseTo(2)
    expect(rounded.points[18][0]).toBeCloseTo(2)
    expect(square.points).toHaveLength(4)
  })
  it('creates an outward semicircular DogEar and a closed extrudable contour',()=>{
    const relief=directCornerTool(square,0,2,'dogear')
    expect(area(relief)).toBeCloseTo(100+2*Math.PI-4,2)
    expect(Math.min(...relief.points.map(p=>p[0]))).toBeLessThan(0)
    expect(Math.min(...relief.points.map(p=>p[1]))).toBeLessThan(0)
    expect(inspectPolygonMesh(extrudeDirectSketch(relief,3,'body').mesh).closed).toBe(true)
    expect(area(directCornerTool({...square,points:[...square.points].reverse()},3,2,'dogear'))).toBeCloseTo(area(relief))
  })
  it('rounds a concave corner and rejects oversized, collinear and unsupported relief corners',()=>{
    const l:DirectSketch={...square,points:[[0,0],[10,0],[10,4],[4,4],[4,10],[0,10]]}
    expect(area(directCornerTool(l,3,1,'fillet'))).toBeGreaterThan(area(l))
    expect(()=>directCornerTool(square,0,10,'fillet')).toThrow('adjacent')
    expect(()=>directCornerTool(square,0,NaN,'fillet')).toThrow('Radius')
    expect(()=>directCornerTool({...square,closed:false},0,1,'fillet')).toThrow('closed')
    expect(()=>directCornerTool({...square,points:[[0,0],[10,0],[5,8]]},0,1,'dogear')).toThrow('right-angle')
    expect(()=>directCornerTool({...square,points:[[0,0],[5,0],[10,0],[10,10],[0,10]]},1,1,'fillet')).toThrow('collinear')
  })
  it('revolves a profile touching either axis into a cylinder with correct bounds and volume',()=>{
    for(const axis of ['x','y'] as const) for(const side of [1,-1]) {
      const s={...square,points:square.points.map(p=>[p[0]*side,p[1]*side] as [number,number])}
      const body=directRevolveTool(s,{...opts,axis}),report=inspectPolygonMesh(body.mesh)
      expect(report.closed).toBe(true);expect(report.signedVolumeMm3/(Math.PI*1000)).toBeCloseTo(1,2)
      const z=body.mesh.positions.filter((_,i)=>i%3===2)
      expect(Math.min(...z)).toBeCloseTo(-10);expect(Math.max(...z)).toBeCloseTo(10)
    }
  })
  it('caps partial and negative revolutions and supports an offset axis',()=>{
    for(const angle of [180,-180,90]) {
      const b=directRevolveTool(square,{...opts,offset:-2,angle}),r=inspectPolygonMesh(b.mesh)
      expect(r.closed).toBe(true)
      expect(r.signedVolumeMm3/(Math.PI*(144-4)*10*Math.abs(angle)/360)).toBeCloseTo(1,2)
    }
    expect(()=>directRevolveTool(square,{...opts,offset:5})).toThrow('crosses')
    expect(()=>directRevolveTool(square,{...opts,angle:0})).toThrow()
    expect(()=>directRevolveTool(square,{...opts,segments:1})).toThrow()
  })
  it('confirms revolve add/cut atomically with undo/redo and independent source sketches',()=>{
    const d=emptyDirectDocument(); d.sketches.push(square,{...square,id:'hole',points:[[0,0],[2,0],[2,10],[0,10]]})
    const solid=applyDirectRevolve(d,'s',opts,'new','','body'),history=new DirectHistory(solid)
    const cut=applyDirectRevolve(solid,'hole',opts,'difference','body','unused')
    expect(cut.bodies).toHaveLength(1)
    expect(inspectPolygonMesh(cut.bodies[0].mesh).closed).toBe(true)
    expect(inspectPolygonMesh(cut.bodies[0].mesh).signedVolumeMm3/(Math.PI*960)).toBeCloseTo(1,2)
    history.commit(cut);expect(history.undo()).toEqual(solid);expect(history.redo()).toEqual(cut)
    const restored=applyDirectRevolve(cut,'hole',opts,'union','body','unused')
    expect(inspectPolygonMesh(restored.bodies[0].mesh).signedVolumeMm3).toBeCloseTo(inspectPolygonMesh(solid.bodies[0].mesh).signedVolumeMm3)
    expect(cut.sketches).toEqual(d.sketches)
    expect(()=>applyDirectRevolve(solid,'s',opts,'difference','missing','unused')).toThrow('target')
  })
})
