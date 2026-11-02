import type {NurbsSurface} from '../../src/services/nurbsSurface'
type Case={axis?:0|1;name:string;surface:()=>NurbsSurface;parameters:[number,number];position:[number,number,number]}
function nonlinear():NurbsSurface{return{degreeU:2,degreeV:1,knotsU:[0,0,0,1,2,3,3,3],knotsV:[0,0,1,1],controlPoints:[[-1,-1],[0,0],[1,1],[1,-1],[-1,1]].map(([x,y])=>[[x!,y!,0],[x!,y!,2]]),weights:Array.from({length:5},()=>[1,1]),periodicU:false,periodicV:false}}
function loop():NurbsSurface{return{degreeU:3,degreeV:1,knotsU:[0,0,0,0,1,1,1,1],knotsV:[0,0,1,1],controlPoints:[[9/16,-9/32],[-7/16,13/32],[-7/16,-13/32],[9/16,9/32]].map(([x,y])=>[[x!,y!,0],[x!,y!,2]]),weights:Array.from({length:4},()=>[1,1]),periodicU:false,periodicV:false}}
const x=(11-4*Math.sqrt(6))/25,t=(1+Math.sqrt(6))/5
export const nonlinearSurfaceCases:Case[]=[
 {name:'original-non-bezier-spans',surface:nonlinear,parameters:[(2-Math.sqrt(2-2*x))/3,(2+t)/3],position:[x,x,1]},
 {name:'within-single-cubic-span',surface:loop,parameters:[.25,.75],position:[0,0,1]},
 {name:'exact-oblique-source-plane',surface:()=>{const s=loop();for(const row of s.controlPoints)for(const p of row)p[2]!+=p[0]!+2*p[1]!;return s},parameters:[.25,.75],position:[0,0,1]},
 {name:'original-rational-surface-bounds',surface:()=>{const s=nonlinear();s.knotsU=[0,0,0,1,1,2,2,2];s.degreeV=2;s.knotsV=[0,0,0,1,1,1];s.controlPoints=s.controlPoints.map(row=>[0,1,2].map(z=>[row[0]![0]!+z,row[0]![1]!+z,z]));s.weights=Array.from({length:5},()=>[1,2,2]);return s},parameters:[5/18,5/6],position:[79/63,79/63,8/7]},
]

nonlinearSurfaceCases.push({name:'original-rational-v-axis',axis:1,surface:()=>{const s=nonlinearSurfaceCases[3]!.surface();s.controlPoints=Array.from({length:3},(_,j)=>Array.from({length:5},(_,i)=>s.controlPoints[i]![j]!));s.weights=Array.from({length:3},(_,j)=>Array.from({length:5},(_,i)=>s.weights[i]![j]!));[s.degreeU,s.degreeV]=[s.degreeV,s.degreeU];[s.knotsU,s.knotsV]=[s.knotsV,s.knotsU];return s},parameters:[5/18,5/6],position:[79/63,79/63,8/7]})

export const spatialSurfaceCases=([0,1] as const).map(axis=>({axis,name:`original-rational-spatial-axis-${axis}`,surface:():NurbsSurface=>{
 const s=loop();s.weights=s.weights.map(()=>[1,2]);
 s.controlPoints.forEach((row,i)=>{const z=i===3?.25:0;row[0]![2]=z;row[1]![2]=2+z})
 if(axis===1){s.controlPoints=[0,1].map(j=>s.controlPoints.map(row=>row[j]!));s.weights=[0,1].map(j=>s.weights.map(row=>row[j]!));[s.degreeU,s.degreeV]=[s.degreeV,s.degreeU];[s.knotsU,s.knotsV]=[s.knotsV,s.knotsU]}
 return s
}}))

export function isoTargetFixture(axis:0|1){return{
 source:spatialSurfaceCases[axis]!.surface(),axis,parameter:.5,
 target:{degreeU:1,degreeV:1,knotsU:[-2,-2,2,2],knotsV:[10,10,14,14],controlPoints:[[[0,-1,0],[0,-1,2]],[[0,1,0],[0,1,2]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false},
 window:[[0,1],[-2,2],[10,14]] as [[number,number],[number,number],[number,number]],maxCells:10000
}}

export function offCenterSpatialSurface(){const s=spatialSurfaceCases[0]!.surface();s.weights=s.weights.map(()=>[1,1]);s.controlPoints.forEach((row,i)=>{row[0]![2]=i*.375;row[1]![2]=i*.375+1});return s}
