import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {evaluateNurbsSurface,insertNurbsSurfaceKnot} from '../src/services/nurbsSurface'
import {expect,it} from 'vitest'
import {createBrepSphere,createBrepCylinder,createBrepFrustum,createBrepTorus,transformNurbsBrep} from '../src/services/geometry/brep'
import {intersectionTraceToNurbsCurve,intersectNurbsSurfaceSurface,intersectNurbsCurveSegment,evaluateIntersectionTrace,intersectNurbsCurvePlane,intersectNurbsCurveSurface,intersectNurbsSurfacePlane,intersectNurbsCurveCurve,intersectNurbsCurveRuledSurface,intersectSphereSphere,intersectSphereCylinder,intersectSphereCone,intersectConeCone,intersectCylinderCylinder,intersectPlaneSphere,intersectPlaneCylinder,intersectPlaneCone,intersectPlaneTorus,intersectSphereTorus,intersectCylinderTorus,intersectConeTorus,intersectTorusTorus} from '../src/services/geometry/intersections'
import type {NurbsBrep} from '../src/services/geometry/brep'
import type {NurbsCurve} from '../src/services/nurbsCurve'
import {intersectionTraceToNurbsCurveSegments} from '../src/services/geometry/intersections'

it('admits the entire ruled trace interval before returning native curves',()=>{
 for(const middle of [3,.25,1]){
  const original={degreeU:2,degreeV:1,knotsU:[0,0,0,1,1,1],knotsV:[0,0,1,1],
   controlPoints:[-1,middle,-1].map((z,i)=>[[i/2,0,z],[i/2,1,z+2]]),weights:[[1,1],[1,1],[1,1]]}
  const transposed={degreeU:1,degreeV:2,knotsU:original.knotsV,knotsV:original.knotsU,
   controlPoints:original.controlPoints[0].map((_,v)=>original.controlPoints.map(row=>row[v])),
   weights:original.weights[0].map((_,v)=>original.weights.map(row=>row[v]))}
  for(const surface of [original,transposed])for(const reverse of [false,true]){
   const interval=(reverse?[1,0]:[0,1]) as [number,number]
   const plane={normal:[0,0,1] as [number,number,number],offset:0}
   const trace=surface.degreeV===1?{kind:'ruled' as const,surface,plane,uInterval:interval}
    :{kind:'ruled_u' as const,surface,plane,vInterval:interval}
   expect(()=>evaluateIntersectionTrace(trace,0)).not.toThrow()
   expect(()=>evaluateIntersectionTrace(trace,1)).not.toThrow()
   if(middle===3){
    expect(()=>evaluateIntersectionTrace(trace,.5)).toThrow('left the source domain')
    expect(()=>intersectionTraceToNurbsCurve(trace)).toThrow('leaves the source parameter domain')
    expect(()=>intersectionTraceToNurbsCurveSegments(trace)).toThrow('leaves the source parameter domain')
   }else{
    const pieces=intersectionTraceToNurbsCurveSegments(trace)
    expect(pieces).toHaveLength(1)
    for(let i=0;i<=40;i++){
     const t=i/40,p=evaluateNurbsCurve(pieces[0],t).point
     evaluateIntersectionTrace(trace,t).point.forEach((x,k)=>expect(Math.abs(x-p[k])).toBeLessThan(1e-10))
     expect(p[1]).toBeGreaterThanOrEqual(0);expect(p[1]).toBeLessThanOrEqual(1)
     expect(Math.abs(p[2])).toBeLessThan(1e-12)
    }
   }
  }
 }
})

it('refuses rounded-zero residuals that do not establish ruled domain admission',()=>{
 const source={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],
  controlPoints:[0,1].map(z=>[[268435456,1e-8,z],[268435456,4,z]]),weights:[[1,1],[1,1]]}
 const transposed={...source,controlPoints:source.controlPoints[0].map((_,v)=>source.controlPoints.map(row=>row[v]))}
 for(const [axis,surface] of [source,transposed].entries()){
  const plane={normal:[1,1,0] as [number,number,number],offset:268435456}
  const trace=axis===0?{kind:'ruled' as const,surface,plane,uInterval:[0,1] as [number,number]}
   :{kind:'ruled_u' as const,surface,plane,vInterval:[0,1] as [number,number]}
  expect(evaluateIntersectionTrace(trace,.5).planeResidual).toBe(0)
  for(const convert of [intersectionTraceToNurbsCurve,intersectionTraceToNurbsCurveSegments]){
   expect(()=>convert(trace)).toThrowError(expect.objectContaining({code:'BREP_INTERSECTION_UNRESOLVED'}))
  }
 }
})

it('preserves cropped ruled section fractions along either surface axis',()=>{
 const original={degreeU:2,degreeV:1,knotsU:[-3,-3,-3,-1,-1,5,5,5],knotsV:[10,10,14,14],
  controlPoints:Array.from({length:5},(_,i)=>[[i/2,0,0],[i/2,0,4]]),
  weights:[[1,2],[2,3],[3,4],[2,5],[1,3]]}
 const transposed={degreeU:1,degreeV:2,knotsU:original.knotsV,knotsV:original.knotsU,
  controlPoints:original.controlPoints[0].map((_,v)=>original.controlPoints.map(row=>row[v])),
  weights:original.weights[0].map((_,v)=>original.weights.map(row=>row[v]))}
 for(const surface of [original,transposed])for(const reverse of [false,true]){
  const interval=(reverse?[4,-2]:[-2,4]) as [number,number]
  const plane={normal:[.25,0,1] as [number,number,number],offset:2}
  const trace=surface.degreeV===1?{kind:'ruled' as const,surface,plane,uInterval:interval}
   :{kind:'ruled_u' as const,surface,plane,vInterval:interval}
  const pieces=intersectionTraceToNurbsCurveSegments(trace)
  const middle=reverse?5/6:1/6
  expect(pieces).toHaveLength(2)
  expect(pieces.map(c=>[c.knots[c.degree],c.knots[c.controlPoints.length]])).toEqual([[0,middle],[middle,1]])
  for(const curve of pieces){
   const lo=curve.knots[curve.degree],hi=curve.knots[curve.controlPoints.length]
   for(let i=0;i<=20;i++){
    const t=i===20?hi:lo+(hi-lo)*(i/20),p=evaluateNurbsCurve(curve,t).point
    evaluateIntersectionTrace(trace,t).point.forEach((x,k)=>expect(Math.abs(x-p[k])).toBeLessThan(1e-10))
    expect(Math.abs(.25*p[0]+p[2]-2)).toBeLessThan(1e-10)
   }
  }
 }
})

it('returns native rational pieces across nonuniform tensor knots without welding',()=>{
 let surface=createBrepSphere(2).faces[0].surface
 surface=insertNurbsSurfaceKnot(insertNurbsSurfaceKnot(surface,'u',.25),'u',.875)
 surface=insertNurbsSurfaceKnot(surface,'v',.5)
 for(const reverse of [false,true]){
  const trace={kind:'line' as const,surface,plane:{normal:[0,0,1] as [number,number,number],offset:0},
   start:(reverse?[1,1]:[0,0]) as [number,number],end:(reverse?[0,0]:[1,1]) as [number,number]}
  const pieces=intersectionTraceToNurbsCurveSegments(JSON.parse(JSON.stringify(trace)))
  const boundaries=reverse?[0,.125,.5,.75,1]:[0,.25,.5,.875,1]
  expect(pieces).toHaveLength(4)
  pieces.forEach((curve,index)=>{
   const lo=boundaries[index],hi=boundaries[index+1]
   expect(curve.knots[curve.degree]).toBe(lo)
   expect(curve.knots[curve.controlPoints.length]).toBe(hi)
   for(let i=0;i<=20;i++){
    const t=lo+(hi-lo)*i/20,p=evaluateNurbsCurve(curve,t).point
    evaluateIntersectionTrace(trace,t).point.forEach((x,k)=>expect(Math.abs(x-p[k])).toBeLessThan(1e-10))
    expect(Math.abs(p.reduce((sum,x)=>sum+x*x,0)-4)).toBeLessThan(1e-10)
   }
  })
 }
})

it('preserves plane intersections through WASM at extreme binary64 coefficient scales',()=>{
 const curve:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[0,0,0],[2,0,0]],weights:[1,1]}
 for(const magnitude of [Number.MIN_VALUE,2**-1022,1,Number.MAX_VALUE]){
  for(const sign of [-1,1]){
   const scale=magnitude*sign
   const report=intersectNurbsCurvePlane(curve,{normal:[scale,scale,0],offset:scale})
   expect(report.coverage).toBe('numerically_resolved')
   expect(report.permitsTopologyChange).toBe(false)
   expect(report.components).toHaveLength(1)
   const component=report.components[0]
   if(component.kind!=='point')throw Error('Expected root')
   expect(component.curve.parameter).toBe(.5)
   expect(component.curve.point).toEqual([1,0,0])
   expect(component.curve.planeResidual).toBe(0)
  }
 }
 expect(()=>intersectNurbsCurvePlane(curve,{normal:[Number.MIN_VALUE,0,0],offset:1})).toThrow('Normalized plane offset')
})

it('queries exact cylinder sections and re-evaluates retained traces after JSON roundtrip',()=>{
 const body=createBrepCylinder(2,4),surface=body.faces.find(f=>f.surface.degreeU===2)!.surface
 for(const plane of [{normal:[.2,-.1,1] as [number,number,number],offset:2},{normal:[1,0,0] as [number,number,number],offset:1}]){
  const report=intersectNurbsSurfacePlane(surface,plane)
  expect(report.coverage).toBe('numerically_resolved')
  expect(report.permitsTopologyChange).toBe(false)
  expect(report.evidence).toBe('numerical_uncertified')
  expect(report.components).toHaveLength(1)
  const component=report.components[0]
  if(component.kind!=='curve')throw Error('Expected section curve')
  const trace=JSON.parse(JSON.stringify(component.trace))
  for(let i=0;i<=25;i++){
   const sample=evaluateIntersectionTrace(trace,i/25)
   expect(sample.planeResidual).toBeLessThan(1e-9)
   expect(sample.point[0]**2+sample.point[1]**2).toBeCloseTo(4,10)
  }
 }
})
it('returns root intervals for rational arcs and explicit unresolved tangent/budget reports',()=>{
 const arc=createBrepCylinder(2,4).edges.find(e=>e.curve.degree===2)!.curve
 const roots=intersectNurbsCurvePlane(arc,{normal:[1,0,0],offset:1})
 expect(roots.components).toHaveLength(1)
 const hit=roots.components[0]
 if(hit.kind!=='point')throw Error('Expected point')
 expect(hit.curve.planeResidual).toBeLessThan(1e-9)
 expect(hit.curve.parameterInterval[1]-hit.curve.parameterInterval[0]).toBeLessThanOrEqual(1e-10)
 const tangent:NurbsCurve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0,.25],[.5,0,-.25],[1,0,.25]],weights:[1,1,1]}
 const report=intersectNurbsCurvePlane(tangent,{normal:[0,0,1],offset:0})
 expect(report.coverage).toBe('incomplete')
 expect(report.unresolved.some(u=>u.reason==='tangency_or_multiple_root')).toBe(true)
 const limited=intersectNurbsCurvePlane(arc,{normal:[1,0,0],offset:1},{maxBoxes:1})
 expect(limited.coverage).toBe('incomplete')
 expect(limited.boxesVisited).toBe(1)
 expect(limited.unresolved.some(u=>u.reason==='budget_exceeded')).toBe(true)
})
it('distinguishes untrimmed frustum sections and unsupported curved curve/surface queries',()=>{
 const body=createBrepFrustum(2,1,4),surface=body.faces.find(f=>f.surface.degreeU===2)!.surface
 const report=intersectNurbsSurfacePlane(surface,{normal:[0,0,1],offset:2})
 expect(report.coverage).toBe('numerically_resolved')
 const section=report.components[0]
 if(section.kind!=='curve')throw Error('Expected section curve')
 const sample=evaluateIntersectionTrace(section.trace,.31)
 expect(sample.point[0]**2+sample.point[1]**2).toBeCloseTo(2.25,10)
 const unsupported=intersectNurbsCurveSurface(body.edges[0].curve,surface)
 expect(unsupported.coverage).toBe('incomplete')
 expect(unsupported.unresolved[0].reason).toBe('unsupported_surface')
 expect(()=>evaluateIntersectionTrace(section.trace,2)).toThrow(/fraction/i)
})

it('roundtrips linear-U section traces through the WASM adapter in original UV coordinates',()=>{
 const original=createBrepCylinder(2,4).faces.find(f=>f.surface.degreeU===2)!.surface
 const surface={
  degreeU:original.degreeV,degreeV:original.degreeU,
  knotsU:original.knotsV.map(v=>-4+7*v),knotsV:original.knotsU.map(u=>3+2*u),
  controlPoints:original.controlPoints[0].map((_,v)=>original.controlPoints.map(row=>row[v])),
  weights:original.weights[0].map((_,v)=>original.weights.map(row=>row[v])),
  periodicU:original.periodicV,periodicV:original.periodicU,
 }
 for(const plane of [{normal:[.2,-.1,1] as [number,number,number],offset:2},{normal:[1,0,0] as [number,number,number],offset:1}]){
  const report=intersectNurbsSurfacePlane(surface,plane)
  expect(report.coverage).toBe('numerically_resolved')
  expect(report.permitsTopologyChange).toBe(false)
  expect(report.components).toHaveLength(1)
  const section=report.components[0]
  if(section.kind!=='curve')throw Error('Expected section')
  expect(section.trace.kind).toBe(plane.normal[2]===1?'ruled_u':'line')
  const trace=JSON.parse(JSON.stringify(section.trace))
  for(let i=0;i<=25;i++){
   const sample=evaluateIntersectionTrace(trace,i/25)
   expect(sample.uv[0]).toBeGreaterThanOrEqual(-4)
   expect(sample.uv[0]).toBeLessThanOrEqual(3)
   expect(sample.uv[1]).toBeGreaterThanOrEqual(3)
   expect(sample.uv[1]).toBeLessThanOrEqual(5)
   expect(sample.planeResidual).toBeLessThan(1e-9)
   expect(sample.point[0]**2+sample.point[1]**2).toBeCloseTo(4,10)
  }
  expect(()=>evaluateIntersectionTrace(trace,-.1)).toThrow(/fraction/i)
  const corrupted=JSON.parse(JSON.stringify(trace))
  if(corrupted.kind==='ruled_u')corrupted.vInterval[1]=6
  else corrupted.end[1]=6
  expect(()=>evaluateIntersectionTrace(corrupted,0)).toThrow(/domain/i)

 }
 const limited=intersectNurbsSurfacePlane(surface,{normal:[.2,-.1,1],offset:0},{maxBoxes:1})
 expect(limited.coverage).toBe('incomplete')
 expect(limited.unresolved.length).toBeGreaterThan(0)
 for(const region of limited.unresolved){
  expect(region.parameterBox[0]).toBeGreaterThanOrEqual(-4)
  expect(region.parameterBox[1]).toBeLessThanOrEqual(3)
  expect(region.parameterBox[2]).toBeGreaterThanOrEqual(3)
  expect(region.parameterBox[3]).toBeLessThanOrEqual(5)
 }
})

it('sections rational rulings with unequal endpoint weights through WASM',()=>{
 const surface=structuredClone(createBrepCylinder(2,4).faces.find(f=>f.surface.degreeU===2)!.surface)
 for(const row of surface.weights)row[1]*=3
 const report=intersectNurbsSurfacePlane(surface,{normal:[0,0,1],offset:2})
 expect(report.coverage).toBe('numerically_resolved')
 const section=report.components[0]
 if(section.kind!=='curve')throw Error('Expected section')
 for(let i=0;i<=20;i++){
  const point=evaluateIntersectionTrace(JSON.parse(JSON.stringify(section.trace)),i/20)
  expect(point.uv[1]).toBeCloseTo(.25,12)
  expect(point.point[2]).toBeCloseTo(2,12)
 }
 const base=surface.weights.map(row=>row[0])
 surface.weights.forEach((row,i)=>{row[1]=row[0]*[2,3,5][i]})
 const varying=intersectNurbsSurfacePlane(surface,{normal:[0,0,1],offset:2}).components[0]
 if(varying.kind!=='curve')throw Error('Expected varying-weight section')
 for(let i=0;i<=20;i++){
  const sample=evaluateIntersectionTrace(varying.trace,i/20),u=sample.uv[0]
  const bernstein=[(1-u)**2,2*u*(1-u),u*u]
  const lower=bernstein.reduce((sum,b,i)=>sum+b*base[i],0)
  const upper=bernstein.reduce((sum,b,i)=>sum+b*base[i]*[2,3,5][i],0)
  expect(sample.uv[1]).toBeCloseTo(lower/(lower+upper),12)
  expect(sample.point[2]).toBeCloseTo(2,12)
 }

})

it('retains linear-U section support after geometry-preserving knot refinement',()=>{
 const original=createBrepCylinder(2,4).faces.find(f=>f.surface.degreeU===2)!.surface
 const rows=original.controlPoints[0].map((_,v)=>original.controlPoints.map(row=>row[v]))
 const middle=rows[0].map((point,i)=>point.map((x,k)=>(x+rows[1][i][k])/2))
 const weights=original.weights.map(row=>row[0])
 const surface={degreeU:1,degreeV:original.degreeU,knotsU:[0,0,.5,1,1],knotsV:original.knotsU,
  controlPoints:[rows[0],middle,rows[1]],weights:[weights,weights,weights]}
 for(const height of [0,1,2,3,4]){
  const report=intersectNurbsSurfacePlane(surface,{normal:[0,0,1],offset:height})
  expect(report.coverage).toBe('numerically_resolved')
  expect(report.components).toHaveLength(1)
  const section=report.components[0]
  if(section.kind!=='curve')throw Error('Expected section')
  const sample=evaluateIntersectionTrace(JSON.parse(JSON.stringify(section.trace)),.37)
  expect(sample.uv[0]).toBeCloseTo(height/4,12)
  expect(sample.point[2]).toBeCloseTo(height,12)
 }
})

it('retains generator cuts when rational boundary weights differ by a positive scale',()=>{
 const surface=structuredClone(createBrepCylinder(2,4).faces.find(f=>f.surface.degreeU===2)!.surface)
 for(const row of surface.weights)row[1]*=3
 const report=intersectNurbsSurfacePlane(surface,{normal:[1,0,0],offset:1})
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.components).toHaveLength(1)
 const section=report.components[0]
 if(section.kind!=='curve')throw Error('Expected generator')
 for(let i=0;i<=20;i++){
  const t=i/20,point=evaluateIntersectionTrace(JSON.parse(JSON.stringify(section.trace)),t)
  expect(point.point[0]).toBeCloseTo(1,9)
  expect(point.point[2]).toBeCloseTo(12*t/(1+2*t),10)
 }
})

it('does not turn varying boundary weights into a generator or hide unresolved regions',()=>{
 const surface=structuredClone(createBrepCylinder(2,4).faces.find(f=>f.surface.degreeU===2)!.surface)
 surface.weights.forEach((row,i)=>{row[1]*=[2,3,5][i]})
 const report=intersectNurbsSurfacePlane(surface,{normal:[1,0,0],offset:1},{maxBoxes:128})
 expect(report.coverage).toBe('incomplete')
 expect(report.unresolved.length).toBeGreaterThan(0)
 expect(report.components.length).toBeGreaterThan(0)
 for(const component of report.components){
  if(component.kind!=='curve')throw Error('Expected curve')
  expect(component.trace.kind).toBe('ruled')
  for(let i=0;i<=20;i++)expect(evaluateIntersectionTrace(component.trace,i/20).planeResidual).toBeLessThan(1e-9)
 }
})

it('spends a bounded section budget across source spans before refining one difficult branch',()=>{
 const surface={degreeU:2,degreeV:1,knotsU:[0,0,0,.5,.5,1,1,1],knotsV:[0,0,1,1],
  controlPoints:[-2,-2,-2,5,-2].map((z,i)=>[[i,0,z],[i,1,i===3?5:2]]),weights:Array.from({length:5},()=>[1,1])}
 const report=intersectNurbsSurfacePlane(surface,{normal:[0,0,1],offset:0},{maxBoxes:8})
 expect(report.coverage).toBe('incomplete')
 expect(report.boxesVisited).toBeLessThanOrEqual(8)
 expect(report.components.some(c=>c.kind==='curve'&&JSON.stringify(c.parameterBox)===JSON.stringify([0,.5,0,1]))).toBe(true)
 expect(report.unresolved.length).toBeGreaterThan(0)
})

it('finds a later-span endpoint before exhausting the curve query budget on early roots',()=>{
 const curve={degree:2,knots:[0,0,0,.5,.5,1,1,1],controlPoints:[1,-2,1,1,0].map((z,i)=>[i,0,z]),weights:[1,1,1,1,1]}
 const report=intersectNurbsCurvePlane(curve,{normal:[0,0,1],offset:0},{maxBoxes:4})
 expect(report.coverage).toBe('incomplete')
 expect(report.boxesVisited).toBeLessThanOrEqual(4)
 expect(report.components.some(c=>c.kind==='point'&&c.curve.parameter===1)).toBe(true)
})

