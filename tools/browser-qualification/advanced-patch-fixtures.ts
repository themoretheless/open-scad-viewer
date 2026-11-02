import type {OriginalPatchContactsQuery,OriginalPatchContactsReport} from '../../src/services/originalPatchContactsProtocol'
import {originalPatchFixture} from './original-patch-fixtures.ts'
type Case={name:string;query:OriginalPatchContactsQuery;check:(r:OriginalPatchContactsReport)=>void}
function require(value:unknown,message:string):asserts value{if(!value)throw new Error(message)}
function present(r:OriginalPatchContactsReport,method:string,order:number,uv?:[number,number]){
 require(r.status==='present'&&r.curvePatchCoverageComplete,'Missing complete contact');require(r.contacts.length===1,'Wrong contact count')
 const c=r.contacts[0]!;require(c.uvMethod===method,'Wrong UV proof method');require(c.multiplicities.length===1&&c.multiplicities[0]===order,'Wrong contact order')
 if(uv)for(let i=0;i<2;i++){const b=c.parameterBox[i+1]!;require(b[0]<=uv[i]!&&uv[i]!<=b[1],'Analytic UV outside original box')}
}
function broad(r:OriginalPatchContactsReport){const c=r.contacts[0]!;require(JSON.stringify(c.parameterBox.slice(1))==='[[-2,2],[10,14]]','Global enclosure must expose the whole chart')}
function flat(q:OriginalPatchContactsQuery){q.patch.controlPoints=[[[0,0,0],[0,0,1]],[[0,1,0],[0,1,1]]];q.patch.weights=[[1,2],[3,5]];return q}
export const advancedPatchCases:Case[]=[]
{
 const q=originalPatchFixture(2);q.patch.weights=[[1,3],[2,6]]
 advancedPatchCases.push({name:'separable-original-target-clocks',query:q,check:r=>present(r,'separable-projective',2,[-2/3,11])})
}
for(const order of [2,4] as const){
 const q=originalPatchFixture(order);q.patch.weights=order===2?[[1,1],[1,2]]:[[1,1],[2,2.5]]
 const u=order===2?Math.SQRT2-1:1/(1+Math.sqrt(5)),v=order===2?u:Math.SQRT2/(Math.SQRT2+Math.sqrt(2.5))
 advancedPatchCases.push({name:`coupled-refined-uv-order-${order}`,query:q,check:r=>{present(r,'injective-bilinear-krawczyk',order,[-2+4*u,10+4*v]);require(r.targetWeightsSeparable===false&&r.targetUvInjectivityProven,'Coupled inverse lacks its global proof')}})
}
{
 const q=originalPatchFixture(2);q.patch.weights=[[1,1],[1,1e12]]
 advancedPatchCases.push({name:'coupled-global-broad-uv',query:q,check:r=>{present(r,'positive-bilinear-global-inverse',2);broad(r)}})
}
{
 const q=originalPatchFixture(2);q.patch.weights=[[1,2],[3,5]];for(const p of q.curve.controlPoints)p[1]=0
 advancedPatchCases.push({name:'exact-coupled-original-edge',query:q,check:r=>present(r,'exact-boundary-projective',2,[-1,10])})
}
{
 const q=flat(originalPatchFixture(2));q.curve.controlPoints=[1,-.5,.25].map(x=>[x,.5,0]);q.curve.weights=[1,2,4]
 advancedPatchCases.push({name:'exact-weighted-source-edge',query:q,check:r=>present(r,'exact-boundary-projective',2,[-1,10])})
}
{
 const q=flat(originalPatchFixture(2));q.curve.degree=25;q.curve.knots=[...Array(26).fill(0),...Array(26).fill(1)];q.curve.controlPoints=Array.from({length:26},(_,i)=>[i%2===0?1:-1,0,0]);q.curve.weights=Array(26).fill(1);q.maxExactWork=1000000
 advancedPatchCases.push({name:'exact-degree-25-corner',query:q,check:r=>{present(r,'exact-boundary-projective',25,[-2,10]);require(r.contacts[0]!.parameterBox[0][0]===.5&&r.contacts[0]!.parameterBox[0][1]===.5,'Source root lost singleton ownership')}})
}
{
 const q=originalPatchFixture(2);q.curve.knots=[.1,.1,.1,3.1,3.1,3.1];for(const p of q.curve.controlPoints)p[1]=0
 advancedPatchCases.push({name:'exact-nonbinary-natural-source-t',query:q,check:r=>{present(r,'exact-boundary-projective',2,[0,10]);const b=r.contacts[0]!.parameterBox[0];require(b[0]<b[1]&&b[0]<=1.6&&1.6<=b[1],'Nonbinary source parameter was rounded into a point')}})
}
for(const sign of [-1,1]){
 const q=originalPatchFixture(2),delta=sign*Number.MIN_VALUE;for(const p of q.curve.controlPoints)p[1]=delta
 advancedPatchCases.push({name:sign<0?'exact-subnormal-exterior':'exact-subnormal-interior',query:q,check:r=>{if(sign<0){require(r.status==='absent-on-patch'&&r.curvePatchCoverageComplete&&r.contacts.length===0,'Tiny exterior point snapped onto patch')}else{present(r,'positive-bilinear-global-inverse',2);broad(r);require(r.contacts[0]!.positionBounds[1][0]===delta&&r.contacts[0]!.positionBounds[1][1]===delta,'Tiny interior point snapped onto edge')}}})
}
for(const [i,[a,b,uv]] of ([ [0,.5,[-2,10+4/3]],[1,.5,[2,11.5]],[.5,0,[-1,10]],[.5,1,[-6/7,14]],[0,0,[-2,10]],[1,1,[2,14]] ] as [number,number,[number,number]][]).entries()){
 const q=flat(originalPatchFixture(2));q.curve.controlPoints=[.25,-.25,.25].map(x=>[x,a,b])
 advancedPatchCases.push({name:`exact-flat-edge-corner-${i}`,query:q,check:r=>present(r,'exact-boundary-projective',2,uv)})
}
