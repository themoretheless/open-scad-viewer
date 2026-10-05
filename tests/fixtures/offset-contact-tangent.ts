import type {NurbsSurface} from '../../src/services/nurbsSurface'
import {evaluateNurbsSurfaceOffset,type OffsetContactBandOptions} from '../../src/services/nurbsSurfaceOffset'
/** Numerical tube proposal; the query independently certifies its interval. */
export function curvedTangentFixture(rotated=false):{options:OffsetContactBandOptions;tangents:number[][]}{
 const a:NurbsSurface={degreeU:2,degreeV:1,knotsU:[0,0,0,1,1,1],knotsV:[0,0,1,1],controlPoints:[[3,0],[3,3],[0,3]].map(p=>[[p[0],p[1],0],[p[0],p[1],5]]),weights:[1,Math.SQRT1_2,1].map(w=>[w,w]),periodicU:false,periodicV:false}
 const b:NurbsSurface={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[2,0,0],[-3,0,5]],[[2,4,0],[-3,4,5]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false}
 const x=2+.2*Math.SQRT2-.5,y=Math.sqrt(3.2**2-x*x);let u=.5
 for(let i=0;i<8;i++){const e=evaluateNurbsSurfaceOffset(a,[u,.1],.2);u-=(e.point[0]-x)/e.du[0]}
 if(rotated)for(const s of [a,b])for(const row of s.controlPoints)for(const p of row){const [x,y,z]=p;p[0]=z+17;p[1]=x-9;p[2]=y+23}
 const options:OffsetContactBandOptions={a,b,distances:[.2,.2],fixedAxis:1,fixedInterval:[.099999,.100001],firstOther:[u-1e-4,u+1e-4],secondDomain:[[y/4-1e-4,y/4+1e-4],[(.5-.2/Math.SQRT2)/5-1e-4,(.5-.2/Math.SQRT2)/5+1e-4]],maxSpans:2}
 const tangents=[.099999,.1,.100001].map(t=>{const x=2+.2*Math.SQRT2-5*t,y=Math.sqrt(3.2**2-x*x);return rotated?[5,-5,5*x/y]:[-5,5*x/y,5]})
 return {options,tangents}
}
