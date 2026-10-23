// Transport to Rust constructors, audits, evaluation and STEP serialization.
import {mkdirSync,writeFileSync,readFileSync} from 'node:fs'
import {resolve} from 'node:path'
import {createHash} from 'node:crypto'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
import {createProgressiveBrepProfileBody} from '../src/services/geometry/brep'
import {evaluateNurbsCurve,type NurbsCurve} from '../src/services/nurbsCurve'
import {evaluateNurbsSurface} from '../src/services/nurbsSurface'
import {exportDirectStepV5} from '../src/services/cadNurbsStep'
import {inspectSweepCoedgeExact} from '../src/services/nurbsSweepAudit'
const root=resolve(process.argv[2]??'/tmp/profile-solid-step')
mkdirSync(root,{recursive:true})
const sha=(value:string|Buffer)=>createHash('sha256').update(value).digest('hex')
const line=(a:number[],b:number[]):NurbsCurve=>({degree:1,knots:[0,0,1,1],controlPoints:[a,b],weights:[1,1],periodic:false})
const scale={degree:1,knots:[0,0,1,1],values:[1,1],weights:[1,1]},twist={...scale,values:[0,0]}
const planar:NurbsCurve={degree:2,knots:[0,0,0,.25,.25,.5,.5,.75,.75,1,1,1],controlPoints:[[3,0,0],[3,3,0],[0,3,0],[-3,3,0],[-3,0,0],[-3,-3,0],[0,-3,0],[3,-3,0],[3,0,0]],weights:[1,1,2,2,4,4,8,8,16],periodic:false}
const spatial:NurbsCurve={degree:3,knots:[0,0,0,0,.25,.25,.25,.5,.5,.5,.75,.75,.75,1,1,1,1],controlPoints:[[3,0,0],[3,1,.125],[1,3,.125],[0,3,0],[-1,3,-.125],[-3,1,-.125],[-3,0,0],[-3,-1,.125],[-1,-3,.125],[0,-3,0],[1,-3,-.125],[3,-1,-.125],[3,0,0]],weights:Array(13).fill(1),periodic:false}
const catalogBytes=readFileSync(new URL('../docs/design/sweep-qualification-catalog.json',import.meta.url))
const catalog=JSON.parse(catalogBytes.toString('utf8'))
const cases=[]
for(const {mode,stations:count,file,nativeAdmission,maxDeviation,strictRefusalBudget} of catalog.step.profileSolid){
 if(!['planar','spatial'].includes(mode)||nativeAdmission!=='certify')throw Error('Unsupported catalog case')
 const y=mode==='spatial'?.015625:0
 const points=[[2.875,y,-.125],[2.875,-y,.125],[3.125,-y,.125],[3.125,y,-.125]]
 const profiles=points.map((p,i)=>line(p,points[(i+1)%4]!))
 const options={normal:[0,0,1] as [number,number,number],orientation:'rmf' as const,initialSections:count,maxSections:count,maxDeviation}
 if(strictRefusalBudget!==undefined){
  let refused=false
  try{createProgressiveBrepProfileBody([profiles],mode==='spatial'?spatial:planar,scale,twist,{...options,maxDeviation:strictRefusalBudget})}
  catch(error){if(!/continuous retained-patch error or refinement exceeds budget/.test(String(error)))throw error;refused=true}
  if(!refused)throw Error(`Expected strict source-budget refusal: ${mode}/${count}`)
 }
 const body=createProgressiveBrepProfileBody([profiles],mode==='spatial'?spatial:planar,scale,twist,options)
 if(!body.volume.solidGeometryCertified)throw Error(`Native body unproved: ${mode}/${count}`)
 const model=body.model,text=exportDirectStepV5(model).text
 writeFileSync(resolve(root,file),text)
 let exactWork=0
 const nativeExactUses=model.faces.flatMap((face,faceId)=>model.loops[face.outer]!.coedges.map((c,index)=>{
  const report=inspectSweepCoedgeExact(face.surface,model.edges[c.edge]!.curve,c.pcurve,{reversed:c.reversed,maxWork:1000000-exactWork})
  exactWork+=report.work
  return {face:faceId,wire:face.outer,coedge:index,edge:c.edge,...report}
 }))
 cases.push({file,mode,stations:count,constructionBudget:{maxDeviation,strictRefusalBudget:strictRefusalBudget??null},boundaryContinuousBound:body.boundaryContinuousBound,boundaryErrorUpper:body.boundaryErrorUpper,faces:model.faces.length,solids:1,shells:1,capHoleFaces:0,capFaces:[],requireNativeSolid:true,
  nativeVolume:body.volume,nativeExactUses,exactWork,sourceShells:model.shells,
  edges:model.edges.length,faceLoops:model.faces.map(face=>({outer:model.loops[face.outer]!.coedges.map(c=>c.edge),holes:[]})),
  edgeCurves:model.edges.map(edge=>({curve:edge.curve,samples:[0,.25,.5,.75,1].map(u=>({u,point:evaluateNurbsCurve(edge.curve,u).point}))})),
  wallCoedges:model.faces.map(face=>model.loops[face.outer]!.coedges.map(c=>({edge:c.edge,reversed:c.reversed,pcurve:c.pcurve}))),
  wallSurfaces:model.faces.map(face=>face.surface),wallSamples:model.faces.map(face=>[0,.17,.5,.83,1].flatMap(u=>[0,.13,.5,.87,1].map(v=>{const e=evaluateNurbsSurface(face.surface,u,v);return {u,v,point:e.point,du:e.du,dv:e.dv}}))),
  surfaceToleranceMm:1e-8,relativeVolumeTolerance:1e-7,sha256:sha(text),expectedVolume:null})
}
writeFileSync(resolve(root,'manifest.json'),JSON.stringify({schema:'sweep-external-step/2',units:'mm',selectionCatalogSha256:sha(catalogBytes),artifactProvenance:sweepStepArtifactProvenance(),exporterSha256:sha(readFileSync(new URL(import.meta.url))),cases},null,2)+'\n')
console.log(root)
