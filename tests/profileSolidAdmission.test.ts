import {expect,it} from 'vitest'
import {createNativeGeometryArtifact} from '../src/core/nativeGeometry'
import {createProgressiveBrepProfileBody} from '../src/services/geometry/brep'
import {inspectProgressiveSweepSolidAdmission} from '../src/services/sweepSolidAdmission'
import type {NurbsCurve} from '../src/services/nurbsCurve'

const line=(a:number[],b:number[]):NurbsCurve=>({degree:1,knots:[0,0,1,1],controlPoints:[a,b],weights:[1,1],periodic:false})
const scale={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]}
const twist={...scale,values:[0,0]}
// Fixture inputs only. Construction, certificates and admission run in Rust.
const points=[[2.875,.015625,-.125],[2.875,-.015625,.125],[3.125,-.015625,.125],[3.125,.015625,-.125]]
const profiles=points.map((p,i)=>line(p,points[(i+1)%4]!))
const path:NurbsCurve={degree:3,knots:[0,0,0,0,.25,.25,.25,.5,.5,.5,.75,.75,.75,1,1,1,1],
 controlPoints:[[3,0,0],[3,1,.125],[1,3,.125],[0,3,0],[-1,3,-.125],[-3,1,-.125],[-3,0,0],[-3,-1,.125],[-1,-3,.125],[0,-3,0],[1,-3,-.125],[3,-1,-.125],[3,0,0]],
 weights:Array(13).fill(1),periodic:false}
const document={nodes:[{id:'body',op:'brep_progressive_sweep'},{id:'placed',op:'transform',input:'body'}]}

it.each([6,10])('delivers whole closed spatial body proofs and fresh Solid admission at %i stations',count=>{
 const body=createProgressiveBrepProfileBody([profiles],path,scale,twist,{normal:[0,0,1],orientation:'rmf',initialSections:count,maxSections:count,maxDeviation:1})
 expect(body.globalEmbeddingCertified).toBe(false)
 expect(body.volume).toMatchObject({solidGeometryCertified:true,boundaryEmbeddingCertified:true,allFacesInjective:true,allPairsClassified:true})
 const artifact=createNativeGeometryArtifact('placed','brep',body.model,document)
 expect(inspectProgressiveSweepSolidAdmission(artifact,body.model)).toMatchObject({solidGeometryCertified:true})
 const changed=structuredClone(body.model)
 changed.faces[0]!.surface.controlPoints[0]![0]![0]!+=.0001
 const forged=createNativeGeometryArtifact('placed','brep',{...changed,volume:{solidGeometryCertified:true}},document)
 expect(()=>inspectProgressiveSweepSolidAdmission(forged,changed)).toThrow(/exact boundary agreement/)
})