it('intersects a retained rational arc with a finite segment through the WASM adapter',()=>{
 const arc=createBrepCylinder(2,4).edges.find(e=>e.curve.degree===2)!.curve
 const report=intersectNurbsCurveSegment(arc,[0,1,0],[3,1,0])
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.evidence).toBe('numerical_uncertified')
 expect(report.components).toHaveLength(1)
 const hit=report.components[0]
 if(hit.kind!=='point')throw Error('Expected arc hit')
 expect(hit.segmentParameter).toBeCloseTo(Math.sqrt(3)/3,9)
 expect(hit.lineResidual).toBeLessThan(1e-9)
 expect(intersectNurbsCurveSegment(arc,[3,1,0],[4,1,0]).components).toHaveLength(0)
 const line={degree:1,knots:[0,0,1,1],controlPoints:[[0,0,0],[1,0,0]],weights:[1,1]}
 expect(intersectNurbsCurveSegment(line,[.25,0,0],[.75,0,0]).components).toEqual([{kind:'overlap',curveInterval:[.25,.75]}])
 expect(()=>intersectNurbsCurveSegment(arc,[0,0,0],[0,0,0])).toThrow(/nonzero/i)
})

it('clips rational linear coincidences using source parameters',()=>{
 const line={degree:1,knots:[0,0,1,1],controlPoints:[[0,0,0],[1,0,0]],weights:[1,3]}
 for(const [start,end] of [[[.25,0,0],[.75,0,0]],[[.75,0,0],[.25,0,0]]] as [[number,number,number],[number,number,number]][]){
  const report=intersectNurbsCurveSegment(line,start,end)
  expect(report.coverage).toBe('numerically_resolved')
  const overlap=report.components[0]
  if(overlap.kind!=='overlap')throw Error('Expected overlap')
  expect(overlap.curveInterval[0]).toBeCloseTo(.1,12)
  expect(overlap.curveInterval[1]).toBeCloseTo(.5,12)
 }
 const contact=intersectNurbsCurveSegment(line,[1,0,0],[2,0,0]).components[0]
 if(contact.kind!=='point')throw Error('Expected endpoint')
 expect(contact.curve.parameter).toBe(1)
 expect(contact.segmentParameter).toBe(0)
})

it('returns one curve-parameter contact at a shared knot after segment clipping',()=>{
 const curve={degree:1,knots:[0,0,.5,1,1],controlPoints:[[-1,0,0],[0,0,0],[-1,0,0]],weights:[1,1,1]}
 const report=intersectNurbsCurveSegment(curve,[0,0,0],[1,0,0])
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.components).toHaveLength(1)
 const contact=report.components[0]
 if(contact.kind!=='point')throw Error('Expected contact')
 expect(contact.curve.parameter).toBe(.5)
 expect(contact.segmentParameter).toBe(0)
})

it('clips higher-degree coincident curves without replacing their parameterization',()=>{
 const curve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0,0],[0,0,0],[1,0,0]],weights:[1,1,1]}
 for(const [start,end] of [[[.25,0,0],[.5625,0,0]],[[.5625,0,0],[.25,0,0]]] as [[number,number,number],[number,number,number]][]){
  const report=intersectNurbsCurveSegment(curve,start,end)
  expect(report.coverage).toBe('numerically_resolved')
  expect(report.components).toEqual([{kind:'overlap',curveInterval:[.5,.75]}])
 }
 const uncertain=intersectNurbsCurveSegment(curve,[.3,0,0],[.6,0,0])
 expect(uncertain.coverage).toBe('incomplete')
 expect(uncertain.unresolved.length).toBeGreaterThan(0)
 expect(uncertain.boxesVisited).toBeLessThanOrEqual(8192)
 const limited=intersectNurbsCurveSegment(curve,[.25,0,0],[.5625,0,0],{maxBoxes:3})
 expect(limited.coverage).toBe('incomplete')
 expect(limited.boxesVisited).toBeLessThanOrEqual(3)
})

it('keeps separate visits of a backtracking coincident curve',()=>{
 const curve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0,0],[2,0,0],[0,0,0]],weights:[1,1,1]}
 const report=intersectNurbsCurveSegment(curve,[.4375,0,0],[.75,0,0])
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.components).toEqual([{kind:'overlap',curveInterval:[.125,.25]},{kind:'overlap',curveInterval:[.75,.875]}])
 const uncertain=intersectNurbsCurveSegment(curve,[.3,0,0],[.7,0,0])
 expect(uncertain.coverage).toBe('incomplete')
 for(const q of [.3,.7])for(const root of [(1-Math.sqrt(1-q))/2,(1+Math.sqrt(1-q))/2]){
  expect(uncertain.unresolved.some(r=>r.parameterBox[0]<=root&&r.parameterBox[1]>=root)).toBe(true)
 }
})

it('clips coplanar curved and edge-coincident curves to a finite affine surface',()=>{
 const surface={degreeU:1,degreeV:1,knotsU:[2,2,4,4],knotsV:[-1,-1,3,3],
  controlPoints:[[[.25,0,0],[.25,1,0]],[[.75,0,0],[.75,1,0]]],weights:[[1,1],[1,1]]}
 for(const points of [[[0,0,0],[.5,0,0],[1,1,0]],[[0,0,0],[.5,0,0],[1,0,0]]]){
  const curve={degree:2,knots:[0,0,0,1,1,1],controlPoints:points,weights:[1,1,1]}
  const report=intersectNurbsCurveSurface(curve,surface)
  expect(report.coverage).toBe('numerically_resolved')
  expect(report.components).toEqual([{kind:'overlap',curveInterval:[.25,.75]}])
  const limited=intersectNurbsCurveSurface(curve,surface,{maxBoxes:2})
  expect(limited.coverage).toBe('incomplete')
  expect(limited.boxesVisited).toBeLessThanOrEqual(2)
 }
})

it('preserves affine shear and reports a rectangle corner only once',()=>{
 const curve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0,0],[.5,0,0],[2,1,0]],weights:[1,1,1]}
 const surface={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],
  controlPoints:[[[.25,0,0],[1.25,1,0]],[[.75,0,0],[1.75,1,0]]],weights:[[1,1],[1,1]]}
 const report=intersectNurbsCurveSurface(curve,surface)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.components).toEqual([{kind:'overlap',curveInterval:[.25,.75]}])
 const cornerSurface={...surface,controlPoints:[[[0,0,0],[1,1,0]],[[1,0,0],[2,1,0]]]}
 const diagonal={degree:1,knots:[0,0,1,1],controlPoints:[[0,1,0],[0,-1,0]],weights:[1,1]}
 const contact=intersectNurbsCurveSurface(diagonal,cornerSurface)
 expect(contact.coverage).toBe('numerically_resolved')
 expect(contact.components).toHaveLength(1)
 const hit=contact.components[0]
 if(hit.kind!=='point')throw Error('Expected corner')
 expect(hit.curve.parameter).toBe(.5)
 expect(hit.uv).toEqual([0,0])
})

it('intersects finite affine surface pairs with corresponding retained UV traces',()=>{
 const first={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],
  controlPoints:[[[-1,-1,0],[-1,1,0]],[[1,-1,0],[1,1,0]]],weights:[[1,1],[1,1]]}
 const second={...first,controlPoints:[[[-.5,0,-1],[-.5,0,1]],[[.5,0,-1],[.5,0,1]]]}
 const report=intersectNurbsSurfaceSurface(first,second)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.components).toHaveLength(1)
 const hit=report.components[0]
 if(hit.kind!=='curve')throw Error('Expected segment')
 for(let i=0;i<=20;i++){
  const a=evaluateIntersectionTrace(JSON.parse(JSON.stringify(hit.first)),i/20)
  const b=evaluateIntersectionTrace(JSON.parse(JSON.stringify(hit.second)),i/20)
  a.point.forEach((x,k)=>expect(x).toBeCloseTo(b.point[k],12))
  expect(a.point[0]).toBeGreaterThanOrEqual(-.5)
  expect(a.point[0]).toBeLessThanOrEqual(.5)
 }
 expect(intersectNurbsSurfaceSurface(first,first).components[0].kind).toBe('overlap')
 expect(intersectNurbsSurfaceSurface(first,second,{maxBoxes:1}).coverage).toBe('incomplete')
 const touching={...second,controlPoints:second.controlPoints.map(row=>row.map(p=>[p[0]+1.5,p[1],p[2]]))}
 const contact=intersectNurbsSurfaceSurface(first,touching)
 expect(contact.coverage).toBe('numerically_resolved')
 expect(contact.components).toHaveLength(1)
 expect(contact.components[0].kind).toBe('point')
 const outside={...second,controlPoints:second.controlPoints.map(row=>row.map(p=>[p[0]+2.5,p[1],p[2]]))}
 expect(intersectNurbsSurfaceSurface(first,outside).components).toHaveLength(0)
 const parallel={...first,controlPoints:first.controlPoints.map(row=>row.map(p=>[p[0],p[1],1]))}
 expect(intersectNurbsSurfaceSurface(first,parallel).coverage).toBe('numerically_resolved')
 expect(intersectNurbsSurfaceSurface(first,parallel).components).toHaveLength(0)
 const curved=createBrepCylinder(2,4).faces.find(f=>f.surface.degreeU===2)!.surface
 expect(intersectNurbsSurfaceSurface(first,curved).unresolved[0].reason).toBe('unsupported_surface')

})

it('retains the paired UV polygon of a coplanar affine overlap',()=>{
 const square={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],
  controlPoints:[[[-1,-1,0],[-1,1,0]],[[1,-1,0],[1,1,0]]],weights:[[1,1],[1,1]]}
 const diamond={...square,controlPoints:[[[0,-1.5,0],[-1.5,0,0]],[[1.5,0,0],[0,1.5,0]]]}
 const report=intersectNurbsSurfaceSurface(square,diamond)
 expect(report.coverage).toBe('numerically_resolved')
 const overlap=report.components[0]
 if(overlap.kind!=='overlap')throw Error('Expected area')
 expect(overlap.points).toHaveLength(8)
 expect(overlap.firstBoundary).toHaveLength(8)
 expect(overlap.secondBoundary).toHaveLength(8)
 const area=overlap.points.reduce((sum,a,i)=>{const b=overlap.points[(i+1)%8];return sum+a[0]*b[1]-a[1]*b[0]},0)/2
 expect(area).toBeCloseTo(3.5,12)
 for(const [dx,dy,kind] of [[2,0,'curve'],[2,2,'point'],[3,0,'empty']] as [number,number,string][]){
  const target={...square,controlPoints:square.controlPoints.map(row=>row.map(p=>[p[0]+dx,p[1]+dy,p[2]]))}
  const result=intersectNurbsSurfaceSurface(square,target)
  expect(result.coverage).toBe('numerically_resolved')
  if(kind==='empty')expect(result.components).toHaveLength(0)
  else expect(result.components[0].kind).toBe(kind)
 }
})

it('keeps coplanar area correspondence under operand and UV orientation changes',()=>{
 const square={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],
  controlPoints:[[[-1,-1,0],[-1,1,0]],[[1,-1,0],[1,1,0]]],weights:[[1,1],[1,1]]}
 const diamond={...square,controlPoints:[[[0,-1.5,0],[-1.5,0,0]],[[1.5,0,0],[0,1.5,0]]]}
 for(const swap of [false,true])for(const reverse of [false,true])for(const transpose of [false,true]){
  const a=structuredClone(swap?diamond:square),b=structuredClone(swap?square:diamond)
  if(reverse){a.controlPoints.reverse();a.weights.reverse()}
  if(transpose){b.controlPoints=b.controlPoints[0].map((_,v)=>b.controlPoints.map(row=>row[v]));b.weights=b.weights[0].map((_,v)=>b.weights.map(row=>row[v]))}
  a.knotsU=a.knotsU.map(t=>-3+8*t);b.knotsV=b.knotsV.map(t=>10+4*t)
  b.weights=b.weights.map(row=>row.map(w=>32*w))
  const report=intersectNurbsSurfaceSurface(a,b)
  expect(report.coverage).toBe('numerically_resolved')
  const hit=report.components[0]
  if(hit.kind!=='overlap')throw Error('Expected area')
  expect(hit.points).toHaveLength(8)
  const area=Math.abs(hit.points.reduce((sum,p,i)=>{const q=hit.points[(i+1)%8];return sum+p[0]*q[1]-p[1]*q[0]},0))/2
  expect(area).toBeCloseTo(3.5,11)
  hit.points.forEach((point,i)=>{
   const p=evaluateNurbsSurface(a,...hit.firstBoundary[i]).point,q=evaluateNurbsSurface(b,...hit.secondBoundary[i]).point
   point.forEach((x,k)=>{expect(x).toBeCloseTo(p[k],11);expect(x).toBeCloseTo(q[k],11)})
  })
 }
})

it('converts a retained rational section to a NURBS curve without sample fitting',()=>{
 const surface=structuredClone(createBrepCylinder(2,4).faces.find(f=>f.surface.degreeU===2)!.surface)
 surface.weights.forEach(row=>{row[1]*=3})
 for(const plane of [{normal:[.2,-.1,1] as [number,number,number],offset:2},{normal:[1,0,0] as [number,number,number],offset:1}]){
  const section=intersectNurbsSurfacePlane(surface,plane).components[0]
  if(section.kind!=='curve')throw Error('Expected section')
  const curve=JSON.parse(JSON.stringify(intersectionTraceToNurbsCurve(section.trace)))
  const a=curve.knots[curve.degree],b=curve.knots[curve.controlPoints.length]
  for(let i=0;i<=40;i++){
   const t=i/40,expected=evaluateIntersectionTrace(section.trace,t).point,actual=evaluateNurbsCurve(curve,a+t*(b-a)).point
   expected.forEach((x,k)=>expect(x).toBeCloseTo(actual[k],10))
   expect(actual[0]**2+actual[1]**2).toBeCloseTo(4,10)
   expect(Math.abs(actual.reduce((sum,x,k)=>sum+x*plane.normal[k],0)-plane.offset)).toBeLessThan(1e-9)

  }
 }
})

it('preserves reversed trace fractions on shifted knot domains through WASM',()=>{
 const source=structuredClone(createBrepCylinder(2,4).faces.find(f=>f.surface.degreeU===2)!.surface)
 source.knotsU=source.knotsU.map(t=>-3+8*t)
 source.knotsV=source.knotsV.map(t=>10+4*t)
 const transposed={...source,degreeU:source.degreeV,degreeV:source.degreeU,knotsU:source.knotsV,knotsV:source.knotsU,
  controlPoints:source.controlPoints[0].map((_,v)=>source.controlPoints.map(row=>row[v])),
  weights:source.weights[0].map((_,v)=>source.weights.map(row=>row[v]))}
 for(const surface of [source,transposed])for(const plane of [{normal:[0,0,1] as [number,number,number],offset:2},{normal:[1,0,0] as [number,number,number],offset:1}]){
  const section=intersectNurbsSurfacePlane(surface,plane).components[0]
  if(section.kind!=='curve')throw Error('Expected section')
  const reverse=structuredClone(section.trace)
  if(reverse.kind==='line')[reverse.start,reverse.end]=[reverse.end,reverse.start]
  else if(reverse.kind==='ruled')reverse.uInterval.reverse()
  else reverse.vInterval.reverse()
  const curve=intersectionTraceToNurbsCurve(reverse)
  const a=curve.knots[curve.degree],b=curve.knots[curve.controlPoints.length]
  for(let i=0;i<=20;i++){
   const actual=evaluateNurbsCurve(curve,a+(b-a)*i/20).point
   evaluateIntersectionTrace(section.trace,1-i/20).point.forEach((x,k)=>expect(Math.abs(x-actual[k])).toBeLessThan(1e-10))
  }
 }
})

it('refuses ambiguous ruling conversion even when both endpoints evaluate',()=>{
 const trace={kind:'ruled' as const,surface:{degreeU:2,degreeV:1,knotsU:[0,0,0,1,1,1],knotsV:[0,0,1,1],
  controlPoints:[-1,2,-1].map((z,i)=>[[i,0,z],[i,1,-z]]),weights:[[1,1],[1,1],[1,1]],periodicU:false,periodicV:false},
  plane:{normal:[0,0,1] as [number,number,number],offset:0},uInterval:[0,1] as [number,number]}
 expect(evaluateIntersectionTrace(trace,0).planeResidual).toBe(0)
 expect(evaluateIntersectionTrace(trace,1).planeResidual).toBe(0)
 expect(()=>intersectionTraceToNurbsCurve(trace)).toThrow()
})

it('converts a multi-span ruled trace to one parameter-preserving NURBS curve',()=>{
 const trace={kind:'ruled' as const,surface:{degreeU:2,degreeV:1,
  knotsU:[0,0,0,.5,.5,1,1,1],knotsV:[0,0,1,1],
  controlPoints:[[0,0],[.5,1],[1,0],[1.5,-1],[2,0]].map(([x,y])=>[[x,y,0],[x,y,4]]),
  weights:Array.from({length:5},()=>[1,2]),periodicU:false,periodicV:false},
  plane:{normal:[0,0,1] as [number,number,number],offset:2},uInterval:[0,1] as [number,number]}
 for(const interval of [[0,1],[1,0]] as [number,number][]){
  trace.uInterval=interval
  const curve=intersectionTraceToNurbsCurve(trace)
  expect(curve.degree).toBe(4)
  expect(curve.controlPoints).toHaveLength(9)
  for(let i=0;i<=100;i++){
   const actual=evaluateNurbsCurve(curve,i/100).point
   evaluateIntersectionTrace(trace,i/100).point.forEach((x,k)=>expect(Math.abs(x-actual[k])).toBeLessThan(1e-10))
  }
 }
})

it('preserves oblique multi-span sections with independently varying boundary weights',()=>{
 const source={degreeU:2,degreeV:1,knotsU:[0,0,0,.5,.5,1,1,1],knotsV:[0,0,1,1],
  controlPoints:Array.from({length:5},(_,i)=>[[i/2,0,0],[i/2,0,4]]),
  weights:[[1,2],[2,3],[3,4],[2,5],[1,3]],periodicU:false,periodicV:false}
 const transpose={...source,degreeU:1,degreeV:2,knotsU:source.knotsV,knotsV:source.knotsU,
  controlPoints:source.controlPoints[0].map((_,v)=>source.controlPoints.map(row=>row[v])),
  weights:source.weights[0].map((_,v)=>source.weights.map(row=>row[v]))}
 for(const surface of [source,transpose])for(const interval of [[0,1],[1,0]] as [number,number][]){
  const plane={normal:[.25,0,1] as [number,number,number],offset:2}
  const trace=surface.degreeV===1?{kind:'ruled' as const,surface,plane,uInterval:interval}:{kind:'ruled_u' as const,surface,plane,vInterval:interval}
  const curve=intersectionTraceToNurbsCurve(trace)
  for(let i=0;i<=100;i++){
   const p=evaluateNurbsCurve(curve,i/100).point
   evaluateIntersectionTrace(trace,i/100).point.forEach((x,k)=>expect(Math.abs(x-p[k])).toBeLessThan(1e-10))
   expect(Math.abs(.25*p[0]+p[2]-2)).toBeLessThan(1e-10)
  }
 }
})

it('lifts tensor-patch UV diagonals to rational curves without fitting',()=>{
 const surface=structuredClone(createBrepSphere(2).faces[0].surface)
 surface.knotsU=surface.knotsU.map(t=>-3+8*t)
 surface.knotsV=surface.knotsV.map(t=>10+4*t)
 for(const u of [[-2.2,4.2],[4.2,-2.2]])for(const v of [[10.8,13.2],[13.2,10.8]]){
  const trace={kind:'line' as const,surface,plane:{normal:[0,0,1] as [number,number,number],offset:0},
   start:[u[0],v[0]] as [number,number],end:[u[1],v[1]] as [number,number]}
  const curve=intersectionTraceToNurbsCurve(trace)
  expect(curve.degree).toBe(surface.degreeU+surface.degreeV)
  for(let i=0;i<=100;i++){
   const p=evaluateNurbsCurve(curve,i/100).point
   evaluateIntersectionTrace(trace,i/100).point.forEach((x,k)=>expect(Math.abs(x-p[k])).toBeLessThan(1e-10))
   expect(Math.abs(p.reduce((sum,x)=>sum+x*x,0)-4)).toBeLessThan(1e-10)
  }
 }
})

