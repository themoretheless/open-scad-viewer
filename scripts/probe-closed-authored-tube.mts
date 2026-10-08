import {circleNurbsCurve} from '../src/services/nurbsConstructors'
import {createProgressiveBrepProfileBody} from '../src/services/geometry/brep'
import {inspectSweepVolume,DEFAULT_SWEEP_VOLUME_BUDGETS} from '../src/services/nurbsSweepEmbedding'

const path=circleNurbsCurve([0,0,0],[0,0,1],4)
const scalar=(value:number)=>({degree:1,knots:[0,0,1,1],values:[value,value],weights:[1,1]})
const frameAxis={degree:path.degree,knots:path.knots,weights:path.weights,
 values:path.controlPoints.map(p=>[-p[1]!/4,p[0]!/4,0] as [number,number,number])}
const frameNormal={degree:1,knots:[0,0,1,1],weights:[1,1],
 values:[[0,0,1],[0,0,1]] as [number,number,number][]}
const body=createProgressiveBrepProfileBody([[circleNurbsCurve([4,0,0],[0,1,0],.2)]],path,
 scalar(1),scalar(0),{orientation:'authored',normal:[0,0,1],frameAxis,frameNormal,
 initialSections:17,maxSections:65,maxDeviation:2,
 ...(process.argv.includes('--arc-length')?{spacing:'arc_length' as const,lengthTolerance:.001,lengthMaxCells:100000}:{})})
console.log(JSON.stringify({stage:'construction',faces:body.model.faces.length,
 boundaryContinuousBound:body.boundaryContinuousBound,boundaryErrorUpper:body.boundaryErrorUpper,
 retainedWalls:body.retainedWalls,retainedCaps:body.retainedCaps,report:body.approximation.report}))
console.log(JSON.stringify({stage:'volume',report:inspectSweepVolume(body.model,[],DEFAULT_SWEEP_VOLUME_BUDGETS)}))
