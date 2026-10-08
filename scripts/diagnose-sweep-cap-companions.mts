import {writeFileSync} from 'node:fs'
import {createHash} from 'node:crypto'
import {bezierNurbsCurve,circleNurbsCurve} from '../src/services/nurbsConstructors'
import {reverseNurbsCurve} from '../src/services/nurbsCurve'
import {createProgressiveBrepProfileBody} from '../src/services/geometry/brep'
import {inspectSweepVolume,inspectSweepEmbedding,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'
import {sweepStepArtifactProvenance} from './sweep-step-provenance.mjs'
const scalar=(values:number[])=>({degree:1,knots:[0,0,1,1],values,weights:[1,1]})
const cases=[]
for(const mode of ['oblique-rmf','curved-frenet','curved-rmf'] as const){
 if(process.argv.some(a=>a.startsWith('--mode='))&&!process.argv.includes('--mode='+mode))continue
 const oblique=mode==='oblique-rmf'
 const axis:[number,number,number]=oblique?[3,4,0]:[1,0,0]
 const loops=[[circleNurbsCurve([0,0,0],axis,.1)],[reverseNurbsCurve(circleNurbsCurve([0,0,0],axis,.05))]]
 const path=bezierNurbsCurve(oblique?[[0,0,0],[3,4,0]]:[[0,0,0],[.5,0,0],[1,1,0]])
 for(const corrected of [false,true]){
  const body=createProgressiveBrepProfileBody(loops,path,scalar(oblique?[1,2]:[1,1]),scalar([0,0]),{
   normal:[0,0,1],orientation:mode==='curved-frenet'?'frenet':'rmf',initialSections:oblique?2:3,maxSections:oblique?2:129,maxDeviation:.01,
   ...(corrected?{capCorrection:{quantum:2**-40,tolerance:1e-9,maxWork:1000000}}:{}),
  })
  const caps=[body.model.faces.length-2,body.model.faces.length-1]
  if(process.argv.includes('--construction-only')){
   writeFileSync(process.argv[2]!.replace(/\.json$/,`-${mode}-${corrected?'corrected':'raw'}-construction.json`),JSON.stringify({artifactProvenance:sweepStepArtifactProvenance(),body},null,2)+'\n')
   console.log(JSON.stringify({mode,corrected,phase:'construction',boundaryContinuousBound:body.boundaryContinuousBound}));continue
  }
  const volume=inspectSweepVolume(body.model,caps,{...DEFAULT_SWEEP_VOLUME_BUDGETS,maxTrimPairs:10000,maxTrimCells:100000,maxTrimDomainCells:1000000,maxPairs:10000,maxCells:100000,maxDomainCells:1000000,cellsPerPair:1000,domainCellsPerPair:10000})
  const result={mode,corrected,faces:body.model.faces.length,modelSha256:createHash('sha256').update(JSON.stringify(body.model)).digest('hex'),boundaryContinuousBound:body.boundaryContinuousBound,boundaryErrorWithinBudget:body.boundaryErrorWithinBudget,boundaryErrorUpper:body.boundaryErrorUpper,capCorrectionErrorUpper:body.capCorrectionErrorUpper,volume}
  if(process.argv.includes('--details')){
   const embedding=inspectSweepEmbedding(body.model,caps,{...DEFAULT_SWEEP_VOLUME_BUDGETS,maxTrimPairs:10000,maxTrimCells:100000,maxTrimDomainCells:1000000,maxPairs:10000,maxCells:100000,maxDomainCells:1000000,cellsPerPair:1000,domainCellsPerPair:10000})
   writeFileSync(process.argv[2]!.replace(/\.json$/,`-${mode}-${corrected?'corrected':'raw'}-details.json`),JSON.stringify({artifactProvenance:sweepStepArtifactProvenance(),body,embedding},null,2)+'\n')
  }
  cases.push(result);console.log(JSON.stringify({mode,corrected,faces:result.faces,boundaryContinuousBound:result.boundaryContinuousBound,solid:volume.solidGeometryCertified}))
 }
}
writeFileSync(process.argv[2]!,JSON.stringify({artifactProvenance:sweepStepArtifactProvenance(),scope:'Exact legacy source inputs with and without explicit native cap correction; diagnostics only, no independent STEP or rendered UI claim.',cases},null,2)+'\n')
