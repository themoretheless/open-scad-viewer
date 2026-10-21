import type {NurbsCurve} from '../../src/services/nurbsCurve'
export const bezierCurve=(controlPoints:number[][]):NurbsCurve=>({degree:controlPoints.length-1,knots:[...Array(controlPoints.length).fill(0),...Array(controlPoints.length).fill(1)],controlPoints,weights:Array(controlPoints.length).fill(1),periodic:false})
export const bezierLine=(a:number[],b:number[])=>bezierCurve([a,b])
export const nonlinearFirst=bezierCurve([[0,0],[0,0],[10,0],[30,0],[60,1]])
export const nonlinearSecond=bezierCurve([[0,0],[0,0],[0,0],[3,0],[12,0],[30,0],[60,1]])
export const nonlinearWire=(c:NurbsCurve)=>[c,bezierLine([60,1],[60,10]),bezierLine([60,10],[0,10]),bezierLine([0,10],[0,0])]
export const fourthOrderContact=bezierCurve([[0,1],[1,-1],[2,1],[3,-1],[4,1]])
export const contactLine=bezierLine([0,0],[4,0])
export const degreeEightLoop=bezierCurve(Array.from({length:9},(_,i)=>{const c2=i*(i-1)/2,c3=i*(i-1)*(i-2)/6;return [10.5*i-3*c2,10.5*i-16*c2+8*c3]}))
export const exactCusp=bezierCurve([[2,4],[0,-2],[-2,0],[4,2]])
export const cornerWire=[bezierLine([0,0],[10,0]),bezierLine([10,0],[10,10])]
export const squareWire=[bezierLine([0,0],[10,0]),bezierLine([10,0],[10,10]),bezierLine([10,10],[0,10]),bezierLine([0,10],[0,0])]
const loopPoles=[[0,0],[13440,2520],[13440,-8400],[0,7560]]
export const nonprimitiveLoop={...bezierCurve(Array.from({length:5},(_,i)=>[0,1].map(axis=>((i<4?(4-i)*loopPoles[i][axis]:0)+(i>0?2*i*loopPoles[i-1][axis]:0))/(4+i)))),weights:[4,5,6,7,8]}
export const quarticContactFirst=bezierCurve([[0,0],[1,0],[2,2],[3,1],[4,13]])
export const quarticContactSecond=bezierCurve([[0,0],[1,0],[2,1],[3,3],[4,7]])
