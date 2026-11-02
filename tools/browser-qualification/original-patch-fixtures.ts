import type {OriginalPatchContactsQuery} from '../../src/services/originalPatchContactsProtocol'
export function originalPatchFixture(order:2|4):OriginalPatchContactsQuery{return{
 curve:{degree:order,knots:[...Array(order+1).fill(0),...Array(order+1).fill(1)],controlPoints:Array.from({length:order+1},(_,i)=>[i%2===0?.1:1.1,i/order,i%2===0?.1:0]),weights:Array(order+1).fill(1),periodic:false},
 patch:{degreeU:1,degreeV:1,knotsU:[-2,-2,2,2],knotsV:[10,10,14,14],controlPoints:[[[.1,0,0],[.1,1,0]],[[1.1,0,.1],[1.1,1,.1]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false},tolerance:1e-9,maxSpans:10,maxExactWork:100000
}}
export function originalPatchKnotFixture():OriginalPatchContactsQuery{const q=originalPatchFixture(2);q.curve={degree:2,knots:[0,0,0,1,1,2,2,2],controlPoints:[1,0,0,.5,1].map((x,i)=>[x,.25+i*.125,1]),weights:Array(5).fill(1),periodic:false};q.patch={degreeU:1,degreeV:1,knotsU:[0,0,1,1],knotsV:[0,0,1,1],controlPoints:[[[0,0,0],[0,0,2]],[[0,1,0],[0,1,2]]],weights:[[1,1],[1,1]],periodicU:false,periodicV:false};return q}
export function originalPatchAlgebraicBoundaryFixture(commonRoot=false):OriginalPatchContactsQuery{
 const q=originalPatchFixture(2)
 q.patch.controlPoints=[[[0,0,0],[0,0,1]],[[0,1,0],[0,1,1]]];q.patch.weights=[[1,2],[3,5]]
 q.curve=commonRoot?{degree:3,knots:[0,0,0,0,1,1,1,1],controlPoints:[[1,1,.5],[-1,0,.5],[0,-1,.5],[4,-2,.5]],weights:[1,1,1,1],periodic:false}:{degree:2,knots:[0,0,0,1,1,1],controlPoints:[[-.5,-.5,.5],[-.5,-.5,.5],[.5,.5,.5]],weights:[1,1,1],periodic:false}
 return q
}
