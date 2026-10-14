import {expect,it} from 'vitest'
import {inspectSweepCapBoundary} from '../src/services/nurbsSweepAudit'
import {createRationalBrepSectionLoft} from '../src/services/geometry/brep'
import {circleNurbsCurve} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
const loops=(z:number)=>[[circleNurbsCurve([0,0,z],[0,0,1],.5)],[reverseNurbsCurve(circleNurbsCurve([0,0,z],[0,0,1],.2))]]
it('bounds every stored cap coedge against its retained world edge without sampling',()=>{
 const model=createRationalBrepSectionLoft([loops(0),loops(10)]),before=structuredClone(model)
 const caps=model.faces.filter(face=>face.surface.degreeU===1&&face.surface.degreeV===1)
 expect(caps).toHaveLength(2)
 let uses=0
 for(const cap of caps)for(const wire of [cap.outer,...cap.holes])for(const coedge of model.loops[wire]!.coedges){
  const stored=model.edges[coedge.edge]!.curve
  const world=coedge.reversed?reverseNurbsCurve(stored):stored
  const report=inspectSweepCapBoundary(cap.surface,world,coedge.pcurve,{tolerance:1e-9,maxProducts:9})
  expect(report).toMatchObject({withinBudget:true,products:9,reason:null,capGeometryCertified:false,globalEmbeddingCertified:false})
  expect(report.errorUpper).toBeLessThanOrEqual(1e-9)
  expect(inspectSweepCapBoundary(cap.surface,world,coedge.pcurve,{tolerance:1e-9,maxProducts:8})).toMatchObject({withinBudget:false,errorUpper:null,reason:'cap-product-budget-exhausted'})
  const changed=structuredClone(coedge.pcurve)
  changed.controlPoints[1]=changed.controlPoints[1]!.map((value,axis)=>(value+(axis===0?(cap.surface.knotsU[1]!+cap.surface.knotsU[2]!)/2:(cap.surface.knotsV[1]!+cap.surface.knotsV[2]!)/2))/2)
  expect(inspectSweepCapBoundary(cap.surface,world,changed,{tolerance:1e-9,maxProducts:9}).withinBudget).toBe(false)
  uses++
 }
 expect(uses).toBe(16)
 expect(model).toEqual(before)
})
