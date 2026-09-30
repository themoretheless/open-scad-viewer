import {constructSolidSurface,buildSolidSurface} from '../src/services/solidSurfaceConstruction'
import { describe, expect, it } from 'vitest'
import { emptyDirectDocument, parseDirectDocument } from '../src/services/directModeling'
import { solidDocumentToMeshDocument } from '../src/services/solidBridge'
import { importModelGraphNurbs } from '../src/services/solidNurbsImport'
import {
  createSolidNurbsCurve,
  createSolidNurbsSurface,
  matchSolidNurbsCurvesG1,
  matchSolidNurbsSurfacesG1,
  nurbsCurveToSketch,
  sampleSolidNurbsCurve,
  updateSolidNurbsControlPoint,
} from '../src/services/solidNurbs'
import { evaluateNurbsSurface, trimNurbsSurface } from '../src/services/nurbsSurface'
import { evaluateNurbsCurve, insertNurbsKnot } from '../src/services/nurbsCurve'

describe('Solid native NURBS bridge', () => {
  it('retains rational definitions in the Solid document and validates edits', () => {
    const document = emptyDirectDocument()
    document.curves!.push(createSolidNurbsCurve('curve'))
    document.surfaces!.push(createSolidNurbsSurface('surface'))
    const parsed = parseDirectDocument(JSON.stringify(document))
    expect(parsed.curves?.[0].curve.degree).toBe(3)
    expect(parsed.surfaces?.[0].surface.controlPoints[1][1][2]).toBe(10)

    parsed.curves![0].curve.weights[0] = 0
    expect(() => parseDirectDocument(JSON.stringify(parsed))).toThrow()
  })

  it('samples curves only on explicit sketch conversion', () => {
    const native = createSolidNurbsCurve('curve')
    const sampled = sampleSolidNurbsCurve(native.curve, 12)
    const sketch = nurbsCurveToSketch(native)
    expect(sampled).toHaveLength(13)
    expect(sketch.points.length).toBeGreaterThan(12)
    expect(native.curve.controlPoints).toHaveLength(4)
  })

  it('tessellates native surfaces only at the Solid to Mesh boundary', () => {
    const document = emptyDirectDocument()
    document.surfaces!.push(createSolidNurbsSurface('surface'))
    const mesh = solidDocumentToMeshDocument(document)
    expect(mesh.objects).toHaveLength(1)
    expect(mesh.objects[0].name).toContain('tessellated')
    expect(mesh.objects[0].mesh.indices.length).toBeGreaterThan(100)
    expect(document.bodies).toHaveLength(0)
  })

  it('imports reachable native definitions from ModelGraph/NURBS', () => {
    const imported = importModelGraphNurbs({
      language: 'modelgraph/nurbs-1',
      units: 'mm',
      parameters: [],
      nodes: [{
        id: 'path', op: 'curve', degree: 2,
        knots: [0, 0, 0, 1, 1, 1],
        control_points: [[10, 0, 0], [10, 10, 0], [0, 10, 0]],
        weights: [1, Math.SQRT1_2, 1], periodic: false,
      }, {
        id: 'skin', op: 'surface_extrude', input: 'path', vector: [0, 0, 5],
      }],
      root: 'skin',
    })
    expect(imported.curves.map(item => item.name)).toEqual(['path'])
    expect(imported.surfaces.map(item => item.name)).toEqual(['skin'])
  })

  it('edits curve and surface CVs without baking either definition', () => {
    const document = emptyDirectDocument()
    document.curves!.push(createSolidNurbsCurve('curve'))
    document.surfaces!.push(createSolidNurbsSurface('surface'))
    const curveEdit = updateSolidNurbsControlPoint(document, 'curve', 1, 0, [-7, 15, 4], 1.5)
    const surfaceEdit = updateSolidNurbsControlPoint(curveEdit, 'surface', 1, 1, [0, 0, 14], 2)
    expect(surfaceEdit.curves![0].curve.controlPoints[1]).toEqual([-7, 15, 4])
    expect(surfaceEdit.curves![0].curve.weights[1]).toBe(1.5)
    expect(surfaceEdit.surfaces![0].surface.controlPoints[1][1]).toEqual([0, 0, 14])
    expect(surfaceEdit.surfaces![0].surface.weights[1][1]).toBe(2)
    expect(surfaceEdit.bodies).toEqual([])
    expect(document.curves![0].curve.controlPoints[1]).toEqual([-8, 18, 0])
  })

  it('trims a native surface to an exact rectangular UV subdomain', () => {
    const source = createSolidNurbsSurface('surface').surface
    const trimmed = trimNurbsSurface(source, [.2, .8, .1, .9])
    expect(trimmed.knotsU[trimmed.degreeU]).toBeCloseTo(.2)
    expect(trimmed.knotsU[trimmed.controlPoints.length]).toBeCloseTo(.8)
    expect(trimmed.knotsV[trimmed.degreeV]).toBeCloseTo(.1)
    expect(trimmed.knotsV[trimmed.controlPoints[0].length]).toBeCloseTo(.9)
  })

  it('matches two native curve endpoints with G1 continuity', () => {
    const document = emptyDirectDocument()
    const reference = createSolidNurbsCurve('reference')
    const edited = createSolidNurbsCurve('edited')
    edited.curve.controlPoints = edited.curve.controlPoints.map(point => point.map((value, axis) => value + (axis === 0 ? 50 : 0)))
    document.curves!.push(reference, edited)

    const matched = matchSolidNurbsCurvesG1(document, 'reference', 'edited')
    const a = evaluateNurbsCurve(matched.curves![0].curve, 1)
    const b = evaluateNurbsCurve(matched.curves![1].curve, 0)
    expect(b.point).toEqual(a.point)
    const unit = (v: number[]) => v.map(value => value / Math.hypot(...v))
    expect(unit(b.d1!)[0]).toBeCloseTo(unit(a.d1!)[0], 10)
    expect(unit(b.d1!)[1]).toBeCloseTo(unit(a.d1!)[1], 10)
    expect(document.curves![1].curve.controlPoints[0][0]).toBe(30)
  })

  it('matches compatible non-rational surface boundaries with exact G1 continuity', () => {
    const document = emptyDirectDocument()
    const reference = createSolidNurbsSurface('reference')
    const edited = createSolidNurbsSurface('edited')
    edited.surface.controlPoints = edited.surface.controlPoints.map(row => row.map(point => [point[0] + 80, point[1], point[2] - 7]))
    document.surfaces!.push(reference, edited)

    const matched = matchSolidNurbsSurfacesG1(document, 'reference', 'edited')
    for (const v of [0, .25, .5, .75, 1]) {
      const a = evaluateNurbsSurface(matched.surfaces![0].surface, 1, v)
      const b = evaluateNurbsSurface(matched.surfaces![1].surface, 0, v)
      expect(b.point[0]).toBeCloseTo(a.point[0], 10)
      expect(b.point[1]).toBeCloseTo(a.point[1], 10)
      expect(b.point[2]).toBeCloseTo(a.point[2], 10)
      const cosine = a.du!.reduce((sum, value, axis) => sum + value * b.du![axis], 0) /
        (Math.hypot(...a.du!) * Math.hypot(...b.du!))
      expect(cosine).toBeCloseTo(1, 10)
    }
    expect(document.surfaces![1].surface.controlPoints[0][0][0]).toBe(60)
  })

  it('accepts rational surface strips through the native G1 gate', () => {
    const document = emptyDirectDocument()
    const reference = createSolidNurbsSurface('reference')
    const edited = createSolidNurbsSurface('edited')
    reference.surface.weights[1][1] = 1.1
    document.surfaces!.push(reference, edited)
    const matched=matchSolidNurbsSurfacesG1(document, 'reference', 'edited')
    const a=evaluateNurbsSurface(reference.surface,1,.5),b=evaluateNurbsSurface(matched.surfaces![1].surface,0,.5)
    for(const field of ['point','du','dv'] as const)a[field]!.forEach((x,i)=>expect(b[field]![i]).toBeCloseTo(x,9))
  })
})

