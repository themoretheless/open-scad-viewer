import type {OriginalPatchContactsQuery,OriginalPatchContactsReport} from '../../src/services/originalPatchContactsProtocol'
import {originalPatchAlgebraicBoundaryFixture} from './original-patch-fixtures.ts'
type Case={name:string;query:OriginalPatchContactsQuery;check:(r:OriginalPatchContactsReport)=>void}
function require(x:unknown,message:string):asserts x{if(!x)throw new Error(message)}
function work(r:OriginalPatchContactsReport){require(r.exactWork<=r.maxExactWork,'Exceeded work budget');require(r.scalarProofs.reduce((sum,p)=>sum+p.workUsed,0)===r.exactWork,'Auxiliary predicate work missing from report')}
function present(r:OriginalPatchContactsReport,uv:[number,number]){
 work(r);require(r.status==='present'&&r.curvePatchCoverageComplete&&r.contacts.length===1,'Expected one complete boundary contact')
 const c=r.contacts[0]!;require(c.uvMethod==='original-polynomial-common-root','Wrong proof method');require(c.multiplicities[0]===2,'Lost original plane root order')
 for(let k=0;k<2;k++){const b=c.parameterBox[k+1]!;require(b[0]<=uv[k]!&&uv[k]!<=b[1],'Analytic edge UV outside box')}
 require(c.parameterBox[0][0]<c.parameterBox[0][1],'Expected non-singleton scalar root')
}
export const commonRootPatchCases:Case[]=[]
for(let edge=0;edge<6;edge++){
 const q=originalPatchAlgebraicBoundaryFixture(true),y=[1,0,-1,-2]
 q.curve.controlPoints=[1,-1,0,4].map((x,i)=>{
  const a=y[i]!,ab=edge===0?[a,.5]:edge===1?[1+a,.5]:edge===2?[.5,a]:edge===3?[.5,1+a]:edge===4?[a,a]:[1+a,1+a]
  return[x,...ab]
 })
 const uv:[number,number]=edge===0?[-2,10+4/3]:edge===1?[2,11.5]:edge===2?[-1,10]:edge===3?[-6/7,14]:edge===4?[-2,10]:[2,14]
 commonRootPatchCases.push({name:`polynomial-common-root-edge-${edge}`,query:q,check:r=>present(r,uv)})
}
{
 const q=originalPatchAlgebraicBoundaryFixture(true)
 q.curve={degree:4,knots:[0,0,0,0,0,1,1,1,1,1],controlPoints:[3,3,1,-3,3].map((x,i)=>[x,[-3,-3,-2,0,3][i]!, .5]),weights:[1,1,1,1,1],periodic:false}
 commonRootPatchCases.push({name:'nonproportional-irrational-common-root',query:q,check:r=>{present(r,[-2,10+4/3]);const c=r.contacts[0]!,t=Math.sqrt(.5);require(c.parameterBox[0][0]<=t&&t<=c.parameterBox[0][1],'Irrational root outside source box');require(JSON.stringify(c.positionBounds)==='[[0,0],[0,0],[0.5,0.5]]','Lost exact edge position')}})
}
{
 const q=originalPatchAlgebraicBoundaryFixture(true),o=[2,3,5],u=[1,2,3],v=[-1,1,2],n=[1,-5,3]
 q.patch.controlPoints=[0,1].map(i=>[0,1].map(j=>o.map((x,k)=>x+i*u[k]!+j*v[k]!)))
 q.curve.controlPoints=[1,-1,0,4].map((x,i)=>o.map((b,k)=>b+[1,0,-1,-2][i]!*u[k]!+.5*v[k]!+x*n[k]!))
 commonRootPatchCases.push({name:'skew-original-patch-common-root',query:q,check:r=>{present(r,[-2,10+4/3]);for(const [k,x] of [1.5,3.5,6].entries()){const b=r.contacts[0]!.positionBounds[k]!;require(b[0]<=x&&x<=b[1],'Skew boundary position outside box')}}})
}
{
 const q=originalPatchAlgebraicBoundaryFixture(true);for(const p of q.curve.controlPoints)p[1]-=2**-40
 commonRootPatchCases.push({name:'nearby-distinct-edge-root-refuses-contact',query:q,check:r=>{work(r);require(r.status==='absent-on-patch'&&r.contacts.length===0&&r.curvePatchCoverageComplete,'Exact sign did not classify exterior root')}})
}
{
 const q=originalPatchAlgebraicBoundaryFixture(true);q.maxExactWork=1
 commonRootPatchCases.push({name:'common-root-work-exhaustion-refuses-contact',query:q,check:r=>{work(r);require(r.status==='unresolved'&&r.contacts.length===0&&!r.curvePatchCoverageComplete,'Budget exhaustion admitted contact')}})
}
for(const reflected of [false,true])for(let edge=0;edge<4;edge++)for(const inward of [false,true]){
 const q=originalPatchAlgebraicBoundaryFixture(true),delta=inward?2**-40:-(2**-40)
 if(reflected)for(const row of q.patch.controlPoints)for(const p of row)p[1]=-p[1]
 q.curve.controlPoints=[1,-1,0,4].map((x,i)=>{
  const y=[1,0,-1,-2][i]!,coordinate=edge%2===0?y+delta:1+y-delta
  const a=edge<2?coordinate:.5,b=edge<2?.5:coordinate
  return[x,reflected?-a:a,b]
 })
 commonRootPatchCases.push({name:`coordinate-sign-edge-${edge}-inward-${inward}-reflected-${reflected}`,query:q,check:r=>{
  work(r);require(r.curvePatchCoverageComplete,'Missing sign coverage')
  require(r.status===(inward?'present':'absent-on-patch'),'Wrong exact side classification')
  require(r.contacts.length===(inward?1:0),'Wrong side contact count')
  if(inward)require(r.contacts[0]!.uvMethod==='original-polynomial-coordinate-sign','Missing coordinate-sign proof method')
 }})
}
for(const inward of [false,true]){
 const q=originalPatchAlgebraicBoundaryFixture(true),delta=inward?2**-40:-(2**-40)
 q.curve={degree:4,knots:[0,0,0,0,0,1,1,1,1,1],controlPoints:[3,3,1,-3,3].map((x,i)=>[x,[-3,-3,-2,0,3][i]!+delta,.5]),weights:[1,1,1,1,1],periodic:false}
 commonRootPatchCases.push({name:`irrational-coordinate-sign-inward-${inward}`,query:q,check:r=>{
  work(r);require(r.curvePatchCoverageComplete&&r.status===(inward?'present':'absent-on-patch'),'Wrong irrational side classification')
  require(r.contacts.length===(inward?1:0),'Wrong irrational contact count')
  if(inward){const c=r.contacts[0]!,t=Math.sqrt(.5);require(c.uvMethod==='original-polynomial-coordinate-sign','Missing irrational sign proof');require(c.parameterBox[0][0]<=t&&t<=c.parameterBox[0][1],'Lost irrational source root')}
 }})
}
