import {expect,it} from 'vitest'
import {prepareSolidProfile} from '../src/services/solidProfilePreparation'
import {DirectHistory,emptyDirectDocument,parseDirectDocument,serializeDirectDocument} from '../src/services/directModeling'
import {evaluateNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'
import {worldPoint} from '../src/services/directSketchGeometry'
import {mainSolidExpectation,mainSolidResult,type MainSolidJob} from '../src/services/mainSolidProtocol'

function curves():NurbsCurve[]{
 const p=[[0,0],[2,0],[2,2],[0,2]]
 const loops=p.map((a,i)=>({degree:1,knots:[0,0,1,1],controlPoints:[a,p[(i+1)%4]],weights:[1,1]}))
 return [{degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0],[1,-1],[2,0]],weights:[1,1,1]},...loops.slice(1)]
}
it('prepares a mixed sketch and NURBS chain while preserving identity, source and history',()=>{
 const document=emptyDirectDocument()
 document.curves=[{id:'nurbs',name:'NURBS',curve:curves()[0],group:'part'}]
 document.sketches=[{id:'line',name:'Line',closed:false,points:[[2,0],[2,2],[0,2],[0,0]]}]
 document.groups=[{name:'part',source:'manual'}]
 const before=structuredClone(document),result=prepareSolidProfile(document,['nurbs','line'],0)
 expect(result.report.accepted).toBe(true)
 expect(result.document.curves).toEqual([])
 expect(result.document.sketches).toHaveLength(1)
 expect(result.document.sketches[0]).toMatchObject({id:'nurbs',name:'NURBS',group:'part',closed:true})
 expect(result.document.sketches[0].retainedProfile?.loops[0][0]).toMatchObject(curves()[0])
 expect(result.report.curveSources?.[0]).toMatchObject({chain:0,segment:0,connector:false})
 const job:MainSolidJob={kind:'profilePrepare',document,ids:['nurbs','line'],tolerance:0}
 expect(mainSolidResult(mainSolidExpectation(job),result)).toBe(true)
 expect(mainSolidResult(mainSolidExpectation(job),{...result,report:{...result.report,projectionMaxDeviationMm:1}})).toBe(false)
 const history=new DirectHistory(document);history.commit(result.document)
 expect(history.undo()).toEqual(before);expect(history.redo()).toEqual(result.document)
 expect(parseDirectDocument(serializeDirectDocument(result.document))).toEqual(result.document)
 expect(document).toEqual(before)
})
it('infers a spatial sketch plane and keeps the original rational curves within measured projection error',()=>{
 const document=emptyDirectDocument()
 document.curves=curves().map((c,i)=>({id:`c${i}`,name:`Curve ${i}`,curve:{...c,controlPoints:c.controlPoints.map(p=>[5,p[0],p[1]])}}))
 const before=structuredClone(document),result=prepareSolidProfile(document,['c0','c1','c2','c3'],0)
 expect(result.report.accepted).toBe(true)
 expect(result.report.projectionMaxDeviationMm).toBeLessThanOrEqual(1e-7)
 const profile=result.document.sketches[0].retainedProfile!
 for(const source of result.report.curveSources??[]){
  if(source.connector)continue
  const index=result.report.curveSources!.indexOf(source),c=profile.loops[0][index],original=before.curves![source.chain].curve
  expect(c.knots).toEqual(original.knots);expect(c.weights).toEqual(original.weights)
  for(const t of [0,.2,.5,.9,1]){
   const p=worldPoint(evaluateNurbsCurve(c,t).point,result.plane),q=evaluateNurbsCurve(original,source.reversed?1-t:t).point
   expect(Math.hypot(...p.map((x,k)=>x-q[k]))).toBeLessThan(1e-7)
  }
 }
 expect(document).toEqual(before)
})
it('refuses nonplanar control nets and leaves ambiguous endpoint inputs untouched',()=>{
 const document=emptyDirectDocument()
 document.curves=curves().map((c,i)=>({id:`c${i}`,name:`Curve ${i}`,curve:{...c,controlPoints:c.controlPoints.map(p=>[p[0],p[1],0])}}))
 document.curves[0].curve.controlPoints[1][2]=.1
 const before=structuredClone(document)
 expect(()=>prepareSolidProfile(document,['c0','c1','c2','c3'],0)).toThrow(/same sketch plane/)
 expect(document).toEqual(before)
 const flat=emptyDirectDocument();flat.curves=[{id:'curve',name:'Curve',curve:curves()[0]}]
 const result=prepareSolidProfile(flat,['curve'],0)
 expect(result.report.accepted).toBe(false);expect(result.document).toEqual(flat)
})
it('localizes both rational crossings of a refused contour and keeps the document unchanged',async()=>{
 const document=emptyDirectDocument()
 document.curves=[{id:'arch',name:'Arch',curve:{degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0],[1,5],[2,0]],weights:[1,.8,1]}}]
 document.sketches=[{id:'lines',name:'Lines',closed:false,points:[[2,0],[2,2],[0,2],[0,0]]}]
 const before=structuredClone(document),result=prepareSolidProfile(document,['arch','lines'],0)
 expect(result.report).toMatchObject({accepted:false,reason:'invalid-contour'})
 expect(result.report.diagnosticLoops?.[0][0]).toEqual({...document.curves[0].curve,periodic:false})
 const {profileIntersectionDiagnostics}=await import('../src/services/profileIntersectionDiagnostics')
 const events=profileIntersectionDiagnostics(result.report.diagnosticLoops!,result.report.intersections!)
 expect(events.points).toHaveLength(2)
 expect(events.points.every(p=>p.first.curve===0&&p.second.curve===2)).toBe(true)
 expect(events.points.every(p=>Math.abs(p.point[1]-2)<1e-7)).toBe(true)
 expect(result.report.diagnosticDisplay?.filter(s=>s.kind==='intersection').map(s=>s.curve).sort()).toEqual([0,2])
 expect(result.report.intersectionDiagnosticError).toBeUndefined()
 expect(mainSolidResult(mainSolidExpectation({kind:'profilePrepare',document,ids:['arch','lines'],tolerance:0}),result)).toBe(true)
 expect(result.document).toEqual(before);expect(document).toEqual(before)
})