describe('Solid surface construction',()=>{
 it('preserves rational sweep geometry and immutable source identities',()=>{
  const d=emptyDirectDocument(),p=createSolidNurbsCurve('profile'),q=createSolidNurbsCurve('path')
  q.curve.controlPoints=q.curve.controlPoints.map(([x,y,z])=>[z,x,y+10]);p.curve.weights[1]=2
  d.curves!.push(p,q);const before=structuredClone(d)
  const result=constructSolidSurface(d,['profile','path'],'nurbs-sweep','new','Assembly')
  const start=evaluateNurbsCurve(q.curve,0).point
  for(const u of [0,.13,.5,.91,1])for(const v of [0,.27,.6,1]){
   const a=evaluateNurbsCurve(p.curve,u).point,b=evaluateNurbsCurve(q.curve,v).point,s=evaluateNurbsSurface(result.surface,u,v).point
   s.forEach((x,i)=>expect(x).toBeCloseTo(a[i]+b[i]-start[i],9))
  }
  expect(result.group).toBe('Assembly');expect(d).toEqual(before)
  expect(()=>constructSolidSurface(d,['profile','profile'],'nurbs-sweep','bad')).toThrow('distinct')
  expect(()=>constructSolidSurface(d,['profile','missing'],'nurbs-sweep','bad')).toThrow('Curve missing. Choose inputs again.')
 })
 it('interpolates ordered loft sections and preserves the result through document reload',()=>{
  const d=emptyDirectDocument()
  d.curves=Array.from({length:3},(_,i)=>{const c=createSolidNurbsCurve('c'+i);c.curve.controlPoints=c.curve.controlPoints.map(([x,y,z])=>[x,y,z+10*i]);return c})
  const result=constructSolidSurface(d,['c0','c1','c2'],'nurbs-loft','loft')
  for(let i=0;i<3;i++)for(const u of [0,.17,.5,.88,1]){
   const a=evaluateNurbsCurve(d.curves[i].curve,u).point,b=evaluateNurbsSurface(result.surface,u,i).point
   b.forEach((x,j)=>expect(x).toBeCloseTo(a[j],9))
  }
  d.surfaces!.push(result)
  expect(parseDirectDocument(JSON.stringify(d)).surfaces?.[0]).toEqual(result)
 })
})

import {measureSurfaceBoundaries} from '../src/services/solidSurfaceDiagnostics'
it('measures a surface seam without treating sampled agreement as a whole-domain certificate',()=>{
 const a=createSolidNurbsSurface('a').surface,b=structuredClone(a)
 a.controlPoints=a.controlPoints.map(row=>row.map(([x,y])=>[x,y,0]))
 b.controlPoints=b.controlPoints.map(row=>row.map(([x,y])=>[x+40,y,0]))
 const options={boundaryA:'uMax',boundaryB:'uMin',reverse:false,samples:65,toleranceMm:.01,angleToleranceDeg:1} as const
 const before=structuredClone([a,b]),joined=measureSurfaceBoundaries(a,b,options)
 expect(joined.samplingOnly).toBe(true);expect(joined.sampledWithinTolerance).toBe(true)
 expect(joined.maxGapMm).toBe(0);expect(joined.maxTangentPlaneAngleDeg).toBe(0)
 expect(joined.samples).toHaveLength(65);expect([a,b]).toEqual(before)
 b.controlPoints.forEach(row=>row.forEach(p=>p[0]+=.25))
 const gap=measureSurfaceBoundaries(a,b,options)
 expect(gap.maxGapMm).toBeCloseTo(.25,10);expect(gap.sampledWithinTolerance).toBe(false)
 expect(measureSurfaceBoundaries(a,b,{...options,reverse:true}).maxGapMm).toBeGreaterThan(39)
 expect(()=>measureSurfaceBoundaries(a,b,{...options,toleranceMm:0})).toThrow('positive')
})

