import type {OffsetEnvelopeOptions} from '../../src/services/nurbsSurfaceOffset'
export function planeEnvelopeFixture():OffsetEnvelopeOptions {
 const a={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,1,0]],[[1,0,0],[1,1,0]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false}
 const b=structuredClone(a);for(const row of b.controlPoints)for(const p of row){const z=p[1];p[1]=.5;p[2]=z}
 return {a,b,distances:[.2,.2],fixedAxis:0,fixedInterval:[.35,.39],firstOther:[.25,.35],secondDomain:[[.30,.44],[.15,.25]],maxSpans:2,maxCells:255}
}
