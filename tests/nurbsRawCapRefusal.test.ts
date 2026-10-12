import {expect,it} from 'vitest'
import {bezierNurbsCurve,circleNurbsCurve} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {createProgressiveBrepProfileBody,type NurbsBrep} from '../src/services/geometry/brep'
import {constructMiterOwnedBoundary} from '../src/services/nurbsMiterOwnedBoundary'
import {inspectNativeSweepCapContacts} from '../src/services/nurbsSweepCapContacts'
import {inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'
import {createNativeGeometryArtifact} from '../src/core/nativeGeometry'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'

const expectSolidRefusal=(model:NurbsBrep,op:'brep_progressive_sweep'|'brep_miter_sweep')=>{
 const artifact=createNativeGeometryArtifact('raw','brep',{geometry:model,
  sweepBodyBoundaryEvidence:{closedPath:false},sweepEvidence:{volume:{solidGeometryCertified:true}}},
  {nodes:[{id:'raw',op,closed:false}],root:'raw'})
 expect(()=>inspectProgressiveSweepSolidAdmission(artifact,model)).toThrow(/Solid geometry could not be proved/)
}

const scalar=(values:number[])=>({degree:1,knots:[0,0,1,1],values,weights:[1,1]})
it.each(['oblique-rmf','frenet','rmf'] as const)('transports exact negative cap identity for the original raw %s body',orientation=>{
 const oblique=orientation==='oblique-rmf'
 const axis:[number,number,number]=oblique?[3,4,0]:[1,0,0]
 const loops=[[circleNurbsCurve([0,0,0],axis,.1)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],axis,.05))]]
 const path=bezierNurbsCurve(oblique?[[0,0,0],axis]:[[0,0,0],[.5,0,0],[1,1,0]])
 const options={normal:[0,0,1] as [number,number,number],orientation:oblique?'rmf' as const:orientation,
  initialSections:oblique?2:3,maxSections:oblique?2:129,maxDeviation:.01}
 const before=JSON.stringify({loops,path,options})
 const body=createProgressiveBrepProfileBody(loops,path,scalar([1,oblique?2:1]),scalar([0,0]),options)
 expect(JSON.stringify({loops,path,options})).toBe(before)
 expect(body.retainedWalls?.certified).toBe(true)
 expect(body.retainedCaps).toMatchObject({exact:false,reason:'cap-coedge-identity-different'})
 expect(body.boundaryContinuousBound).toBe(false)
 expect(body.filledCapErrorUpper).toBeNull()
 const saved=JSON.stringify(body.model)
 expect(inspectSweepVolume(body.model,[body.model.faces.length-2,body.model.faces.length-1],DEFAULT_SWEEP_VOLUME_BUDGETS).solidGeometryCertified).toBe(false)
 expect(JSON.stringify(body.model)).toBe(saved)
 expectSolidRefusal(body.model,'brep_progressive_sweep')
})

it('transports spatial direct final-cap rejection separately from explicit final-only correction',()=>{
 const source={direct:true as const,loops:[[circleNurbsCurve([0,0,0],[0,0,1],.5)],
  [reverseNurbsCurve(circleNurbsCurve([0,0,0],[0,0,1],.2))]],
  points:[[0,0,0],[0,0,10],[10,0,10],[10,10,15]] as [number,number,number][],
  normal:[1,0,0] as [number,number,number],closed:false,miterLimit:2,maxDeviation:.01}
 const before=JSON.stringify(source),body=constructMiterOwnedBoundary(source)
 expect(JSON.stringify(source)).toBe(before)
 const caps=[body.model.faces.length-2,body.model.faces.length-1]
 const budgets={maxWalls:1024,maxExactWork:1000000,maxChartCells:1000,maxTrimPairs:100000,maxTrimCells:100000,maxTrimDomainCells:1000000}
 expect(inspectNativeSweepCapContacts(body.model,caps[0]!,caps,budgets).capCertified).toBe(true)
 expect(inspectNativeSweepCapContacts(body.model,caps[1]!,caps,budgets)).toMatchObject({capCertified:false,reason:'cap-coedge-identity-different'})
 expect(body.boundaryCertificate.continuousBound).toBe(false)
 expect(inspectSweepVolume(body.model,caps,DEFAULT_SWEEP_VOLUME_BUDGETS).solidGeometryCertified).toBe(false)
 expectSolidRefusal(body.model,'brep_miter_sweep')
 const corrected=constructMiterOwnedBoundary({...source,capEndpoints:[false,true],
  capCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:1000000}})
 expect(corrected.boundaryCertificate).toMatchObject({continuousBound:true,withinBudget:true})
 expect(corrected.model.faces[caps[0]!]!.surface).toEqual(body.model.faces[caps[0]!]!.surface)
})