import {reduceSolidCurve} from '../src/services/solidCurveReduction'
it('accepts exact curve reduction and retains the original on a refused tolerance',()=>{
 const d=emptyDirectDocument(),c=createSolidNurbsCurve('line');c.curve.controlPoints=[[0,0,0],[1,0,0],[2,0,0],[3,0,0]];d.curves!.push(c)
 const before=structuredClone(d),accepted=reduceSolidCurve(d,'line',1,0)
 expect(accepted.certificate.accepted).toBe(true);expect(accepted.certificate.hausdorffErrorUpper).toBe(0)
 expect(accepted.document.curves![0].id).toBe('line');expect(accepted.document.curves![0].curve.degree).toBe(1);expect(d).toEqual(before)
 d.curves![0]=createSolidNurbsCurve('curve')
 const rejected=reduceSolidCurve(d,'curve',1,0)
 expect(rejected.certificate.accepted).toBe(false);expect(rejected.certificate.rolledBack).toBe(true)
 expect(rejected.certificate.hausdorffErrorUpper).toBeGreaterThan(0);expect(rejected.document).toEqual(d)
})

import {reduceSolidSurface} from '../src/services/solidCurveReduction'
it.each(['u','v'] as const)('reduces a planar surface in %s with preserved identity and an accepted bound',axis=>{
 const d=emptyDirectDocument(),s=createSolidNurbsSurface('plane');s.surface.controlPoints=Array.from({length:3},(_,i)=>Array.from({length:3},(_,j)=>[i,j,0]));s.surface.weights=Array.from({length:3},()=>[1,1,1]);d.surfaces!.push(s)
 const before=structuredClone(d),result=reduceSolidSurface(d,'plane',axis,1,0)
 expect(result.certificate.accepted).toBe(true);expect(result.certificate.hausdorffErrorUpper).toBe(0)
 const out=result.document.surfaces![0];expect(out.id).toBe('plane')
 expect(axis==='u'?out.surface.degreeU:out.surface.degreeV).toBe(1);expect(d).toEqual(before)
 for(const degree of [0,2,3])expect(()=>reduceSolidSurface(d,'plane',axis,degree,0)).toThrow('Target degree')
})
it('retains the original surface when reduction exceeds the permitted error',()=>{
 const d=emptyDirectDocument(),s=createSolidNurbsSurface('bump');s.surface.controlPoints=Array.from({length:3},(_,i)=>Array.from({length:3},(_,j)=>[i,j,i===1&&j===1?1:0]));s.surface.weights=Array.from({length:3},()=>[1,1,1]);d.surfaces!.push(s)
 const result=reduceSolidSurface(d,'bump','u',1,0)
 expect(result.certificate.accepted).toBe(false);expect(result.certificate.rolledBack).toBe(true)
 expect(result.certificate.hausdorffErrorUpper).toBeGreaterThan(0);expect(result.document).toEqual(d)
})

function patchCurves(){
 const points=[[[0,0,0],[1,0,1],[2,0,0]],[[0,2,0],[2,2,0]],[[0,0,0],[0,2,0]],[[2,0,0],[2,1,2],[2,2,0]]]
 return points.map((controlPoints,i)=>({id:'edge'+i,name:['Bottom','Top','Left','Right'][i],curve:{degree:controlPoints.length-1,knots:[...Array(controlPoints.length).fill(0),...Array(controlPoints.length).fill(1)],controlPoints,weights:Array(controlPoints.length).fill(1)}}))
}
it('builds a four-sided patch with unchanged source curves and exact analytic interior',()=>{
 const d=emptyDirectDocument();d.curves=patchCurves();const before=structuredClone(d)
 const p=constructSolidSurface(d,d.curves.map(c=>c.id),'nurbs-patch','patch')
 const u=.3,v=.7,point=evaluateNurbsSurface(p.surface,u,v).point
 expect(point[0]).toBeCloseTo(2*u,12);expect(point[1]).toBeCloseTo(2*v,12);expect(point[2]).toBeCloseTo((1-v)*2*u*(1-u)+u*4*v*(1-v),12)
 expect(d).toEqual(before);d.surfaces!.push(p);expect(parseDirectDocument(JSON.stringify(d)).surfaces?.[0]).toEqual(p)
})
it('keeps patch source geometry on refusal and supports an explicitly reversed boundary',()=>{
 const d=emptyDirectDocument();d.curves=patchCurves();d.curves[2].curve.controlPoints.reverse();const before=structuredClone(d)
 expect(()=>constructSolidSurface(d,d.curves!.map(c=>c.id),'nurbs-patch','bad')).toThrow('corner')
 expect(constructSolidSurface(d,d.curves.map(c=>c.id),'nurbs-patch','patch',undefined,[false,false,true,false]).surface.degreeU).toBe(2)
 expect(d).toEqual(before)
 d.curves[0].curve.weights[2]=.5
 expect(()=>constructSolidSurface(d,d.curves!.map(c=>c.id),'nurbs-patch','bad',undefined,[false,false,true,false])).toThrow('corner weights')
})

