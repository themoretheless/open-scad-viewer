import type {OriginalQuadricCarrierQuery,OriginalQuadricCarrierReport} from '../../src/services/originalQuadricCarrierProtocol'
type Case={name:string;query:OriginalQuadricCarrierQuery;check:(r:OriginalQuadricCarrierReport)=>void}
function require(x:unknown,message:string):asserts x{if(!x)throw new Error(message)}
function base():OriginalQuadricCarrierQuery{return{curve:{degree:1,knots:[-2,-2,5,5],controlPoints:[[-2,0,0],[2,0,0]],weights:[1,1],periodic:false},quadric:[[1,0,0,0],[0,1,0,0],[0,0,1,0],[0,0,0,-1]],constantProducts:[],tolerance:1e-9,maxSpans:10,maxExactWork:100000}}
function complete(r:OriginalQuadricCarrierReport,parameters:number[],orders:number[]){
 require(r.curveCarrierCoverageComplete,'Incomplete carrier coverage');require(r.roots.length===parameters.length,'Wrong root count')
 require(r.status===(parameters.length?'present-on-carrier':'absent-on-carrier'),'Wrong carrier status')
 require(!r.finiteSurfaceUvAdmission&&!r.bodyAdmission&&!r.trimAdmission&&!r.allSurfaceBranchesEnumerated,'Carrier promoted to surface/Body admission')
 for(let i=0;i<parameters.length;i++){const root=r.roots[i]!,t=parameters[i]!;require(root.parameter[0]<=t&&t<=root.parameter[1],'Natural parameter outside root box');require(root.multiplicities[0]===orders[i],'Wrong implicit root order')}
}
function unresolved(r:OriginalQuadricCarrierReport){require(r.status==='unresolved'&&!r.curveCarrierCoverageComplete&&r.roots.length===0&&r.unresolvedSpans.length>0,'Missing explicit unresolved carrier coverage')}
export const quadricCarrierCases:Case[]=[]
quadricCarrierCases.push({name:'quadric-sphere-transverse',query:base(),check:r=>complete(r,[-.25,3.25],[1,1])})
{
 const q=base();for(const p of q.curve.controlPoints)p[1]=1
 quadricCarrierCases.push({name:'quadric-sphere-tangent-order-two',query:q,check:r=>complete(r,[1.5],[2])})
}
{
 const q=base();for(const p of q.curve.controlPoints)p[1]=2
 quadricCarrierCases.push({name:'quadric-sphere-exterior',query:q,check:r=>complete(r,[],[])})
}
{
 const q=base();q.curve.weights=[1,3]
 quadricCarrierCases.push({name:'quadric-original-rational-source',query:q,check:r=>complete(r,[-1.3,1.5],[1,1])})
}
{
 const q=base();q.curve={degree:3,knots:[-3,-2,-1,0,1,2,3,4],controlPoints:[-3,-1,1,3].map(x=>[x,0,0]),weights:[1,1,1,1],periodic:false};q.quadric[3][3]=-.25;q.maxExactWork=1000000
 quadricCarrierCases.push({name:'quadric-unclamped-original-span',query:q,check:r=>complete(r,[.25,.75],[1,1])})
}
{
 const q=base();q.curve={degree:2,knots:[0,0,0,1,1,1],controlPoints:[[75,0,0],[75,100,0],[-21,72,0]],weights:[5,3,5],periodic:false};q.quadric[3][3]=-5625
 quadricCarrierCases.push({name:'quadric-exact-circle-coincidence',query:q,check:r=>{unresolved(r);require(r.scalarProofs[0]!.status==='coincident','Lost exact circle identity')}})
}
for(const rounded of [false,true]){
 const q=base();q.curve.controlPoints=[[1.1,0,0],[1.1,0,0]];q.quadric[3][3]=rounded?-(1.1*1.1):0;q.constantProducts=rounded?[]:[[1.1,-1.1]]
 quadricCarrierCases.push({name:`quadric-radius-product-rounded-${rounded}`,query:q,check:r=>{if(rounded)complete(r,[],[]);else{unresolved(r);require(r.scalarProofs[0]!.status==='coincident','Original radius product lost')}}})
}
{
 const q=base();q.curve={degree:25,knots:[...Array(26).fill(0),...Array(26).fill(1)],controlPoints:Array.from({length:26},(_,i)=>[i===25?1:0,0,0]),weights:Array(26).fill(1),periodic:false};q.quadric=[[1,0,0,0],[0,0,0,0],[0,0,0,0],[0,0,0,0]];q.maxExactWork=1000000
 quadricCarrierCases.push({name:'quadric-original-degree-50-order',query:q,check:r=>complete(r,[0],[50])})
}
function knot(){const q=base();q.curve={degree:2,knots:[0,0,0,1,1,2,2,2],controlPoints:[2,1,1,1.5,2].map(x=>[x,0,0]),weights:[1,1,1,1,1],periodic:false};return q}
quadricCarrierCases.push({name:'quadric-two-one-sided-knot-orders',query:knot(),check:r=>{complete(r,[1],[2]);require(JSON.stringify(r.roots[0]!.multiplicities)==='[2,1]','One-sided orders collapsed')}})
{
 const q=base();q.maxExactWork=0
 quadricCarrierCases.push({name:'quadric-zero-exact-work',query:q,check:unresolved})
}
{
 const q=knot();q.maxSpans=1
 quadricCarrierCases.push({name:'quadric-incomplete-span-budget',query:q,check:r=>{require(r.status==='present-on-carrier'&&!r.curveCarrierCoverageComplete&&r.unresolvedSpans.includes(4),'Partial carrier presence hid missing span')}})
}
{
 const q=base();q.quadric[2][2]=0;q.curve.controlPoints=[[-2,0,100],[2,0,-100]]
 quadricCarrierCases.push({name:'quadric-cylinder-carrier',query:q,check:r=>complete(r,[-.25,3.25],[1,1])})
}
{
 const q=base();q.quadric=[[1,2,0,0],[0,1,0,0],[0,0,0,0],[0,0,0,-1]];q.curve.controlPoints=[[-2,.5,100],[2,.5,-100]]
 quadricCarrierCases.push({name:'quadric-nonsymmetric-cross-terms',query:q,check:r=>complete(r,[-1.125,2.375],[1,1])})
}
