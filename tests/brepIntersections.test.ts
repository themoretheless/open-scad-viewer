import {evaluateNurbsCurve} from '../src/services/nurbsCurve'
import {evaluateNurbsSurface,insertNurbsSurfaceKnot} from '../src/services/nurbsSurface'
import {expect,it} from 'vitest'
import {createBrepSphere,createBrepCylinder,createBrepFrustum} from '../src/services/geometry/brep'
import {intersectionTraceToNurbsCurve,intersectNurbsSurfaceSurface,intersectNurbsCurveSegment,evaluateIntersectionTrace,intersectNurbsCurvePlane,intersectNurbsCurveSurface,intersectNurbsSurfacePlane} from '../src/services/geometry/intersections'
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