import {rebuildSolidCurve} from '../src/services/solidCurveReduction'
it('rebuilds a curve with requested controls while preserving identity and source on refusal',()=>{
 const d=emptyDirectDocument(),c=createSolidNurbsCurve('rebuild')
 c.curve={degree:2,knots:[2,2,2,5,5,5],controlPoints:[[0,0,0],[1,2,0],[2,0,0]],weights:[1,1,1]}
 d.curves!.push(c);const before=structuredClone(d)
 const accepted=rebuildSolidCurve(d,c.id,3,6,.1)
 expect(accepted.certificate.accepted).toBe(true)
 expect(accepted.document.curves![0].id).toBe(c.id)
 expect(accepted.document.curves![0].curve.controlPoints).toHaveLength(6)
 expect(accepted.document.curves![0].curve.degree).toBe(3)
 expect(d).toEqual(before)
 const refused=rebuildSolidCurve(d,c.id,1,2,0)
 expect(refused.certificate.accepted).toBe(false)
 expect(refused.document).toEqual(before)
 expect(()=>rebuildSolidCurve(d,c.id,3,3,.1)).toThrow('control count')
})

import {rebuildSolidSurface} from '../src/services/solidCurveReduction'
it.each(['u','v'] as const)('rebuilds a rational surface in %s while preserving identity and rollback',axis=>{
 const d=emptyDirectDocument(),s=createSolidNurbsSurface('rebuild-surface')
 s.surface.controlPoints=Array.from({length:3},(_,i)=>Array.from({length:3},(_,j)=>[i,j,i===1&&j===1?1:0]))
 s.surface.weights=Array.from({length:3},(_,i)=>Array.from({length:3},(_,j)=>i===1&&j===1?.6:1))
 d.surfaces!.push(s);const before=structuredClone(d)
 const result=rebuildSolidSurface(d,s.id,axis,3,6,.001)
 expect(result.certificate.accepted).toBe(true);expect(result.document.surfaces![0].id).toBe(s.id)
 const out=result.document.surfaces![0].surface
 expect(axis==='u'?out.controlPoints.length:out.controlPoints[0].length).toBe(6)
 expect(parseDirectDocument(JSON.stringify(result.document)).surfaces![0].surface).toEqual(out)
 expect(d).toEqual(before)
 const refused=rebuildSolidSurface(d,s.id,axis,1,2,0)
 expect(refused.certificate.rolledBack).toBe(true);expect(refused.document).toEqual(before)
})

it('preserves a rational cylindrical patch through Solid history and serialization',async()=>{
 const {DirectHistory,serializeDirectDocument}=await import('../src/services/directModeling')
 const d=emptyDirectDocument()
 const controls=[[[1,0,0],[1,1,0],[0,1,0]],[[1,0,2],[1,1,2],[0,1,2]],[[1,0,0],[1,0,2]],[[0,1,0],[0,1,2]]]
 d.curves=controls.map((controlPoints,i)=>({id:`rational-${i}`,name:`Boundary ${i}`,curve:{degree:controlPoints.length-1,controlPoints,knots:[...Array(controlPoints.length).fill(0),...Array(controlPoints.length).fill(1)],weights:i<2?[1,Math.SQRT1_2,1]:[1,1]}}))
 const before=structuredClone(d),history=new DirectHistory(d)
 const surface=constructSolidSurface(d,d.curves.map(c=>c.id),'nurbs-patch','cylinder')
 d.surfaces!.push(surface);history.commit(d)
 expect(history.undo().surfaces).toHaveLength(0)
 const restored=parseDirectDocument(serializeDirectDocument(history.redo()))
 expect(restored.curves).toEqual(before.curves)
 const patch=restored.surfaces![0].surface
 for(let i=0;i<=20;i++)for(let j=0;j<=10;j++){
  const u=i/20,v=j/10,p=evaluateNurbsSurface(patch,u,v).point
  expect(p[0]*p[0]+p[1]*p[1]).toBeCloseTo(1,12)
  expect(p[2]).toBeCloseTo(2*v,12)
 }
 for(let i=0;i<=20;i++){
  const t=i/20
  for(const [edge,u,v] of [[0,t,0],[1,t,1],[2,0,t],[3,1,t]]){
   const p=evaluateNurbsSurface(patch,u,v).point,q=evaluateNurbsCurve(restored.curves![edge].curve,t).point
   p.forEach((x,k)=>expect(x).toBeCloseTo(q[k],12))
  }
 }
})

import {framedSweepNurbsCurve,prepareCoonsBoundaryWeights,coonsNurbsPatch} from '../src/services/nurbsConstructors'
it('rotates a profile along a rational arc and refuses excessive sampled deviation',()=>{
 const profile={degree:1,controlPoints:[[1,0,0],[1.2,0,0]],weights:[1,1],knots:[0,0,1,1]}
 const path={degree:2,controlPoints:[[1,0,0],[1,1,0],[0,1,0]],weights:[1,Math.SQRT1_2,1],knots:[0,0,0,1,1,1]}
 const before=structuredClone({profile,path})
 const rejected=framedSweepNurbsCurve(profile,path,[0,0,1],3,.001)
 expect(rejected.surface).toBeNull();expect(rejected.report.accepted).toBe(false)
 const accepted=framedSweepNurbsCurve(profile,path,[0,0,1],32,.001)
 expect(accepted.report.accepted).toBe(true);expect(accepted.report.continuousBound).toBe(false)
 expect(accepted.report.sampledControlDeviation).toBeLessThan(.001)
 for(let i=0;i<32;i++){
  const point=evaluateNurbsSurface(accepted.surface!,1,i/31).point
  expect(Math.hypot(point[0],point[1])).toBeCloseTo(1.2,11)
 }
 expect({profile,path}).toEqual(before)
 const d=emptyDirectDocument();d.curves=[{...createSolidNurbsCurve('p'),curve:profile},{...createSolidNurbsCurve('q'),curve:path}]
 const options={mode:'framed' as const,normal:[0,0,1] as [number,number,number],sections:3,maxDeviation:.001}
 expect(()=>constructSolidSurface(d,['p','q'],'nurbs-sweep','s',undefined,[],options)).toThrow('Adjust sections or budget.')
 const surface=constructSolidSurface(d,['p','q'],'nurbs-sweep','s',undefined,[],{...options,sections:32})
 expect(surface.name).toBe('Framed sweep');d.surfaces!.push(surface)
 expect(parseDirectDocument(JSON.stringify(d)).surfaces![0]).toEqual(surface)
})