it('intersects two retained curves through WASM with paired source parameters',()=>{
 const first:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[0,0,0],[2,0,0]],weights:[1,1]}
 const second:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[1,-1,0],[1,1,0]],weights:[1,1]}
 const report=intersectNurbsCurveCurve(first,second)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.evidence).toBe('numerical_uncertified')
 expect(report.components).toHaveLength(1)
 const hit=JSON.parse(JSON.stringify(report.components[0]))
 if(hit.kind!=='point')throw Error('Expected crossing point')
 // Independent oracle: A(t)=(2t,0,0), B(u)=(1,2u-1,0) cross at t=u=1/2.
 expect(hit.first).toBeCloseTo(.5,10)
 expect(hit.second).toBeCloseTo(.5,10)
 hit.point.forEach((x:number,k:number)=>expect(x).toBeCloseTo([1,0,0][k],9))
 expect(hit.residual).toBeLessThan(1e-9)
 expect(hit.contact).toBe('transverse')
 const swapped=intersectNurbsCurveCurve(second,first)
 const mapped=swapped.components[0]
 if(mapped.kind!=='point')throw Error('Expected swapped point')
 expect(mapped.first).toBeCloseTo(hit.second,12)
 expect(mapped.second).toBeCloseTo(hit.first,12)
})

it('resolves a line crossing a rational quarter circle and rejects tangencies',()=>{
 const w=Math.SQRT1_2
 const arc:NurbsCurve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[1,0,0],[1,1,0],[0,1,0]],weights:[1,w,1]}
 const diagonal:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[0,0,0],[1,1,0]],weights:[1,1]}
 const report=intersectNurbsCurveCurve(diagonal,arc)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.components).toHaveLength(1)
 const hit=report.components[0]
 if(hit.kind!=='point')throw Error('Expected arc crossing')
 expect(hit.first).toBeCloseTo(Math.SQRT1_2,9)
 expect(hit.second).toBeCloseTo(.5,9)
 expect(hit.point[0]**2+hit.point[1]**2).toBeCloseTo(1,10)
 // Tangent contact: parabola y=2t(1-t) against y=1/2 stays unresolved.
 const parabola:NurbsCurve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[0,0,0],[1,1,0],[2,0,0]],weights:[1,1,1]}
 const tangent=intersectNurbsCurveCurve(parabola,{degree:1,knots:[0,0,1,1],controlPoints:[[0,.5,0],[2,.5,0]],weights:[1,1]})
 expect(tangent.coverage).toBe('incomplete')
 expect(tangent.components).toHaveLength(0)
 expect(tangent.unresolved.some(u=>u.reason==='tangency_or_multiple_root')).toBe(true)
 expect(tangent.unresolved.every(u=>u.parameterBox.length===4)).toBe(true)
 const limited=intersectNurbsCurveCurve(diagonal,arc,{maxBoxes:1})
 expect(limited.coverage).toBe('incomplete')
 expect(limited.boxesVisited).toBeLessThanOrEqual(1)
 expect(limited.unresolved.some(u=>u.reason==='budget_exceeded')).toBe(true)
})

it('clips coincident collinear curve pairs to explicit trim intervals on both',()=>{
 const a:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[0,0,0],[1,0,0]],weights:[1,1]}
 for(const reversed of [false,true]){
  const b:NurbsCurve=reversed
   ?{degree:1,knots:[0,0,1,1],controlPoints:[[1.5,0,0],[.5,0,0]],weights:[1,1]}
   :{degree:1,knots:[0,0,1,1],controlPoints:[[.5,0,0],[1.5,0,0]],weights:[1,1]}
  const report=intersectNurbsCurveCurve(a,b)
  expect(report.coverage).toBe('numerically_resolved')
  expect(report.components).toHaveLength(1)
  const overlap=report.components[0]
  if(overlap.kind!=='overlap')throw Error('Expected overlap')
  expect(overlap.firstInterval).toEqual([.5,1])
  expect(overlap.secondInterval).toEqual(reversed?[.5,1]:[0,.5])
  expect(overlap.reversed).toBe(reversed)
  expect(overlap.maxControlResidual).toBeLessThanOrEqual(1e-9)
 }
 // Shared endpoint: exactly one boundary event with exact knot parameters.
 const tip:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[1,0,0],[2,1,0]],weights:[1,1]}
 const contact=intersectNurbsCurveCurve(a,tip)
 expect(contact.components).toHaveLength(1)
 const event=contact.components[0]
 if(event.kind!=='point')throw Error('Expected endpoint event')
 expect(event.first).toBe(1)
 expect(event.second).toBe(0)
 expect(event.contact).toBe('boundary')
 // Distinct parallel lines stay a resolved empty result.
 const parallel:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[0,1,0],[1,1,0]],weights:[1,1]}
 const disjoint=intersectNurbsCurveCurve(a,parallel)
 expect(disjoint.coverage).toBe('numerically_resolved')
 expect(disjoint.components).toHaveLength(0)
})

it('keeps curve/curve parameters under knot shift and weight scaling through WASM',()=>{
 const a:NurbsCurve={degree:1,knots:[3,3,7,7],controlPoints:[[0,0,0],[2,0,0]],weights:[7,7]}
 const b:NurbsCurve={degree:1,knots:[-2,-2,0,0],controlPoints:[[1,-1,0],[1,1,0]],weights:[.5,.5]}
 const report=intersectNurbsCurveCurve(a,b)
 expect(report.coverage).toBe('numerically_resolved')
 const hit=report.components[0]
 if(hit.kind!=='point')throw Error('Expected shifted crossing')
 expect(hit.first).toBeCloseTo(5,9)
 expect(hit.second).toBeCloseTo(-1,9)
 const flat={degree:1,knots:[0,0,1,1],controlPoints:[[0,0],[1,0]],weights:[1,1]} as unknown as NurbsCurve
 expect(()=>intersectNurbsCurveCurve(flat,a)).toThrow()
 expect(()=>intersectNurbsCurveCurve(a,b,{maxDepth:0})).toThrow()
})

const ruledCylinder=(top:number)=>{
 const w=Math.SQRT1_2
 const ring=[[2,0],[2,2],[0,2],[-2,2],[-2,0],[-2,-2],[0,-2],[2,-2],[2,0]]
 const weights=[1,w,1,w,1,w,1,w,1]
 return {degreeU:2,degreeV:1,knotsU:[0,0,0,.25,.25,.5,.5,.75,.75,1,1,1],knotsV:[0,0,1,1],
  controlPoints:ring.map(([x,y])=>[[x,y,0],[x,y,4]]),
  weights:weights.map(w0=>[w0,w0*top]),periodicU:false,periodicV:false}
}

it('intersects a curve with a ruled surface through WASM with (t,u,v) parameters',()=>{
 const plane={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],
  controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]]}
 const curve:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[.25,.25,-1],[.25,.25,1]],weights:[1,1]}
 const report=intersectNurbsCurveRuledSurface(curve,plane)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.evidence).toBe('numerical_uncertified')
 expect(report.components).toHaveLength(1)
 const hit=JSON.parse(JSON.stringify(report.components[0]))
 if(hit.kind!=='point')throw Error('Expected piercing point')
 // Independent oracle: C(t)=(1/4,1/4,2t-1), S(u,v)=(u,v,0); t=1/2, u=v=1/4.
 expect(hit.t).toBeCloseTo(.5,10)
 expect(hit.uv[0]).toBeCloseTo(.25,10)
 expect(hit.uv[1]).toBeCloseTo(.25,10)
 hit.point.forEach((x:number,k:number)=>expect(x).toBeCloseTo([.25,.25,0][k],9))
 expect(hit.residual).toBeLessThan(1e-9)
 expect(hit.contact).toBe('transverse')
 expect(hit.uvBox).toHaveLength(4)
 const reversed=intersectNurbsCurveRuledSurface({...curve,controlPoints:[...curve.controlPoints].reverse()} as NurbsCurve,plane)
 const mirrored=reversed.components[0]
 if(mirrored.kind!=='point')throw Error('Expected reversed point')
 expect(mirrored.t).toBeCloseTo(.5,10)
 expect(mirrored.uv).toEqual(hit.uv)
})

it('pierces a rational ruled cylinder twice at oracle parameters through WASM',()=>{
 const surface=ruledCylinder(1)
 // Line y=x at z=2 hits the circle at 45 and 225 degrees: midpoints of
 // quadratic spans 0 and 2, so u=1/8 and 5/8, v=1/2.
 const curve:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[-3,-3,2],[3,3,2]],weights:[1,1]}
 const report=intersectNurbsCurveRuledSurface(curve,surface)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.components).toHaveLength(2)
 const s2=Math.SQRT2,oracle=[[(3-s2)/6,.625],[(3+s2)/6,.125]]
 report.components.forEach((component,i)=>{
  if(component.kind!=='point')throw Error('Expected point')
  const [t,u]=oracle[i]
  expect(component.t).toBeCloseTo(t,9)
  expect(component.uv[0]).toBeCloseTo(u,9)
  expect(component.uv[1]).toBeCloseTo(.5,9)
  expect(component.point[0]**2+component.point[1]**2).toBeCloseTo(4,9)
  expect(component.point[2]).toBeCloseTo(2,9)
  expect(component.residual).toBeLessThan(1e-9)
 })
 // A displaced line misses entirely with resolved-empty coverage.
 const miss:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[-3,-3,6],[3,3,6]],weights:[1,1]}
 const empty=intersectNurbsCurveRuledSurface(miss,surface)
 expect(empty.coverage).toBe('numerically_resolved')
 expect(empty.components).toHaveLength(0)
})

it('reports ruling and iso-v coincidences as overlaps with lifted UV paths',()=>{
 const surface=ruledCylinder(3)
 // Ruling at 45 degrees (u=1/8): z(v)=4·3v/(1+2v), so z=1 -> v=.1, z=3 -> v=.5.
 const ruling:NurbsCurve={degree:1,knots:[0,0,1,1],
  controlPoints:[[Math.SQRT2,Math.SQRT2,1],[Math.SQRT2,Math.SQRT2,3]],weights:[1,1]}
 const report=intersectNurbsCurveRuledSurface(ruling,surface)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.components).toHaveLength(1)
 const overlap=JSON.parse(JSON.stringify(report.components[0]))
 if(overlap.kind!=='overlap')throw Error('Expected ruling overlap')
 expect(overlap.curveInterval).toEqual([0,1])
 expect(overlap.uvStart[0]).toBeCloseTo(.125,9)
 expect(overlap.uvEnd[0]).toBeCloseTo(.125,9)
 expect(overlap.uvStart[1]).toBeCloseTo(.1,9)
 expect(overlap.uvEnd[1]).toBeCloseTo(.5,9)
 expect(overlap.maxControlResidual).toBeLessThan(1e-9)
 // Endpoint correspondence only: re-evaluate the lifted ends on the surface.
 const start=evaluateNurbsSurface(surface,overlap.uvStart[0],overlap.uvStart[1]).point
 const end=evaluateNurbsSurface(surface,overlap.uvEnd[0],overlap.uvEnd[1]).point
 start.forEach((x,k)=>expect(x).toBeCloseTo([Math.SQRT2,Math.SQRT2,1][k],9))
 end.forEach((x,k)=>expect(x).toBeCloseTo([Math.SQRT2,Math.SQRT2,3][k],9))
 // Iso-v coincidence: the bottom directrix arc of an open ruled patch.
 const w=Math.SQRT1_2,bottom=[[2,0,0],[2,2,0],[0,2,0]]
 const patch={degreeU:2,degreeV:1,knotsU:[0,0,0,1,1,1],knotsV:[0,0,1,1],
  controlPoints:bottom.map(p=>[p,[p[0],p[1],4]]),
  weights:[[1,1],[w,w],[1,1]],periodicU:false,periodicV:false}
 const arc:NurbsCurve={degree:2,knots:[0,0,0,1,1,1],controlPoints:bottom,weights:[1,w,1]}
 const iso=intersectNurbsCurveRuledSurface(arc,patch)
 expect(iso.coverage).toBe('numerically_resolved')
 const component=iso.components[0]
 if(component.kind!=='overlap')throw Error('Expected iso-v overlap')
 expect(component.curveInterval).toEqual([0,1])
 expect(component.uvStart).toEqual([0,0])
 expect(component.uvEnd).toEqual([1,0])
})

it('keeps ruled-surface tangency, budget and unsupported regions explicit through WASM',()=>{
 const surface=ruledCylinder(1)
 // Tangent to the cylinder at (2,0,2): never a guessed point.
 const tangent:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[2,-1,2],[2,1,2]],weights:[1,1]}
 const grazing=intersectNurbsCurveRuledSurface(tangent,surface)
 expect(grazing.coverage).toBe('incomplete')
 expect(grazing.components.filter(c=>c.kind==='point')).toHaveLength(0)
 expect(grazing.unresolved.some(u=>u.reason==='tangency_or_multiple_root'&&u.parameterBox.length===6)).toBe(true)
 // Budget exhaustion preserves pending 6-parameter regions.
 const pierce:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[-3,-3,2],[3,3,2]],weights:[1,1]}
 const limited=intersectNurbsCurveRuledSurface(pierce,surface,{maxBoxes:2})
 expect(limited.coverage).toBe('incomplete')
 expect(limited.boxesVisited).toBeLessThanOrEqual(2)
 expect(limited.unresolved.length).toBeGreaterThan(0)
 expect(limited.unresolved.every(u=>u.reason==='budget_exceeded'&&u.parameterBox.length===6)).toBe(true)
 // A biquadratic tensor patch is an explicit unsupported region.
 const curved={degreeU:2,degreeV:2,knotsU:[0,0,0,1,1,1],knotsV:[0,0,0,1,1,1],
  controlPoints:[0,1,2].map(i=>[0,1,2].map(j=>[i,j,i*j*.25])),weights:[[1,1,1],[1,1,1],[1,1,1]]}
 const refused=intersectNurbsCurveRuledSurface(pierce,curved)
 expect(refused.coverage).toBe('incomplete')
 expect(refused.unresolved[0].reason).toBe('unsupported_surface')
})

it('unifies seam contacts on a closed ruled cylinder through WASM',()=>{
 const r3=Math.sqrt(3)
 for(const top of [1,3]){
  const surface=ruledCylinder(top)
  // Chord through the seam point (2,0,2) and the 30-degree point (sqrt3,1,2),
  // non-dyadic offsets: seam crossing at t=7/31, second crossing at t=28/31.
  const d=[r3-2,1]
  const curve:NurbsCurve={degree:1,knots:[0,0,1,1],
   controlPoints:[[2-d[0]/3,-d[1]/3,2],[r3+d[0]/7,1+d[1]/7,2]],weights:[1,1]}
  const report=intersectNurbsCurveRuledSurface(curve,surface)
  expect(report.coverage).toBe('numerically_resolved')
  expect(report.components).toHaveLength(2)
  const seam=report.components.filter(c=>c.kind==='point'&&(c.uv[0]<1e-6||c.uv[0]>1-1e-6))
  // One event for the seam contact — never one per seam side.
  expect(seam).toHaveLength(1)
  const hit=report.components[0]
  if(hit.kind!=='point')throw Error('Expected point')
  expect(hit.t).toBeCloseTo(7/31,9)
  expect(hit.uv[1]).toBeCloseTo(.5/(top-.5*(top-1)),9)
  const second=report.components[1]
  if(second.kind!=='point')throw Error('Expected point')
  expect(second.t).toBeCloseTo(28/31,9)
  expect(second.point[0]).toBeCloseTo(r3,9)
  expect(second.point[1]).toBeCloseTo(1,9)
 }
 // Endpoint exactly on the seam directrix: admitted from both seam sides,
 // unified to one canonical u=0 boundary event.
 const curve:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[4,-2,0],[2,0,0]],weights:[1,1]}
 const report=intersectNurbsCurveRuledSurface(curve,ruledCylinder(1))
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.components).toHaveLength(1)
 const hit=JSON.parse(JSON.stringify(report.components[0]))
 if(hit.kind!=='point')throw Error('Expected seam endpoint event')
 expect(hit.t).toBe(1)
 expect(hit.uv).toEqual([0,0])
 expect(hit.contact).toBe('boundary')
 // A near-closed seam (1e-7 perturbation) never merges by tolerance: both
 // seam bands stay explicit and the contacts stay separate.
 const perturbed=ruledCylinder(1) as {controlPoints:number[][][]}
 perturbed.controlPoints[8][0][0]+=1e-7
 perturbed.controlPoints[8][1][0]+=1e-7
 const near=intersectNurbsCurveRuledSurface(curve,perturbed as never)
 expect(near.coverage).toBe('incomplete')
 expect(near.unresolved.some(u=>u.reason==='near_coincidence'&&u.parameterBox[2]===0&&u.parameterBox[3]===0)).toBe(true)
 expect(near.unresolved.some(u=>u.reason==='near_coincidence'&&u.parameterBox[2]===1&&u.parameterBox[3]===1)).toBe(true)
})

it('roundtrips the overlap correspondence map through packed WASM',()=>{
 const surface=ruledCylinder(3)
 // Curve lying on the seam ruling x=2, y=0, z in [1,3]: one wrapped overlap.
 const seam:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[2,0,1],[2,0,3]],weights:[1,1]}
 const report=intersectNurbsCurveRuledSurface(seam,surface)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.components).toHaveLength(1)
 const overlap=JSON.parse(JSON.stringify(report.components[0]))
 if(overlap.kind!=='overlap')throw Error('Expected seam ruling overlap')
 expect(overlap.seamWrap).toBe(true)
 expect(overlap.uvStart[0]).toBe(0)
 expect(overlap.uvEnd[0]).toBe(0)
 const correspondence=overlap.correspondence
 if(!correspondence||correspondence.kind!=='mobius_v')throw Error('Expected mobius_v correspondence')
 expect(correspondence.samples).toHaveLength(3)
 // Reconstruct the Möbius t->v map by the cross-ratio identity and verify
 // against direct surface evaluation at five interior parameters.
 const ms=correspondence.samples
 const t0=ms[0][0],v0=ms[0][2],t1=ms[1][0],v1=ms[1][2],t2=ms[2][0],v2=ms[2][2]
 expect(ms[0][1]).toBe(0)
 const mobius=(t:number)=>{const k=(t-t0)*(t1-t2)/((t-t2)*(t1-t0));return (v0*(v1-v2)-k*v2*(v1-v0))/((v1-v2)-k*(v1-v0))}
 for(let i=1;i<=5;i++){
  const t=i/6,v=mobius(t)
  const p=evaluateNurbsSurface(surface,0,v).point
  const q=evaluateNurbsCurve(seam,t).point
  p.forEach((x,k)=>expect(x).toBeCloseTo(q[k],9))
 }
 // Iso-v coincidence carries an affine_u correspondence.
 const w=Math.SQRT1_2,bottom=[[2,0,0],[2,2,0],[0,2,0]]
 const patch={degreeU:2,degreeV:1,knotsU:[0,0,0,1,1,1],knotsV:[0,0,1,1],
  controlPoints:bottom.map(p=>[p,[p[0],p[1],4]]),
  weights:[[1,1],[w,w],[1,1]],periodicU:false,periodicV:false}
 const arc:NurbsCurve={degree:2,knots:[0,0,0,1,1,1],controlPoints:bottom,weights:[1,w,1]}
 const iso=intersectNurbsCurveRuledSurface(arc,patch)
 const component=JSON.parse(JSON.stringify(iso.components[0]))
 if(component.kind!=='overlap')throw Error('Expected iso-v overlap')
 expect(component.seamWrap).toBe(false)
 const affine=component.correspondence
 if(!affine||affine.kind!=='affine_u')throw Error('Expected affine_u correspondence')
 const as_=affine.samples
 const s0=as_[0][0],a0=as_[0][1],s2=as_[2][0],a2=as_[2][1]
 for(let i=1;i<=5;i++){
  const t=i/6,u=a0+(a2-a0)*(t-s0)/(s2-s0)
  const p=evaluateNurbsSurface(patch as never,u,0).point
  const q=evaluateNurbsCurve(arc,t).point
  p.forEach((x,k)=>expect(x).toBeCloseTo(q[k],9))
 }
 expect(as_[1][1]).toBeCloseTo((a0+a2)/2,12)
})


it('resolves a transverse plane root exactly at a C0 knot through WASM',()=>{
 // Piecewise-linear kink at (1,1,0) on the plane z=0: both one-sided
 // tangents leave the plane transversally; the knot root must resolve.
 const kink:NurbsCurve={degree:1,knots:[0,0,1,2,2],controlPoints:[[0,0,-1],[1,1,0],[2,0,1]],weights:[1,1,1]}
 const report=intersectNurbsCurvePlane(kink,{normal:[0,0,1],offset:0})
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.unresolved).toHaveLength(0)
 expect(report.components).toHaveLength(1)
 const hit=report.components[0]
 if(hit.kind!=='point')throw Error('Expected kink root')
 expect(hit.curve.parameter).toBe(1)
 hit.curve.point.forEach((x,k)=>expect(x).toBeCloseTo([1,1,0][k],12))
 expect(hit.curve.contact).toBe('boundary')
 // The same kink crossed by a line transverse to both one-sided tangents.
 const line:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[1,0,-1],[1,2,1]],weights:[1,1]}
 const pair=intersectNurbsCurveCurve(kink,line)
 expect(pair.coverage).toBe('numerically_resolved')
 expect(pair.components).toHaveLength(1)
 const cross=pair.components[0]
 if(cross.kind!=='point')throw Error('Expected knot crossing')
 expect(cross.first).toBe(1)
 expect(cross.second).toBe(.5)
})