it('rebuilds a periodic rational curve with wrapped storage, matching seam and rollback',()=>{
 const curve={degree:2,knots:Array.from({length:9},(_,i)=>i),controlPoints:[[1,0,0],[0,1,0],[-1,0,0],[0,-1,0],[1,0,0],[0,1,0]],weights:[1,.8,1.2,1,1,.8],periodic:true}
 const d=emptyDirectDocument();d.curves=[{...createSolidNurbsCurve('periodic-rebuild'),curve}]
 const before=structuredClone(d)
 const refused=rebuildSolidCurve(d,'periodic-rebuild',1,5,0)
 expect(refused.certificate.accepted).toBe(false);expect(refused.document).toEqual(before)
 const result=rebuildSolidCurve(d,'periodic-rebuild',3,15,.2)
 expect(result.certificate.accepted).toBe(true)
 expect(result.certificate.wrappedStorage).toBe(true)
 expect(result.certificate.seam?.c0.residualUpper).toBeLessThan(1e-12)
 const rebuilt=result.document.curves![0].curve
 expect(rebuilt.periodic).toBe(true);expect(rebuilt.controlPoints).toHaveLength(15)
 expect(rebuilt.controlPoints.slice(-3)).toEqual(rebuilt.controlPoints.slice(0,3))
 const first=evaluateNurbsCurve(rebuilt,2),last=evaluateNurbsCurve(rebuilt,6)
 first.point.forEach((x,i)=>expect(x).toBeCloseTo(last.point[i],11))
 expect(d).toEqual(before)
 expect(parseDirectDocument(JSON.stringify(result.document)).curves![0].curve).toEqual(rebuilt)
})

it.each(['u','v'] as const)('rebuilds the periodic %s surface axis and preserves both seams',axis=>{
 const points=Array.from({length:6},(_,i)=>Array.from({length:6},(_,j)=>{
  const u=i%4*Math.PI/2,v=j%4*Math.PI/2;return [(2+.5*Math.cos(v))*Math.cos(u),(2+.5*Math.cos(v))*Math.sin(u),.5*Math.sin(v)]
 }))
 const surface={degreeU:2,degreeV:2,knotsU:Array.from({length:9},(_,i)=>i),knotsV:Array.from({length:9},(_,i)=>i),controlPoints:points,weights:points.map((row,i)=>row.map((_,j)=>[1,.8,1.2,1][i%4]*[1,.9,1.1,1][j%4])),periodicU:true,periodicV:true}
 const d=emptyDirectDocument();d.surfaces=[{id:'periodic-surface',name:'Periodic surface',surface,segmentsU:16,segmentsV:16}]
 const before=structuredClone(d),result=rebuildSolidSurface(d,'periodic-surface',axis,3,15,.2)
 expect(result.certificate.accepted).toBe(true)
 const target=result.document.surfaces![0].surface
 expect(target.periodicU&&target.periodicV).toBe(true)
 expect(axis==='u'?target.controlPoints.length:target.controlPoints[0].length).toBe(15)
 for(let i=0;i<=16;i++){
  const t=2+4*i/16
  for(const [a,b] of [[[2,t],[6,t]],[[t,2],[t,6]]]){
   const p=evaluateNurbsSurface(target,a[0],a[1]).point,q=evaluateNurbsSurface(target,b[0],b[1]).point
   expect(Math.hypot(...p.map((x,k)=>x-q[k]))).toBeLessThan(1e-12)
  }
 }
 expect(d).toEqual(before)
 expect(parseDirectDocument(JSON.stringify(result.document)).surfaces![0].surface).toEqual(target)
 expect(rebuildSolidSurface(d,'periodic-surface',axis,1,5,0).document).toEqual(before)
})

import {matchNurbsSurfaceJets} from '../src/services/nurbsConstructors'
it('matches rational surface first and second jets through the native protocol',()=>{
 const reference={degreeU:3,degreeV:3,knotsU:[2,2,2,2,5,5,5,5],knotsV:[-1,-1,-1,-1,3,3,3,3],controlPoints:Array.from({length:4},(_,i)=>Array.from({length:4},(_,j)=>[i,j,.15*i*i+.1*i*j+.2*j*j])),weights:Array.from({length:4},(_,i)=>Array.from({length:4},(_,j)=>1+.03*i+.02*j+.01*i*j))}
 const edited=structuredClone(reference);edited.controlPoints.forEach(row=>row.forEach(p=>p[0]+=20))
 const before=structuredClone({reference,edited})
 const result=matchNurbsSurfaceJets(reference,edited,'uMax','uMin',2,1)
 expect(result.report.continuityOrder).toBe(2)
 for(let i=0;i<=32;i++){
  const v=-1+4*i/32,a=evaluateNurbsSurface(reference,5,v),b=evaluateNurbsSurface(result.surface,2,v)
  for(const field of ['point','du','dv','duu','duv','dvv'] as const){a[field]!.forEach((x,k)=>expect(b[field]![k]).toBeCloseTo(x,9))}
 }
 expect({reference,edited}).toEqual(before)
})

import {matchSolidSurface} from '../src/services/solidSurfaceMatching'
it.each([false,true])('gates surface matching and preserves identities (reversed=%s)',reverse=>{
 const d=emptyDirectDocument(),a=createSolidNurbsSurface('reference'),b=createSolidNurbsSurface('edited')
 a.surface.weights=a.surface.weights.map((row,i)=>row.map((_,j)=>1+.03*i+.02*j))
 b.surface.controlPoints.forEach(row=>row.forEach(p=>p[0]+=20))
 d.surfaces=[a,b];const before=structuredClone(d)
 const options={referenceBoundary:'uMax' as const,editedBoundary:'uMin' as const,order:2 as const,scale:1,reverse,maxError:1e-6}
 const result=matchSolidSurface(d,a.id,b.id,options)
 expect(result.report.accepted).toBe(true)
 expect(result.report.regularityCertified).toBe(true)
 expect(result.report.errorBounds.wholeSeam).toBe(true)
 expect(result.report.errorUpper).toBeLessThanOrEqual(options.maxError)
 expect(result.document.surfaces![0]).toEqual(a)
 expect({...result.document.surfaces![1],surface:b.surface}).toEqual(b)
 expect(result.document.surfaces![1].surface).not.toEqual(b.surface)
 expect(parseDirectDocument(JSON.stringify(result.document))).toEqual(result.document)
 const refused=matchSolidSurface(d,a.id,b.id,{...options,maxError:0})
 expect(refused.report.accepted).toBe(false)
 expect(refused.report.reason).toBe('deviation-exceeds-budget')
 expect(refused.document).toEqual(before)
 expect(d).toEqual(before)
 expect(()=>matchSolidSurface(d,a.id,a.id,options)).toThrow('different')
})

import {prepareSolidSurfaceSeams} from '../src/services/solidSurfaceMatching'
it('prepares incompatible surface bases as one immutable, bounded document edit',()=>{
 const d=emptyDirectDocument(),a=createSolidNurbsSurface('prepare-a'),b=createSolidNurbsSurface('prepare-b')
 b.surface.degreeV=3;b.surface.knotsV=[0,0,0,0,1,1,1,1]
 b.surface.controlPoints=b.surface.controlPoints.map(row=>[row[0],row[1],row[1],row[2]])
 b.surface.weights=b.surface.weights.map(()=>[1,1.1,.9,1])
 d.surfaces=[a,b];const before=structuredClone(d)
 const options={referenceBoundary:'uMax' as const,editedBoundary:'uMin' as const,reverse:true,maxError:1e-6}
 const result=prepareSolidSurfaceSeams(d,a.id,b.id,options)
 expect(result.report.accepted).toBe(true)
 expect(result.document.surfaces!.map(s=>s.id)).toEqual([a.id,b.id])
 expect(result.document.surfaces![0].surface.knotsV).toEqual(result.document.surfaces![1].surface.knotsV)
 expect(result.document.surfaces![0].surface.degreeV).toBe(3)
 expect(prepareSolidSurfaceSeams(d,a.id,b.id,{...options,maxError:0}).document).toEqual(before)
 expect(d).toEqual(before)
 expect(parseDirectDocument(JSON.stringify(result.document))).toEqual(result.document)
})

it('requires explicit periodic unlinking for mixed seam preparation and rolls back refusal',()=>{
 const d=emptyDirectDocument(),a=createSolidNurbsSurface('mixed-periodic'),b=createSolidNurbsSurface('mixed-open')
 a.surface.periodicV=true;a.surface.knotsV=[0,1,2,3,4,5,6,7,8]
 a.surface.controlPoints=Array.from({length:3},(_,i)=>Array.from({length:6},(_,j)=>[i,Math.cos(j%4*Math.PI/2),Math.sin(j%4*Math.PI/2)]))
 a.surface.weights=Array.from({length:3},()=>[1,.9,1.1,1,1,.9])
 d.surfaces=[a,b];const before=structuredClone(d)
 const options={referenceBoundary:'uMax' as const,editedBoundary:'uMin' as const,reverse:false,maxError:1e-6}
 expect(()=>prepareSolidSurfaceSeams(d,a.id,b.id,options)).toThrow(/matching periodicity/)
 const result=prepareSolidSurfaceSeams(d,a.id,b.id,{...options,openPeriodic:true})
 expect(result.report.accepted).toBe(true)
 expect(result.basis.periodicityRemoved).toEqual({reference:true,edited:false})
 expect(result.document.surfaces![0].surface.periodicV).toBe(false)
 expect(prepareSolidSurfaceSeams(d,a.id,b.id,{...options,openPeriodic:true,maxError:0}).document).toEqual(before)
 expect(d).toEqual(before)
})

it('accepts identical seam bases with a zero positional budget',()=>{
 const d=emptyDirectDocument();d.surfaces=[createSolidNurbsSurface('identity-a'),createSolidNurbsSurface('identity-b')]
 const before=structuredClone(d)
 const result=prepareSolidSurfaceSeams(d,'identity-a','identity-b',{referenceBoundary:'uMax',editedBoundary:'uMin',reverse:false,maxError:0})
 expect(result.report.accepted).toBe(true)
 expect(result.report.referenceErrorUpper).toBe(0);expect(result.report.editedErrorUpper).toBe(0)
 expect(result.basis.periodicityRemoved).toEqual({reference:false,edited:false})
 expect(result.document.surfaces!.map(s=>s.surface.controlPoints)).toEqual(before.surfaces!.map(s=>s.surface.controlPoints))
 expect(d).toEqual(before)
})