it('reports a dyadic box-boundary root once at the exact parameter through WASM',()=>{
 // Cubic with a single transverse root of z=0 bitwise at t=1/2; the thirds
 // in the polygon defeat a bitwise-zero split coefficient.
 const cubic:NurbsCurve={degree:3,knots:[0,0,0,0,1,1,1,1],
  controlPoints:[[0,0,-2],[1/3,0,-1/3],[2/3,0,2/3],[1,0,1]],weights:[1,1,1,1]}
 const report=intersectNurbsCurvePlane(cubic,{normal:[0,0,1],offset:0})
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.unresolved).toHaveLength(0)
 expect(report.components).toHaveLength(1)
 const hit=report.components[0]
 if(hit.kind!=='point')throw Error('Expected boundary root')
 expect(hit.curve.parameter).toBe(.5)
 hit.curve.point.forEach((x,k)=>expect(x).toBeCloseTo([.5,0,0][k],9))
 // Curve/curve: both parameters land bitwise on subdivision faces (1/2,1/2).
 const line:NurbsCurve={degree:1,knots:[0,0,1,1],controlPoints:[[.5,-1,-1],[.5,1,1]],weights:[1,1]}
 const pair=intersectNurbsCurveCurve(cubic,line)
 expect(pair.coverage).toBe('numerically_resolved')
 expect(pair.unresolved).toHaveLength(0)
 expect(pair.components).toHaveLength(1)
 const cross=pair.components[0]
 if(cross.kind!=='point')throw Error('Expected boundary crossing')
 expect(cross.first).toBe(.5)
 expect(cross.second).toBe(.5)
})

it('intersects two canonical spheres analytically through packed WASM',()=>{
 const first=createBrepSphere(2)
 const second=transformNurbsBrep(createBrepSphere(2),[[1,0,0,2],[0,1,0,0],[0,0,1,0],[0,0,0,1]])
 const report=intersectSphereSphere(first,second)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.evidence).toBe('numerical_uncertified')
 expect(report.unresolved).toHaveLength(0)
 expect(report.components).toHaveLength(1)
 const circle=report.components[0]
 if(circle.kind!=='circle')throw Error('Expected circle')
 // Oracle: radius sqrt(r^2-d^2/4)=sqrt(3), center [1,0,0], axis [1,0,0].
 expect(Math.abs(circle.radius-Math.sqrt(3))).toBeLessThan(1e-12)
 circle.center.forEach((x,k)=>expect(Math.abs(x-[1,0,0][k])).toBeLessThan(1e-12))
 circle.normal.forEach((x,k)=>expect(Math.abs(x-[1,0,0][k])).toBeLessThan(1e-12))
 // Exact rational circle: four 90-degree arcs, weights cos(pi/4).
 expect(circle.curve.degree).toBe(2)
 expect(circle.curve.knots).toEqual([0,0,0,1,1,2,2,3,3,4,4,4])
 expect(circle.curve.weights).toEqual([1,Math.SQRT1_2,1,Math.SQRT1_2,1,Math.SQRT1_2,1,Math.SQRT1_2,1])
 // Sixteen samples satisfy both sphere equations.
 for(let i=0;i<16;i++){
  const p=evaluateNurbsCurve(circle.curve,i/4).point
  expect(Math.abs(Math.hypot(...p)-2)).toBeLessThan(1e-12)
  expect(Math.abs(Math.hypot(p[0]-2,p[1],p[2])-2)).toBeLessThan(1e-12)
 }
 expect(circle.maxSampleResidual).toBeLessThan(1e-12)
 // Lifted UV arcs: quadrants 0,3 (both hemispheres) on the first sphere,
 // 1,2 on the second; every lifted point evaluates onto both spheres.
 expect(circle.firstUv.map(l=>l.patch)).toEqual([0,3,4,7])
 expect(circle.secondUv.map(l=>l.patch)).toEqual([1,2,5,6])
 for(const [model,lifts,center] of [[first,circle.firstUv,[0,0,0]],[second,circle.secondUv,[2,0,0]]] as const){
  for(const lift of lifts)for(const arc of lift.arcs)for(let k=0;k<=8;k++){
   const uv=evaluateNurbsCurve(arc,k/8).point
   expect(uv[0]).toBeGreaterThan(-1e-9);expect(uv[1]).toBeGreaterThan(-1e-9)
   // Arc endpoints sit bitwise on the patch boundary; clamp rounding hairs.
   const p=evaluateNurbsSurface(model.faces[lift.patch].surface,Math.min(1,Math.max(0,uv[0])),Math.min(1,Math.max(0,uv[1]))).point
   expect(Math.abs(Math.hypot(p[0]-center[0],p[1]-center[1],p[2]-center[2])-2)).toBeLessThan(1e-12)
   expect(Math.abs(Math.hypot(...p)-2)<1e-12||Math.abs(Math.hypot(p[0]-2,p[1],p[2])-2)<1e-12).toBe(true)
  }
 }
 // JSON roundtrip preserves the component.
 const round=JSON.parse(JSON.stringify(report)) as typeof report
 const again=round.components[0]
 if(again.kind!=='circle')throw Error('Expected circle after roundtrip')
 expect(again.radius).toBe(circle.radius)
 expect(again.firstUv.map(l=>l.patch)).toEqual([0,3,4,7])
})

it('classifies axis-aligned sphere pairs with exact lifted UV radii through WASM',()=>{
 const first=createBrepSphere(3)
 const second=transformNurbsBrep(createBrepSphere(3),[[1,0,0,0],[0,1,0,0],[0,0,1,2],[0,0,0,1]])
 const report=intersectSphereSphere(first,second)
 expect(report.coverage).toBe('numerically_resolved')
 const circle=report.components[0]
 if(circle.kind!=='circle')throw Error('Expected circle')
 expect(Math.abs(circle.radius-Math.sqrt(8))).toBeLessThan(1e-12)
 circle.center.forEach((x,k)=>expect(Math.abs(x-[0,0,1][k])).toBeLessThan(1e-12))
 // Parallel at t=1 on r=3: north patches of the first sphere, south of the
 // second; UV circle radius sqrt((r-t)/(r+t)) = tan(pi/4-asin(t/r)/2).
 expect(circle.firstUv.map(l=>l.patch)).toEqual([0,1,2,3])
 expect(circle.secondUv.map(l=>l.patch)).toEqual([4,5,6,7])
 const rho=Math.sqrt((3-1)/(3+1))
 expect(Math.abs(rho-Math.tan(Math.PI/4-Math.asin(1/3)/2))).toBeLessThan(1e-15)
 for(const lift of [...circle.firstUv,...circle.secondUv]){
  expect(lift.arcs).toHaveLength(1)
  const arc=lift.arcs[0]
  expect(arc.weights).toEqual([1,Math.SQRT1_2,1])
  for(const endpoint of [arc.controlPoints[0],arc.controlPoints[2]])
   expect(Math.abs(Math.hypot(endpoint[0],endpoint[1])-rho)).toBeLessThan(1e-12)
 }
})