import {matchSolidCurve} from '../src/services/solidCurveMatching'
describe('Certified curve endpoint matching',()=>{
 it.each([['start','start'],['start','end'],['end','start'],['end','end']] as const)('preserves identities and rational definitions for %s to %s',(a,b)=>{
  const document=emptyDirectDocument();document.curves=[createSolidNurbsCurve('a'),createSolidNurbsCurve('b')]
  document.curves[0].curve.weights=[1,.7,1.3,.9];document.curves[1].curve.weights=[.8,1.2,1,.6]
  const before=structuredClone(document),result=matchSolidCurve(document,'a','b',a,b)
  expect(result.report.accepted).toBe(true);expect(result.report.angleDegreesUpper).toBeLessThanOrEqual(1e-6)
  expect(document).toEqual(before);expect(result.document.curves![0]).toEqual(before.curves![0])
  expect(result.candidate.weights).toEqual(before.curves![1].curve.weights);expect(result.candidate.knots).toEqual(before.curves![1].curve.knots)
  expect(result.document.curves!.map(c=>c.id)).toEqual(['a','b'])
  expect(result.candidate.controlPoints[b==='start'?0:3]).toEqual(before.curves![0].curve.controlPoints[a==='start'?0:3])
 })
 it('refuses an unproven zero angular budget without modifying the document',()=>{
  const document=emptyDirectDocument();document.curves=[createSolidNurbsCurve('a'),createSolidNurbsCurve('b')]
  const before=structuredClone(document),result=matchSolidCurve(document,'a','b','end','start',0)
  expect(result.report.accepted).toBe(false);expect(result.document).toEqual(before);expect(document).toEqual(before)
 })
})

it('builds a closed rational sweep with periodic storage and an explicit C0 seam report',()=>{
 const profile={degree:1,controlPoints:[[1,0,0],[1.2,0,0]],weights:[1,1],knots:[0,0,1,1]}
 const path={degree:2,controlPoints:[[1,0,0],[1,1,0],[0,1,0],[-1,1,0],[-1,0,0],[-1,-1,0],[0,-1,0],[1,-1,0],[1,0,0]],weights:Array.from({length:9},(_,i)=>i%2?Math.SQRT1_2:1),knots:[0,0,0,.25,.25,.5,.5,.75,.75,1,1,1]}
 const before=structuredClone({profile,path}),result=framedSweepNurbsCurve(profile,path,[1,0,0],17,.1)
 expect(result.report).toMatchObject({accepted:true,closedPath:true,seamContinuity:'C0',continuousBound:false,stations:65})
 expect(result.surface?.periodicV).toBe(true)
 for(const row of result.surface!.controlPoints)expect(row.at(-1)).toEqual(row[0])
 for(let i=0;i<=16;i++){
  const p=evaluateNurbsSurface(result.surface!,1,i/16).point
  expect(Math.hypot(p[0],p[1])).toBeCloseTo(1.2,11)
 }
 const d=emptyDirectDocument();d.curves=[{...createSolidNurbsCurve('profile'),curve:profile},{...createSolidNurbsCurve('path'),curve:path}]
 const surface=constructSolidSurface(d,['profile','path'],'nurbs-sweep','closed',undefined,[],{mode:'framed',normal:[1,0,0],sections:17,maxDeviation:.1})
 d.surfaces=[surface];expect(parseDirectDocument(JSON.stringify(d)).surfaces![0]).toEqual(surface)
 expect(framedSweepNurbsCurve(profile,path,[1,0,0],17,0).surface).toBeNull()
 expect(()=>framedSweepNurbsCurve(profile,path,[1,0,0],3,.1)).toThrow('at least four')
 expect({profile,path}).toEqual(before)
})

it('closes a nonplanar periodic sweep without changing section length or source curves',()=>{
 const controls=Array.from({length:6},(_,i)=>{const a=i*2*Math.PI/6;return [Math.cos(a),Math.sin(a),.3*Math.sin(2*a)]})
 const path={degree:3,knots:Array.from({length:13},(_,i)=>(i-3)/6),controlPoints:[...controls,...controls.slice(0,3)],weights:Array(9).fill(1),periodic:true}
 const start=evaluateNurbsCurve(path,0).point
 const profile={degree:1,knots:[0,0,1,1],controlPoints:[[start[0],start[1],start[2]+.1],[start[0],start[1],start[2]+.2]],weights:[1,.8]}
 const before=structuredClone({profile,path}),result=framedSweepNurbsCurve(profile,path,[0,0,1],25,1)
 expect(result.report).toMatchObject({accepted:true,closedPath:true,seamContinuity:'C0',continuousBound:false,stations:97})
 expect(result.surface?.periodicV).toBe(true)
 for(const row of result.surface!.controlPoints)expect(row.at(-1)).toEqual(row[0])
 for(let i=0;i<25;i++){
  const a=evaluateNurbsSurface(result.surface!,0,i/24).point,b=evaluateNurbsSurface(result.surface!,1,i/24).point
  expect(Math.hypot(...a.map((v,k)=>v-b[k]))).toBeCloseTo(.1,11)
 }
 expect(evaluateNurbsSurface(result.surface!,.4,0).point).toEqual(evaluateNurbsSurface(result.surface!,.4,1).point)
 expect({profile,path}).toEqual(before)
})

it('keeps an open path open even when transported profile sections coincide',()=>{
 const profile={degree:1,knots:[0,0,1,1],controlPoints:[[0,0,0],[0,0,1]],weights:[1,1]}
 const path={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[1,0,0],[1,1,0],[0,1,0]],weights:[1,Math.SQRT1_2,1]}
 const result=framedSweepNurbsCurve(profile,path,[0,0,1],17,1)
 expect(result.report).toMatchObject({accepted:true,closedPath:false,seamContinuity:'open'})
 expect(result.surface?.periodicV).toBe(false)
})

it('preserves mixed rational patch boundary parameters across domains and weight scaling',()=>{
 const d=emptyDirectDocument();d.curves=patchCurves()
 const weights=[[1,.8,2],[3,4],[1,3],[2,1.5,4]],domains=[[-3,7],[11,13],[.2,.9],[-20,-10]]
 d.curves.forEach((c,i)=>{c.curve.weights=weights[i]!})
 d.curves[0]!.curve=insertNurbsKnot(insertNurbsKnot(d.curves[0]!.curve,.17),.61)
 d.curves[3]!.curve=insertNurbsKnot(d.curves[3]!.curve,.42)
 d.curves.forEach((c,i)=>{const [a,b]=domains[i]!;c.curve.knots=c.curve.knots.map(k=>a!+(b!-a!)*k)})
 const before=structuredClone(d),patch=constructSolidSurface(d,d.curves.map(c=>c.id),'nurbs-patch','mixed')
 expect(d).toEqual(before)
 for(let i=0;i<=100;i++){
  const t=i/100
  for(const [edge,u,v] of [[0,t,0],[1,t,1],[2,0,t],[3,1,t]]){
   const [a,b]=domains[edge!]!,p=evaluateNurbsSurface(patch.surface,u!,v!).point,q=evaluateNurbsCurve(d.curves[edge!]!.curve,a!+(b!-a!)*t).point
   for(let k=0;k<3;k++)expect(Math.abs(p[k]!-q[k]!)).toBeLessThan(2e-12)
  }
 }
 d.surfaces!.push(patch);expect(parseDirectDocument(JSON.stringify(d)).surfaces?.[0]).toEqual(patch)
})

it('refuses nonpositive Coons interior weights without changing valid rational boundaries',()=>{
 const d=emptyDirectDocument(),points=[[[0,0,0],[1,0,0],[2,0,0]],[[0,2,0],[1,2,0],[2,2,0]],[[0,0,0],[0,1,0],[0,2,0]],[[2,0,0],[2,1,0],[2,2,0]]]
 d.curves=points.map((controlPoints,i)=>({id:`boundary-${i}`,name:`Boundary ${i}`,curve:{degree:2,controlPoints,weights:[1,.1,1],knots:[0,0,0,1,1,1]}}))
 const before=structuredClone(d)
 expect(()=>constructSolidSurface(d,d.curves!.map(c=>c.id),'nurbs-patch','invalid')).toThrow('positive finite weights')
 expect(d).toEqual(before)
})

it('prepares incompatible rational Coons weights with a whole-domain error gate and exact rollback',()=>{
 const sources=patchCurves().map(c=>c.curve);sources[0]!.weights=[1,.8,.5]
 const before=structuredClone(sources)
 expect(()=>coonsNurbsPatch(sources)).toThrow('corner weights')
 const prepared=sources.map(c=>prepareCoonsBoundaryWeights(c,1e-6))
 expect(prepared.every(p=>p.report.accepted&&p.report.wholeCurve&&p.report.errorUpper<=1e-6)).toBe(true)
 const patch=coonsNurbsPatch(prepared.map(p=>p.curve))
 for(let i=0;i<=100;i++){
  const t=i/100
  for(const [edge,u,v] of [[0,t,0],[1,t,1],[2,0,t],[3,1,t]]){
   const p=evaluateNurbsSurface(patch,u!,v!).point,q=evaluateNurbsCurve(sources[edge!]!,t).point
   expect(Math.hypot(...p.map((x,k)=>x-q[k]!))).toBeLessThan(1e-6)
  }
 }
 const refused=prepareCoonsBoundaryWeights(sources[0]!,0)
 expect(refused.report.accepted).toBe(false);expect(refused.curve).toEqual(sources[0])
 expect(sources).toEqual(before)
})

it('prepares nonbinary corner weights without a spurious corner-cycle refusal',()=>{
 for(const seed of [1,7,31,100]){
  const sources=patchCurves().map((c,edge)=>({...c.curve,weights:c.curve.weights.map((_,i)=>.1+(seed*7+edge*13+i*17)/31)}))
  const before=structuredClone(sources),prepared=sources.map(c=>prepareCoonsBoundaryWeights(c,1e-6))
  for(const p of prepared){
   expect(p.report.accepted).toBe(true)
   expect(p.curve.weights[0]).toBe(1);expect(p.curve.weights.at(-1)).toBe(1)
  }
  const surface=coonsNurbsPatch(prepared.map(p=>p.curve))
  for(let i=0;i<=20;i++)for(const [edge,u,v] of [[0,i/20,0],[1,i/20,1],[2,0,i/20],[3,1,i/20]]){
   const actual=evaluateNurbsSurface(surface,u!,v!).point,expected=evaluateNurbsCurve(sources[edge!]!,i/20).point
   expect(Math.hypot(...actual.map((x,k)=>x-expected[k]!))).toBeLessThan(1e-6)
  }
  expect(sources).toEqual(before)
 }
})

it('keeps identical clamped corners under nonbinary uniform weight scaling',()=>{
 const sources=patchCurves().map((c,i)=>({...c.curve,weights:c.curve.weights.map(()=>[.1,.7,.3,.9][i]!),controlPoints:c.curve.controlPoints.map(p=>[.1+.3*p[0]!, .1+.7*p[1]!,p[2]!])}))
 const before=structuredClone(sources)
 const patch=coonsNurbsPatch(sources)
 expect(patch).toEqual(coonsNurbsPatch(sources.map(c=>({...c,weights:c.weights.map(()=>1)}))))
 expect(sources).toEqual(before)
})

it('identifies the refused boundary role independently of document order',()=>{
 const d=emptyDirectDocument();d.curves=patchCurves();d.curves[0]!.curve.weights=[1,.8,.5]
 const before=structuredClone(d),ids=d.curves.map(c=>c.id)
 for(let shift=0;shift<4;shift++){
  const roles=[...ids.slice(shift),...ids.slice(0,shift)]
  const result=buildSolidSurface(d,{kind:'nurbs-patch',ids:roles,id:'refused',reversed:[false,false,false,false],sweep:{mode:'translation',normal:[0,0,1],sections:24,maxDeviation:1e-6},patchPreparation:{enabled:true,maxError:0}})
  expect(result.document).toBeNull()
  expect(result.error).toBe(`PATCH_BUDGET:${roles.indexOf(ids[0]!)}`)
  expect(d).toEqual(before)
 }
})