it('keeps sphere/sphere tangency, coincidence and refusals explicit through WASM',()=>{
 const sphere=createBrepSphere(2)
 const place=(x:number)=>transformNurbsBrep(createBrepSphere(2),[[1,0,0,x],[0,1,0,0],[0,0,1,0],[0,0,0,1]])
 // Coincident concentric equal spheres: coincident_trim, never a curve.
 const coincident=intersectSphereSphere(sphere,createBrepSphere(2))
 expect(coincident.coverage).toBe('incomplete')
 expect(coincident.components).toHaveLength(0)
 expect(coincident.unresolved.map(u=>u.reason)).toEqual(['coincident_trim'])
 expect(coincident.unresolved[0].parameterBox).toEqual([0,1,0,1,0,1,0,1])
 // Concentric unequal and contained-without-contact: empty, resolved.
 expect(intersectSphereSphere(createBrepSphere(2),createBrepSphere(3)).components).toHaveLength(0)
 expect(intersectSphereSphere(createBrepSphere(2),createBrepSphere(3)).coverage).toBe('numerically_resolved')
 const contained=intersectSphereSphere(createBrepSphere(1),transformNurbsBrep(createBrepSphere(3),[[1,0,0,1],[0,1,0,0],[0,0,1,0],[0,0,0,1]]))
 expect(contained.components).toHaveLength(0)
 expect(contained.coverage).toBe('numerically_resolved')
 // Separate: empty, resolved.
 const separate=intersectSphereSphere(sphere,place(5))
 expect(separate.components).toHaveLength(0)
 expect(separate.unresolved).toHaveLength(0)
 expect(separate.coverage).toBe('numerically_resolved')
 // Exact external tangency and internal tangency: tangency regions, no points.
 const external=intersectSphereSphere(sphere,place(4))
 expect(external.components).toHaveLength(0)
 expect(external.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 const internal=intersectSphereSphere(createBrepSphere(1),transformNurbsBrep(createBrepSphere(3),[[1,0,0,2],[0,1,0,0],[0,0,1,0],[0,0,0,1]]))
 expect(internal.components).toHaveLength(0)
 expect(internal.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 // Non-canonical operands: explicit unsupported_surface, no numerical fallback.
 for(const [a,b] of [[sphere,createBrepCylinder(1,2)],[createBrepCylinder(1,2),sphere],[sphere,createBrepFrustum(1,1,2)]] as const){
  const refused=intersectSphereSphere(a,b)
  expect(refused.coverage).toBe('incomplete')
  expect(refused.components).toHaveLength(0)
  expect(refused.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
  expect(refused.permitsTopologyChange).toBe(false)
 }
})

it('intersects an axial sphere/cylinder pair analytically through packed WASM',()=>{
 const sphere=transformNurbsBrep(createBrepSphere(3),[[1,0,0,0],[0,1,0,0],[0,0,1,4],[0,0,0,1]])
 const cylinder=createBrepCylinder(2,8)
 const report=intersectSphereCylinder(sphere,cylinder)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.evidence).toBe('numerical_uncertified')
 expect(report.unresolved).toHaveLength(0)
 expect(report.components).toHaveLength(2)
 const oracle=Math.sqrt(9-4)
 for(const [index,z] of [4-oracle,4+oracle].entries()){
  const circle=report.components[index]
  if(circle.kind!=='circle')throw Error('Expected circle')
  expect(Math.abs(circle.radius-2)).toBeLessThan(1e-12)
  circle.center.forEach((x,k)=>expect(Math.abs(x-[0,0,z][k])).toBeLessThan(1e-12))
  expect(Math.abs(circle.normal[2])).toBeGreaterThan(1-1e-12)
  // Exact rational circle: four 90-degree arcs, weights cos(pi/4).
  expect(circle.curve.degree).toBe(2)
  expect(circle.curve.knots).toEqual([0,0,0,1,1,2,2,3,3,4,4,4])
  expect(circle.curve.weights).toEqual([1,Math.SQRT1_2,1,Math.SQRT1_2,1,Math.SQRT1_2,1,Math.SQRT1_2,1])
  // Sixteen samples satisfy the sphere and the cylinder side equations.
  for(let i=0;i<16;i++){
   const p=evaluateNurbsCurve(circle.curve,i/4).point
   expect(Math.abs(Math.hypot(p[0],p[1],p[2]-4)-3)).toBeLessThan(1e-12)
   expect(Math.abs(Math.hypot(p[0],p[1])-2)).toBeLessThan(1e-12)
  }
  expect(circle.maxSampleResidual).toBeLessThan(1e-12)
  // Side circles lift to iso-v lines on all four side patches; sphere lifts
  // are per-patch UV pieces. Every lifted point evaluates onto both surfaces.
  expect(circle.cylinderUv).toHaveLength(4)
  for(const lift of circle.cylinderUv){
   expect(lift.arcs).toHaveLength(1)
   const arc=lift.arcs[0]
   expect(arc.degree).toBe(1)
   expect(Math.abs(arc.controlPoints[0][1]-z/8)).toBeLessThan(1e-12)
  }
  expect(circle.sphereUv.length).toBeGreaterThan(0)
  for(const [model,lifts,check] of [[cylinder,circle.cylinderUv,(p:number[])=>Math.abs(Math.hypot(p[0],p[1])-2)],[sphere,circle.sphereUv,(p:number[])=>Math.abs(Math.hypot(p[0],p[1])-2)]] as const){
   for(const lift of lifts)for(const arc of lift.arcs)for(let k=0;k<=8;k++){
    const uv=evaluateNurbsCurve(arc,k/8).point
    const p=evaluateNurbsSurface(model.faces[lift.patch].surface,Math.min(1,Math.max(0,uv[0])),Math.min(1,Math.max(0,uv[1]))).point
    expect(Math.abs(Math.hypot(p[0],p[1],p[2]-4)-3)).toBeLessThan(1e-9)
    expect(check(p)).toBeLessThan(1e-9)
   }
  }
 }
 // JSON roundtrip preserves the components.
 const round=JSON.parse(JSON.stringify(report)) as typeof report
 expect(round.components).toHaveLength(2)
 expect(round.components[0].kind).toBe('circle')
 if(round.components[0].kind==='circle')expect(round.components[0].cylinderUv).toHaveLength(4)
})

it('resolves clipped side circles and cap circles through WASM',()=>{
 const cylinder=createBrepCylinder(2,8)
 // Sphere r=3 centered on the bottom cap plane: only the upper side circle
 // survives the finite height, at z = sqrt(9-4).
 const clipped=intersectSphereCylinder(createBrepSphere(3),cylinder)
 expect(clipped.coverage).toBe('numerically_resolved')
 expect(clipped.components).toHaveLength(1)
 const side=clipped.components[0]
 if(side.kind!=='circle')throw Error('Expected circle')
 expect(Math.abs(side.center[2]-Math.sqrt(5))).toBeLessThan(1e-12)
 expect(Math.abs(side.radius-2)).toBeLessThan(1e-12)
 // Small sphere poking through the top cap while radially inside: an exact
 // circle of radius sqrt(r^2-d^2)=sqrt(2) in the cap plane.
 const poking=transformNurbsBrep(createBrepSphere(1.5),[[1,0,0,0],[0,1,0,0],[0,0,1,7.5],[0,0,0,1]])
 const cap=intersectSphereCylinder(poking,cylinder)
 expect(cap.coverage).toBe('numerically_resolved')
 expect(cap.components).toHaveLength(1)
 const circle=cap.components[0]
 if(circle.kind!=='circle')throw Error('Expected circle')
 expect(Math.abs(circle.radius-Math.sqrt(2))).toBeLessThan(1e-12)
 circle.center.forEach((x,k)=>expect(Math.abs(x-[0,0,8][k])).toBeLessThan(1e-12))
 // The cap lift is one UV circle of radius sqrt(2)/(2R) about [1/2,1/2].
 expect(circle.cylinderUv).toHaveLength(1)
 expect(circle.cylinderUv[0].arcs).toHaveLength(4)
 for(const arc of circle.cylinderUv[0].arcs){
  expect(arc.weights).toEqual([1,Math.SQRT1_2,1])
  const e=arc.controlPoints[0]
  expect(Math.abs(Math.hypot(e[0]-.5,e[1]-.5)-Math.sqrt(2)/4)).toBeLessThan(1e-12)
  for(let k=0;k<=8;k++){
   const uv=evaluateNurbsCurve(arc,k/8).point
   const p=evaluateNurbsSurface(cylinder.faces[circle.cylinderUv[0].patch].surface,uv[0],uv[1]).point
   expect(Math.abs(Math.hypot(p[0],p[1],p[2]-7.5)-1.5)).toBeLessThan(1e-9)
   expect(Math.abs(p[2]-8)).toBeLessThan(1e-9)
  }
 }
})

it('keeps sphere/cylinder bands, tangencies and refusals explicit through WASM',()=>{
 const cylinder=createBrepCylinder(2,8)
 const place=(radius:number,x:number,z:number)=>transformNurbsBrep(createBrepSphere(radius),[[1,0,0,x],[0,1,0,0],[0,0,1,z],[0,0,0,1]])
 // r == R exactly and within the band: coincident_trim, never a circle.
 for(const sphere of [place(2,0,4),place(2+2e-15,0,4),place(2,0,9)]){
  const report=intersectSphereCylinder(sphere,cylinder)
  expect(report.coverage).toBe('incomplete')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved.map(u=>u.reason)).toEqual(['coincident_trim'])
  expect(report.permitsTopologyChange).toBe(false)
 }
 // Cap-plane tangency: tangency_or_multiple_root, never a point.
 const tangent=intersectSphereCylinder(place(1.5,0,6.5),cylinder)
 expect(tangent.components).toHaveLength(0)
 expect(tangent.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 // Rim tangency: sqrt(r^2-R^2) == h/2 lands both crossings on the caps.
 const rim=intersectSphereCylinder(place(Math.sqrt(20),0,4),cylinder)
 expect(rim.components).toHaveLength(0)
 expect(rim.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 // Near-axial within the recognition band: near_coincidence, not forced axial.
 const near=intersectSphereCylinder(place(3,1e-10,4),cylinder)
 expect(near.unresolved.map(u=>u.reason)).toEqual(['near_coincidence'])
 // Clearly off-axis: unsupported_surface, the general quartic is out of scope.
 const off=intersectSphereCylinder(place(3,.5,4),cylinder)
 expect(off.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 // Empty resolved: small sphere inside, sphere beyond the caps, big swallow.
 for(const sphere of [place(1,0,4),place(1.5,0,11),place(20,0,4)]){
  const report=intersectSphereCylinder(sphere,cylinder)
  expect(report.coverage).toBe('numerically_resolved')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved).toHaveLength(0)
 }
 // Non-canonical operands: explicit unsupported_surface, no numerical fallback.
 for(const [a,b] of [[createBrepSphere(2),createBrepFrustum(1,2,3)],[createBrepSphere(2),transformNurbsBrep(createBrepSphere(1),[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]])]] as const){
  const refused=intersectSphereCylinder(a,b)
  expect(refused.coverage).toBe('incomplete')
  expect(refused.components).toHaveLength(0)
  expect(refused.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 }
})

it('intersects two parallel canonical cylinders analytically through packed WASM',()=>{
 const first=createBrepCylinder(2,8)
 const second=transformNurbsBrep(createBrepCylinder(3,8),[[1,0,0,3.5],[0,1,0,0],[0,0,1,0],[0,0,0,1]])
 const report=intersectCylinderCylinder(first,second)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.evidence).toBe('numerical_uncertified')
 expect(report.unresolved).toHaveLength(0)
 expect(report.components).toHaveLength(2)
 // Oracle from the planar circle/circle section: x=(d^2+r1^2-r2^2)/(2d), y=sqrt(r1^2-x^2).
 const d=3.5,x=(d*d+4-9)/(2*d),y=Math.sqrt(4-x*x)
 for(const [index,sign] of [-1,1].entries()){
  const line=report.components[index]
  if(line.kind!=='line')throw Error('Expected line')
  line.start.forEach((v,k)=>expect(Math.abs(v-[x,sign*y,0][k])).toBeLessThan(1e-12))
  line.end.forEach((v,k)=>expect(Math.abs(v-[x,sign*y,8][k])).toBeLessThan(1e-12))
  line.direction.forEach((v,k)=>expect(Math.abs(v-[0,0,1][k])).toBeLessThan(1e-12))
  expect(line.contact).toBe('boundary')
  // Exact degree-1 line, unit knots and weights.
  expect(line.curve.degree).toBe(1)
  expect(line.curve.knots).toEqual([0,0,1,1])
  expect(line.curve.weights).toEqual([1,1])
  // Nine samples satisfy both implicit cylinder equations.
  for(let k=0;k<=8;k++){
   const p=evaluateNurbsCurve(line.curve,k/8).point
   expect(Math.abs(Math.hypot(p[0],p[1])-2)).toBeLessThan(1e-12)
   expect(Math.abs(Math.hypot(p[0]-3.5,p[1])-3)).toBeLessThan(1e-12)
  }
  expect(line.maxSampleResidual).toBeLessThan(1e-12)
  // Iso-u lifts on one side patch per cylinder, v sweeping the full height;
  // every lifted point evaluates through the patch surface onto both walls.
  for(const [model,lifts] of [[first,line.firstUv],[second,line.secondUv]] as const){
   expect(lifts).toHaveLength(1)
   expect(lifts[0].arcs).toHaveLength(1)
   const arc=lifts[0].arcs[0]
   expect(arc.degree).toBe(1)
   expect(Math.abs(arc.controlPoints[0][1])).toBeLessThan(1e-12)
   expect(Math.abs(arc.controlPoints[1][1]-1)).toBeLessThan(1e-12)
   expect(arc.controlPoints[0][0]).toBe(arc.controlPoints[1][0])
   for(let k=0;k<=8;k++){
    const uv=evaluateNurbsCurve(arc,k/8).point
    expect(uv[0]).toBeGreaterThan(-1e-12);expect(uv[0]).toBeLessThan(1+1e-12)
    const p=evaluateNurbsSurface(model.faces[lifts[0].patch].surface,Math.min(1,Math.max(0,uv[0])),Math.min(1,Math.max(0,uv[1]))).point
    expect(Math.abs(Math.hypot(p[0],p[1])-2)).toBeLessThan(1e-9)
    expect(Math.abs(Math.hypot(p[0]-3.5,p[1])-3)).toBeLessThan(1e-9)
   }
  }
 }
 // JSON roundtrip preserves the components.
 const round=JSON.parse(JSON.stringify(report)) as typeof report
 expect(round.components).toHaveLength(2)
 if(round.components[0].kind!=='line')throw Error('Expected line after roundtrip')
 expect(round.components[0].start).toEqual(report.components[0].kind==='line'?report.components[0].start:[])
 expect(round.components[0].firstUv).toHaveLength(1)
})

it('clips parallel cylinder rulings by both finite heights through WASM',()=>{
 const first=createBrepCylinder(2,8)
 // Partial overlap: the second cylinder spans z in 5..9 -> lines z in 5..8.
 const partial=transformNurbsBrep(createBrepCylinder(3,4),[[1,0,0,3.5],[0,1,0,0],[0,0,1,5],[0,0,0,1]])
 const clipped=intersectCylinderCylinder(first,partial)
 expect(clipped.coverage).toBe('numerically_resolved')
 expect(clipped.components).toHaveLength(2)
 const d=3.5,x=(d*d+4-9)/(2*d),y=Math.sqrt(4-x*x)
 for(const [index,sign] of [-1,1].entries()){
  const line=clipped.components[index]
  if(line.kind!=='line')throw Error('Expected line')
  line.start.forEach((v,k)=>expect(Math.abs(v-[x,sign*y,5][k])).toBeLessThan(1e-12))
  line.end.forEach((v,k)=>expect(Math.abs(v-[x,sign*y,8][k])).toBeLessThan(1e-12))
  // The second lift spans v in 0..3/4 of its own height.
  const arc=line.secondUv[0].arcs[0]
  expect(Math.abs(arc.controlPoints[0][1])).toBeLessThan(1e-12)
  expect(Math.abs(arc.controlPoints[1][1]-.75)).toBeLessThan(1e-12)
 }
 // Disjoint heights: the clip interval is empty — resolved empty.
 const disjoint=transformNurbsBrep(createBrepCylinder(3,4),[[1,0,0,3.5],[0,1,0,0],[0,0,1,9],[0,0,0,1]])
 const empty=intersectCylinderCylinder(first,disjoint)
 expect(empty.coverage).toBe('numerically_resolved')
 expect(empty.components).toHaveLength(0)
 expect(empty.unresolved).toHaveLength(0)
 // Heights touching at one cap plane: the band-thin clip degenerates.
 const touching=transformNurbsBrep(createBrepCylinder(3,4),[[1,0,0,3.5],[0,1,0,0],[0,0,1,8],[0,0,0,1]])
 const degenerate=intersectCylinderCylinder(first,touching)
 expect(degenerate.coverage).toBe('incomplete')
 expect(degenerate.components).toHaveLength(0)
 expect(degenerate.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
})

it('keeps cylinder/cylinder bands, tangencies and refusals explicit through WASM',()=>{
 const first=createBrepCylinder(2,8)
 const place=(radius:number,height:number,x:number,z:number)=>transformNurbsBrep(createBrepCylinder(radius,height),[[1,0,0,x],[0,1,0,0],[0,0,1,z],[0,0,0,1]])
 // Coaxial equal radii (exact and within the band): coincident_trim.
 for(const second of [createBrepCylinder(2,8),createBrepCylinder(2+2e-15,8)]){
  const report=intersectCylinderCylinder(first,second)
  expect(report.coverage).toBe('incomplete')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved.map(u=>u.reason)).toEqual(['coincident_trim'])
  expect(report.permitsTopologyChange).toBe(false)
 }
 // Stacked equal radii sharing a cap plane: rim tangency, never a circle.
 const stacked=intersectCylinderCylinder(first,place(2,4,0,8))
 expect(stacked.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 // Coaxial unequal radii without a coincident cap plane: empty, resolved.
 const nested=intersectCylinderCylinder(first,place(1,4,0,2))
 expect(nested.coverage).toBe('numerically_resolved')
 expect(nested.components).toHaveLength(0)
 // Coaxial unequal radii sharing cap planes: the smaller cap disk coincides.
 for(const second of [createBrepCylinder(1,8),place(1,4,0,8)]){
  const report=intersectCylinderCylinder(first,second)
  expect(report.unresolved.map(u=>u.reason)).toEqual(['coincident_trim'])
 }
 // External and internal tangencies: tangency regions, never lines.
 expect(intersectCylinderCylinder(first,place(3,8,5,0)).unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 expect(intersectCylinderCylinder(first,place(5,8,3,0)).unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 // Just clear of the external band: separate, empty and resolved.
 const clear=intersectCylinderCylinder(first,place(3,8,5+1e-9,0))
 expect(clear.coverage).toBe('numerically_resolved')
 expect(clear.components).toHaveLength(0)
 // Radially nested without contact: d < |r1-r2| provably.
 const inside=intersectCylinderCylinder(first,place(5,8,2,0))
 expect(inside.coverage).toBe('numerically_resolved')
 expect(inside.components).toHaveLength(0)
 // Near-parallel within the recognition band: near_coincidence, not forced.
 const angle=1e-10
 const tilted=transformNurbsBrep(createBrepCylinder(3,8),[[1,0,0,3.5],[0,Math.cos(angle),-Math.sin(angle),0],[0,Math.sin(angle),Math.cos(angle),0],[0,0,0,1]])
 expect(intersectCylinderCylinder(first,tilted).unresolved.map(u=>u.reason)).toEqual(['near_coincidence'])
 // Clearly non-parallel: unsupported_surface, the quartic is out of scope.
 const skew=transformNurbsBrep(createBrepCylinder(3,8),[[1,0,0,3.5],[0,Math.cos(.3),-Math.sin(.3),0],[0,Math.sin(.3),Math.cos(.3),0],[0,0,0,1]])
 const off=intersectCylinderCylinder(first,skew)
 expect(off.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 expect(off.unresolved[0].parameterBox).toEqual([0,1,0,1,0,1,0,1])
 // Non-canonical operands: explicit unsupported_surface, no numerical fallback.
 for(const [a,b] of [[createBrepFrustum(1,2,3),first],[createBrepSphere(2),first],[first,createBrepFrustum(1,2,3)]] as const){
  const refused=intersectCylinderCylinder(a,b)
  expect(refused.coverage).toBe('incomplete')
  expect(refused.components).toHaveLength(0)
  expect(refused.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
  expect(refused.permitsTopologyChange).toBe(false)
 }
})

/** The canonical finite rectangular planar patch operand: one-face open model, exact bilinear affine surface over the unit square with boundary trims. */
const createPlanePatch=(origin:[number,number,number],u:[number,number,number],v:[number,number,number]):NurbsBrep=>{
 const add=(a:number[],b:number[]):[number,number,number]=>[a[0]+b[0],a[1]+b[1],a[2]+b[2]]
 const corners=[origin,add(origin,u),add(add(origin,u),v),add(origin,v)]
 const uv=[[0,0],[1,0],[1,1],[0,1]]
 const line=(a:number[],b:number[]):NurbsCurve=>({degree:1,knots:[0,0,1,1],controlPoints:[a,b],weights:[1,1],periodic:false})
 return {
  vertices:corners.map(point=>({point})),
  edges:[0,1,2,3].map(i=>({vertices:[i,(i+1)%4] as [number,number],curve:line(corners[i],corners[(i+1)%4])})),
  loops:[{coedges:[0,1,2,3].map(i=>({edge:i,reversed:false,pcurve:line(uv[i],uv[(i+1)%4])}))}],
  faces:[{surface:{degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],
   controlPoints:[[corners[0],corners[3]],[corners[1],corners[2]]],weights:[[1,1],[1,1]],
   periodicU:false,periodicV:false},outer:0,holes:[]}],
  shells:[{faces:[{face:0,reversed:false}],closed:false}],
  bodies:[],
  toleranceMm:1e-7,
 }
}

it('intersects a canonical plane patch and sphere analytically through packed WASM',()=>{
 // Plane z = 1, patch [-3,3]^2; sphere r = 2: the exact circle of radius
 // sqrt(3) at z = 1, fully inside the patch.
 const plane=createPlanePatch([-3,-3,1],[6,0,0],[0,6,0])
 const sphere=createBrepSphere(2)
 const report=intersectPlaneSphere(plane,sphere)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.evidence).toBe('numerical_uncertified')
 expect(report.unresolved).toHaveLength(0)
 expect(report.components).toHaveLength(1)
 const circle=report.components[0]
 if(circle.kind!=='circle')throw Error('Expected circle')
 expect(Math.abs(circle.radius-Math.sqrt(3))).toBeLessThan(1e-12)
 circle.center.forEach((x,k)=>expect(Math.abs(x-[0,0,1][k])).toBeLessThan(1e-12))
 expect(circle.normal).toEqual([0,0,1])
 expect(circle.full).toBe(true)
 expect(circle.curve.weights).toHaveLength(9)
 expect(circle.curve.weights[1]).toBe(Math.SQRT1_2)
 for(let i=0;i<=16;i++){
  const p=evaluateNurbsCurve(circle.curve,i/16).point
  expect(Math.abs(Math.hypot(p[0],p[1],p[2])-2)).toBeLessThan(1e-12)
  expect(Math.abs(p[2]-1)).toBeLessThan(1e-12)
 }
 // Plane UV: four exact ellipse arcs inside the unit square, on both surfaces.
 expect(circle.planeUv).toHaveLength(1)
 expect(circle.planeUv[0].arcs).toHaveLength(4)
 for(const arc of circle.planeUv[0].arcs){
  expect(arc.weights).toEqual([1,Math.SQRT1_2,1])
  for(let k=0;k<=8;k++){
   const q=evaluateNurbsCurve(arc,k/8).point
   expect(q[0]).toBeGreaterThanOrEqual(-1e-9);expect(q[0]).toBeLessThanOrEqual(1+1e-9)
   expect(q[1]).toBeGreaterThanOrEqual(-1e-9);expect(q[1]).toBeLessThanOrEqual(1+1e-9)
   const p=evaluateNurbsSurface(plane.faces[0].surface,Math.min(1,Math.max(0,q[0])),Math.min(1,Math.max(0,q[1]))).point
   expect(Math.abs(Math.hypot(p[0],p[1],p[2])-2)).toBeLessThan(1e-9)
   expect(Math.abs(p[2]-1)).toBeLessThan(1e-9)
  }
 }
 expect(circle.sphereUv.length).toBeGreaterThan(0)
 for(const lift of circle.sphereUv)for(const arc of lift.arcs)for(let k=0;k<=8;k++){
  const q=evaluateNurbsCurve(arc,k/8).point
  const p=evaluateNurbsSurface(sphere.faces[lift.patch].surface,Math.min(1,Math.max(0,q[0])),Math.min(1,Math.max(0,q[1]))).point
  expect(Math.abs(Math.hypot(p[0],p[1],p[2])-2)).toBeLessThan(1e-9)
  expect(Math.abs(p[2]-1)).toBeLessThan(1e-9)
 }
 expect(circle.maxSampleResidual).toBeLessThanOrEqual(1e-12)
 // JSON roundtrip preserves the component.
 const round=JSON.parse(JSON.stringify(report)) as typeof report
 expect(round.components).toHaveLength(1)
 expect(round.components[0].kind).toBe('circle')
})

it('resolves plane/cylinder rulings and the oblique ellipse through WASM',()=>{
 const cylinder=createBrepCylinder(2,8)
 // Plane x = 1 parallel to the axis: two exact rulings (1,+-sqrt(3),z).
 const plane=createPlanePatch([1,-3,0],[0,6,0],[0,0,10])
 const report=intersectPlaneCylinder(plane,cylinder)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.unresolved).toHaveLength(0)
 expect(report.components).toHaveLength(2)
 const h=Math.sqrt(3)
 const seen:number[]=[]
 for(const line of report.components){
  if(line.kind!=='line')throw Error('Expected lines')
  expect(line.contact).toBe('boundary')
  expect(line.curve.degree).toBe(1)
  expect(Math.abs(line.start[0]-1)).toBeLessThan(1e-12)
  expect(Math.abs(Math.abs(line.start[1])-h)).toBeLessThan(1e-12)
  expect(Math.abs(line.start[2])).toBeLessThan(1e-12)
  expect(Math.abs(line.end[2]-8)).toBeLessThan(1e-12)
  expect(line.direction).toEqual([0,0,1])
  seen.push(line.start[1])
  expect(line.maxSampleResidual).toBeLessThanOrEqual(1e-12)
  // Lifts evaluate onto both surfaces.
  for(const lift of line.planeUv)for(const arc of lift.arcs)for(let k=0;k<=8;k++){
   const q=evaluateNurbsCurve(arc,k/8).point
   const p=evaluateNurbsSurface(plane.faces[lift.patch].surface,Math.min(1,Math.max(0,q[0])),Math.min(1,Math.max(0,q[1]))).point
   expect(Math.abs(p[0]-1)).toBeLessThan(1e-9)
   expect(Math.abs(Math.hypot(p[0],p[1])-2)).toBeLessThan(1e-9)
  }
  for(const lift of line.cylinderUv)for(const arc of lift.arcs)for(let k=0;k<=8;k++){
   const q=evaluateNurbsCurve(arc,k/8).point
   const p=evaluateNurbsSurface(cylinder.faces[lift.patch].surface,Math.min(1,Math.max(0,q[0])),Math.min(1,Math.max(0,q[1]))).point
   expect(Math.abs(p[0]-1)).toBeLessThan(1e-9)
   expect(Math.abs(Math.hypot(p[0],p[1])-2)).toBeLessThan(1e-9)
  }
 }
 expect(seen[0]*seen[1]).toBeLessThan(0)
 // Oblique plane through the cylinder center, beta = 30 degrees: the exact
 // ellipse with semi-major 4/sqrt(3), semi-minor 2; the cylinder side lift
 // is null (no exact rational UV form), the 3D curve and plane lift exact.
 const beta=Math.PI/6,s=Math.sin(beta),c=Math.cos(beta)
 const oblique=createPlanePatch([-6,-8*c,4-8*s],[12,0,0],[0,16*c,16*s])
 const section=intersectPlaneCylinder(oblique,cylinder)
 expect(section.coverage).toBe('numerically_resolved')
 expect(section.components).toHaveLength(1)
 const ellipse=section.components[0]
 if(ellipse.kind!=='ellipse')throw Error('Expected ellipse')
 expect(ellipse.full).toBe(true)
 expect(ellipse.cylinderUv).toBeNull()
 expect(Math.abs(ellipse.semiMajor-2/c)).toBeLessThan(1e-12)
 expect(Math.abs(ellipse.semiMinor-2)).toBeLessThan(1e-12)
 ellipse.center.forEach((x,k)=>expect(Math.abs(x-[0,0,4][k])).toBeLessThan(1e-12))
 ellipse.major.forEach((x,k)=>expect(Math.abs(x-[0,c,s][k])).toBeLessThan(1e-12))
 ellipse.minor.forEach((x,k)=>expect(Math.abs(x-[-1,0,0][k])).toBeLessThan(1e-12))
 for(let i=0;i<16;i++){
  const p=evaluateNurbsCurve(ellipse.curve,i/4).point
  expect(Math.abs(Math.hypot(p[0],p[1])-2)).toBeLessThan(1e-12)
  expect(Math.abs(-p[1]*s+(p[2]-4)*c)).toBeLessThan(1e-12)
 }
 expect(ellipse.maxSampleResidual).toBeLessThanOrEqual(1e-12)
 const round=JSON.parse(JSON.stringify(section)) as typeof section
 if(round.components[0].kind!=='ellipse')throw Error('Expected ellipse')
 expect(round.components[0].cylinderUv).toBeNull()
})

it('keeps plane/quadric bands, tangencies and refusals explicit through WASM',()=>{
 const sphere=createBrepSphere(2)
 const cylinder=createBrepCylinder(2,8)
 const at=(z:number)=>createPlanePatch([-1,-1,z],[2,0,0],[0,2,0])
 // Plane/sphere tangency and within the band: never a point.
 for(const z of [2,2-3e-14]){
  const report=intersectPlaneSphere(at(z),sphere)
  expect(report.coverage).toBe('incomplete')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
  expect(report.permitsTopologyChange).toBe(false)
 }
 // Clear miss: empty and resolved.
 const miss=intersectPlaneSphere(at(5),sphere)
 expect(miss.coverage).toBe('numerically_resolved')
 expect(miss.components).toHaveLength(0)
 expect(miss.unresolved).toHaveLength(0)
 // Sphere as the plane operand: fixed order, explicit refusal.
 const swapped=intersectPlaneSphere(sphere,at(1))
 expect(swapped.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 // Plane/cylinder: cap-plane coincidence is coincident_trim, never a circle.
 const coincident=intersectPlaneCylinder(createPlanePatch([-3,-3,8],[6,0,0],[0,6,0]),cylinder)
 expect(coincident.coverage).toBe('incomplete')
 expect(coincident.components).toHaveLength(0)
 expect(coincident.unresolved.map(u=>u.reason)).toEqual(['coincident_trim'])
 // Tangent parallel plane: tangency region, never a line.
 const tangent=intersectPlaneCylinder(createPlanePatch([2,-3,0],[0,6,0],[0,0,10]),cylinder)
 expect(tangent.components).toHaveLength(0)
 expect(tangent.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 // Provable miss: empty and resolved.
 const empty=intersectPlaneCylinder(createPlanePatch([3,-3,0],[0,6,0],[0,0,10]),cylinder)
 expect(empty.coverage).toBe('numerically_resolved')
 expect(empty.components).toHaveLength(0)
 // Recognition-scale tilt off perpendicular: near_coincidence, never forced.
 const a=1e-6
 const nearPerp=createPlanePatch([-3,-3*Math.cos(a),4-3*Math.sin(a)],[6,0,0],[0,6*Math.cos(a),6*Math.sin(a)])
 expect(intersectPlaneCylinder(nearPerp,cylinder).unresolved.map(u=>u.reason)).toEqual(['near_coincidence'])
 // Non-canonical operands: explicit unsupported_surface, no numerical fallback.
 const skewed=createPlanePatch([-3,-3,4],[6,0,0],[3,6,0])
 for(const [p,q] of [[skewed,cylinder],[createPlanePatch([-3,-3,4],[6,0,0],[0,6,0]),createBrepFrustum(1,2,3)]] as const){
  const refused=intersectPlaneCylinder(p,q)
  expect(refused.coverage).toBe('incomplete')
  expect(refused.components).toHaveLength(0)
  expect(refused.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 }
})

it('resolves plane/cone circle and parabola sections analytically through WASM',()=>{
 const frustum=createBrepFrustum(3,1,5)
 // Plane z = 2.5: the exact circle of linearly interpolated radius
 // 3 - 0.4*2.5 = 2, fully inside the patch, with iso-v side lifts at v = 0.5.
 const plane=createPlanePatch([-3,-3,2.5],[6,0,0],[0,6,0])
 const report=intersectPlaneCone(plane,frustum)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.unresolved).toHaveLength(0)
 expect(report.components).toHaveLength(1)
 const circle=report.components[0]
 if(circle.kind!=='circle')throw Error('Expected circle')
 expect(Math.abs(circle.radius-2)).toBeLessThan(1e-12)
 circle.center.forEach((x,k)=>expect(Math.abs(x-[0,0,2.5][k])).toBeLessThan(1e-12))
 expect(circle.normal).toEqual([0,0,1])
 expect(circle.full).toBe(true)
 for(let i=0;i<=16;i++){
  const p=evaluateNurbsCurve(circle.curve,i/16).point
  expect(Math.abs(Math.hypot(p[0],p[1])-2)).toBeLessThan(1e-12)
  expect(Math.abs(p[2]-2.5)).toBeLessThan(1e-12)
 }
 expect(circle.coneUv).toHaveLength(4)
 for(const lift of circle.coneUv)for(const arc of lift.arcs)for(let k=0;k<=8;k++){
  const q=evaluateNurbsCurve(arc,k/8).point
  expect(Math.abs(q[1]-0.5)).toBeLessThan(1e-12)
  const p=evaluateNurbsSurface(frustum.faces[lift.patch].surface,Math.min(1,Math.max(0,q[0])),Math.min(1,Math.max(0,q[1]))).point
  expect(Math.abs(Math.hypot(p[0],p[1])-2)).toBeLessThan(1e-9)
  expect(Math.abs(p[2]-2.5)).toBeLessThan(1e-9)
 }
 for(const lift of circle.planeUv)for(const arc of lift.arcs)for(let k=0;k<=8;k++){
  const q=evaluateNurbsCurve(arc,k/8).point
  const p=evaluateNurbsSurface(plane.faces[lift.patch].surface,Math.min(1,Math.max(0,q[0])),Math.min(1,Math.max(0,q[1]))).point
  expect(Math.abs(Math.hypot(p[0],p[1])-2)).toBeLessThan(1e-9)
  expect(Math.abs(p[2]-2.5)).toBeLessThan(1e-9)
 }
 expect(circle.maxSampleResidual).toBeLessThanOrEqual(1e-12)
 // JSON roundtrip preserves the component.
 const round=JSON.parse(JSON.stringify(report)) as typeof report
 expect(round.components).toHaveLength(1)
 expect(round.components[0].kind).toBe('circle')
 // Ruling-parallel oblique plane through (0,0,2): the exact parabola, all
 // weights 1 (pure-rounding angle snap), cone-side lift null. Frustum r 3->1
 // over z 0..5: sin(alpha)=0.4/sqrt(1.16), normal (0,cos,sin), patch exactly
 // through (0,0,2) with u along world x and v along the in-plane axis
 // projection, half-extents 6 and 7 covering the whole clipped arc.
 const sigma=0.4/Math.sqrt(1.16),cs=Math.sqrt(1-sigma*sigma)
 const n:[number,number,number]=[0,cs,sigma]
 const parabolaPatch=createPlanePatch([-6,7*sigma,2-7*cs],[12,0,0],[0,-14*sigma,14*cs])
 const section=intersectPlaneCone(parabolaPatch,frustum)
 expect(section.coverage).toBe('numerically_resolved')
 expect(section.unresolved).toHaveLength(0)
 expect(section.components).toHaveLength(1)
 const parabola=section.components[0]
 if(parabola.kind!=='parabola')throw Error('Expected parabola')
 expect(parabola.curve.degree).toBe(2)
 for(const w of parabola.curve.weights)expect(w).toBe(1)
 expect(parabola.coneUv).toBeNull()
 // Exact focal-length relation |L| = 1.1 sin(alpha); direction (0,sin,-cos).
 expect(Math.abs(parabola.focalLength-1.1*sigma)).toBeLessThan(1e-12)
 parabola.vertex.forEach((x,k)=>expect(Math.abs(x-[0,-1.0998,4.7504][k])).toBeLessThan(1e-3))
 parabola.direction.forEach((x,k)=>expect(Math.abs(x-[0,sigma,-cs][k])).toBeLessThan(1e-12))
 for(let i=0;i<=8;i++){
  const p=evaluateNurbsCurve(parabola.curve,i/8).point
  expect(Math.abs(n[1]*p[1]+n[2]*(p[2]-2))).toBeLessThan(1e-12)
  expect(Math.abs(p[0]*p[0]+p[1]*p[1]-Math.pow(3-0.4*p[2],2))).toBeLessThan(1e-9)
 }
 // Both clip endpoints sit exactly on the bottom ring plane z=0, radius 3.
 for(const t of [0,1]){
  const p=evaluateNurbsCurve(parabola.curve,t).point
  expect(Math.abs(p[2])).toBeLessThan(1e-12)
  expect(Math.abs(Math.hypot(p[0],p[1])-3)).toBeLessThan(1e-9)
 }
 expect(parabola.maxSampleResidual).toBeLessThanOrEqual(1e-12)
 const roundP=JSON.parse(JSON.stringify(section)) as typeof section
 if(roundP.components[0].kind!=='parabola')throw Error('Expected parabola')
 expect(roundP.components[0].coneUv).toBeNull()
})

it('keeps plane/cone ring coincidences, apex contacts and refusals explicit through WASM',()=>{
 const frustum=createBrepFrustum(3,1,5)
 // Ring-plane coincidence at the top ring z=5: coincident_trim, never a circle.
 const coincident=intersectPlaneCone(createPlanePatch([-3,-3,5],[6,0,0],[0,6,0]),frustum)
 expect(coincident.coverage).toBe('incomplete')
 expect(coincident.components).toHaveLength(0)
 expect(coincident.unresolved.map(u=>u.reason)).toEqual(['coincident_trim'])
 expect(coincident.permitsTopologyChange).toBe(false)
 // True-apex cone: the plane through the apex stays tangency_or_multiple_root,
 // never a point component.
 const apex=intersectPlaneCone(createPlanePatch([-3,-3,5],[6,0,0],[0,6,0]),createBrepFrustum(3,0,5))
 expect(apex.components).toHaveLength(0)
 expect(apex.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 // Provable miss above the solid: empty and resolved.
 const miss=intersectPlaneCone(createPlanePatch([-3,-3,7],[6,0,0],[0,6,0]),frustum)
 expect(miss.coverage).toBe('numerically_resolved')
 expect(miss.components).toHaveLength(0)
 expect(miss.unresolved).toHaveLength(0)
 // Non-canonical operands: explicit unsupported_surface, no numerical fallback.
 // An equal-radius frustum is a cylinder (refused as a cone operand); a frustum
 // as the plane operand is refused by the fixed operand order.
 for(const [p,q] of [[createPlanePatch([-3,-3,2],[6,0,0],[0,6,0]),createBrepCylinder(2,5)],[createBrepFrustum(1,2,3),createPlanePatch([-3,-3,2],[6,0,0],[0,6,0])]] as const){
  const refused=intersectPlaneCone(p,q)
  expect(refused.coverage).toBe('incomplete')
  expect(refused.components).toHaveLength(0)
  expect(refused.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 }
 // JSON roundtrip preserves the regions.
 const round=JSON.parse(JSON.stringify(coincident)) as typeof coincident
 expect(round.components).toHaveLength(0)
 expect(round.unresolved.map(u=>u.reason)).toEqual(['coincident_trim'])
})

it('intersects an axial sphere/cone pair analytically through packed WASM',()=>{
 // Frustum r 1 -> 3 over z 0..6 (slope 1/3); sphere r=2 centered on the axis
 // at z=3: side quadratic roots q = (-2/3 +- 2/3) * 9/10 — circles at z = 1.8
 // (radius 1.6) and z = 3 (radius 2), clear of both cap planes and rims.
 const sphere=transformNurbsBrep(createBrepSphere(2),[[1,0,0,0],[0,1,0,0],[0,0,1,3],[0,0,0,1]])
 const frustum=createBrepFrustum(1,3,6)
 const report=intersectSphereCone(sphere,frustum)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.evidence).toBe('numerical_uncertified')
 expect(report.unresolved).toHaveLength(0)
 expect(report.components).toHaveLength(2)
 const q=(sign:number)=>(-2/3+sign*2/3)*0.9
 for(const [index,qi] of [q(-1),q(1)].entries()){
  const z=3+qi,rho=1+z/3
  const circle=report.components[index]
  if(circle.kind!=='circle')throw Error('Expected circle')
  expect(Math.abs(circle.radius-rho)).toBeLessThan(1e-12)
  circle.center.forEach((x,k)=>expect(Math.abs(x-[0,0,z][k])).toBeLessThan(1e-12))
  expect(Math.abs(circle.normal[2])).toBeGreaterThan(1-1e-12)
  // Exact rational circle: four 90-degree arcs, weights cos(pi/4).
  expect(circle.curve.degree).toBe(2)
  expect(circle.curve.knots).toEqual([0,0,0,1,1,2,2,3,3,4,4,4])
  expect(circle.curve.weights).toEqual([1,Math.SQRT1_2,1,Math.SQRT1_2,1,Math.SQRT1_2,1,Math.SQRT1_2,1])
  // Sixteen samples satisfy the sphere and the cone side profile equations.
  for(let i=0;i<16;i++){
   const p=evaluateNurbsCurve(circle.curve,i/4).point
   expect(Math.abs(Math.hypot(p[0],p[1],p[2]-3)-2)).toBeLessThan(1e-12)
   expect(Math.abs(Math.hypot(p[0],p[1])-rho)).toBeLessThan(1e-12)
   expect(Math.abs(p[2]-z)).toBeLessThan(1e-12)
  }
  expect(circle.maxSampleResidual).toBeLessThan(1e-12)
  // Side circles lift to iso-v lines on all four side patches at v = z/6.
  expect(circle.coneUv).toHaveLength(4)
  for(const lift of circle.coneUv){
   expect(lift.arcs).toHaveLength(1)
   const arc=lift.arcs[0]
   expect(arc.degree).toBe(1)
   expect(Math.abs(arc.controlPoints[0][1]-z/6)).toBeLessThan(1e-12)
  }
  expect(circle.sphereUv.length).toBeGreaterThan(0)
  // Every lifted UV point on either surface evaluates onto both equations.
  for(const [model,lifts] of [[frustum,circle.coneUv],[sphere,circle.sphereUv]] as const){
   for(const lift of lifts)for(const arc of lift.arcs)for(let k=0;k<=8;k++){
    const uv=evaluateNurbsCurve(arc,k/8).point
    const p=evaluateNurbsSurface(model.faces[lift.patch].surface,Math.min(1,Math.max(0,uv[0])),Math.min(1,Math.max(0,uv[1]))).point
    expect(Math.abs(Math.hypot(p[0],p[1],p[2]-3)-2)).toBeLessThan(1e-9)
    expect(Math.abs(Math.hypot(p[0],p[1])-(1+p[2]/3))).toBeLessThan(1e-9)
   }
  }
 }
 // JSON roundtrip preserves the components.
 const round=JSON.parse(JSON.stringify(report)) as typeof report
 expect(round.components).toHaveLength(2)
 expect(round.components[0].kind).toBe('circle')
 if(round.components[0].kind==='circle')expect(round.components[0].coneUv).toHaveLength(4)
 // Small sphere poking through the top cap within the disk: an exact circle
 // of radius sqrt(2) in the cap plane z=6 (ring radius 4), one UV-circle lift.
 const poking=transformNurbsBrep(createBrepSphere(1.5),[[1,0,0,0],[0,1,0,0],[0,0,1,5.5],[0,0,0,1]])
 const cap=intersectSphereCone(poking,createBrepFrustum(2,4,6))
 expect(cap.coverage).toBe('numerically_resolved')
 expect(cap.components).toHaveLength(1)
 const circle=cap.components[0]
 if(circle.kind!=='circle')throw Error('Expected circle')
 expect(Math.abs(circle.radius-Math.sqrt(2))).toBeLessThan(1e-12)
 circle.center.forEach((x,k)=>expect(Math.abs(x-[0,0,6][k])).toBeLessThan(1e-12))
 expect(circle.coneUv).toHaveLength(1)
 expect(circle.coneUv[0].arcs).toHaveLength(4)
 const capModel=createBrepFrustum(2,4,6)
 for(const arc of circle.coneUv[0].arcs){
  expect(arc.weights).toEqual([1,Math.SQRT1_2,1])
  const e=arc.controlPoints[0]
  expect(Math.abs(Math.hypot(e[0]-.5,e[1]-.5)-Math.sqrt(2)/8)).toBeLessThan(1e-12)
  for(let k=0;k<=8;k++){
   const uv=evaluateNurbsCurve(arc,k/8).point
   const p=evaluateNurbsSurface(capModel.faces[circle.coneUv[0].patch].surface,uv[0],uv[1]).point
   expect(Math.abs(Math.hypot(p[0],p[1],p[2]-5.5)-1.5)).toBeLessThan(1e-9)
   expect(Math.abs(p[2]-6)).toBeLessThan(1e-9)
  }
 }
})

it('keeps sphere/cone bands, tangencies and refusals explicit through WASM',()=>{
 const frustum=createBrepFrustum(2,4,6)
 const place=(radius:number,x:number,z:number)=>transformNurbsBrep(createBrepSphere(radius),[[1,0,0,x],[0,1,0,0],[0,0,1,z],[0,0,0,1]])
 // Side tangency (double root): r == rho_c/sqrt(1+m^2) = 9/sqrt(10) at z=3,
 // exactly and within the band — never a guessed circle.
 for(const sphere of [place(9/Math.sqrt(10),0,3),place(9/Math.sqrt(10)+2e-15,0,3)]){
  const report=intersectSphereCone(sphere,frustum)
  expect(report.coverage).toBe('incomplete')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
  expect(report.permitsTopologyChange).toBe(false)
 }
 // Rim contact: r == r_bottom with the sphere centered on the bottom ring
 // plane — the whole rim circle lies on the sphere, a tangent boundary contact.
 const rim=intersectSphereCone(place(2,0,0),frustum)
 expect(rim.components).toHaveLength(0)
 expect(rim.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 // Apex contact: the sphere through the true apex stays unresolved while the
 // transverse side circle still reports.
 const apex=intersectSphereCone(place(2,0,2),createBrepFrustum(0,3,5))
 expect(apex.coverage).toBe('incomplete')
 expect(apex.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 expect(apex.components).toHaveLength(1)
 // Cap-plane touch: tangency_or_multiple_root, never a point.
 const touch=intersectSphereCone(place(1.5,0,4.5),frustum)
 expect(touch.components).toHaveLength(0)
 expect(touch.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 // Near-axial within the recognition band: near_coincidence, not forced axial.
 const near=intersectSphereCone(place(2.5,1e-10,.5),frustum)
 expect(near.unresolved.map(u=>u.reason)).toEqual(['near_coincidence'])
 // Clearly off-axis: unsupported_surface, the general quartic is out of scope.
 const off=intersectSphereCone(place(2.5,.5,.5),frustum)
 expect(off.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 // Empty resolved: small sphere inside, sphere beyond the top, big swallow.
 for(const sphere of [place(.5,0,3),place(1,0,8),place(20,0,3)]){
  const report=intersectSphereCone(sphere,frustum)
  expect(report.coverage).toBe('numerically_resolved')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved).toHaveLength(0)
 }
 // Non-canonical operands: explicit unsupported_surface, no numerical
 // fallback — an equal-radius frustum is a cylinder, and swapped order is
 // refused by the fixed operand order.
 for(const [a,b] of [[createBrepSphere(2),createBrepCylinder(1,3)],[createBrepFrustum(1,2,3),createBrepSphere(2)],[createBrepSphere(2),transformNurbsBrep(createBrepSphere(1),[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,1]])]] as const){
  const refused=intersectSphereCone(a,b)
  expect(refused.coverage).toBe('incomplete')
  expect(refused.components).toHaveLength(0)
  expect(refused.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 }
 // JSON roundtrip preserves the regions.
 const round=JSON.parse(JSON.stringify(touch)) as typeof touch
 expect(round.components).toHaveLength(0)
 expect(round.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
})

it('intersects a coaxial cone/cone pair analytically through packed WASM',()=>{
 // Cone1 r 1 -> 3 over z 0..6 (rho = 1 + s/3); cone2 r 4 -> 2 over z 1..5
 // (rho = 4.5 - s/2). Root: (5/6) s = 3.5 — s* = 4.2 strictly inside both
 // ranges, radius 2.4.
 const first=createBrepFrustum(1,3,6)
 const second=transformNurbsBrep(createBrepFrustum(4,2,4),[[1,0,0,0],[0,1,0,0],[0,0,1,1],[0,0,0,1]])
 const report=intersectConeCone(first,second)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.evidence).toBe('numerical_uncertified')
 expect(report.unresolved).toHaveLength(0)
 expect(report.components).toHaveLength(1)
 const circle=report.components[0]
 if(circle.kind!=='circle')throw Error('Expected circle')
 expect(Math.abs(circle.radius-2.4)).toBeLessThan(1e-12)
 circle.center.forEach((x,k)=>expect(Math.abs(x-[0,0,4.2][k])).toBeLessThan(1e-12))
 expect(Math.abs(circle.normal[2])).toBeGreaterThan(1-1e-12)
 // Exact rational circle: four 90-degree arcs, weights cos(pi/4).
 expect(circle.curve.degree).toBe(2)
 expect(circle.curve.knots).toEqual([0,0,0,1,1,2,2,3,3,4,4,4])
 expect(circle.curve.weights).toEqual([1,Math.SQRT1_2,1,Math.SQRT1_2,1,Math.SQRT1_2,1,Math.SQRT1_2,1])
 // Sixteen samples satisfy both implicit side equations.
 for(let i=0;i<16;i++){
  const p=evaluateNurbsCurve(circle.curve,i/4).point
  expect(Math.abs(Math.hypot(p[0],p[1])-2.4)).toBeLessThan(1e-12)
  expect(Math.abs(p[2]-4.2)).toBeLessThan(1e-12)
 }
 expect(circle.maxSampleResidual).toBeLessThan(1e-12)
 // Iso-v lifts on all four side patches of both cones (v=0.7 and v=0.8).
 expect(circle.firstUv).toHaveLength(4)
 expect(circle.secondUv).toHaveLength(4)
 for(const lift of circle.firstUv){
  expect(lift.arcs).toHaveLength(1)
  expect(Math.abs(lift.arcs[0].controlPoints[0][1]-.7)).toBeLessThan(1e-12)
 }
 for(const lift of circle.secondUv){
  expect(lift.arcs).toHaveLength(1)
  expect(Math.abs(lift.arcs[0].controlPoints[0][1]-.8)).toBeLessThan(1e-12)
 }
 // Every lifted UV point on either cone evaluates onto both side equations.
 for(const [model,lifts] of [[first,circle.firstUv],[second,circle.secondUv]] as const){
  for(const lift of lifts)for(const arc of lift.arcs)for(let k=0;k<=8;k++){
   const uv=evaluateNurbsCurve(arc,k/8).point
   const p=evaluateNurbsSurface(model.faces[lift.patch].surface,uv[0],uv[1]).point
   expect(Math.abs(Math.hypot(p[0],p[1])-2.4)).toBeLessThan(1e-9)
   expect(Math.abs(p[2]-4.2)).toBeLessThan(1e-9)
  }
 }
 // Anti-axial pair: cone2 flipped down (apex side up) still yields the exact
 // circle — rho = 2 + (8 - s)/3 meets rho = 1 + s/3 at s* = 5.5, radius 17/6.
 const flipped=transformNurbsBrep(createBrepFrustum(2,4,6),[[1,0,0,0],[0,-1,0,0],[0,0,-1,8],[0,0,0,1]])
 const anti=intersectConeCone(createBrepFrustum(1,3,6),flipped)
 expect(anti.coverage).toBe('numerically_resolved')
 expect(anti.components).toHaveLength(1)
 const antiCircle=anti.components[0]
 if(antiCircle.kind!=='circle')throw Error('Expected circle')
 expect(Math.abs(antiCircle.radius-17/6)).toBeLessThan(1e-12)
 antiCircle.center.forEach((x,k)=>expect(Math.abs(x-[0,0,5.5][k])).toBeLessThan(1e-12))
 expect(antiCircle.secondUv).toHaveLength(4)
 for(const lift of antiCircle.secondUv)expect(Math.abs(lift.arcs[0].controlPoints[0][1]-5/12)).toBeLessThan(1e-12)
 // JSON roundtrip preserves the components.
 const round=JSON.parse(JSON.stringify(report)) as typeof report
 expect(round.components).toHaveLength(1)
 expect(round.components[0].kind).toBe('circle')
 if(round.components[0].kind==='circle')expect(round.components[0].secondUv).toHaveLength(4)
})

it('keeps cone/cone coincidences, tangencies and refusals explicit through WASM',()=>{
 const first=createBrepFrustum(1,3,6)
 const place=(rBottom:number,rTop:number,height:number,z:number)=>transformNurbsBrep(createBrepFrustum(rBottom,rTop,height),[[1,0,0,0],[0,1,0,0],[0,0,1,z],[0,0,0,1]])
 // Equal-taper coincident profiles over the overlap z 2..6: coincident_trim,
 // never a surface component.
 const coincident=intersectConeCone(first,place(5/3,11/3,6,2))
 expect(coincident.coverage).toBe('incomplete')
 expect(coincident.components).toHaveLength(0)
 expect(coincident.unresolved.map(u=>u.reason)).toEqual(['coincident_trim'])
 expect(coincident.permitsTopologyChange).toBe(false)
 // Stacked rim/rim contact at the shared ring plane z=6.
 const rim=intersectConeCone(first,place(3,5,6,6))
 expect(rim.components).toHaveLength(0)
 expect(rim.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 // Rim-on-side: cone2's bottom ring (radius 2 at z=3) lies exactly on
 // cone1's side — never a guessed circle.
 const rimOnSide=intersectConeCone(first,place(2,5,6,3))
 expect(rimOnSide.components).toHaveLength(0)
 expect(rimOnSide.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 // Apex meeting: two true-apex cones apex-to-apex at the origin.
 const apex=intersectConeCone(createBrepFrustum(0,3,5),transformNurbsBrep(createBrepFrustum(0,2,4),[[1,0,0,0],[0,-1,0,0],[0,0,-1,0],[0,0,0,1]]))
 expect(apex.components).toHaveLength(0)
 expect(apex.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 // Equal-taper distinct profiles, clipped root and clear axial separation:
 // empty and numerically resolved.
 for(const [a,b] of [[createBrepFrustum(1,3,6),place(2,4,6,2)],[createBrepFrustum(3,1,6),place(4,2,2,2)],[createBrepFrustum(1,3,6),place(1,2,3,6.5)]] as const){
  const report=intersectConeCone(a,b)
  expect(report.coverage).toBe('numerically_resolved')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved).toHaveLength(0)
 }
 // Near-coaxial within the recognition band: near_coincidence, never forced.
 const near=intersectConeCone(first,transformNurbsBrep(createBrepFrustum(4,2,4),[[1,0,0,1e-10],[0,1,0,0],[0,0,1,1],[0,0,0,1]]))
 expect(near.unresolved.map(u=>u.reason)).toEqual(['near_coincidence'])
 // Clearly off-axis and non-parallel: unsupported_surface, the general
 // quartic is out of scope.
 const off=intersectConeCone(first,transformNurbsBrep(createBrepFrustum(4,2,4),[[1,0,0,.5],[0,1,0,0],[0,0,1,1],[0,0,0,1]]))
 expect(off.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 const tilted=intersectConeCone(first,transformNurbsBrep(createBrepFrustum(4,2,4),[[1,0,0,0],[0,Math.cos(.5),-Math.sin(.5),0],[0,Math.sin(.5),Math.cos(.5),1],[0,0,0,1]]))
 expect(tilted.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 // Non-canonical operands: explicit unsupported_surface, no numerical
 // fallback — an equal-radius frustum is a cylinder.
 for(const [a,b] of [[createBrepFrustum(1,3,6),createBrepCylinder(1,3)],[createBrepSphere(2),createBrepFrustum(1,3,6)]] as const){
  const refused=intersectConeCone(a,b)
  expect(refused.coverage).toBe('incomplete')
  expect(refused.components).toHaveLength(0)
  expect(refused.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 }
 // JSON roundtrip preserves the regions.
 const round=JSON.parse(JSON.stringify(coincident)) as typeof coincident
 expect(round.components).toHaveLength(0)
 expect(round.unresolved.map(u=>u.reason)).toEqual(['coincident_trim'])
})

it('resolves plane/torus axial sections analytically through packed WASM',()=>{
 const torus=createBrepTorus(3,1)
 // Torus implicit: (x^2+y^2+z^2+R^2-r^2)^2 = 4R^2(x^2+y^2), R=3, r=1.
 const F=(p:number[])=>{const t=p[0]*p[0]+p[1]*p[1]+p[2]*p[2],r2=p[0]*p[0]+p[1]*p[1],b=t+9-1;return Math.abs(b*b-36*r2)}
 // Perpendicular plane z = 0.4, patch [-5,5]^2: the exact parallel pair
 // 3 +- sqrt(0.84), fully inside the patch.
 const plane=createPlanePatch([-5,-5,0.4],[10,0,0],[0,10,0])
 const report=intersectPlaneTorus(plane,torus)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.unresolved).toHaveLength(0)
 expect(report.components).toHaveLength(2)
 const s=Math.sqrt(1-0.16)
 report.components.forEach((circle,k)=>{
  if(circle.kind!=='circle')throw Error('Expected circles')
  expect(Math.abs(circle.radius-(3+(k===0?s:-s)))).toBeLessThan(1e-12)
  circle.center.forEach((x,i)=>expect(Math.abs(x-[0,0,0.4][i])).toBeLessThan(1e-12))
  expect(circle.full).toBe(true)
  expect(circle.curve.weights).toHaveLength(9)
  for(let i=0;i<16;i++){
   const p=evaluateNurbsCurve(circle.curve,i/4).point
   expect(F(p)).toBeLessThan(1e-10)
   expect(Math.abs(p[2]-0.4)).toBeLessThan(1e-12)
  }
  // Torus UV: one iso-v degree-1 line per revolution quadrant patch; each
  // lift evaluates through its own patch surface onto both equations.
  expect(circle.torusUv).toHaveLength(4)
  for(const lift of circle.torusUv)for(const arc of lift.arcs){
   expect(arc.degree).toBe(1)
   expect(arc.controlPoints[0][1]).toBe(arc.controlPoints[1][1])
   for(let q=0;q<=8;q++){
    const uv=evaluateNurbsCurve(arc,q/8).point
    const p=evaluateNurbsSurface(torus.faces[lift.patch].surface,Math.min(1,Math.max(0,uv[0])),Math.min(1,Math.max(0,uv[1]))).point
    expect(F(p)).toBeLessThan(1e-9)
    expect(Math.abs(p[2]-0.4)).toBeLessThan(1e-9)
   }
  }
  for(const lift of circle.planeUv)for(const arc of lift.arcs)for(let q=0;q<=8;q++){
   const uv=evaluateNurbsCurve(arc,q/8).point
   const p=evaluateNurbsSurface(plane.faces[lift.patch].surface,Math.min(1,Math.max(0,uv[0])),Math.min(1,Math.max(0,uv[1]))).point
   expect(F(p)).toBeLessThan(1e-9)
   expect(Math.abs(p[2]-0.4)).toBeLessThan(1e-9)
  }
  expect(circle.maxSampleResidual).toBeLessThanOrEqual(1e-10)
 })
 // Symmetry plane z = 0: the exact equator pair 4 and 2.
 const equator=intersectPlaneTorus(createPlanePatch([-5,-5,0],[10,0,0],[0,10,0]),torus)
 expect(equator.components.map(c=>c.kind==='circle'?Math.round(c.radius):0)).toEqual([4,2])
 // Through-axis plane y = 0: the meridian pair of radius 1 at (+-3,0,0).
 const axial=intersectPlaneTorus(createPlanePatch([-5,0,-5],[10,0,0],[0,0,10]),torus)
 expect(axial.coverage).toBe('numerically_resolved')
 expect(axial.components).toHaveLength(2)
 const signs:number[]=[]
 for(const circle of axial.components){
  if(circle.kind!=='circle')throw Error('Expected circles')
  expect(Math.abs(circle.radius-1)).toBeLessThan(1e-12)
  expect(Math.abs(Math.abs(circle.center[0])-3)).toBeLessThan(1e-12)
  expect(Math.abs(circle.center[1])).toBeLessThan(1e-12)
  signs.push(circle.center[0])
  for(let i=0;i<16;i++){
   const p=evaluateNurbsCurve(circle.curve,i/4).point
   expect(F(p)).toBeLessThan(1e-10)
   expect(Math.abs(p[1])).toBeLessThan(1e-12)
  }
  // Torus UV: one iso-u degree-1 line per profile quadrant patch.
  expect(circle.torusUv).toHaveLength(4)
  for(const lift of circle.torusUv)for(const arc of lift.arcs){
   expect(arc.controlPoints[0][0]).toBe(arc.controlPoints[1][0])
   for(let q=0;q<=8;q++){
    const uv=evaluateNurbsCurve(arc,q/8).point
    const p=evaluateNurbsSurface(torus.faces[lift.patch].surface,Math.min(1,Math.max(0,uv[0])),Math.min(1,Math.max(0,uv[1]))).point
    expect(F(p)).toBeLessThan(1e-9)
    expect(Math.abs(p[1])).toBeLessThan(1e-9)
   }
  }
  expect(circle.maxSampleResidual).toBeLessThanOrEqual(1e-10)
 }
 expect(signs[0]*signs[1]).toBeLessThan(0)
 // JSON roundtrip preserves the components.
 const round=JSON.parse(JSON.stringify(report)) as typeof report
 expect(round.components).toHaveLength(2)
 expect(round.components[0].kind).toBe('circle')
})

it('keeps plane/torus tangencies, Cassini and oblique refusals explicit through WASM',()=>{
 const torus=createBrepTorus(3,1)
 const at=(z:number)=>createPlanePatch([-5,-5,z],[10,0,0],[0,10,0])
 // Tangency |h| == r and within the band: never a guessed circle.
 for(const z of [1,1-3e-14]){
  const report=intersectPlaneTorus(at(z),torus)
  expect(report.coverage).toBe('incomplete')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
  expect(report.permitsTopologyChange).toBe(false)
 }
 // Clear miss: empty and resolved.
 const miss=intersectPlaneTorus(at(2),torus)
 expect(miss.coverage).toBe('numerically_resolved')
 expect(miss.components).toHaveLength(0)
 expect(miss.unresolved).toHaveLength(0)
 // Axis-parallel plane at distance 0.5: a Cassini oval, unsupported.
 const cassini=intersectPlaneTorus(createPlanePatch([-5,.5,-5],[10,0,0],[0,0,10]),torus)
 expect(cassini.components).toHaveLength(0)
 expect(cassini.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 // Within the recognition-scale offset band: near_coincidence, never snapped.
 const nearAxis=intersectPlaneTorus(createPlanePatch([-5,1e-12,-5],[10,0,0],[0,0,10]),torus)
 expect(nearAxis.unresolved.map(u=>u.reason)).toEqual(['near_coincidence'])
 // Oblique plane (quartic with Villarceau degeneracies): unsupported.
 const c=Math.SQRT1_2
 const oblique=intersectPlaneTorus(createPlanePatch([-5,-5*c,-5*c],[10,0,0],[0,10*c,10*c]),torus)
 expect(oblique.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 // Recognition-scale tilt off perpendicular: near_coincidence, never forced.
 const a=1e-6
 const tilted=intersectPlaneTorus(createPlanePatch([-5,-5*Math.cos(a),-5*Math.sin(a)],[10,0,0],[0,10*Math.cos(a),10*Math.sin(a)]),torus)
 expect(tilted.unresolved.map(u=>u.reason)).toEqual(['near_coincidence'])
 // Non-canonical operands: explicit unsupported_surface, no numerical
 // fallback — a sphere is not the torus; a solid is not the planar patch.
 for(const [p,q] of [[at(0.4),createBrepSphere(2)],[createBrepSphere(2),torus],[createPlanePatch([-5,-5,0.4],[10,0,0],[3,10,0]),torus]] as const){
  const refused=intersectPlaneTorus(p,q)
  expect(refused.coverage).toBe('incomplete')
  expect(refused.components).toHaveLength(0)
  expect(refused.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 }
 // JSON roundtrip preserves the regions.
 const round=JSON.parse(JSON.stringify(cassini)) as typeof cassini
 expect(round.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
})

it('resolves sphere/torus axial circle pairs analytically through packed WASM',()=>{
 const torus=createBrepTorus(3,1)
 // Torus implicit: (x^2+y^2+z^2+R^2-r^2)^2 = 4R^2(x^2+y^2), R=3, r=1.
 const F=(p:number[])=>{const t=p[0]*p[0]+p[1]*p[1]+p[2]*p[2],r2=p[0]*p[0]+p[1]*p[1],b=t+9-1;return Math.abs(b*b-36*r2)}
 // Sphere r = sqrt(5.2) centered at the torus center: the exact circle
 // pair of radius 2.2 at z = +-0.6.
 const sphere=createBrepSphere(Math.sqrt(5.2))
 const report=intersectSphereTorus(sphere,torus)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.unresolved).toHaveLength(0)
 expect(report.components).toHaveLength(2)
 report.components.forEach((circle,k)=>{
  if(circle.kind!=='circle')throw Error('Expected circles')
  expect(Math.abs(circle.radius-2.2)).toBeLessThan(1e-12)
  circle.center.forEach((x,i)=>expect(Math.abs(x-[0,0,k===0?-.6:.6][i])).toBeLessThan(1e-12))
  expect(circle.curve.weights).toHaveLength(9)
  for(let i=0;i<16;i++){
   const p=evaluateNurbsCurve(circle.curve,i/4).point
   expect(F(p)).toBeLessThan(1e-10)
   expect(Math.abs(Math.hypot(p[0],p[1],p[2])-Math.sqrt(5.2))).toBeLessThan(1e-12)
  }
  // Torus UV: one iso-v degree-1 line per revolution quadrant patch; each
  // lift evaluates through its own patch surface onto both equations.
  expect(circle.torusUv).toHaveLength(4)
  for(const lift of circle.torusUv)for(const arc of lift.arcs){
   expect(arc.degree).toBe(1)
   expect(arc.controlPoints[0][1]).toBe(arc.controlPoints[1][1])
   for(let q=0;q<=8;q++){
    const uv=evaluateNurbsCurve(arc,q/8).point
    const p=evaluateNurbsSurface(torus.faces[lift.patch].surface,Math.min(1,Math.max(0,uv[0])),Math.min(1,Math.max(0,uv[1]))).point
    expect(F(p)).toBeLessThan(1e-9)
    expect(Math.abs(Math.hypot(p[0],p[1],p[2])-Math.sqrt(5.2))).toBeLessThan(1e-9)
   }
  }
  // Sphere UV: per-patch stereographic lifts on both equations.
  expect(circle.sphereUv.length).toBeGreaterThan(0)
  for(const lift of circle.sphereUv)for(const arc of lift.arcs)for(let q=0;q<=8;q++){
   const uv=evaluateNurbsCurve(arc,q/8).point
   const p=evaluateNurbsSurface(sphere.faces[lift.patch].surface,Math.min(1,Math.max(0,uv[0])),Math.min(1,Math.max(0,uv[1]))).point
   expect(F(p)).toBeLessThan(1e-9)
   expect(Math.abs(Math.hypot(p[0],p[1],p[2])-Math.sqrt(5.2))).toBeLessThan(1e-9)
  }
  expect(circle.maxSampleResidual).toBeLessThanOrEqual(1e-10)
 })
 // Off-center sphere r = sqrt(26) at z = -4: circles 3.8 at z=-0.6 and
 // 2.2 at z=+0.6.
 const off=intersectSphereTorus(transformNurbsBrep(createBrepSphere(Math.sqrt(26)),[[1,0,0,0],[0,1,0,0],[0,0,1,-4],[0,0,0,1]]),torus)
 expect(off.coverage).toBe('numerically_resolved')
 expect(off.components).toHaveLength(2)
 off.components.forEach((circle,k)=>{
  if(circle.kind!=='circle')throw Error('Expected circles')
  const [rho,z]=k===0?[3.8,-.6]:[2.2,.6]
  expect(Math.abs(circle.radius-rho)).toBeLessThan(1e-12)
  circle.center.forEach((x,i)=>expect(Math.abs(x-[0,0,z][i])).toBeLessThan(1e-12))
  for(let i=0;i<16;i++){
   const p=evaluateNurbsCurve(circle.curve,i/4).point
   expect(F(p)).toBeLessThan(1e-10)
   expect(Math.abs(Math.hypot(p[0],p[1],p[2]+4)-Math.sqrt(26))).toBeLessThan(1e-12)
  }
  expect(circle.maxSampleResidual).toBeLessThanOrEqual(1e-10)
 })
 // JSON roundtrip preserves the components.
 const round=JSON.parse(JSON.stringify(report)) as typeof report
 expect(round.components).toHaveLength(2)
 expect(round.components[0].kind).toBe('circle')
})

it('keeps sphere/torus tangencies and refusals explicit through WASM',()=>{
 const torus=createBrepTorus(3,1)
 const at=(r:number,z=0)=>transformNurbsBrep(createBrepSphere(r),[[1,0,0,0],[0,1,0,0],[0,0,1,z],[0,0,0,1]])
 // Inner-equator tangency r == R - r_t = 2 and within the band: never a
 // guessed circle.
 for(const r of [2,2-2e-15]){
  const report=intersectSphereTorus(at(r),torus)
  expect(report.coverage).toBe('incomplete')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
  expect(report.permitsTopologyChange).toBe(false)
 }
 // Just clear of the band on the outside: provable miss, resolved.
 const clear=intersectSphereTorus(at(2-1e-9),torus)
 expect(clear.coverage).toBe('numerically_resolved')
 expect(clear.components).toHaveLength(0)
 expect(clear.unresolved).toHaveLength(0)
 // Outer-equator internal tangency r == R + r_t = 4.
 expect(intersectSphereTorus(at(4),torus).unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
 // Clear misses: small sphere in the hole, huge sphere swallowing the torus.
 for(const s of [at(1,.5),at(10)]){
  const report=intersectSphereTorus(s,torus)
  expect(report.coverage).toBe('numerically_resolved')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved).toHaveLength(0)
 }
 // Near-axial offset within the recognition band: near_coincidence, never snapped.
 const near=intersectSphereTorus(transformNurbsBrep(createBrepSphere(Math.sqrt(5.2)),[[1,0,0,1e-10],[0,1,0,0],[0,0,1,0],[0,0,0,1]]),torus)
 expect(near.unresolved.map(u=>u.reason)).toEqual(['near_coincidence'])
 // Clearly off-axis: the general quartic is unsupported, no fallback.
 const off=intersectSphereTorus(transformNurbsBrep(createBrepSphere(Math.sqrt(5.2)),[[1,0,0,.5],[0,1,0,0],[0,0,1,0],[0,0,0,1]]),torus)
 expect(off.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 // Non-canonical operands and swapped order: explicit unsupported_surface.
 for(const [a,b] of [[createBrepSphere(2),createBrepCylinder(1,3)],[torus,createBrepSphere(2)],[createBrepSphere(2),createBrepSphere(2)]] as const){
  const refused=intersectSphereTorus(a,b)
  expect(refused.coverage).toBe('incomplete')
  expect(refused.components).toHaveLength(0)
  expect(refused.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 }
 // JSON roundtrip preserves the regions.
 const round=JSON.parse(JSON.stringify(off)) as typeof off
 expect(round.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
})


it('resolves cylinder/torus coaxial circle pairs analytically through packed WASM',()=>{
 const torus=createBrepTorus(3,1)
 // Torus implicit: (x^2+y^2+z^2+R^2-r^2)^2 = 4R^2(x^2+y^2), R=3, r=1.
 const F=(p:number[])=>{const t=p[0]*p[0]+p[1]*p[1]+p[2]*p[2],r2=p[0]*p[0]+p[1]*p[1],b=t+9-1;return Math.abs(b*b-36*r2)}
 // Cylinder R_c = 2.2 spanning z in -4..4: the exact side circle pair of
 // radius 2.2 at z = +-sqrt(1 - 0.64) = +-0.6.
 const cylinder=transformNurbsBrep(createBrepCylinder(2.2,8),[[1,0,0,0],[0,1,0,0],[0,0,1,-4],[0,0,0,1]])
 const report=intersectCylinderTorus(cylinder,torus)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.unresolved).toHaveLength(0)
 expect(report.components).toHaveLength(2)
 report.components.forEach((circle,k)=>{
  if(circle.kind!=='circle')throw Error('Expected circles')
  expect(Math.abs(circle.radius-2.2)).toBeLessThan(1e-12)
  circle.center.forEach((x,i)=>expect(Math.abs(x-[0,0,k===0?-.6:.6][i])).toBeLessThan(1e-12))
  expect(circle.curve.weights).toHaveLength(9)
  for(let i=0;i<16;i++){
   const p=evaluateNurbsCurve(circle.curve,i/4).point
   expect(F(p)).toBeLessThan(1e-10)
   expect(Math.abs(Math.hypot(p[0],p[1])-2.2)).toBeLessThan(1e-12)
  }
  // Cylinder UV: one iso-v degree-1 line per side patch, evaluating through
  // its own patch surface onto both equations.
  expect(circle.cylinderUv).toHaveLength(4)
  for(const lift of circle.cylinderUv)for(const arc of lift.arcs){
   expect(arc.degree).toBe(1)
   expect(arc.controlPoints[0][1]).toBe(arc.controlPoints[1][1])
   for(let q=0;q<=8;q++){
    const uv=evaluateNurbsCurve(arc,q/8).point
    const p=evaluateNurbsSurface(cylinder.faces[lift.patch].surface,Math.min(1,Math.max(0,uv[0])),Math.min(1,Math.max(0,uv[1]))).point
    expect(F(p)).toBeLessThan(1e-9)
    expect(Math.abs(Math.hypot(p[0],p[1])-2.2)).toBeLessThan(1e-9)
   }
  }
  // Torus UV: one iso-v degree-1 line per revolution quadrant patch.
  expect(circle.torusUv).toHaveLength(4)
  for(const lift of circle.torusUv)for(const arc of lift.arcs){
   expect(arc.degree).toBe(1)
   expect(arc.controlPoints[0][1]).toBe(arc.controlPoints[1][1])
   for(let q=0;q<=8;q++){
    const uv=evaluateNurbsCurve(arc,q/8).point
    const p=evaluateNurbsSurface(torus.faces[lift.patch].surface,Math.min(1,Math.max(0,uv[0])),Math.min(1,Math.max(0,uv[1]))).point
    expect(F(p)).toBeLessThan(1e-9)
    expect(Math.abs(Math.hypot(p[0],p[1])-2.2)).toBeLessThan(1e-9)
   }
  }
  expect(circle.maxSampleResidual).toBeLessThanOrEqual(1e-10)
 })
 // Short wide cylinder R_c = 4.5 spanning -0.25..0.25: four cap circles of
 // radii 3 +- sqrt(1 - 0.0625) in the cap planes z = +-0.25.
 const wide=transformNurbsBrep(createBrepCylinder(4.5,.5),[[1,0,0,0],[0,1,0,0],[0,0,1,-.25],[0,0,0,1]])
 const caps=intersectCylinderTorus(wide,torus)
 expect(caps.coverage).toBe('numerically_resolved')
 expect(caps.components).toHaveLength(4)
 const s=Math.sqrt(1-.0625)
 caps.components.forEach((circle,k)=>{
  if(circle.kind!=='circle')throw Error('Expected circles')
  const [z,rho]=[[-.25,3-s],[-.25,3+s],[.25,3-s],[.25,3+s]][k]
  expect(Math.abs(circle.radius-rho)).toBeLessThan(1e-12)
  circle.center.forEach((x,i)=>expect(Math.abs(x-[0,0,z][i])).toBeLessThan(1e-12))
  for(let i=0;i<16;i++){
   const p=evaluateNurbsCurve(circle.curve,i/4).point
   expect(F(p)).toBeLessThan(1e-10)
   expect(Math.abs(p[2]-z)).toBeLessThan(1e-12)
  }
  // Cap UV: four exact 90-degree arcs on one cap face.
  expect(circle.cylinderUv).toHaveLength(1)
  expect(circle.cylinderUv[0].arcs).toHaveLength(4)
  expect(circle.torusUv).toHaveLength(4)
  expect(circle.maxSampleResidual).toBeLessThanOrEqual(1e-10)
 })
 // JSON roundtrip preserves the components.
 const round=JSON.parse(JSON.stringify(report)) as typeof report
 expect(round.components).toHaveLength(2)
 expect(round.components[0].kind).toBe('circle')
})

it('keeps cylinder/torus tangencies and refusals explicit through WASM',()=>{
 const torus=createBrepTorus(3,1)
 const at=(r:number,x=0)=>transformNurbsBrep(createBrepCylinder(r,8),[[1,0,0,x],[0,1,0,0],[0,0,1,-4],[0,0,0,1]])
 // Meridian tangency R_c == R - r = 2 and R_c == R + r = 4 (plus within the
 // band): never a guessed circle.
 for(const r of [2,2-2e-15,4]){
  const report=intersectCylinderTorus(at(r),torus)
  expect(report.coverage).toBe('incomplete')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
  expect(report.permitsTopologyChange).toBe(false)
 }
 // Just clear of the band: the line rho = 2 - 1e-9 misses the meridian
 // circle, resolved empty.
 const clear=intersectCylinderTorus(at(2-1e-9),torus)
 expect(clear.coverage).toBe('numerically_resolved')
 expect(clear.components).toHaveLength(0)
 expect(clear.unresolved).toHaveLength(0)
 // Empty branches: thin cylinder in the hole; huge cylinder swallowing the
 // torus.
 for(const c of [at(1),transformNurbsBrep(createBrepCylinder(20,40),[[1,0,0,0],[0,1,0,0],[0,0,1,-20],[0,0,0,1]])]){
  const report=intersectCylinderTorus(c,torus)
  expect(report.coverage).toBe('numerically_resolved')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved).toHaveLength(0)
 }
 // Near-coaxial offset within the recognition band: near_coincidence, never
 // snapped.
 const near=intersectCylinderTorus(at(2.2,1e-10),torus)
 expect(near.unresolved.map(u=>u.reason)).toEqual(['near_coincidence'])
 // Clearly off-axis: the general quartic is unsupported, no fallback.
 const off=intersectCylinderTorus(at(2.2,.5),torus)
 expect(off.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 // Non-canonical operands and swapped order: explicit unsupported_surface.
 for(const [a,b] of [[createBrepSphere(2),torus],[torus,createBrepCylinder(2.2,8)],[createBrepCylinder(2.2,8),createBrepSphere(2)]] as const){
  const refused=intersectCylinderTorus(a,b)
  expect(refused.coverage).toBe('incomplete')
  expect(refused.components).toHaveLength(0)
  expect(refused.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 }
 // JSON roundtrip preserves the regions.
 const round=JSON.parse(JSON.stringify(off)) as typeof off
 expect(round.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
})


it('resolves cone/torus coaxial circle pairs analytically through packed WASM',()=>{
 const torus=createBrepTorus(3,1)
 // Torus implicit: (x^2+y^2+z^2+R^2-r^2)^2 = 4R^2(x^2+y^2), R=3, r=1.
 const F=(p:number[])=>{const t=p[0]*p[0]+p[1]*p[1]+p[2]*p[2],r2=p[0]*p[0]+p[1]*p[1],b=t+9-1;return Math.abs(b*b-36*r2)}
 // Frustum r 1 -> 3 over z 0..6 with its bottom ring at z=-3: the meridian
 // quadratic 5 t^2 - 33 t + 54 = 0 has the exact roots t = 3 and t = 3.6 —
 // the side circle pair (rho=2, z=0) and (rho=2.2, z=0.6).
 const cone=transformNurbsBrep(createBrepFrustum(1,3,6),[[1,0,0,0],[0,1,0,0],[0,0,1,-3],[0,0,0,1]])
 const side=(p:number[])=>Math.abs(Math.hypot(p[0],p[1])-(1+(p[2]+3)/3))
 const report=intersectConeTorus(cone,torus)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.unresolved).toHaveLength(0)
 expect(report.components).toHaveLength(2)
 report.components.forEach((circle,k)=>{
  if(circle.kind!=='circle')throw Error('Expected circles')
  const [rho,z]=[[2,0],[2.2,.6]][k]
  expect(Math.abs(circle.radius-rho)).toBeLessThan(1e-12)
  circle.center.forEach((x,i)=>expect(Math.abs(x-[0,0,z][i])).toBeLessThan(1e-12))
  expect(circle.curve.weights).toHaveLength(9)
  for(let i=0;i<16;i++){
   const p=evaluateNurbsCurve(circle.curve,i/4).point
   expect(F(p)).toBeLessThan(1e-10)
   expect(side(p)).toBeLessThan(1e-12)
  }
  // Cone UV: one iso-v degree-1 line per side patch, evaluating through its
  // own patch surface onto both equations.
  expect(circle.coneUv).toHaveLength(4)
  for(const lift of circle.coneUv)for(const arc of lift.arcs){
   expect(arc.degree).toBe(1)
   expect(arc.controlPoints[0][1]).toBe(arc.controlPoints[1][1])
   for(let q=0;q<=8;q++){
    const uv=evaluateNurbsCurve(arc,q/8).point
    const p=evaluateNurbsSurface(cone.faces[lift.patch].surface,Math.min(1,Math.max(0,uv[0])),Math.min(1,Math.max(0,uv[1]))).point
    expect(F(p)).toBeLessThan(1e-9)
    expect(side(p)).toBeLessThan(1e-9)
   }
  }
  // Torus UV: one iso-v degree-1 line per revolution quadrant patch.
  expect(circle.torusUv).toHaveLength(4)
  for(const lift of circle.torusUv)for(const arc of lift.arcs){
   expect(arc.degree).toBe(1)
   expect(arc.controlPoints[0][1]).toBe(arc.controlPoints[1][1])
   for(let q=0;q<=8;q++){
    const uv=evaluateNurbsCurve(arc,q/8).point
    const p=evaluateNurbsSurface(torus.faces[lift.patch].surface,Math.min(1,Math.max(0,uv[0])),Math.min(1,Math.max(0,uv[1]))).point
    expect(F(p)).toBeLessThan(1e-9)
    expect(side(p)).toBeLessThan(1e-9)
   }
  }
  expect(circle.maxSampleResidual).toBeLessThanOrEqual(1e-10)
 })
 // Frustum r 2 -> 4 over z 0..6 against the torus lifted to z=6.5: the side
 // line misses the meridian circle and the top cap plane at h_c=-0.5 cuts
 // the cap circle pair 3 +- sqrt(0.75), both inside the r_top=4 disk.
 const lifted=transformNurbsBrep(createBrepTorus(3,1),[[1,0,0,0],[0,1,0,0],[0,0,1,6.5],[0,0,0,1]])
 const Fu=(p:number[])=>F([p[0],p[1],p[2]-6.5])
 const wide=createBrepFrustum(2,4,6)
 const caps=intersectConeTorus(wide,lifted)
 expect(caps.coverage).toBe('numerically_resolved')
 expect(caps.components).toHaveLength(2)
 const s=Math.sqrt(.75)
 caps.components.forEach((circle,k)=>{
  if(circle.kind!=='circle')throw Error('Expected circles')
  const rho=[3-s,3+s][k]
  expect(Math.abs(circle.radius-rho)).toBeLessThan(1e-12)
  circle.center.forEach((x,i)=>expect(Math.abs(x-[0,0,6][i])).toBeLessThan(1e-12))
  for(let i=0;i<16;i++){
   const p=evaluateNurbsCurve(circle.curve,i/4).point
   expect(Fu(p)).toBeLessThan(1e-10)
   expect(Math.abs(p[2]-6)).toBeLessThan(1e-12)
  }
  // Cap UV: four exact 90-degree arcs on one cap face.
  expect(circle.coneUv).toHaveLength(1)
  expect(circle.coneUv[0].arcs).toHaveLength(4)
  expect(circle.torusUv).toHaveLength(4)
  expect(circle.maxSampleResidual).toBeLessThanOrEqual(1e-10)
 })
 // Anti-axial orientation: the cone flipped (axis -z) yields the same exact
 // circles mirrored in z.
 const flipped=transformNurbsBrep(cone,[[1,0,0,0],[0,-1,0,0],[0,0,-1,0],[0,0,0,1]])
 const anti=intersectConeTorus(flipped,torus)
 expect(anti.coverage).toBe('numerically_resolved')
 expect(anti.components).toHaveLength(2)
 anti.components.forEach((circle,k)=>{
  if(circle.kind!=='circle')throw Error('Expected circles')
  const [rho,z]=[[2.2,-.6],[2,0]][k]
  expect(Math.abs(circle.radius-rho)).toBeLessThan(1e-12)
  circle.center.forEach((x,i)=>expect(Math.abs(x-[0,0,z][i])).toBeLessThan(1e-12))
  expect(circle.normal[2]).toBeLessThan(-1+1e-12)
  expect(circle.maxSampleResidual).toBeLessThanOrEqual(1e-10)
 })
 // JSON roundtrip preserves the components.
 const round=JSON.parse(JSON.stringify(report)) as typeof report
 expect(round.components).toHaveLength(2)
 expect(round.components[0].kind).toBe('circle')
})

it('keeps cone/torus tangencies and refusals explicit through WASM',()=>{
 const cone=transformNurbsBrep(createBrepFrustum(1,3,6),[[1,0,0,0],[0,1,0,0],[0,0,1,-3],[0,0,0,1]])
 const at=(z:number)=>transformNurbsBrep(createBrepTorus(3,1),[[1,0,0,0],[0,1,0,0],[0,0,1,z],[0,0,0,1]])
 // Meridian tangency: against the frustum r 1 -> 3 with its bottom ring at
 // z=0, the torus at z = 6 - sqrt(10) makes the side line tangent to the
 // meridian circle (distance |2 + z_b/3| * 3/sqrt(10) == 1) — a double root,
 // never a guessed circle.
 const cone0=createBrepFrustum(1,3,6)
 const tangentZ=6-Math.sqrt(10)
 for(const dz of [0,-2e-15]){
  const report=intersectConeTorus(cone0,at(tangentZ+dz))
  expect(report.coverage).toBe('incomplete')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
  expect(report.permitsTopologyChange).toBe(false)
 }
 // Just clear of the band: provable miss, resolved empty.
 const clear=intersectConeTorus(cone0,at(tangentZ-1e-9))
 expect(clear.coverage).toBe('numerically_resolved')
 expect(clear.components).toHaveLength(0)
 expect(clear.unresolved).toHaveLength(0)
 // Just across: two small transverse side circles.
 expect(intersectConeTorus(cone0,at(tangentZ+1e-9)).components).toHaveLength(2)
 // Empty branches: the side line misses and cap circles fall outside the
 // disks (frustum r 1 -> 3 with its bottom ring at the torus center plane);
 // a frustum far beyond the tube.
 const torus=createBrepTorus(3,1)
 for(const c of [createBrepFrustum(1,3,6),transformNurbsBrep(createBrepFrustum(1,3,6),[[1,0,0,0],[0,1,0,0],[0,0,1,100],[0,0,0,1]])]){
  const report=intersectConeTorus(c,torus)
  expect(report.coverage).toBe('numerically_resolved')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved).toHaveLength(0)
 }
 // Near-coaxial offset within the recognition band: near_coincidence, never
 // snapped.
 const near=intersectConeTorus(transformNurbsBrep(cone,[[1,0,0,1e-10],[0,1,0,0],[0,0,1,0],[0,0,0,1]]),torus)
 expect(near.unresolved.map(u=>u.reason)).toEqual(['near_coincidence'])
 // Clearly off-axis: the general quartic is unsupported, no fallback.
 const off=intersectConeTorus(transformNurbsBrep(cone,[[1,0,0,.5],[0,1,0,0],[0,0,1,0],[0,0,0,1]]),torus)
 expect(off.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 // Non-canonical operands and swapped order: explicit unsupported_surface.
 for(const [a,b] of [[createBrepSphere(2),torus],[createBrepCylinder(2.2,8),torus],[torus,createBrepFrustum(1,3,6)],[createBrepFrustum(1,3,6),createBrepSphere(2)]] as const){
  const refused=intersectConeTorus(a,b)
  expect(refused.coverage).toBe('incomplete')
  expect(refused.components).toHaveLength(0)
  expect(refused.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 }
 // JSON roundtrip preserves the regions.
 const round=JSON.parse(JSON.stringify(off)) as typeof off
 expect(round.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
})

it('resolves torus/torus coaxial circle pairs analytically through packed WASM',()=>{
 const first=createBrepTorus(3,1)
 // Torus implicit: (x^2+y^2+z^2+R^2-r^2)^2 = 4R^2(x^2+y^2).
 const F=(p:number[],R:number,r:number,z0=0)=>{const t=p[0]*p[0]+p[1]*p[1]+(p[2]-z0)*(p[2]-z0),r2=p[0]*p[0]+p[1]*p[1],b=t+R*R-r*r;return Math.abs(b*b-4*R*R*r2)}
 // Torus R=3, r=1 at the origin against torus R=3, r=sqrt(5.2) lifted to
 // z=3: the meridian centers (3,0) and (3,3) sit at d=3, a=0.8, l=0.6 —
 // the exact circle pair of radii 2.4 and 3.6, both at z=0.8 (sorted by
 // height then radius).
 const second=transformNurbsBrep(createBrepTorus(3,Math.sqrt(5.2)),[[1,0,0,0],[0,1,0,0],[0,0,1,3],[0,0,0,1]])
 const r2=Math.sqrt(5.2)
 const report=intersectTorusTorus(first,second)
 expect(report.coverage).toBe('numerically_resolved')
 expect(report.permitsTopologyChange).toBe(false)
 expect(report.unresolved).toHaveLength(0)
 expect(report.components).toHaveLength(2)
 report.components.forEach((circle,k)=>{
  if(circle.kind!=='circle')throw Error('Expected circles')
  const rho=[2.4,3.6][k]
  expect(Math.abs(circle.radius-rho)).toBeLessThan(1e-12)
  circle.center.forEach((x,i)=>expect(Math.abs(x-[0,0,.8][i])).toBeLessThan(1e-12))
  expect(circle.curve.weights).toHaveLength(9)
  for(let i=0;i<16;i++){
   const p=evaluateNurbsCurve(circle.curve,i/4).point
   expect(F(p,3,1)).toBeLessThan(1e-10)
   expect(F(p,3,r2,3)).toBeLessThan(1e-10)
  }
  // Both torus UV lifts: one iso-v degree-1 line per revolution quadrant
  // patch, evaluating through its own patch surface onto both equations.
  for(const [model,lifts] of [[first,circle.firstUv],[second,circle.secondUv]] as const){
   expect(lifts).toHaveLength(4)
   for(const lift of lifts)for(const arc of lift.arcs){
    expect(arc.degree).toBe(1)
    expect(arc.controlPoints[0][1]).toBe(arc.controlPoints[1][1])
    for(let q=0;q<=8;q++){
     const uv=evaluateNurbsCurve(arc,q/8).point
     const p=evaluateNurbsSurface(model.faces[lift.patch].surface,Math.min(1,Math.max(0,uv[0])),Math.min(1,Math.max(0,uv[1]))).point
     expect(F(p,3,1)).toBeLessThan(1e-9)
     expect(F(p,3,r2,3)).toBeLessThan(1e-9)
    }
   }
  }
  expect(circle.maxSampleResidual).toBeLessThanOrEqual(1e-10)
 })
 // Equal minor radii: torus R=3, r=1 at z=1 — d=1, a=1/2, l=sqrt(3)/2, the
 // exact circles of radii 3 -+ sqrt(3)/2, both at z=1/2.
 const equal=transformNurbsBrep(createBrepTorus(3,1),[[1,0,0,0],[0,1,0,0],[0,0,1,1],[0,0,0,1]])
 const pair=intersectTorusTorus(first,equal)
 expect(pair.coverage).toBe('numerically_resolved')
 expect(pair.components).toHaveLength(2)
 const s=Math.sqrt(.75)
 pair.components.forEach((circle,k)=>{
  if(circle.kind!=='circle')throw Error('Expected circles')
  const rho=[3-s,3+s][k]
  expect(Math.abs(circle.radius-rho)).toBeLessThan(1e-12)
  circle.center.forEach((x,i)=>expect(Math.abs(x-[0,0,.5][i])).toBeLessThan(1e-12))
  expect(circle.firstUv).toHaveLength(4)
  expect(circle.secondUv).toHaveLength(4)
  expect(circle.maxSampleResidual).toBeLessThanOrEqual(1e-10)
 })
 // JSON roundtrip preserves the components.
 const round=JSON.parse(JSON.stringify(report)) as typeof report
 expect(round.components).toHaveLength(2)
 expect(round.components[0].kind).toBe('circle')
})

it('keeps torus/torus tangencies, coincidence and refusals explicit through WASM',()=>{
 const first=createBrepTorus(3,1)
 const at=(z:number)=>transformNurbsBrep(createBrepTorus(3,1),[[1,0,0,0],[0,1,0,0],[0,0,1,z],[0,0,0,1]])
 // External meridian tangency: torus R=3, r=1 at z=2 touches the first tube
 // (d = 2 = r1 + r2) at rho=3, z=1 — a double root, never a guessed circle.
 for(const dz of [0,-2e-15]){
  const report=intersectTorusTorus(first,at(2+dz))
  expect(report.coverage).toBe('incomplete')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved.map(u=>u.reason)).toEqual(['tangency_or_multiple_root'])
  expect(report.permitsTopologyChange).toBe(false)
 }
 // Just clear of the band: provable miss, resolved empty.
 const clear=intersectTorusTorus(first,at(2+1e-9))
 expect(clear.coverage).toBe('numerically_resolved')
 expect(clear.components).toHaveLength(0)
 expect(clear.unresolved).toHaveLength(0)
 // Just across: two small transverse circles around the touch point.
 expect(intersectTorusTorus(first,at(2-1e-9)).components).toHaveLength(2)
 // Coincident tori: coincident_trim, never a guessed curve.
 const coincident=intersectTorusTorus(first,createBrepTorus(3,1))
 expect(coincident.coverage).toBe('incomplete')
 expect(coincident.components).toHaveLength(0)
 expect(coincident.unresolved.map(u=>u.reason)).toEqual(['coincident_trim'])
 // Empty branches: concentric containment (r=0.5), nested without contact
 // (R=3.2, r=2.5 contains the first tube), separate far along the axis.
 for(const b of [createBrepTorus(3,.5),createBrepTorus(3.2,2.5),at(10)]){
  const report=intersectTorusTorus(first,b)
  expect(report.coverage).toBe('numerically_resolved')
  expect(report.components).toHaveLength(0)
  expect(report.unresolved).toHaveLength(0)
 }
 // Near-coaxial offset within the recognition band: near_coincidence, never
 // snapped.
 const near=intersectTorusTorus(first,transformNurbsBrep(at(3),[[1,0,0,1e-10],[0,1,0,0],[0,0,1,0],[0,0,0,1]]))
 expect(near.unresolved.map(u=>u.reason)).toEqual(['near_coincidence'])
 // Clearly off-axis: the general quartic is unsupported, no fallback.
 const off=intersectTorusTorus(first,transformNurbsBrep(at(3),[[1,0,0,.5],[0,1,0,0],[0,0,1,0],[0,0,0,1]]))
 expect(off.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 // Non-canonical operands on either side: explicit unsupported_surface.
 for(const [a,b] of [[createBrepSphere(2),first],[first,createBrepCylinder(2.2,8)],[createBrepFrustum(1,3,6),first],[first,createBrepSphere(2)]] as const){
  const refused=intersectTorusTorus(a,b)
  expect(refused.coverage).toBe('incomplete')
  expect(refused.components).toHaveLength(0)
  expect(refused.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
 }
 // JSON roundtrip preserves the regions.
 const round=JSON.parse(JSON.stringify(off)) as typeof off
 expect(round.unresolved.map(u=>u.reason)).toEqual(['unsupported_surface'])
})
